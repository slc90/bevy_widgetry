use crate::retention::cancel_if_requested;
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendStatus, SubmitFailure};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum Reply {
    Loaded(
        Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, crate::retention::WorkError>,
    ),
    Saved(
        BTreeMap<String, u64>,
        Result<(), crate::retention::WorkError>,
    ),
}

pub(crate) fn load(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    path: PathBuf,
) -> Result<(JobId, JobReceiver<Reply>), SubmitFailure> {
    spawn(manager, "FileDialog load preferences", move || {
        Reply::Loaded(backend.0.load_preferences(&path).map_err(|error| {
            crate::error::preference_io(
                FileDialogPreferenceContext {
                    path: path.clone(),
                    resource: None,
                    operation: FileDialogPreferenceOperation::Load,
                    scope: None,
                    revisions: Default::default(),
                },
                error,
            )
        }))
    })
}

pub(crate) fn save(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    path: PathBuf,
    scopes: BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
    revisions: BTreeMap<String, u64>,
) -> Result<(JobId, JobReceiver<Reply>), SubmitFailure> {
    spawn(manager, "FileDialog save preferences", move || {
        let outcome = backend.0.save_preferences(&path, &scopes).map_err(|error| {
            crate::error::preference_io(
                FileDialogPreferenceContext {
                    path: path.clone(),
                    resource: None,
                    operation: FileDialogPreferenceOperation::Save,
                    scope: None,
                    revisions: std::sync::Arc::new(revisions.clone()),
                },
                error,
            )
        });
        Reply::Saved(revisions, outcome)
    })
}

fn spawn(
    manager: &JobManager,
    label: &str,
    operation: impl FnOnce() -> Reply + Send + 'static,
) -> Result<(JobId, JobReceiver<Reply>), SubmitFailure> {
    manager.spawn_thread::<_, _>(
        JobOptions {
            label: Some(label.into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                publish(&context, operation()).await
            })
        },
    )
}

async fn publish(
    context: &bevy_widgetry_job::JobContext<Reply>,
    reply: Reply,
) -> Result<(), crate::retention::WorkError> {
    let completion = match &reply {
        Reply::Loaded(result) => result.as_ref().map(|_| ()).map_err(Clone::clone),
        Reply::Saved(_, result) => result.clone(),
    };
    // A late cancellation does not undo an already completed load or durable write.
    match context.send(reply).await {
        Ok(SendStatus::Sent | SendStatus::Canceled) => completion,
        Err(error) => Err(crate::retention::delivery_error(completion, error)),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobKind, JobOutcome, JobRead};
    use std::io;
    use std::path::Path;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    fn context_of(failure: &bevy_widgetry_job::JobFailure) -> &FileDialogPreferenceContext {
        let mut cause = failure.diagnostics().next().unwrap().source();
        while let Some(error) = cause {
            match error.downcast_ref::<FileDialogError>() {
                Some(FileDialogError::PreferenceIo { context, .. }) => return context,
                Some(FileDialogError::PreferenceManaged { context, .. }) => return context,
                _ => cause = error.source(),
            }
        }
        panic!("preference context missing")
    }

    fn failed_reply(receiver: &mut JobReceiver<Reply>) -> bevy_widgetry_job::JobFailure {
        let JobRead::Item(reply) = receiver.recv_blocking().unwrap() else {
            panic!("missing preference reply")
        };
        let failure = match reply.as_ref() {
            Reply::Loaded(Err(error)) | Reply::Saved(_, Err(error)) => error.clone(),
            _ => panic!("expected failure reply"),
        };
        let JobRead::Finished(JobOutcome::Failed(terminal)) = receiver.recv_blocking().unwrap()
        else {
            panic!("missing failed terminal outcome")
        };
        assert_eq!(failure, terminal);
        assert!(Arc::ptr_eq(
            failure.diagnostics().next().unwrap().origin(),
            terminal.diagnostics().next().unwrap().origin()
        ));
        failure
    }

    #[test]
    fn native_preferences_preserve_source_scope_revisions_and_one_source_log() {
        const CHILD: &str = "WIDGETRY_PREFERENCE_SOURCE_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "preferences_job::tests::native_preferences_preserve_source_scope_revisions_and_one_source_log", "--nocapture"]).env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        use std::os::windows::fs::OpenOptionsExt;
        let logs = LogCapture::default();
        bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        let (id, mut receiver) = load(&manager, Default::default(), path.clone()).unwrap();
        assert!(
            matches!(receiver.recv_blocking().unwrap(), JobRead::Item(reply) if matches!(reply.as_ref(), Reply::Loaded(Ok(scopes)) if scopes.is_empty()))
        );
        assert!(matches!(
            receiver.recv_blocking().unwrap(),
            JobRead::Finished(JobOutcome::Completed)
        ));
        assert!(logs.records().is_empty());
        manager.remove(id).unwrap();

        std::fs::write(&path, b"{broken").unwrap();
        let (id, mut receiver) = load(&manager, Default::default(), path.clone()).unwrap();
        let parse = failed_reply(&mut receiver);
        let context = context_of(&parse);
        assert_eq!(context.path, path);
        assert_eq!(context.operation, FileDialogPreferenceOperation::DecodeJson);
        let mut cause = parse.diagnostics().next().unwrap().source();
        let mut parser = false;
        while let Some(error) = cause {
            parser |= error.is::<serde_json::Error>();
            cause = error.source();
        }
        assert!(parser);
        manager.remove(id).unwrap();

        let revisions = BTreeMap::from([("open".into(), 7), ("other".into(), 9)]);
        let mut scopes = BTreeMap::from([(
            "open".into(),
            WidgetryFileDialogStorageSnapshot {
                pinned: vec!["relative".into()],
                ..Default::default()
            },
        )]);
        let (id, mut receiver) = save(
            &manager,
            Default::default(),
            path.clone(),
            scopes.clone(),
            revisions.clone(),
        )
        .unwrap();
        let invalid = failed_reply(&mut receiver);
        let context = context_of(&invalid);
        assert_eq!(context.path, path);
        assert_eq!(
            context.operation,
            FileDialogPreferenceOperation::EncodeValues
        );
        assert_eq!(context.scope.as_deref(), Some("open"));
        assert_eq!(*context.revisions, revisions);
        let mut cause = invalid.diagnostics().next().unwrap().source();
        let mut path_fault = false;
        while let Some(error) = cause {
            path_fault |= matches!(error.downcast_ref::<FileDialogPreferenceFault>(), Some(FileDialogPreferenceFault::Path { path, .. }) if path == Path::new("relative"));
            cause = error.source();
        }
        assert!(path_fault);
        manager.remove(id).unwrap();

        scopes.get_mut("open").unwrap().pinned.clear();
        WidgetryFileDialogNativeFileSystem
            .save_preferences(&path, &scopes)
            .unwrap();
        let original = std::fs::read(&path).unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        let (id, mut receiver) = save(
            &manager,
            Default::default(),
            path.clone(),
            scopes,
            revisions.clone(),
        )
        .unwrap();
        let io = failed_reply(&mut receiver);
        let context = context_of(&io);
        assert_eq!(context.path, path);
        assert_eq!(context.resource.as_ref(), Some(&path));
        assert_eq!(context.operation, FileDialogPreferenceOperation::Replace);
        assert_eq!(*context.revisions, revisions);
        let mut cause = io.diagnostics().next().unwrap().source();
        let mut raw_os = false;
        while let Some(error) = cause {
            raw_os |= error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.raw_os_error().is_some());
            cause = error.source();
        }
        assert!(raw_os);
        drop(locked);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        manager.remove(id).unwrap();
        for failure in [&parse, &invalid, &io] {
            let origin = failure.diagnostics().next().unwrap().origin();
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            assert_eq!(
                origin.backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
            assert!(
                !logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)[0]
                    .fields
                    .contains_key("backtrace")
            );
        }
    }

    #[test]
    fn io_and_reply_delivery_failures_keep_both_original_diagnostics() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let (sent, ready) = mpsc::channel();
        let (id, mut receiver) = manager
            .spawn_thread::<Reply, _>(Default::default(), move |context| {
                sent.send(context).unwrap();
                Ok(())
            })
            .unwrap();
        let context = ready.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            receiver.recv_blocking().unwrap(),
            JobRead::Finished(JobOutcome::Completed)
        ));
        let logs = LogCapture::default();
        logs.run(|| {
            let failure = crate::error::preference_io(FileDialogPreferenceContext { path: "C:/prefs.json".into(), resource: None, operation: FileDialogPreferenceOperation::Load, scope: None, revisions: Default::default() }, io::Error::new(io::ErrorKind::PermissionDenied, "load denied"));
            let origin = failure.diagnostics().next().unwrap().origin().clone();
            let combined = bevy::tasks::block_on(publish(&context, Reply::Loaded(Err(failure)))).unwrap_err();
            let diagnostics: Vec<_> = combined.diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            assert!(Arc::ptr_eq(diagnostics[0].origin(), &origin));
            let mut cause = diagnostics[1].source();
            let mut delivery = false;
            while let Some(error) = cause { delivery |= matches!(error.downcast_ref::<bevy_widgetry_job::SendError>(), Some(bevy_widgetry_job::SendError::Closed { job }) if *job == id); cause = error.source(); }
            assert!(delivery);
            for diagnostic in diagnostics {
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
                assert!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled).is_empty());
            }
        });
        manager.remove(id).unwrap();
    }

    struct Backend {
        gate: Mutex<mpsc::Receiver<()>>,
        entered: mpsc::Sender<()>,
        failure: bool,
        supplied: Mutex<Option<io::Error>>,
    }
    impl Backend {
        fn wait(&self) -> io::Result<()> {
            self.entered.send(()).map_err(io::Error::other)?;
            self.gate
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?
                .recv()
                .map_err(io::Error::other)?;
            if let Some(error) = self.supplied.lock().unwrap().take() {
                return Err(error);
            }
            if self.failure {
                Err(io::Error::other("controlled persistence failure"))
            } else {
                Ok(())
            }
        }
    }
    impl WidgetryFileDialogFileSystem for Backend {
        fn resolve_directory(&self, _: Option<&Path>) -> io::Result<PathBuf> {
            Err(io::Error::other("unsupported"))
        }
        fn read_directory(
            &self,
            _: &Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            Err(io::Error::other("unsupported"))
        }
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            Err(io::Error::other("unsupported"))
        }
        fn create_directory(&self, _: &Path) -> io::Result<()> {
            Err(io::Error::other("unsupported"))
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Err(io::Error::other("unsupported"))
        }
        fn load_preferences(
            &self,
            _: &Path,
        ) -> io::Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>> {
            self.wait()?;
            Ok(BTreeMap::from([(
                "open".into(),
                WidgetryFileDialogStorageSnapshot::default(),
            )]))
        }
        fn save_preferences(
            &self,
            path: &Path,
            scopes: &BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
        ) -> io::Result<()> {
            self.wait()?;
            WidgetryFileDialogNativeFileSystem.save_preferences(path, scopes)
        }
    }

    #[test]
    fn managed_backend_failures_keep_identity_and_typed_preference_context() {
        const CHILD: &str = "WIDGETRY_PREFERENCE_MANAGED_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "preferences_job::tests::managed_backend_failures_keep_identity_and_typed_preference_context", "--nocapture"]).env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let logs = LogCapture::default();
        bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for saving in [false, true] {
            for send_failure in [false, true] {
                let manager = JobManager::default();
                let (sent, ready) = mpsc::channel();
                let (closed_id, mut receiver) = manager
                    .spawn_thread::<Reply, _>(Default::default(), move |context| {
                        sent.send(context).unwrap();
                        Ok(())
                    })
                    .unwrap();
                let closed = ready.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(matches!(
                    receiver.recv_blocking().unwrap(),
                    JobRead::Finished(JobOutcome::Completed)
                ));
                let (raw, origin) = if send_failure {
                    let failure = closed
                        .send_blocking(Reply::Loaded(Ok(BTreeMap::new())))
                        .unwrap_err();
                    let origin = failure.origin().clone();
                    (io::Error::other(failure), origin)
                } else {
                    let failure = bevy_widgetry_job::JobFailure::from(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "external preference failure",
                    ));
                    let origin = failure.diagnostics().next().unwrap().origin().clone();
                    (io::Error::other(failure), origin)
                };
                let (release, gate) = mpsc::channel();
                let (entered, started) = mpsc::channel();
                let backend = WidgetryFileDialogBackend(Arc::new(Backend {
                    gate: Mutex::new(gate),
                    entered,
                    failure: false,
                    supplied: Mutex::new(Some(raw)),
                }));
                let path = PathBuf::from("C:/fixture/preferences.json");
                let revisions = BTreeMap::from([("open".into(), 7), ("other".into(), 9)]);
                let (id, mut receiver) = if saving {
                    save(
                        &manager,
                        backend,
                        path.clone(),
                        BTreeMap::new(),
                        revisions.clone(),
                    )
                } else {
                    load(&manager, backend, path.clone())
                }
                .unwrap();
                started.recv_timeout(Duration::from_secs(5)).unwrap();
                release.send(()).unwrap();
                let failure = failed_reply(&mut receiver);
                assert!(Arc::ptr_eq(
                    failure.diagnostics().next().unwrap().origin(),
                    &origin
                ));
                let context = context_of(&failure);
                assert_eq!(context.path, path);
                assert_eq!(
                    context.operation,
                    if saving {
                        FileDialogPreferenceOperation::Save
                    } else {
                        FileDialogPreferenceOperation::Load
                    }
                );
                if saving {
                    assert_eq!(*context.revisions, revisions);
                } else {
                    assert!(context.revisions.is_empty());
                }
                let mut cause = failure.diagnostics().next().unwrap().source();
                let mut retained = false;
                while let Some(error) = cause {
                    if send_failure {
                        retained |= matches!(error.downcast_ref::<bevy_widgetry_job::SendError>(), Some(bevy_widgetry_job::SendError::Closed { job }) if *job == closed_id);
                    } else {
                        retained |= matches!(error.downcast_ref::<bevy_widgetry_job::JobError>(), Some(bevy_widgetry_job::JobError::External { source }) if source.downcast_ref::<io::Error>().is_some_and(|error| error.kind() == io::ErrorKind::PermissionDenied));
                    }
                    cause = error.source();
                }
                assert!(retained);
                assert_eq!(
                    logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                        .len(),
                    1
                );
                assert!(
                    logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                        .is_empty()
                );
                manager.remove(id).unwrap();
                manager.remove(closed_id).unwrap();
            }
        }
    }

    #[test]
    fn complete_load_and_save_keep_durable_facts_and_failures_after_late_cancel() {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for loading in [false, true] {
            for failure in [false, true] {
                for cancel in [false, true] {
                    let directory = tempfile::tempdir().unwrap();
                    let path = directory.path().join("prefs.json");
                    let (release, gate) = mpsc::channel();
                    let (entered, ready) = mpsc::channel();
                    let backend = WidgetryFileDialogBackend(Arc::new(Backend {
                        gate: Mutex::new(gate),
                        entered,
                        failure,
                        supplied: Mutex::new(None),
                    }));
                    let manager = JobManager::default();
                    let (id, mut receiver) = if loading {
                        load(&manager, backend, path.clone())
                    } else {
                        save(
                            &manager,
                            backend,
                            path.clone(),
                            BTreeMap::from([("open".into(), Default::default())]),
                            BTreeMap::from([("open".into(), 7)]),
                        )
                    }
                    .unwrap();
                    ready.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert_eq!(manager.status(id).unwrap().kind, JobKind::Thread);
                    assert!(!manager.status(id).unwrap().state.is_finished());
                    assert!(matches!(receiver.try_recv(), Ok(JobRead::Empty)));
                    if cancel {
                        manager.cancel(id).unwrap();
                    }
                    release.send(()).unwrap();
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !manager.status(id).unwrap().state.is_finished() {
                        assert!(Instant::now() < deadline);
                        std::thread::yield_now();
                    }
                    let outcome = manager.outcome(id).unwrap().unwrap();
                    assert_eq!(matches!(outcome, JobOutcome::Completed), !failure);
                    assert!(!matches!(outcome, JobOutcome::Canceled));
                    if !loading && !failure {
                        assert!(
                            WidgetryFileDialogNativeFileSystem
                                .load_preferences(&path)
                                .unwrap()
                                .contains_key("open")
                        );
                    }
                    if !cancel {
                        let JobRead::Item(reply) = receiver.recv_blocking().unwrap() else {
                            panic!("missing preferences reply");
                        };
                        match reply.as_ref() {
                            Reply::Loaded(result) if loading => {
                                assert_eq!(result.is_ok(), !failure);
                                if let Err(error) = result {
                                    assert!(
                                        matches!(&outcome, JobOutcome::Failed(terminal) if terminal == error)
                                    );
                                }
                            }
                            Reply::Saved(revisions, result) if !loading => {
                                assert_eq!(revisions.get("open"), Some(&7));
                                assert_eq!(result.is_ok(), !failure);
                                if let Err(error) = result {
                                    assert!(
                                        matches!(&outcome, JobOutcome::Failed(terminal) if terminal == error)
                                    );
                                }
                            }
                            _ => panic!("wrong preferences reply"),
                        }
                    }
                    assert!(
                        matches!(receiver.recv_blocking().unwrap(), JobRead::Finished(actual) if actual == outcome)
                    );
                    manager.remove(id).unwrap();
                }
            }
        }
    }
}
