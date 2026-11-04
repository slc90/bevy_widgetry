#![allow(clippy::disallowed_macros)]

use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use bevy_widgetry_test_utils::LogCapture;
use std::{sync::mpsc, time::Duration};

#[test]
fn lifecycle_events_match_real_execution_and_are_not_repeated_by_observation() {
    const CHILD: &str = "WIDGETRY_JOB_LIFECYCLE_LOG_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "lifecycle_events_match_real_execution_and_are_not_repeated_by_observation",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let logs = LogCapture::default();
    tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
    IoTaskPool::get_or_init(TaskPool::new);
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let manager = JobManager::default();
    for kind in [JobKind::Io, JobKind::Cpu, JobKind::Thread] {
        for mode in 0..4 {
            let (started, running) = mpsc::channel();
            let release = event_listener::Event::new();
            let released = release.listen();
            let work = move |context: JobContext<()>| async move {
                started.send(()).unwrap();
                released.await;
                if mode == 1 || mode == 3 {
                    assert!(context.acknowledge_cancel());
                }
                if mode == 3 {
                    Err(std::io::Error::other("lifecycle failure").into())
                } else {
                    Ok(())
                }
            };
            let label = format!("{kind:?}-{mode}");
            let options = JobOptions {
                label: Some(label.clone()),
                ..Default::default()
            };
            let (id, mut receiver) = match kind {
                JobKind::Io => manager.spawn_io(options, work),
                JobKind::Cpu => manager.spawn_cpu(options, work),
                JobKind::Thread => {
                    manager.spawn_thread(options, move |context| block_on(work(context)))
                }
            }
            .unwrap();
            running.recv_timeout(Duration::from_secs(5)).unwrap();
            if mode != 0 {
                assert_eq!(manager.cancel(id).unwrap(), CancelResult::CancelRequested);
                assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyStopping);
            }
            let records_for_job = || {
                logs.records()
                    .into_iter()
                    .filter(|r| r.matches_field("job_id", &id.get().to_string()))
                    .collect::<Vec<_>>()
            };
            assert!(
                records_for_job()
                    .iter()
                    .all(|r| !r.matches_field("stage", "finished")
                        && !r.matches_field("stage", "joined"))
            );
            release.notify(usize::MAX);
            let JobRead::Finished(outcome) = receiver.recv_blocking().unwrap() else {
                panic!("missing outcome")
            };
            let expected = match mode {
                1 => JobState::Canceled,
                3 => JobState::Failed,
                _ => JobState::Completed,
            };
            assert_eq!(manager.status(id).unwrap().state, expected);
            assert_eq!(matches!(outcome, JobOutcome::Canceled), mode == 1);
            assert_eq!(matches!(outcome, JobOutcome::Failed(_)), mode == 3);
            for _ in 0..3 {
                manager.status(id).unwrap();
                assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyFinished);
                assert!(matches!(receiver.try_recv(), Ok(JobRead::Closed)));
            }
            let records = records_for_job();
            let events: Vec<_> = records
                .iter()
                .filter(|r| r.fields.contains_key("stage"))
                .collect();
            let mut stages = vec!["registered", "running"];
            if mode != 0 {
                stages.push("cancel_requested");
            }
            stages.push("body_exited");
            if kind == JobKind::Thread {
                stages.push("joined");
            }
            stages.push("finished");
            assert_eq!(events.len(), stages.len());
            for (event, stage) in events.iter().zip(stages) {
                assert!(event.matches_field("stage", stage));
            }
            for event in &events {
                assert_eq!(event.level, bevy::log::Level::DEBUG);
                assert!(event.matches_field("kind", &format!("{kind:?}")));
                if !event.matches_field("stage", "joined") {
                    assert!(event.fields["label"].contains(&label));
                }
                assert!(!event.fields.contains_key("backtrace"));
                assert!(!event.fields.contains_key("error"));
            }
            assert!(
                events
                    .last()
                    .unwrap()
                    .matches_field("state", &format!("{expected:?}"))
            );
            manager.remove(id).unwrap();
        }
    }
    check_thread_cleanup_logs(&manager, &logs).unwrap();
    let handle = manager.begin_shutdown(Duration::ZERO).unwrap();
    manager.begin_shutdown(Duration::from_secs(1)).unwrap();
    for _ in 0..3 {
        assert_eq!(handle.status().phase, ShutdownPhase::Finished);
    }
    for stage in ["shutdown_started", "shutdown_finished"] {
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.matches_field("stage", stage))
                .count(),
            1
        );
    }
}

struct ThreadCleanup(mpsc::Sender<()>, mpsc::Receiver<()>);

impl Drop for ThreadCleanup {
    #[allow(clippy::unwrap_used)]
    fn drop(&mut self) {
        self.0.send(()).unwrap();
        self.1.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

thread_local! {
    static CLEANUP: std::cell::RefCell<Option<ThreadCleanup>> = const { std::cell::RefCell::new(None) };
}

fn check_thread_cleanup_logs(
    manager: &JobManager,
    logs: &LogCapture,
) -> Result<(), Box<dyn std::error::Error>> {
    let (entered, cleaning) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (id, mut receiver) = manager.spawn_thread::<(), _>(JobOptions::default(), move |_| {
        CLEANUP.with(|slot| *slot.borrow_mut() = Some(ThreadCleanup(entered, released)));
        Ok(())
    })?;
    cleaning.recv_timeout(Duration::from_secs(5))?;
    let count = |stage| {
        logs.records()
            .iter()
            .filter(|r| {
                r.matches_field("job_id", &id.get().to_string()) && r.matches_field("stage", stage)
            })
            .count()
    };
    assert_eq!(count("body_exited"), 1);
    assert_eq!(manager.status(id)?.state, JobState::Running);
    assert_eq!(count("joined"), 0);
    assert_eq!(count("finished"), 0);
    release.send(())?;
    assert!(matches!(
        receiver.recv_blocking()?,
        JobRead::Finished(JobOutcome::Completed)
    ));
    assert_eq!(count("joined"), 1);
    assert_eq!(count("finished"), 1);
    manager.remove(id)?;
    Ok(())
}
