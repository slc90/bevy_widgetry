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
                let candidate = match job.prepare() {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        return publish(
                            &context,
                            WidgetryFileDialogReply::ValidationFailed { token, error },
                            Ok(()),
                        )
                        .await;
                    }
                };
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
                        let message = error.to_string();
                        let failure = crate::retention::WorkError::from(error);
                        (
                            WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: message,
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
                error: "validation refused".into(),
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
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
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
        let backend = WidgetryFileDialogBackend(Arc::new(Gated {
            gate: gate.clone(),
            entered,
            outcome,
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
                            assert!(error.contains("controlled validation error"));
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
