use crate::retention::{Control, Retained, Retention, cancel_if_requested, retain_result};
use crate::*;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobReceiver, SendStatus, SubmitFailure,
};
use std::sync::Arc;

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    service: Arc<Retention>,
    control: Control,
    job: WidgetryFileDialogValidationJob,
) -> Result<(JobId, JobReceiver<WidgetryFileDialogReply>), SubmitFailure> {
    manager.spawn_thread::<_, _>(
        JobOptions {
            label: Some("FileDialog candidate validation".into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                let token = job.token();
                let mut details = FileDialogRequestContext {
                    root: Some(control.root),
                    token,
                    mode: Some(job.mode),
                    operation: FileDialogRequestOperation::ValidateCandidate,
                    path: Some(job.directory.clone()),
                    resource: None,
                    paths: None,
                    index: None,
                };
                let candidate = match job.prepare() {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        let completion = match error.failure() {
                            Some(failure) => Err(failure.clone()),
                            None => Ok(()),
                        };
                        return publish(
                            &context,
                            WidgetryFileDialogReply::ValidationFailed { token, error },
                            completion,
                        )
                        .await;
                    }
                };
                match candidate.result() {
                    WidgetryFileDialogResult::File(path)
                    | WidgetryFileDialogResult::Directory(path)
                    | WidgetryFileDialogResult::SavePath(path) => details.path = Some(path.clone()),
                    WidgetryFileDialogResult::Files(paths)
                    | WidgetryFileDialogResult::Directories(paths) => {
                        details.path = None;
                        details.paths = Some(paths.clone());
                    }
                    WidgetryFileDialogResult::Cancelled => details.path = None,
                }
                if let WidgetryFileDialogResult::Files(paths)
                | WidgetryFileDialogResult::Directories(paths) = candidate.result()
                    && !retain_result(
                        &context,
                        &service,
                        &control,
                        vec![Retained::Paths(paths.clone())],
                    )
                    .await?
                {
                    return Ok(());
                }
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                let (reply, outcome) = match backend.0.validate(&candidate) {
                    Ok(exists) => (
                        WidgetryFileDialogReply::Validated { candidate, exists },
                        Ok(()),
                    ),
                    Err(error) => {
                        let failure = crate::error::request_io(details, error);
                        (
                            WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: FileDialogRequestError::Failed(failure.clone()),
                            },
                            Err(failure),
                        )
                    }
                };
                publish(&context, reply, outcome).await
            })
        },
    )
}

async fn publish(
    context: &JobContext<WidgetryFileDialogReply>,
    reply: WidgetryFileDialogReply,
    outcome: Result<(), crate::retention::WorkError>,
) -> Result<(), crate::retention::WorkError> {
    // The operation has finished its validation, so late cancellation only suppresses delivery.
    match context.send(reply).await {
        Ok(SendStatus::Sent | SendStatus::Canceled) => outcome,
        Err(error) => Err(crate::retention::delivery_error(outcome, error)),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobError, JobKind, JobOutcome, JobRead};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex, mpsc};
    use std::thread::{self, ThreadId};
    use std::time::{Duration, Instant};

    #[test]
    fn failed_validation_and_closed_delivery_keep_both_original_diagnostics() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let (sent, ready) = mpsc::channel();
        let (_, mut receiver) = manager
            .spawn_thread::<WidgetryFileDialogReply, _>(Default::default(), move |context| {
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
            let business = crate::retention::WorkError::from(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "validation refused",
            ));
            let origin = business.diagnostics().next().unwrap().origin().clone();
            let reply = WidgetryFileDialogReply::ValidationFailed {
                token: WidgetryFileDialogState::new(Default::default())
                    .unwrap()
                    .token(),
                error: FileDialogRequestError::Failed(business.clone()),
            };
            let failure =
                bevy::tasks::block_on(publish(&context, reply, Err(business))).unwrap_err();
            let diagnostics: Vec<_> = failure.diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            assert!(Arc::ptr_eq(diagnostics[0].origin(), &origin));
            assert!(matches!(
                diagnostics[1]
                    .source()
                    .unwrap()
                    .downcast_ref::<bevy_widgetry_job::SendError>(),
                Some(bevy_widgetry_job::SendError::Closed { .. })
            ));
            for diagnostic in diagnostics {
                assert_eq!(
                    logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                        .len(),
                    1
                );
                assert!(
                    logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                        .is_empty()
                );
            }
        });
    }

    struct Gated {
        gate: Arc<(Mutex<bool>, Condvar)>,
        entered: mpsc::Sender<ThreadId>,
        outcome: u8,
        candidate: Arc<Mutex<Option<WidgetryFileDialogResult>>>,
        supplied: Arc<Mutex<Option<io::Error>>>,
    }

    impl WidgetryFileDialogFileSystem for Gated {
        fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
            Ok(path.unwrap_or(Path::new("C:/fixture")).to_owned())
        }
        fn read_directory(
            &self,
            _: &Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            Ok(Box::new(std::iter::empty()))
        }
        fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            *self.candidate.lock().unwrap() = Some(candidate.result().clone());
            self.entered
                .send(thread::current().id())
                .map_err(io::Error::other)?;
            let (lock, changed) = &*self.gate;
            let mut released = lock
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?;
            while !*released {
                released = changed
                    .wait(released)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            }
            drop(released);
            if let Some(error) = self.supplied.lock().unwrap().take() {
                return Err(error);
            }
            match self.outcome {
                0 => Ok(false),
                1 => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "controlled validation error",
                )),
                _ => panic!("controlled validation panic"),
            }
        }
        fn create_directory(&self, _: &Path) -> io::Result<()> {
            Err(io::Error::other("unsupported"))
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    struct Stop {
        backend: WidgetryFileDialogBackend,
        service: Arc<Retention>,
        gate: Arc<(Mutex<bool>, Condvar)>,
        candidate: Arc<Mutex<Option<WidgetryFileDialogResult>>>,
        supplied: Arc<Mutex<Option<io::Error>>>,
    }
    impl Stop {
        fn release(&self) {
            let (lock, changed) = &*self.gate;
            *lock.lock().unwrap() = true;
            changed.notify_all();
        }
    }
    impl Drop for Stop {
        fn drop(&mut self) {
            self.release();
            self.service.stop();
        }
    }

    fn fixture(
        outcome: u8,
    ) -> (
        JobManager,
        Stop,
        Control,
        WidgetryFileDialogValidationJob,
        mpsc::Receiver<ThreadId>,
    ) {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let (entered, receiver) = mpsc::channel();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let candidate: Arc<Mutex<Option<WidgetryFileDialogResult>>> = Arc::default();
        let supplied: Arc<Mutex<Option<io::Error>>> = Arc::default();
        let backend = WidgetryFileDialogBackend(Arc::new(Gated {
            gate: gate.clone(),
            entered,
            outcome,
            candidate: candidate.clone(),
            supplied: supplied.clone(),
        }));
        let manager = JobManager::default();
        let service = Retention::start(&manager).unwrap();
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::SaveFile,
            ..Default::default()
        })
        .unwrap();
        let control = Control {
            root: bevy::prelude::Entity::PLACEHOLDER,
            token: state.token(),
            cancelled: Arc::default(),
        };
        let job = WidgetryFileDialogValidationJob {
            token: state.token(),
            mode: WidgetryFileDialogMode::SaveFile,
            directory: PathBuf::from("C:/fixture"),
            snapshot: None,
            selected: Arc::default(),
            filename: "new.txt".into(),
            filter: state.query().filter,
            allow_different_extension: false,
        };
        (
            manager,
            Stop {
                backend,
                service,
                gate,
                candidate,
                supplied,
            },
            control,
            job,
            receiver,
        )
    }

    fn wait(manager: &JobManager, id: JobId) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
    }

    #[test]
    fn complete_validation_waits_for_io_and_preserves_success_or_error_after_late_cancel() {
        for outcome in [0, 1] {
            for cancel in [false, true] {
                let (manager, stop, control, job, entered) = fixture(outcome);
                let token = job.token();
                let (id, mut receiver) = spawn(
                    &manager,
                    stop.backend.clone(),
                    stop.service.clone(),
                    control,
                    job,
                )
                .unwrap();
                assert_ne!(
                    entered.recv_timeout(Duration::from_secs(5)).unwrap(),
                    thread::current().id()
                );
                assert_eq!(manager.status(id).unwrap().kind, JobKind::Thread);
                assert!(!manager.status(id).unwrap().state.is_finished());
                assert!(matches!(receiver.try_recv(), Ok(JobRead::Empty)));
                let (sentinel, ready) = mpsc::channel();
                bevy::tasks::IoTaskPool::get()
                    .spawn(async move {
                        sentinel.send(()).unwrap();
                    })
                    .detach();
                ready.recv_timeout(Duration::from_secs(5)).unwrap();
                if cancel {
                    manager.cancel(id).unwrap();
                }
                stop.release();
                wait(&manager, id);
                let expected = manager.outcome(id).unwrap().unwrap();
                if outcome == 0 {
                    assert_eq!(expected, JobOutcome::Completed);
                } else {
                    assert!(
                        matches!(&expected, JobOutcome::Failed(error) if error.to_string().contains("controlled validation error"))
                    );
                }
                if !cancel {
                    let JobRead::Item(reply) = receiver.recv_blocking().unwrap() else {
                        panic!("validation result missing");
                    };
                    match reply.as_ref() {
                        WidgetryFileDialogReply::Validated {
                            candidate,
                            exists: false,
                        } if outcome == 0 => assert_eq!(candidate.token(), token),
                        WidgetryFileDialogReply::ValidationFailed {
                            token: received,
                            error,
                        } if outcome == 1 => {
                            assert_eq!(*received, token);
                            assert!(error.to_string().contains("controlled validation error"));
                            let FileDialogRequestError::Failed(reply_failure) = error else {
                                panic!("missing original request failure")
                            };
                            let JobOutcome::Failed(terminal_failure) = &expected else {
                                panic!("missing terminal failure")
                            };
                            assert_eq!(reply_failure, terminal_failure);
                            let mut cause = reply_failure.diagnostics().next().unwrap().source();
                            let mut matched = false;
                            while let Some(error) = cause {
                                if let Some(FileDialogError::RequestIo { context, .. }) =
                                    error.downcast_ref::<FileDialogError>()
                                {
                                    assert_eq!(
                                        context.path.as_deref(),
                                        Some(Path::new("C:/fixture/new.txt"))
                                    );
                                    matched = true;
                                }
                                cause = error.source();
                            }
                            assert!(matched);
                        }
                        _ => panic!("wrong validation result"),
                    }
                }
                assert!(
                    matches!(receiver.recv_blocking().unwrap(), JobRead::Finished(result) if result == expected)
                );
                assert!(stop.service.fault.lock().unwrap().is_none());
                manager.remove(id).unwrap();
            }
        }
    }

    #[test]
    fn multiselect_failures_keep_native_failed_item_or_shared_custom_candidates() {
        for case in 0..4 {
            let native = case == 3;
            let (manager, stop, control, mut job, entered) = fixture(1);
            let mut extra_job = None;
            let original = match case {
                1 => {
                    let failure = bevy_widgetry_job::JobFailure::from(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "shared validation failed",
                    ));
                    let origin = failure.diagnostics().next().unwrap().origin().clone();
                    *stop.supplied.lock().unwrap() = Some(io::Error::other(failure));
                    Some(origin)
                }
                2 => {
                    let (sent, ready) = mpsc::channel();
                    let (id, mut receiver) = manager
                        .spawn_thread::<(), _>(Default::default(), move |context| {
                            sent.send(context).unwrap();
                            Ok(())
                        })
                        .unwrap();
                    let context = ready.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert!(matches!(
                        receiver.recv_blocking().unwrap(),
                        JobRead::Finished(JobOutcome::Completed)
                    ));
                    let failure = context.send_blocking(()).unwrap_err();
                    let origin = failure.origin().clone();
                    *stop.supplied.lock().unwrap() = Some(io::Error::other(failure));
                    extra_job = Some(id);
                    Some(origin)
                }
                _ => None,
            };
            let directory = tempfile::tempdir().unwrap();
            let kept = directory.path().join("kept.txt");
            let missing = directory.path().join("missing.txt");
            std::fs::write(&kept, b"kept").unwrap();
            job.mode = WidgetryFileDialogMode::PickFiles;
            job.directory = directory.path().to_owned();
            let snapshot = Arc::new(
                WidgetryFileDialogSnapshot::prepare(
                    job.token,
                    job.directory.clone(),
                    vec![kept, missing.clone()]
                        .into_iter()
                        .map(|path| WidgetryFileDialogEntryData {
                            name: path.file_name().unwrap().to_owned(),
                            path,
                            kind: WidgetryFileDialogEntryKind::File,
                            size: None,
                            modified: None,
                            hidden: None,
                            system: None,
                        })
                        .collect(),
                    &WidgetryFileDialogState::new(Default::default())
                        .unwrap()
                        .query(),
                    None,
                )
                .unwrap(),
            );
            job.selected = Arc::new(snapshot.entries().iter().map(|entry| entry.id()).collect());
            job.snapshot = Some(snapshot);
            let token = job.token;
            let (id, mut receiver) = spawn(
                &manager,
                if native {
                    Default::default()
                } else {
                    stop.backend.clone()
                },
                stop.service.clone(),
                control,
                job,
            )
            .unwrap();
            if !native {
                entered.recv_timeout(Duration::from_secs(5)).unwrap();
                stop.release();
            }
            let JobRead::Item(reply) = receiver.recv_blocking().unwrap() else {
                panic!("missing validation reply")
            };
            let WidgetryFileDialogReply::ValidationFailed {
                error: FileDialogRequestError::Failed(failure),
                ..
            } = reply.as_ref()
            else {
                panic!("lost typed validation failure")
            };
            let JobRead::Finished(JobOutcome::Failed(terminal)) = receiver.recv_blocking().unwrap()
            else {
                panic!("missing failed terminal result")
            };
            assert_eq!(failure, &terminal);
            if let Some(original) = original {
                assert!(Arc::ptr_eq(
                    failure.diagnostics().next().unwrap().origin(),
                    &original
                ));
            }
            assert!(Arc::ptr_eq(
                failure.diagnostics().next().unwrap().origin(),
                terminal.diagnostics().next().unwrap().origin()
            ));
            let mut cause = failure.diagnostics().next().unwrap().source();
            let mut matched = false;
            let mut source_retained = case == 0 || native;
            while let Some(error) = cause {
                if case == 1
                    && matches!(error.downcast_ref::<JobError>(), Some(JobError::External { source }) if source.downcast_ref::<io::Error>().is_some_and(|error| error.kind() == io::ErrorKind::PermissionDenied))
                {
                    source_retained = true;
                }
                if case == 2
                    && matches!(error.downcast_ref::<bevy_widgetry_job::SendError>(), Some(bevy_widgetry_job::SendError::Closed { job }) if Some(*job) == extra_job)
                {
                    source_retained = true;
                }
                let domain = error.downcast_ref::<FileDialogError>();
                let context = match domain {
                    Some(FileDialogError::RequestIo { context, .. }) => Some(context.as_ref()),
                    Some(FileDialogError::RequestManaged { context, .. }) => Some(context.as_ref()),
                    _ => None,
                };
                if let Some(context) = context {
                    assert_eq!(context.token, token);
                    assert_eq!(context.mode, Some(WidgetryFileDialogMode::PickFiles));
                    let paths = context.paths.as_ref().unwrap();
                    assert_eq!(paths.len(), 2);
                    assert_eq!(paths[1], missing);
                    if native {
                        assert_eq!(
                            context.operation,
                            FileDialogRequestOperation::CandidateMetadata
                        );
                        assert!(context.path.is_none());
                        assert_eq!(context.resource.as_ref(), Some(&missing));
                        assert_eq!(context.index, Some(1));
                        assert!(
                            matches!(domain, Some(FileDialogError::RequestIo { source, .. }) if source.kind() == io::ErrorKind::NotFound)
                        );
                    } else {
                        assert_eq!(
                            context.operation,
                            FileDialogRequestOperation::ValidateCandidate
                        );
                        assert!(context.path.is_none());
                        assert!(context.resource.is_none());
                        assert!(context.index.is_none());
                        if case == 0 {
                            assert!(
                                matches!(domain, Some(FileDialogError::RequestIo { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied)
                            );
                        }
                        let captured = stop.candidate.lock().unwrap();
                        let Some(WidgetryFileDialogResult::Files(captured)) = captured.as_ref()
                        else {
                            panic!("custom backend did not receive multi candidates")
                        };
                        assert!(Arc::ptr_eq(paths, captured));
                    }
                    matched = true;
                }
                cause = error.source();
            }
            assert!(matched);
            assert!(source_retained);
            manager.remove(id).unwrap();
            if let Some(id) = extra_job {
                manager.remove(id).unwrap();
            }
        }
    }

    #[test]
    fn native_save_parent_failure_keeps_the_candidate_filename() {
        let (manager, stop, control, mut job, _) = fixture(0);
        let directory = tempfile::tempdir().unwrap();
        job.directory = directory.path().join("missing");
        job.filename = "report.txt".into();
        let candidate_path = job.directory.join(&job.filename);
        let token = job.token;
        let (id, mut receiver) = spawn(
            &manager,
            Default::default(),
            stop.service.clone(),
            control,
            job,
        )
        .unwrap();
        let JobRead::Item(reply) = receiver.recv_blocking().unwrap() else {
            panic!("missing validation reply")
        };
        let WidgetryFileDialogReply::ValidationFailed {
            error: FileDialogRequestError::Failed(failure),
            ..
        } = reply.as_ref()
        else {
            panic!("missing parent failure")
        };
        let JobRead::Finished(JobOutcome::Failed(terminal)) = receiver.recv_blocking().unwrap()
        else {
            panic!("missing failed terminal")
        };
        assert_eq!(failure, &terminal);
        let mut cause = failure.diagnostics().next().unwrap().source();
        let mut matched = false;
        while let Some(error) = cause {
            if let Some(FileDialogError::RequestIo { context, source }) =
                error.downcast_ref::<FileDialogError>()
            {
                assert_eq!(context.path.as_ref(), Some(&candidate_path));
                assert_eq!(context.resource.as_deref(), candidate_path.parent());
                assert_eq!(context.token, token);
                assert_eq!(context.mode, Some(WidgetryFileDialogMode::SaveFile));
                assert_eq!(
                    context.operation,
                    FileDialogRequestOperation::SaveParentMetadata
                );
                assert_eq!(source.kind(), io::ErrorKind::NotFound);
                matched = true;
            }
            cause = error.source();
        }
        assert!(matched);
        manager.remove(id).unwrap();
    }

    #[test]
    fn invalid_candidate_is_a_business_reply_and_backend_panic_is_an_isolated_failed_job() {
        for panic_backend in [false, true] {
            let (manager, stop, control, mut job, entered) =
                fixture(if panic_backend { 2 } else { 0 });
            if !panic_backend {
                job.filename = "bad?.txt".into();
            }
            let (id, mut receiver) = spawn(
                &manager,
                stop.backend.clone(),
                stop.service.clone(),
                control,
                job,
            )
            .unwrap();
            if panic_backend {
                entered.recv_timeout(Duration::from_secs(5)).unwrap();
                stop.release();
            }
            wait(&manager, id);
            if panic_backend {
                assert!(matches!(
                    manager.outcome(id).unwrap(),
                    Some(JobOutcome::Failed(error)) if error.diagnostics().any(|diagnostic| matches!(diagnostic.source().and_then(|source| source.downcast_ref::<JobError>()), Some(JobError::PanicObserved { .. })))
                ));
                assert!(matches!(
                    receiver.recv_blocking().unwrap(),
                    JobRead::Finished(JobOutcome::Failed(error)) if error.diagnostics().any(|diagnostic| matches!(diagnostic.source().and_then(|source| source.downcast_ref::<JobError>()), Some(JobError::PanicObserved { .. })))
                ));
            } else {
                assert!(matches!(entered.try_recv(), Err(mpsc::TryRecvError::Empty)));
                assert!(
                    matches!(receiver.recv_blocking().unwrap(), JobRead::Item(reply) if matches!(reply.as_ref(), WidgetryFileDialogReply::ValidationFailed { .. }))
                );
                assert!(matches!(
                    receiver.recv_blocking().unwrap(),
                    JobRead::Finished(JobOutcome::Completed)
                ));
            }
            assert!(stop.service.fault.lock().unwrap().is_none());
            manager.remove(id).unwrap();
        }
    }
}
