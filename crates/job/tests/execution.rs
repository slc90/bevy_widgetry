#![allow(clippy::disallowed_macros)]

use std::{sync::mpsc, time::Duration};

use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;

fn manager() -> JobManager {
    IoTaskPool::get_or_init(TaskPool::new);
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    JobManager::default()
}

#[test]
fn io_success_is_retained_until_explicit_removal() {
    let manager = manager();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, String>(
            JobOptions {
                label: Some("read".into()),
            },
            |_| async { Ok(()) },
        )
        .unwrap();
    assert!(matches!(
        block_on(receiver.recv()),
        Some(JobMessage::Finished(JobOutcome::Completed))
    ));
    let status = manager.status(id).unwrap();
    assert_eq!(status.kind, JobKind::Io);
    assert_eq!(status.label.as_deref(), Some("read"));
    assert_eq!(status.state, JobState::Completed);
    assert_eq!(manager.remove(id).unwrap(), status);
    assert_eq!(manager.status(id), Err(JobAccessError::NotFound));
    assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Closed);
}

#[test]
fn cpu_error_and_panic_preserve_diagnostics() {
    let manager = manager();
    let (id, mut receiver) = manager
        .spawn_cpu::<(), _, _, &str>(JobOptions::default(), |_| async { Err("disk unavailable") })
        .unwrap();
    assert!(
        matches!(block_on(receiver.recv()), Some(JobMessage::Finished(JobOutcome::Failed(JobError::Business(message)))) if message == "disk unavailable")
    );
    assert_eq!(manager.status(id).unwrap().kind, JobKind::Cpu);
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, &str>(JobOptions::default(), |_| async { panic!("broken reader") })
        .unwrap();
    assert!(
        matches!(block_on(receiver.recv()), Some(JobMessage::Finished(JobOutcome::Failed(JobError::Panic(message)))) if message == "broken reader")
    );
    assert_eq!(manager.status(id).unwrap().state, JobState::Failed);
}

#[test]
fn running_is_registered_before_work_and_cannot_be_removed() {
    let manager = manager();
    let query = manager.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = futures_channel_pair();
    let inside = manager.clone();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, String>(JobOptions::default(), move |context| async move {
            assert_eq!(
                inside.status(context.id()).unwrap().state,
                JobState::Running
            );
            started_tx.send(()).unwrap();
            release_rx.await;
            Ok(())
        })
        .unwrap();
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        std::thread::spawn(move || query.status(id).unwrap().state)
            .join()
            .unwrap(),
        JobState::Running
    );
    assert_eq!(
        manager.remove(id),
        Err(JobAccessError::NotFinished(JobState::Running))
    );
    release_tx.notify(usize::MAX);
    assert!(matches!(
        block_on(receiver.recv()),
        Some(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert_eq!(manager.status(id).unwrap().state, JobState::Completed);
}

fn futures_channel_pair() -> (
    std::sync::Arc<event_listener::Event>,
    impl std::future::Future<Output = ()> + Send,
) {
    let event = std::sync::Arc::new(event_listener::Event::new());
    let listener = event.listen();
    (event, listener)
}

#[test]
fn concurrent_submissions_are_unique_and_manager_instances_are_independent() {
    let manager = manager();
    let threads: Vec<_> = (0..32)
        .map(|_| {
            let manager = manager.clone();
            std::thread::spawn(move || {
                manager
                    .spawn_cpu::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) })
                    .unwrap()
            })
        })
        .collect();
    let mut ids = Vec::new();
    for thread in threads {
        let (id, mut receiver) = thread.join().unwrap();
        block_on(receiver.recv()).unwrap();
        ids.push(id.get());
    }
    ids.sort_unstable();
    assert_eq!(ids, (1..=32).collect::<Vec<_>>());
    let other = JobManager::default();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) })
        .unwrap();
    assert_eq!(id.get(), 33);
    assert_eq!(other.status(id), Err(JobAccessError::NotFound));
    block_on(receiver.recv()).unwrap();
}

#[test]
fn receiver_retains_unread_failure_after_record_removal() {
    let manager = manager();
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, &str>(JobOptions::default(), |_| async { Err("read failed") })
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !manager.status(id).unwrap().state.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let outcome = manager.outcome(id).unwrap().unwrap();
    manager.remove(id).unwrap();
    assert!(
        matches!(block_on(receiver.recv()), Some(JobMessage::Finished(value)) if value == outcome)
    );
    assert!(block_on(receiver.recv()).is_none());
}
