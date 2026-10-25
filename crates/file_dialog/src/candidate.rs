use crate::worker::{Control, Retained, Service, cancel_if_requested, retain_result};
use crate::*;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError,
};
use std::sync::Arc;

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    service: Arc<Service>,
    control: Control,
    job: WidgetryFileDialogValidationJob,
) -> Result<(JobId, JobReceiver<WidgetryFileDialogReply>), SubmitError> {
    manager.spawn_thread(
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
                        let error = error.to_string();
                        (
                            WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: error.clone(),
                            },
                            Err(error),
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
    outcome: Result<(), String>,
) -> Result<(), String> {
    // The operation has finished its validation, so late cancellation only suppresses delivery.
    match context.send(reply).await {
        Ok(()) | Err(SendError::Canceled) => outcome,
        Err(error) => Err(match outcome {
            Ok(()) => error.to_string(),
            Err(cause) => format!("{cause}; result delivery failed: {error}"),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobError, JobKind, JobMessage, JobOutcome, TryRecvError};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex, mpsc};
    use std::thread::{self, ThreadId};
    use std::time::{Duration, Instant};

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
        service: Arc<Service>,
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
        let service = Service::start(backend.clone(), Box::new(|| Ok(()))).unwrap();
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
            JobManager::default(),
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
                assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
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
                let expected = if outcome == 0 {
                    JobOutcome::Completed
                } else {
                    JobOutcome::Failed(JobError::Business("controlled validation error".into()))
                };
                assert_eq!(manager.outcome(id).unwrap(), Some(expected.clone()));
                if !cancel {
                    let JobMessage::Item(reply) = receiver.recv_blocking().unwrap() else {
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
                    matches!(receiver.recv_blocking().unwrap(), JobMessage::Finished(result) if result == expected)
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
                    Some(JobOutcome::Failed(JobError::Panic(_)))
                ));
                assert!(matches!(
                    receiver.recv_blocking().unwrap(),
                    JobMessage::Finished(JobOutcome::Failed(JobError::Panic(_)))
                ));
            } else {
                assert!(matches!(entered.try_recv(), Err(mpsc::TryRecvError::Empty)));
                assert!(
                    matches!(receiver.recv_blocking().unwrap(), JobMessage::Item(reply) if matches!(reply.as_ref(), WidgetryFileDialogReply::ValidationFailed { .. }))
                );
                assert!(matches!(
                    receiver.recv_blocking().unwrap(),
                    JobMessage::Finished(JobOutcome::Completed)
                ));
            }
            assert!(stop.service.fault.lock().unwrap().is_none());
            manager.remove(id).unwrap();
        }
    }
}
