#![allow(clippy::disallowed_macros)]

use bevy::tasks::{IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use event_listener::Event;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

#[allow(clippy::unwrap_used)]
fn manager(capacity: usize) -> JobManager {
    IoTaskPool::get_or_init(TaskPool::new);
    JobManager::with_buffer_capacity(capacity).unwrap()
}

fn gate() -> (Arc<Event>, impl std::future::Future<Output = ()> + Send) {
    let event = Arc::new(Event::new());
    let listener = event.listen();
    (event, listener)
}

#[allow(clippy::unwrap_used, clippy::panic)]
fn item<T>(receiver: &mut JobReceiver<T>) -> Arc<T> {
    match block_on(receiver.recv()).unwrap() {
        JobMessage::Item(value) => value,
        JobMessage::Finished(outcome) => panic!("unexpected finish: {outcome:?}"),
    }
}

#[allow(clippy::unwrap_used)]
fn wait_finished(manager: &JobManager, id: JobId) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !manager.status(id).unwrap().state.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn independent_receivers_share_payload_and_new_subscriptions_skip_history() {
    #[derive(Debug)]
    struct Payload(usize);
    let manager = manager(4);
    let (start, started) = gate();
    let (more, next) = gate();
    let (sent, received) = mpsc::channel();
    let (id, mut first) = manager
        .spawn_io::<Payload, _, _, SendError>(JobOptions::default(), move |context| async move {
            started.await;
            context.send(Payload(1)).await?;
            sent.send(()).unwrap();
            next.await;
            context.send(Payload(2)).await?;
            Ok(())
        })
        .unwrap();
    let mut second = manager.subscribe::<Payload>(id).unwrap();
    assert!(matches!(
        manager.subscribe::<String>(id),
        Err(JobAccessError::TypeMismatch)
    ));
    start.notify(usize::MAX);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut late = manager.subscribe::<Payload>(id).unwrap();
    let a = item(&mut first);
    let b = item(&mut second);
    assert_eq!(a.0, 1);
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(late.try_recv().unwrap_err(), TryRecvError::Empty);
    more.notify(usize::MAX);
    for receiver in [&mut first, &mut second, &mut late] {
        assert_eq!(item(receiver).0, 2);
        assert!(matches!(
            block_on(receiver.recv()),
            Ok(JobMessage::Finished(JobOutcome::Completed))
        ));
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Closed);
    }
    assert!(matches!(
        manager.subscribe::<Payload>(id),
        Err(JobAccessError::NotSubscribable(JobState::Completed))
    ));
}

#[test]
fn full_buffer_backpressures_until_slow_receiver_is_dropped() {
    let manager = manager(8);
    let (start, started) = gate();
    let (sent, received) = mpsc::channel();
    let (id, mut fast) = manager
        .spawn_io::<usize, _, _, SendError>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| async move {
                started.await;
                context.send(1).await?;
                let mut send = Box::pin(context.send(2));
                assert!(bevy::tasks::poll_once(&mut send).await.is_none());
                sent.send(()).unwrap();
                send.await?;
                Ok(())
            },
        )
        .unwrap();
    let slow = manager.subscribe::<usize>(id).unwrap();
    start.notify(usize::MAX);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(*item(&mut fast), 1);
    assert_eq!(fast.try_recv().unwrap_err(), TryRecvError::Empty);
    assert_eq!(manager.status(id).unwrap().state, JobState::Running);
    drop(slow);
    assert_eq!(*item(&mut fast), 2);
    assert!(matches!(
        block_on(fast.recv()),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
}

#[test]
fn finished_does_not_need_buffer_space_and_removal_preserves_unread_failure() {
    let manager = manager(1);
    let (id, mut receiver) = manager
        .spawn_io::<usize, _, _, String>(JobOptions::default(), |context| async move {
            context.send(7).await.map_err(|error| error.to_string())?;
            Err("source failed".to_owned())
        })
        .unwrap();
    wait_finished(&manager, id);
    manager.remove(id).unwrap();
    assert_eq!(*item(&mut receiver), 7);
    assert!(
        matches!(block_on(receiver.recv()), Ok(JobMessage::Finished(JobOutcome::Failed(JobError::Business(error)))) if error == "source failed")
    );
    assert!(matches!(
        block_on(receiver.recv()),
        Err(TryRecvError::Closed)
    ));
}

#[test]
fn no_receivers_discards_items_without_blocking_or_failing_and_later_subscription_works() {
    let manager = manager(1);
    let (start, started) = gate();
    let (more, next) = gate();
    let (sent, received) = mpsc::channel();
    let (id, initial) = manager
        .spawn_io::<usize, _, _, SendError>(JobOptions::default(), move |context| async move {
            started.await;
            for value in 0..100 {
                context.send(value).await?;
            }
            sent.send(()).unwrap();
            next.await;
            context.send(100).await?;
            Ok(())
        })
        .unwrap();
    drop(initial);
    start.notify(usize::MAX);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut later = manager.subscribe::<usize>(id).unwrap();
    assert_eq!(later.try_recv().unwrap_err(), TryRecvError::Empty);
    more.notify(usize::MAX);
    assert_eq!(*item(&mut later), 100);
    assert!(matches!(
        block_on(later.recv()),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
}

#[test]
fn zero_capacity_is_rejected_in_defaults_and_per_job_options() {
    assert!(matches!(
        JobManager::with_buffer_capacity(0),
        Err(SubmitError::ZeroCapacity)
    ));
    let manager = manager(2);
    let result = manager.spawn_io::<(), _, _, String>(
        JobOptions {
            buffer_capacity: Some(0),
            ..Default::default()
        },
        |_| async { Ok(()) },
    );
    assert!(matches!(result, Err(SubmitError::ZeroCapacity)));
}

#[test]
fn business_progress_and_batches_keep_the_same_order_for_each_receiver() {
    #[derive(Debug, PartialEq)]
    enum Event {
        Progress(usize),
        Batch(Vec<u8>),
    }
    let manager = manager(2);
    let (start, started) = gate();
    let (id, mut a) = manager
        .spawn_io::<Event, _, _, SendError>(JobOptions::default(), move |context| async move {
            started.await;
            context.send(Event::Progress(0)).await?;
            context.send(Event::Batch(vec![42; 128 * 1024])).await?;
            context.send(Event::Progress(100)).await?;
            Ok(())
        })
        .unwrap();
    let mut b = manager.subscribe::<Event>(id).unwrap();
    start.notify(usize::MAX);
    for expected in [
        Event::Progress(0),
        Event::Batch(vec![42; 128 * 1024]),
        Event::Progress(100),
    ] {
        let first = item(&mut a);
        let second = item(&mut b);
        assert_eq!(*first, expected);
        assert!(Arc::ptr_eq(&first, &second));
    }
    assert!(matches!(
        a.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert!(matches!(
        b.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
}
