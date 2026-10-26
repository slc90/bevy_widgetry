use crate::retention::cancel_if_requested;
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError};

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    token: WidgetryFileDialogToken,
) -> Result<(JobId, JobReceiver<WidgetryFileDialogReply>), SubmitError> {
    manager.spawn_thread(
        JobOptions {
            label: Some("FileDialog locations".into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                let outcome = backend
                    .0
                    .locations()
                    .map_err(|error| error.to_string())
                    .and_then(|locations| {
                        if locations.len() > 128 {
                            Err("location capacity exceeded".into())
                        } else {
                            Ok(locations)
                        }
                    });
                let completion = outcome.as_ref().map(|_| ()).map_err(Clone::clone);
                // A completed synchronous query is not undone by a late cancellation request.
                match context
                    .send(WidgetryFileDialogReply::Locations { token, outcome })
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
    use bevy_widgetry_job::{JobKind, JobMessage, JobOutcome, TryRecvError};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    struct Backend {
        count: usize,
        failure: bool,
        gate: Mutex<mpsc::Receiver<()>>,
        entered: mpsc::Sender<()>,
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
            self.entered.send(()).map_err(io::Error::other)?;
            self.gate
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?
                .recv()
                .map_err(io::Error::other)?;
            if self.failure {
                return Err(io::Error::other("controlled location failure"));
            }
            Ok((0..self.count)
                .map(|index| WidgetryFileDialogLocation {
                    name: format!("Location {index}"),
                    path: PathBuf::from(format!("C:/locations/{index}")),
                })
                .collect())
        }
    }

    #[test]
    fn locations_preserve_bounds_errors_and_real_outcomes_after_late_cancel() {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for (count, failure) in [
            (0, false),
            (3, false),
            (128, false),
            (129, false),
            (0, true),
        ] {
            for cancel in [false, true] {
                let (release, gate) = mpsc::channel();
                let (entered, ready) = mpsc::channel();
                let backend = WidgetryFileDialogBackend(Arc::new(Backend {
                    count,
                    failure,
                    gate: Mutex::new(gate),
                    entered,
                }));
                let token = WidgetryFileDialogState::new(Default::default())
                    .unwrap()
                    .token();
                let manager = JobManager::default();
                let (id, mut receiver) = spawn(&manager, backend, token).unwrap();
                ready.recv_timeout(Duration::from_secs(5)).unwrap();
                assert_eq!(manager.status(id).unwrap().kind, JobKind::Thread);
                assert!(!manager.status(id).unwrap().state.is_finished());
                assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
                let (sentinel, received) = mpsc::channel();
                bevy::tasks::IoTaskPool::get()
                    .spawn(async move {
                        sentinel.send(()).unwrap();
                    })
                    .detach();
                received.recv_timeout(Duration::from_secs(5)).unwrap();
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
                assert_eq!(
                    matches!(outcome, JobOutcome::Completed),
                    !failure && count <= 128
                );
                assert!(!matches!(outcome, JobOutcome::Canceled));
                if !cancel {
                    let JobMessage::Item(reply) = receiver.recv_blocking().unwrap() else {
                        panic!("missing location result");
                    };
                    let WidgetryFileDialogReply::Locations {
                        token: actual,
                        outcome: result,
                    } = reply.as_ref()
                    else {
                        panic!("wrong result type");
                    };
                    assert_eq!(*actual, token);
                    if failure {
                        assert_eq!(result.as_ref().unwrap_err(), "controlled location failure");
                    } else if count > 128 {
                        assert_eq!(result.as_ref().unwrap_err(), "location capacity exceeded");
                    } else {
                        assert_eq!(result.as_ref().unwrap().len(), count);
                    }
                }
                assert!(
                    matches!(receiver.recv_blocking().unwrap(), JobMessage::Finished(actual) if actual == outcome)
                );
                manager.remove(id).unwrap();
            }
        }
    }
}
