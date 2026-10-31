#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::tasks::{IoTaskPool, TaskPool, block_on};
use bevy_widgetry_job::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

#[test]
fn configuration_and_access_failures_are_typed_but_probes_and_queue_states_are_normal() {
    IoTaskPool::get_or_init(TaskPool::new);
    let logs = LogCapture::default();
    logs.run(|| {
        let error = JobManager::with_buffer_capacity(0).err().unwrap();
        assert!(matches!(error.kind(), SubmitError::ZeroCapacity { request } if request.capacity == 0));
        let origin = error.origin().clone();
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
        let manager = JobManager::default();
        let gate = event_listener::Event::new();
        let listener = gate.listen();
        let (id, mut receiver) = manager.spawn_io::<u64, _, _, String>(JobOptions::default(), move |_| async move { listener.await; Ok(()) }).unwrap();
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Empty)));
        let mismatch = manager.subscribe::<u32>(id).err().unwrap();
        assert!(matches!(mismatch.kind(), JobAccessError::TypeMismatch { actual, requested, .. } if *actual == std::any::type_name::<u64>() && *requested == std::any::type_name::<u32>()));
        gate.notify(usize::MAX);
        assert!(matches!(block_on(receiver.recv()), Ok(JobRead::Finished(JobOutcome::Completed))));
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Closed)));
        manager.remove(id).unwrap();
        let before = logs.records().len();
        assert!(manager.find_status(id).is_none());
        assert!(manager.cancel_if_present(id).is_none());
        assert!(manager.remove_if_present(id).unwrap().is_none());
        assert_eq!(logs.records().len(), before);
        let invalid = manager.cancel(id).unwrap_err();
        assert!(matches!(invalid.kind(), JobAccessError::NotFound { job } if *job == id));
        assert_eq!(logs.diagnostic_records(invalid.origin().id(), DiagnosticPhase::Source).len(), 1);
        let origin = invalid.origin().clone();
        let mut app = bevy::prelude::App::new();
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        app.world_mut().commands().queue(move |_: &mut bevy::prelude::World| -> bevy::prelude::Result { Err(invalid.into_bevy_error()) });
        errors.run(|| app.world_mut().flush());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].error.severity(), bevy::ecs::error::Severity::Error);
        assert!(std::sync::Arc::ptr_eq(records[0].managed().unwrap().diagnostics().next().unwrap().origin(), &origin));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
    });
}

#[test]
fn missing_pool_preserves_the_requested_kind_label_and_capacity() {
    const FLAG: &str = "WIDGETRY_ERRORS11_MISSING_POOLS";
    if std::env::var_os(FLAG).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "missing_pool_preserves_the_requested_kind_label_and_capacity",
                "--nocapture",
            ])
            .env(FLAG, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let logs = LogCapture::default();
    logs.run(|| {
        let manager = JobManager::default();
        let failure = manager.spawn_cpu::<(), _, _, String>(JobOptions { label: Some("unavailable cpu".into()), buffer_capacity: Some(7) }, |_| async { Ok(()) }).err().unwrap();
        assert!(matches!(failure.kind(), SubmitError::PoolUnavailable { request, pool: JobKind::Cpu } if request.kind == Some(JobKind::Cpu) && request.capacity == 7 && request.label.as_deref() == Some("unavailable cpu")));
        assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled).is_empty());
    });
}

#[test]
fn cancellation_and_natural_end_are_quiet_but_sending_after_completion_is_a_real_failure() {
    use std::sync::{Arc, mpsc};
    use std::time::Duration;
    IoTaskPool::get_or_init(TaskPool::new);
    let manager = JobManager::default();
    let (sender, ready) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_io::<u8, _, _, String>(JobOptions::default(), move |context| async move {
            let context = Arc::new(context);
            sender.send(context.clone()).unwrap();
            context.cancelled().await;
            context.acknowledge_cancel();
            Ok(())
        })
        .unwrap();
    let context = ready.recv_timeout(Duration::from_secs(5)).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        manager.cancel(id).unwrap();
        assert_eq!(block_on(context.send(1)).unwrap(), SendStatus::Canceled);
        assert!(matches!(
            block_on(receiver.recv()).unwrap(),
            JobRead::Finished(JobOutcome::Canceled)
        ));
        assert!(matches!(receiver.try_recv().unwrap(), JobRead::Closed));
        assert!(logs.records().is_empty());
    });
    let (sender, ready) = mpsc::channel();
    let (id, mut receiver) = manager
        .spawn_io::<u8, _, _, String>(JobOptions::default(), move |context| async move {
            sender.send(context).unwrap();
            Ok(())
        })
        .unwrap();
    let context = ready.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(
        block_on(receiver.recv()).unwrap(),
        JobRead::Finished(JobOutcome::Completed)
    ));
    logs.run(|| {
        let failure = block_on(context.send(2)).unwrap_err();
        assert!(matches!(failure.kind(), SendError::Closed { job } if *job == id));
        assert_eq!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    });
}
