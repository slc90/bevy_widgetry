#![allow(clippy::disallowed_macros)]

const TEST_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

use bevy::tasks::{IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use event_listener::Event;
use std::{
    sync::{Arc, mpsc},
    time::Instant,
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
            .spawn_io::<(), _, _>(JobOptions::default(), move |context| async move {
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
        running.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::CancelRequested);
        stopped.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyStopping);
        assert!(
            matches!(manager.remove(id).err().unwrap().kind(), JobAccessError::NotFinished { status } if status.state == JobState::Stopping)
        );
        assert!(
            matches!(manager.subscribe::<()>(id).err().unwrap().kind(), JobAccessError::NotSubscribable { status } if status.state == JobState::Stopping)
        );
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Empty)));
        release.notify(usize::MAX);
        let expected = if acknowledge {
            JobOutcome::Canceled
        } else {
            JobOutcome::Completed
        };
        assert!(
            matches!(block_on(receiver.recv()), Ok(JobRead::Finished(value)) if value == expected)
        );
        let status = manager.status(id).unwrap();
        assert_eq!(status.outcome, Some(expected));
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
        assert_eq!(manager.status(id).unwrap(), status);
        manager.remove(id).unwrap();
        assert!(matches!(
            manager.cancel(id).err().unwrap().kind(),
            JobAccessError::NotFound { .. }
        ));
    }
}

#[test]
fn acknowledged_cancellation_cannot_mask_error_or_panic() {
    for panic in [false, true] {
        let manager = manager();
        let (started, running) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_io::<(), _, _>(JobOptions::default(), move |context| async move {
                started.send(()).unwrap();
                context.cancelled().await;
                assert!(context.acknowledge_cancel());
                if panic {
                    panic!("cleanup panic");
                }
                Err(std::io::Error::other("cleanup failed").into())
            })
            .unwrap();
        running.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
        manager.cancel(id).unwrap();
        let outcome = block_on(receiver.recv()).unwrap();
        if panic {
            assert!(
                matches!(outcome, JobRead::Finished(JobOutcome::Failed(error)) if error.diagnostics().any(|diagnostic| matches!(diagnostic.source().and_then(|source| source.downcast_ref::<JobError>()), Some(JobError::PanicObserved { message }) if message == "cleanup panic")))
            );
        } else {
            assert!(
                matches!(outcome, JobRead::Finished(JobOutcome::Failed(message)) if message.to_string().contains("cleanup failed"))
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
            .spawn_io::<usize, _, _>(
                JobOptions {
                    buffer_capacity: Some(1),
                    ..Default::default()
                },
                move |context| async move {
                    assert_eq!(context.send(1).await?, SendStatus::Sent);
                    let mut pending_send = Box::pin(context.send(2));
                    assert!(bevy::tasks::poll_once(&mut pending_send).await.is_none());
                    started.send(()).unwrap();
                    assert_eq!(pending_send.await.unwrap(), SendStatus::Canceled);
                    if acknowledge {
                        assert!(context.acknowledge_cancel());
                    }
                    Ok(())
                },
            )
            .unwrap();
        waiting.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
        manager.cancel(id).unwrap();
        let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
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
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Item(value)) if *value == 1));
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Finished(value)) if value == expected));
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Closed)));
    }
}

#[test]
fn false_acknowledgement_without_request_leaves_success_completed() {
    let manager = manager();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _>(JobOptions::default(), |context| async move {
            assert!(!context.is_cancelled());
            assert!(!context.acknowledge_cancel());
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        block_on(receiver.recv()),
        Ok(JobRead::Finished(JobOutcome::Completed))
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
                .spawn_io::<(), _, _>(JobOptions::default(), move |_| async move {
                    started.send(()).unwrap();
                    ready.await;
                    if fail {
                        Err(std::io::Error::other("natural failure").into())
                    } else {
                        Ok(())
                    }
                })
                .unwrap();
            running.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
            let sync = Arc::new(std::sync::Barrier::new(2));
            let other = sync.clone();
            let cancel = manager.clone();
            let request = std::thread::spawn(move || {
                other.wait();
                cancel.cancel(id).unwrap()
            });
            sync.wait();
            release.notify(usize::MAX);
            let JobRead::Finished(actual) = block_on(receiver.recv()).unwrap() else {
                panic!("missing terminal result");
            };
            if fail {
                assert!(
                    matches!(&actual, JobOutcome::Failed(error) if error.to_string().contains("natural failure"))
                );
            } else {
                assert_eq!(actual, JobOutcome::Completed);
            }
            assert_eq!(manager.outcome(id).unwrap(), Some(actual.clone()));
            assert!(matches!(
                request.join().unwrap(),
                CancelResult::CancelRequested | CancelResult::AlreadyFinished
            ));
            assert_eq!(manager.outcome(id).unwrap(), Some(actual));
            assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
        }
    }
}

#[test]
fn send_cancellation_is_normal_and_does_not_acknowledge_the_job() {
    let manager = manager();
    let (sent, waiting) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_io::<usize, _, _>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| async move {
                assert_eq!(context.send(1).await?, SendStatus::Sent);
                let mut send = Box::pin(context.send(2));
                assert!(bevy::tasks::poll_once(&mut send).await.is_none());
                sent.send(()).unwrap();
                assert_eq!(send.await?, SendStatus::Canceled);
                Ok(())
            },
        )
        .unwrap();
    waiting.recv_timeout(TEST_WAIT_TIMEOUT).unwrap();
    manager.cancel(id).unwrap();
    let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
    while !manager.status(id).unwrap().state.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(receiver.try_recv(), Ok(JobRead::Item(value)) if *value == 1));
    assert!(matches!(
        receiver.try_recv(),
        Ok(JobRead::Finished(JobOutcome::Completed))
    ));
}
