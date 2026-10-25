#![allow(clippy::disallowed_macros)]

use bevy::tasks::{IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use std::{
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

fn manager() -> JobManager {
    IoTaskPool::get_or_init(TaskPool::new);
    JobManager::default()
}

#[test]
fn named_thread_shares_results_and_is_joined_before_finished() {
    let manager = manager();
    let (start, started) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_thread::<usize, _, SendError>(
            JobOptions {
                label: Some("device-reader".into()),
                ..Default::default()
            },
            move |context| {
                assert_eq!(std::thread::current().name(), Some("device-reader"));
                start.send(()).unwrap();
                released.recv_timeout(Duration::from_secs(5)).unwrap();
                context.send_blocking(42)?;
                Ok(())
            },
        )
        .unwrap();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(manager.status(id).unwrap().kind, JobKind::Thread);
    let mut second = manager.subscribe::<usize>(id).unwrap();
    release.send(()).unwrap();
    assert!(matches!(receiver.recv_blocking(), Ok(JobMessage::Item(value)) if *value == 42));
    assert!(matches!(second.recv_blocking(), Ok(JobMessage::Item(value)) if *value == 42));
    assert!(matches!(
        receiver.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert!(matches!(
        second.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert_eq!(manager.status(id).unwrap().state, JobState::Completed);
}

#[test]
fn cooperative_thread_cancellation_preserves_unacknowledged_success_and_failures() {
    for mode in 0..4 {
        let manager = manager();
        let (start, started) = mpsc::channel();
        let (cleanup, cleaned) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_thread::<(), _, String>(JobOptions::default(), move |context| {
                start.send(()).unwrap();
                block_on(context.cancelled());
                if mode != 0 {
                    assert!(context.acknowledge_cancel());
                }
                cleanup.send(()).unwrap();
                released.recv_timeout(Duration::from_secs(5)).unwrap();
                match mode {
                    2 => Err("thread cleanup failed".into()),
                    3 => panic!("thread panic"),
                    _ => Ok(()),
                }
            })
            .unwrap();
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        manager.cancel(id).unwrap();
        cleaned.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyStopping);
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Empty);
        release.send(()).unwrap();
        let expected = match mode {
            0 => JobOutcome::Completed,
            1 => JobOutcome::Canceled,
            2 => JobOutcome::Failed(JobError::Business("thread cleanup failed".into())),
            _ => JobOutcome::Failed(JobError::Panic("thread panic".into())),
        };
        assert!(
            matches!(receiver.recv_blocking(), Ok(JobMessage::Finished(value)) if value == expected)
        );
        assert_eq!(manager.outcome(id).unwrap(), Some(expected));
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
    }
}

#[test]
fn blocking_send_is_cancelable_without_consuming_buffer() {
    let manager = manager();
    let (start, started) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_thread::<usize, _, SendError>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| {
                context.send_blocking(1)?;
                let mut send = Box::pin(context.send(2));
                assert!(block_on(bevy::tasks::poll_once(&mut send)).is_none());
                start.send(()).unwrap();
                assert_eq!(block_on(send), Err(SendError::Canceled));
                assert!(context.acknowledge_cancel());
                Ok(())
            },
        )
        .unwrap();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    manager.cancel(id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !manager.status(id).unwrap().state.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(value)) if *value == 1));
    assert!(matches!(
        receiver.try_recv(),
        Ok(JobMessage::Finished(JobOutcome::Canceled))
    ));
}

struct ThreadCleanup {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    exited: mpsc::Sender<()>,
}

impl Drop for ThreadCleanup {
    #[allow(clippy::unwrap_used)]
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
        self.exited.send(()).unwrap();
    }
}

thread_local! { static CLEANUP: std::cell::RefCell<Option<ThreadCleanup>> = const { std::cell::RefCell::new(None) }; }

#[test]
fn tls_destructors_must_finish_before_outcome_is_published() {
    let manager = manager();
    let (entered, cleaning) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (exited, done) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_thread::<(), _, String>(JobOptions::default(), move |_| {
            CLEANUP.with(|slot| {
                *slot.borrow_mut() = Some(ThreadCleanup {
                    entered,
                    release: released,
                    exited,
                })
            });
            Ok(())
        })
        .unwrap();
    cleaning.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(manager.status(id).unwrap().state, JobState::Running);
    manager.cancel(id).unwrap();
    assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
    assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Empty);
    release.send(()).unwrap();
    assert!(matches!(
        receiver.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    done.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn dropping_all_external_handles_does_not_abandon_thread_collection() {
    let manager = manager();
    let (entered, started) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (done, finished) = mpsc::channel();
    let (id, receiver) = manager
        .spawn_thread::<usize, _, String>(JobOptions::default(), move |context| {
            entered.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            for value in 0..100 {
                context
                    .send_blocking(value)
                    .map_err(|error| error.to_string())?;
            }
            done.send(()).unwrap();
            Ok(())
        })
        .unwrap();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(manager.status(id).unwrap().state, JobState::Running);
    drop(receiver);
    drop(manager);
    release.send(()).unwrap();
    finished.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn invalid_thread_name_is_rejected_without_running_business_code() {
    let manager = manager();
    let invoked = Arc::new(Mutex::new(false));
    let observed = invoked.clone();
    let result = manager.spawn_thread::<(), _, String>(
        JobOptions {
            label: Some("bad\0name".into()),
            ..Default::default()
        },
        move |_| {
            *invoked.lock().unwrap() = true;
            Ok(())
        },
    );
    assert!(matches!(result, Err(SubmitError::ThreadSpawn(_))));
    assert!(!*observed.lock().unwrap());
}
