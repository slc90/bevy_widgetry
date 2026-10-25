#![allow(clippy::disallowed_macros)]

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, IoTaskPool, TaskPool, block_on},
};
use bevy_widgetry_job::*;
use event_listener::Event;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn manager() -> JobManager {
    IoTaskPool::get_or_init(TaskPool::new);
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    JobManager::default()
}

fn gate() -> (Arc<Event>, impl std::future::Future<Output = ()> + Send) {
    let event = Arc::new(Event::new());
    let listener = event.listen();
    (event, listener)
}

fn wait_finished(handle: &ShutdownHandle) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while handle.status().phase != ShutdownPhase::Finished {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn empty_shutdown_completes_once_and_rejects_all_submission_backends() {
    let manager = manager();
    let handle = manager.begin_shutdown(Duration::from_secs(10)).unwrap();
    assert_eq!(handle.status().phase, ShutdownPhase::Finished);
    let again = manager.begin_shutdown(Duration::ZERO).unwrap();
    assert_eq!(handle.try_recv(), Some(ShutdownMessage::Completed));
    assert_eq!(again.try_recv(), None);
    assert!(matches!(
        manager.spawn_io::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) }),
        Err(SubmitError::ShuttingDown)
    ));
    assert!(matches!(
        manager.spawn_cpu::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) }),
        Err(SubmitError::ShuttingDown)
    ));
    assert!(matches!(
        manager.spawn_thread::<(), _, String>(JobOptions::default(), |_| Ok(())),
        Err(SubmitError::ShuttingDown)
    ));
}

#[test]
fn mixed_jobs_cancel_together_timeout_once_then_complete_without_draining_results() {
    let manager = manager();
    let (release_io, cleanup_io) = gate();
    let (release_cpu, cleanup_cpu) = gate();
    let (release_thread, cleanup_thread) = mpsc::channel();
    let (started, running) = mpsc::channel();
    let started_io = started.clone();
    let (io, mut io_results) = manager
        .spawn_io::<usize, _, _, String>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| async move {
                context.send(1).await.map_err(|error| error.to_string())?;
                started_io.send(()).unwrap();
                context.cancelled().await;
                cleanup_io.await;
                Ok(())
            },
        )
        .unwrap();
    let started_cpu = started.clone();
    let (cpu, mut cpu_results) = manager
        .spawn_cpu::<(), _, _, String>(JobOptions::default(), move |context| async move {
            started_cpu.send(()).unwrap();
            context.cancelled().await;
            assert!(context.acknowledge_cancel());
            cleanup_cpu.await;
            Ok(())
        })
        .unwrap();
    let (thread, mut thread_results) = manager
        .spawn_thread::<(), _, String>(JobOptions::default(), move |context| {
            started.send(()).unwrap();
            block_on(context.cancelled());
            assert!(context.acknowledge_cancel());
            cleanup_thread.recv_timeout(Duration::from_secs(5)).unwrap();
            Err("device cleanup failed".into())
        })
        .unwrap();
    for _ in 0..3 {
        running.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    let handle = manager.begin_shutdown(Duration::ZERO).unwrap();
    let status = handle.status();
    assert_eq!(status.phase, ShutdownPhase::TimedOut);
    assert_eq!(status.remaining.len(), 3);
    assert!(
        status
            .remaining
            .iter()
            .all(|job| job.state == JobState::Stopping)
    );
    assert!(
        matches!(handle.try_recv(), Some(ShutdownMessage::TimedOut { remaining }) if remaining.len() == 3)
    );
    assert_eq!(handle.try_recv(), None);
    assert_eq!(
        manager
            .begin_shutdown(Duration::MAX)
            .unwrap()
            .status()
            .phase,
        ShutdownPhase::TimedOut
    );
    let frames = Arc::new(AtomicUsize::new(0));
    let observed = frames.clone();
    let query = handle.clone();
    let mut app = App::new();
    app.add_systems(Update, move || {
        assert_eq!(query.status().phase, ShutdownPhase::TimedOut);
        observed.fetch_add(1, Ordering::Relaxed);
    });
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(frames.load(Ordering::Relaxed), 8);
    release_io.notify(usize::MAX);
    release_cpu.notify(usize::MAX);
    release_thread.send(()).unwrap();
    wait_finished(&handle);
    assert_eq!(handle.try_recv(), Some(ShutdownMessage::Completed));
    assert_eq!(handle.try_recv(), None);
    assert_eq!(manager.status(io).unwrap().state, JobState::Completed);
    assert_eq!(manager.status(cpu).unwrap().state, JobState::Canceled);
    assert_eq!(manager.status(thread).unwrap().state, JobState::Failed);
    assert!(matches!(io_results.try_recv(), Ok(JobMessage::Item(value)) if *value == 1));
    assert!(matches!(
        io_results.try_recv(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert!(matches!(
        cpu_results.try_recv(),
        Ok(JobMessage::Finished(JobOutcome::Canceled))
    ));
    assert!(matches!(
        thread_results.try_recv(),
        Ok(JobMessage::Finished(JobOutcome::Failed(_)))
    ));
}

#[test]
fn repeated_shutdown_does_not_extend_the_original_global_deadline() {
    let manager = manager();
    let (release, cleanup) = gate();
    let (started, running) = mpsc::channel();
    let (_, receiver) = manager
        .spawn_io::<(), _, _, String>(JobOptions::default(), move |context| async move {
            started.send(()).unwrap();
            context.cancelled().await;
            cleanup.await;
            Ok(())
        })
        .unwrap();
    running.recv_timeout(Duration::from_secs(5)).unwrap();
    let handle = manager.begin_shutdown(Duration::from_millis(30)).unwrap();
    manager.begin_shutdown(Duration::MAX).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while handle.status().phase != ShutdownPhase::TimedOut {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(
        handle.try_recv(),
        Some(ShutdownMessage::TimedOut { .. })
    ));
    assert_eq!(handle.try_recv(), None);
    release.notify(usize::MAX);
    wait_finished(&handle);
    assert_eq!(handle.try_recv(), Some(ShutdownMessage::Completed));
    drop(receiver);
}

#[test]
fn shutdown_unblocks_full_sends_and_retained_terminal_records_do_not_delay_completion() {
    let manager = manager();
    let (sent, waiting) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_io::<usize, _, _, String>(
            JobOptions {
                buffer_capacity: Some(1),
                ..Default::default()
            },
            move |context| async move {
                context.send(1).await.map_err(|error| error.to_string())?;
                let mut send = Box::pin(context.send(2));
                assert!(bevy::tasks::poll_once(&mut send).await.is_none());
                sent.send(()).unwrap();
                assert_eq!(send.await, Err(SendError::Canceled));
                assert!(context.acknowledge_cancel());
                Ok(())
            },
        )
        .unwrap();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let handle = manager.begin_shutdown(Duration::from_secs(5)).unwrap();
    wait_finished(&handle);
    assert_eq!(handle.try_recv(), Some(ShutdownMessage::Completed));
    assert_eq!(handle.try_recv(), None);
    assert_eq!(manager.status(id).unwrap().state, JobState::Canceled);
    manager.remove(id).unwrap();
    assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(value)) if *value == 1));
    assert!(matches!(
        receiver.try_recv(),
        Ok(JobMessage::Finished(JobOutcome::Canceled))
    ));
}

#[test]
fn submission_racing_with_shutdown_is_either_registered_and_canceled_or_rejected() {
    for _ in 0..16 {
        let manager = manager();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let other = barrier.clone();
        let submitter = manager.clone();
        let submit = std::thread::spawn(move || {
            other.wait();
            submitter.spawn_io::<(), _, _, String>(JobOptions::default(), |context| async move {
                context.cancelled().await;
                assert!(context.acknowledge_cancel());
                Ok(())
            })
        });
        barrier.wait();
        let handle =
            std::thread::spawn(move || manager.begin_shutdown(Duration::from_secs(5)).unwrap())
                .join()
                .unwrap();
        let submitted = submit.join().unwrap();
        wait_finished(&handle);
        match submitted {
            Ok((_, mut receiver)) => assert!(matches!(
                receiver.recv_blocking(),
                Ok(JobMessage::Finished(JobOutcome::Canceled))
            )),
            Err(error) => assert_eq!(error, SubmitError::ShuttingDown),
        }
        assert_eq!(handle.status().phase, ShutdownPhase::Finished);
    }
}
