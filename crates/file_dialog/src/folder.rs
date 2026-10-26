use crate::retention::cancel_if_requested;
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError};

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    request: WidgetryFileDialogFolderRequest,
) -> Result<(JobId, JobReceiver<WidgetryFileDialogReply>), SubmitError> {
    manager.spawn_thread(
        JobOptions {
            label: Some("FileDialog create folder".into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                let outcome = backend
                    .0
                    .create_directory(request.path())
                    .map_err(|error| error.to_string());
                let completion = outcome.clone();
                // Once the OS call returns, cancellation cannot undo the side effect or its outcome.
                match context
                    .send(WidgetryFileDialogReply::FolderCreated { request, outcome })
                    .await
                {
                    Ok(()) | Err(SendError::Canceled) => completion,
                    Err(error) => Err(match completion {
                        Ok(()) => error.to_string(),
                        Err(cause) => format!("{cause}; result delivery failed: {error}"),
                    }),
                }
            })
        },
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobError, JobKind, JobMessage, JobOutcome, TryRecvError};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};

    struct Gated {
        gate: Mutex<mpsc::Receiver<()>>,
        entered: mpsc::Sender<thread::ThreadId>,
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
            Ok(false)
        }
        fn create_directory(&self, path: &Path) -> io::Result<()> {
            self.entered
                .send(thread::current().id())
                .map_err(io::Error::other)?;
            self.gate
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?
                .recv()
                .map_err(io::Error::other)?;
            match self.outcome {
                0 => std::fs::create_dir(path),
                1 => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "controlled permission denied",
                )),
                2 => Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "controlled path conflict",
                )),
                _ => panic!("controlled folder panic"),
            }
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    #[test]
    fn blocked_folder_job_preserves_real_side_effect_and_outcome_after_late_cancel() {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for outcome in 0..4 {
            for cancel in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("created");
                let (release, gate) = mpsc::channel();
                let (entered, ready) = mpsc::channel();
                let backend = WidgetryFileDialogBackend(Arc::new(Gated {
                    gate: Mutex::new(gate),
                    entered,
                    outcome,
                }));
                let request = WidgetryFileDialogFolderRequest {
                    token: WidgetryFileDialogState::new(Default::default())
                        .unwrap()
                        .token(),
                    path: path.clone(),
                };
                let token = request.token();
                let manager = JobManager::default();
                let (id, mut receiver) = spawn(&manager, backend, request).unwrap();
                assert_ne!(
                    ready.recv_timeout(Duration::from_secs(5)).unwrap(),
                    thread::current().id()
                );
                assert_eq!(manager.status(id).unwrap().kind, JobKind::Thread);
                assert!(!manager.status(id).unwrap().state.is_finished());
                assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
                if cancel {
                    manager.cancel(id).unwrap();
                }
                release.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(10);
                while !manager.status(id).unwrap().state.is_finished() {
                    assert!(Instant::now() < deadline);
                    thread::yield_now();
                }
                assert_eq!(path.is_dir(), outcome == 0);
                let expected = manager.outcome(id).unwrap().unwrap();
                match (&expected, outcome) {
                    (JobOutcome::Completed, 0) => {}
                    (JobOutcome::Failed(JobError::Business(error)), 1 | 2) => {
                        assert!(error.contains("controlled"))
                    }
                    (JobOutcome::Failed(JobError::Panic(error)), 3) => {
                        assert!(error.contains("controlled folder panic"))
                    }
                    _ => panic!("incorrect folder outcome: {expected:?}"),
                }
                if !cancel && outcome != 3 {
                    let JobMessage::Item(reply) = receiver.recv_blocking().unwrap() else {
                        panic!("folder reply missing");
                    };
                    let WidgetryFileDialogReply::FolderCreated {
                        request,
                        outcome: result,
                    } = reply.as_ref()
                    else {
                        panic!("wrong folder reply");
                    };
                    assert_eq!(request.token(), token);
                    assert_eq!(request.path(), path);
                    assert_eq!(result.is_ok(), outcome == 0);
                }
                assert!(
                    matches!(receiver.recv_blocking().unwrap(), JobMessage::Finished(result) if result == expected)
                );
                manager.remove(id).unwrap();
            }
        }
    }
}
