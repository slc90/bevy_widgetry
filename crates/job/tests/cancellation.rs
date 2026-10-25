#![allow(clippy::disallowed_macros)]

use bevy::tasks::{IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use event_listener::Event;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

fn manager() -> JobManager {
    IoTaskPool::get_or_init(TaskPool::new);
    JobManager::default()
}

fn gate() -> (Arc<Event>, impl std::future::Future<Output = ()> + Send) {
    let event = Arc::new(Event::new());
    let listener = event.listen();
    (event, listener)
}

#[test]
fn observing_cancellation_does_not_acknowledge_it_and_cleanup_keeps_stopping() {
    for acknowledge in [false, true] {
        let manager = manager();
        let (release, cleanup) = gate();
        let (started, running) = mpsc::channel();
        let (observed, stopped) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_io::<(), _, _, String>(JobOptions::default(), move |context| async move {
                assert!(!context.acknowledge_cancel());
                started.send(()).unwrap();
                context.cancelled().await;
                assert!(context.is_cancelled());
                if acknowledge {
                    assert!(context.acknowledge_cancel());
                }
                observed.send(()).unwrap();
                cleanup.await;
                Ok(())
            })
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::CancelRequested);
        stopped.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyStopping);
        assert!(matches!(
            manager.remove(id),
            Err(JobAccessError::NotFinished(JobState::Stopping))
        ));
        assert!(matches!(
            manager.subscribe::<()>(id),
            Err(JobAccessError::NotSubscribable(JobState::Stopping))
        ));
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Empty);
        release.notify(usize::MAX);
        let expected = if acknowledge {
            JobOutcome::Canceled
        } else {
            JobOutcome::Completed
        };
        assert!(
            matches!(block_on(receiver.recv()), Ok(JobMessage::Finished(value)) if value == expected)
        );
        let status = manager.status(id).unwrap();
        assert_eq!(status.outcome, Some(expected));
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
        assert_eq!(manager.status(id).unwrap(), status);
        manager.remove(id).unwrap();
        assert_eq!(manager.cancel(id), Err(JobAccessError::NotFound));
    }
}

#[test]
fn acknowledged_cancellation_cannot_mask_error_or_panic() {
    for panic in [false, true] {
        let manager = manager();
        let (started, running) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_io::<(), _, _, String>(JobOptions::default(), move |context| async move {
                started.send(()).unwrap();
                context.cancelled().await;
                assert!(context.acknowledge_cancel());
                if panic {
                    panic!("cleanup panic");
                }
                Err("cleanup failed".to_owned())
            })
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        manager.cancel(id).unwrap();
        let outcome = block_on(receiver.recv()).unwrap();
        if panic {
            assert!(
                matches!(outcome, JobMessage::Finished(JobOutcome::Failed(JobError::Panic(message))) if message == "cleanup panic")
            );
        } else {
            assert!(
                matches!(outcome, JobMessage::Finished(JobOutcome::Failed(JobError::Business(message))) if message == "cleanup failed")
            );
        }
        assert_eq!(manager.status(id).unwrap().state, JobState::Failed);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
    }
}

#[test]
fn cancellation_wakes_full_buffer_send_without_discarding_prior_items_or_auto_acknowledging() {
    for acknowledge in [false, true] {
        let manager = manager();
        let (started, waiting) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_io::<usize, _, _, String>(
                JobOptions {
                    buffer_capacity: Some(1),
                    ..Default::default()
                },
                move |context| async move {
                    context.send(1).await.map_err(|error| error.to_string())?;
                    let mut pending_send = Box::pin(context.send(2));
                    assert!(bevy::tasks::poll_once(&mut pending_send).await.is_none());
                    started.send(()).unwrap();
                    assert_eq!(pending_send.await, Err(SendError::Canceled));
                    if acknowledge {
                        assert!(context.acknowledge_cancel());
                    }
                    Ok(())
                },
            )
            .unwrap();
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        manager.cancel(id).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let expected = if acknowledge {
            JobOutcome::Canceled
        } else {
            JobOutcome::Completed
        };
        assert_eq!(manager.outcome(id).unwrap(), Some(expected.clone()));
        manager.remove(id).unwrap();
        assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(value)) if *value == 1));
        assert!(
            matches!(receiver.try_recv(), Ok(JobMessage::Finished(value)) if value == expected)
        );
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Closed);
    }
}

#[test]
fn false_acknowledgement_without_request_leaves_success_completed() {
    let manager = manager();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, String>(JobOptions::default(), |context| async move {
            assert!(!context.is_cancelled());
            assert!(!context.acknowledge_cancel());
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        block_on(receiver.recv()),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
}

#[test]
fn completion_races_with_cancellation_preserve_the_actual_unacknowledged_result() {
    for fail in [false, true] {
        for _ in 0..16 {
            let manager = manager();
            let (release, ready) = gate();
            let (started, running) = mpsc::channel();
            let (id, mut receiver) = manager
                .spawn_io::<(), _, _, String>(JobOptions::default(), move |_| async move {
                    started.send(()).unwrap();
                    ready.await;
                    if fail {
                        Err("natural failure".into())
                    } else {
                        Ok(())
                    }
                })
                .unwrap();
            running.recv_timeout(Duration::from_secs(5)).unwrap();
            let sync = Arc::new(std::sync::Barrier::new(2));
            let other = sync.clone();
            let cancel = manager.clone();
            let request = std::thread::spawn(move || {
                other.wait();
                cancel.cancel(id).unwrap()
            });
            sync.wait();
            release.notify(usize::MAX);
            let expected = if fail {
                JobOutcome::Failed(JobError::Business("natural failure".into()))
            } else {
                JobOutcome::Completed
            };
            assert!(
                matches!(block_on(receiver.recv()), Ok(JobMessage::Finished(value)) if value == expected)
            );
            assert!(matches!(
                request.join().unwrap(),
                CancelResult::CancelRequested | CancelResult::AlreadyFinished
            ));
            assert_eq!(manager.outcome(id).unwrap(), Some(expected));
            assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
        }
    }
}

#[test]
fn propagating_send_cancellation_as_a_top_level_error_is_failed() {
    let manager = manager();
    let (sent, waiting) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_io::<usize, _, _, SendError>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| async move {
                context.send(1).await?;
                let mut send = Box::pin(context.send(2));
                assert!(bevy::tasks::poll_once(&mut send).await.is_none());
                sent.send(()).unwrap();
                send.await?;
                Ok(())
            },
        )
        .unwrap();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    manager.cancel(id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !manager.status(id).unwrap().state.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(value)) if *value == 1));
    assert!(
        matches!(receiver.try_recv(), Ok(JobMessage::Finished(JobOutcome::Failed(JobError::Business(error)))) if error == "Canceled")
    );
}
