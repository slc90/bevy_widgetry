use crate::retention::cancel_if_requested;
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum Reply {
    Loaded(Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, String>),
    Saved(BTreeMap<String, u64>, Result<(), String>),
}

pub(crate) fn load(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    path: PathBuf,
) -> Result<(JobId, JobReceiver<Reply>), SubmitError> {
    spawn(manager, "FileDialog load preferences", move || {
        Reply::Loaded(
            backend
                .0
                .load_preferences(&path)
                .map_err(|error| error.to_string()),
        )
    })
}

pub(crate) fn save(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    path: PathBuf,
    scopes: BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
    revisions: BTreeMap<String, u64>,
) -> Result<(JobId, JobReceiver<Reply>), SubmitError> {
    spawn(manager, "FileDialog save preferences", move || {
        Reply::Saved(
            revisions,
            backend
                .0
                .save_preferences(&path, &scopes)
                .map_err(|error| error.to_string()),
        )
    })
}

fn spawn(
    manager: &JobManager,
    label: &str,
    operation: impl FnOnce() -> Reply + Send + 'static,
) -> Result<(JobId, JobReceiver<Reply>), SubmitError> {
    manager.spawn_thread(
        JobOptions {
            label: Some(label.into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if cancel_if_requested(&context) {
                    return Ok(());
                }
                let reply = operation();
                let completion = match &reply {
                    Reply::Loaded(result) => result.as_ref().map(|_| ()).map_err(Clone::clone),
                    Reply::Saved(_, result) => result.clone(),
                };
                // A late cancellation does not undo an already completed load or durable write.
                match context.send(reply).await {
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
    use std::path::Path;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    struct Backend {
        gate: Mutex<mpsc::Receiver<()>>,
        entered: mpsc::Sender<()>,
        failure: bool,
    }
    impl Backend {
        fn wait(&self) -> io::Result<()> {
            self.entered.send(()).map_err(io::Error::other)?;
            self.gate
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?
                .recv()
                .map_err(io::Error::other)?;
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
                    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
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
                        let JobMessage::Item(reply) = receiver.recv_blocking().unwrap() else {
                            panic!("missing preferences reply");
                        };
                        match reply.as_ref() {
                            Reply::Loaded(result) if loading => {
                                assert_eq!(result.is_ok(), !failure)
                            }
                            Reply::Saved(revisions, result) if !loading => {
                                assert_eq!(revisions.get("open"), Some(&7));
                                assert_eq!(result.is_ok(), !failure);
                            }
                            _ => panic!("wrong preferences reply"),
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
}
