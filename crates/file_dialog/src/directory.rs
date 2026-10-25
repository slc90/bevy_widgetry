use crate::worker::{Control, CpuSendError, CpuWork, Service};
use crate::*;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError,
};
use futures_channel::oneshot;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) enum DirectoryEvent {
    Started(PathBuf),
    Snapshot {
        snapshot: Arc<WidgetryFileDialogSnapshot>,
        state: WidgetryFileDialogDirectoryState,
        finished: bool,
    },
}

pub(crate) struct Batch {
    pub control: Control,
    pub previous: Option<Arc<WidgetryFileDialogSnapshot>>,
    pub path: PathBuf,
    pub query: WidgetryFileDialogQuery,
    pub data: Vec<WidgetryFileDialogEntryData>,
    pub finished: bool,
    pub errors: usize,
    pub summary: Vec<String>,
}

pub(crate) enum BatchReply {
    Prepared(DirectoryEvent),
    Retry(Box<Batch>),
    Failed(String),
}

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    service: Arc<Service>,
    control: Control,
    requested: Option<PathBuf>,
    query: WidgetryFileDialogQuery,
) -> Result<(JobId, JobReceiver<DirectoryEvent>), SubmitError> {
    manager.spawn_thread(
        JobOptions {
            label: Some("FileDialog directory".into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                let result =
                    enumerate(&context, backend, &service, control, requested, query).await;
                match result {
                    Err(error)
                        if error.kind() == io::ErrorKind::Interrupted && context.is_cancelled() =>
                    {
                        context.acknowledge_cancel();
                        Ok(())
                    }
                    result => result,
                }
            })
        },
    )
}

fn check(
    context: &JobContext<DirectoryEvent>,
    service: &Service,
    control: &Control,
) -> io::Result<()> {
    if context.is_cancelled() || control.stopped(service) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "directory job canceled",
        ))
    } else {
        Ok(())
    }
}

fn send_error(error: SendError) -> io::Error {
    match error {
        SendError::Canceled => io::Error::new(io::ErrorKind::Interrupted, error),
        error => io::Error::other(error),
    }
}

async fn enumerate(
    context: &JobContext<DirectoryEvent>,
    backend: WidgetryFileDialogBackend,
    service: &Service,
    control: Control,
    requested: Option<PathBuf>,
    query: WidgetryFileDialogQuery,
) -> io::Result<()> {
    check(context, service, &control)?;
    let path = backend.0.resolve_directory(requested.as_deref())?;
    check(context, service, &control)?;
    let entries = backend.0.read_directory(&path)?;
    check(context, service, &control)?;
    context
        .send(DirectoryEvent::Started(path.clone()))
        .await
        .map_err(send_error)?;
    let mut data = Vec::with_capacity(128);
    let mut previous = None;
    let mut errors = 0;
    let mut summary = Vec::new();
    let mut total = 0;
    let mut last_flush = Instant::now();
    for entry in entries {
        check(context, service, &control)?;
        match entry {
            Ok(entry) => {
                data.push(entry);
                total += 1;
            }
            Err(error) => {
                errors += 1;
                if summary.len() < 8 {
                    summary.push(error.to_string());
                }
            }
        }
        if total > 100_000 {
            return Err(io::Error::other("directory snapshot capacity exceeded"));
        }
        if should_flush(data.len(), last_flush.elapsed()) {
            previous = Some(
                prepare(
                    context,
                    service,
                    &control,
                    previous,
                    path.clone(),
                    query.clone(),
                    std::mem::take(&mut data),
                    false,
                    errors,
                    summary.clone(),
                )
                .await?,
            );
            last_flush = Instant::now();
        }
    }
    check(context, service, &control)?;
    prepare(
        context, service, &control, previous, path, query, data, true, errors, summary,
    )
    .await?;
    Ok(())
}

fn should_flush(count: usize, elapsed: Duration) -> bool {
    count == 128 || (count > 0 && elapsed >= Duration::from_millis(8))
}

async fn prepare(
    context: &JobContext<DirectoryEvent>,
    service: &Service,
    control: &Control,
    previous: Option<Arc<WidgetryFileDialogSnapshot>>,
    path: PathBuf,
    query: WidgetryFileDialogQuery,
    data: Vec<WidgetryFileDialogEntryData>,
    finished: bool,
    errors: usize,
    summary: Vec<String>,
) -> io::Result<Arc<WidgetryFileDialogSnapshot>> {
    let mut pending = Batch {
        control: control.clone(),
        previous,
        path,
        query,
        data,
        finished,
        errors,
        summary,
    };
    let event = loop {
        // Listen before the CPU capacity check so a concurrent release cannot be missed.
        let capacity = service.capacity.listen();
        let (reply, ready) = oneshot::channel();
        let mut work = CpuWork::DirectoryBatch(pending, reply);
        loop {
            check(context, service, control)?;
            match service.send_directory_batch(work) {
                Ok(()) => break,
                Err(CpuSendError::Busy(returned)) => {
                    work = *returned;
                    futures_timer::Delay::new(Duration::from_millis(1)).await;
                }
                Err(CpuSendError::Closed) => {
                    check(context, service, control)?;
                    return Err(io::Error::other("FileDialog CPU channel disconnected"));
                }
                Err(CpuSendError::Fault(error)) => return Err(io::Error::other(error)),
            }
        }
        // Wait for the CPU owner even after cancellation so Finished cannot precede its last batch.
        match ready.await {
            Ok(BatchReply::Prepared(event)) => break event,
            Ok(BatchReply::Retry(batch)) => {
                pending = *batch;
                bevy::tasks::futures_lite::future::race(context.cancelled(), capacity).await;
            }
            Ok(BatchReply::Failed(error)) => return Err(io::Error::other(error)),
            Err(error) => {
                check(context, service, control)?;
                return Err(io::Error::other(error));
            }
        }
    };
    check(context, service, control)?;
    let DirectoryEvent::Snapshot { snapshot, .. } = &event else {
        return Err(io::Error::other(
            "FileDialog CPU batch returned an invalid event",
        ));
    };
    let snapshot = snapshot.clone();
    context.send(event).await.map_err(send_error)?;
    Ok(snapshot)
}

impl Batch {
    pub fn prepare(&mut self) -> Result<DirectoryEvent, String> {
        let previous = match self.previous.take() {
            Some(snapshot) => snapshot,
            None => Arc::new(
                WidgetryFileDialogSnapshot::prepare(
                    self.control.token,
                    self.path.clone(),
                    vec![],
                    &self.query,
                    None,
                )
                .map_err(|error| error.to_string())?,
            ),
        };
        let mut snapshot = previous
            .append_batch(std::mem::take(&mut self.data), &self.query)
            .map_err(|error| error.to_string())?;
        if self.finished {
            snapshot = snapshot
                .reproject(self.control.token, &self.query)
                .map_err(|error| error.to_string())?;
        }
        let state = if self.errors > 0 {
            WidgetryFileDialogDirectoryState::Partial {
                errors: self.errors,
                summary: std::mem::take(&mut self.summary).into(),
            }
        } else if self.finished {
            WidgetryFileDialogDirectoryState::Ready
        } else {
            WidgetryFileDialogDirectoryState::Loading
        };
        Ok(DirectoryEvent::Snapshot {
            snapshot: Arc::new(snapshot),
            state,
            finished: self.finished,
        })
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobMessage, JobOutcome, TryRecvError};

    struct ManyEntries;
    impl WidgetryFileDialogFileSystem for ManyEntries {
        fn resolve_directory(&self, path: Option<&std::path::Path>) -> io::Result<PathBuf> {
            path.map(ToOwned::to_owned)
                .ok_or_else(|| io::Error::other("synthetic path missing"))
        }
        fn read_directory(
            &self,
            path: &std::path::Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            let path = path.to_owned();
            Ok(Box::new((0..4096).map(move |index| {
                let name = format!("file{index:04}.txt");
                Ok(WidgetryFileDialogEntryData {
                    path: path.join(&name),
                    name: name.into(),
                    kind: WidgetryFileDialogEntryKind::File,
                    size: None,
                    modified: None,
                    hidden: Some(false),
                    system: Some(false),
                })
            })))
        }
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            Ok(true)
        }
        fn create_directory(&self, _: &std::path::Path) -> io::Result<()> {
            Err(io::Error::other("unsupported"))
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    #[test]
    fn eight_paused_directory_streams_resume_or_cancel_without_retention_failure() {
        for cancel in [false, true] {
            let (manager, service, _, _directory) = setup(32);
            let mut jobs = Vec::new();
            for index in 0..8 {
                let state =
                    WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
                let control = Control {
                    root: bevy::prelude::Entity::PLACEHOLDER,
                    token: state.token(),
                    cancelled: Arc::default(),
                };
                let (id, receiver) = spawn(
                    &manager,
                    WidgetryFileDialogBackend(Arc::new(ManyEntries)),
                    service.0.clone(),
                    control,
                    Some(PathBuf::from(format!("C:/fixture/{index}"))),
                    state.query(),
                )
                .unwrap();
                jobs.push((id, receiver, false, false));
            }
            wait(|| {
                service
                    .0
                    .retained
                    .load(std::sync::atomic::Ordering::Relaxed)
                    >= 192
                    && service.0.capacity.total_listeners() > 0
            });
            for (id, _, _, _) in &jobs {
                assert!(!manager.status(*id).unwrap().state.is_finished());
                if cancel {
                    manager.cancel(*id).unwrap();
                }
            }
            wait(|| {
                for (id, receiver, finished, final_snapshot) in &mut jobs {
                    if *finished {
                        continue;
                    }
                    match receiver.try_recv() {
                        Ok(JobMessage::Item(event)) => {
                            if let DirectoryEvent::Snapshot {
                                snapshot,
                                finished: true,
                                ..
                            } = event.as_ref()
                            {
                                assert_eq!(snapshot.entries().len(), 4096);
                                *final_snapshot = true;
                            }
                        }
                        Ok(JobMessage::Finished(outcome)) => {
                            assert_eq!(
                                outcome,
                                if cancel {
                                    JobOutcome::Canceled
                                } else {
                                    JobOutcome::Completed
                                }
                            );
                            if !cancel {
                                assert!(*final_snapshot);
                            }
                            manager.remove(*id).unwrap();
                            *finished = true;
                        }
                        Err(TryRecvError::Empty) => {}
                        Err(error) => panic!("unexpected directory receive error: {error}"),
                    }
                }
                jobs.iter().all(|(_, _, finished, _)| *finished)
            });
        }
    }

    struct Gated {
        started: std::sync::mpsc::Sender<()>,
        gate: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    }

    struct Release(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);
    impl Drop for Release {
        fn drop(&mut self) {
            if let Ok(mut released) = self.0.0.lock() {
                *released = true;
            }
            self.0.1.notify_all();
        }
    }

    impl WidgetryFileDialogFileSystem for Gated {
        fn resolve_directory(&self, path: Option<&std::path::Path>) -> io::Result<PathBuf> {
            WidgetryFileDialogNativeFileSystem.resolve_directory(path)
        }
        fn read_directory(
            &self,
            path: &std::path::Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            self.started.send(()).map_err(io::Error::other)?;
            let mut released = self
                .gate
                .0
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?;
            while !*released {
                released = self
                    .gate
                    .1
                    .wait(released)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            }
            WidgetryFileDialogNativeFileSystem.read_directory(path)
        }
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            Ok(true)
        }
        fn create_directory(&self, _: &std::path::Path) -> io::Result<()> {
            Err(io::Error::other("unsupported"))
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    #[test]
    fn blocked_directory_backends_do_not_occupy_the_shared_io_pool() {
        let (manager, service, control, directory) = setup(1);
        let pool = bevy::tasks::IoTaskPool::get();
        let workers = pool.thread_num();
        let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let release = Release(gate.clone());
        let (started, entered) = std::sync::mpsc::channel();
        let backend = WidgetryFileDialogBackend(Arc::new(Gated { started, gate }));
        let query = WidgetryFileDialogState::new(WidgetryFileDialogProps::default())
            .unwrap()
            .query();
        let mut jobs = Vec::new();
        for _ in 0..workers {
            jobs.push(
                spawn(
                    &manager,
                    backend.clone(),
                    service.0.clone(),
                    control.clone(),
                    Some(directory.path().to_owned()),
                    query.clone(),
                )
                .unwrap(),
            );
        }
        for _ in 0..workers {
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let (sent, received) = std::sync::mpsc::channel();
        let sentinel = pool.spawn(async move {
            sent.send(()).unwrap();
        });
        let progressed = received.recv_timeout(Duration::from_millis(500));
        for (id, _) in &jobs {
            manager.cancel(*id).unwrap();
        }
        drop(release);
        bevy::tasks::block_on(sentinel);
        for (id, mut receiver) in jobs {
            while !matches!(receiver.recv_blocking().unwrap(), JobMessage::Finished(_)) {}
            manager.remove(id).unwrap();
        }
        assert!(
            progressed.is_ok(),
            "blocking directory calls occupied every I/O pool worker"
        );
    }

    struct Stop(Arc<Service>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.stop();
        }
    }

    fn setup(capacity: usize) -> (JobManager, Stop, Control, tempfile::TempDir) {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let service =
            Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(()))).unwrap();
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let control = Control {
            root: bevy::prelude::Entity::PLACEHOLDER,
            token: state.token(),
            cancelled: Arc::default(),
        };
        (
            JobManager::with_buffer_capacity(capacity).unwrap(),
            Stop(service),
            control,
            tempfile::tempdir().unwrap(),
        )
    }

    fn wait(mut done: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < end, "directory job did not make progress");
            std::thread::yield_now();
        }
    }

    #[test]
    fn batch_flush_threshold_preserves_count_time_and_empty_boundaries() {
        assert!(!should_flush(0, Duration::from_secs(1)));
        assert!(!should_flush(127, Duration::from_millis(7)));
        assert!(should_flush(128, Duration::ZERO));
        assert!(should_flush(1, Duration::from_millis(8)));
    }

    #[test]
    fn native_directory_stream_backpressures_and_finishes_after_the_final_snapshot() {
        let (manager, service, control, directory) = setup(1);
        for index in 0..257 {
            std::fs::write(
                directory.path().join(format!("file{index:03}.txt")),
                b"data",
            )
            .unwrap();
        }
        let query = WidgetryFileDialogState::new(WidgetryFileDialogProps::default())
            .unwrap()
            .query();
        let (id, mut receiver) = spawn(
            &manager,
            WidgetryFileDialogBackend::default(),
            service.0.clone(),
            control,
            Some(directory.path().to_owned()),
            query,
        )
        .unwrap();
        wait(|| {
            service
                .0
                .retained
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        });
        assert!(!manager.status(id).unwrap().state.is_finished());
        assert!(
            matches!(receiver.recv_blocking().unwrap(), JobMessage::Item(event) if matches!(*event, DirectoryEvent::Started(_)))
        );
        let mut batches = 0;
        let mut last_count = 0;
        let mut final_seen = false;
        loop {
            match receiver.recv_blocking().unwrap() {
                JobMessage::Item(event) => {
                    let DirectoryEvent::Snapshot {
                        snapshot,
                        state,
                        finished,
                    } = event.as_ref()
                    else {
                        panic!("duplicate Started");
                    };
                    assert!(!final_seen);
                    let count = snapshot.entries().len();
                    assert!(count >= last_count && count - last_count <= 128);
                    last_count = count;
                    batches += 1;
                    if *finished {
                        assert_eq!(count, 257);
                        assert_eq!(state, &WidgetryFileDialogDirectoryState::Ready);
                        final_seen = true;
                    }
                }
                JobMessage::Finished(outcome) => {
                    assert!(final_seen);
                    assert_eq!(outcome, JobOutcome::Completed);
                    break;
                }
            }
        }
        assert!(batches >= 3);
        manager.remove(id).unwrap();
    }

    #[test]
    fn cancellation_unblocks_a_full_directory_stream_and_missing_receivers_do_not_block_it() {
        for cancel in [false, true] {
            let (manager, service, control, directory) = setup(1);
            let query = WidgetryFileDialogState::new(WidgetryFileDialogProps::default())
                .unwrap()
                .query();
            let (id, receiver) = spawn(
                &manager,
                WidgetryFileDialogBackend::default(),
                service.0.clone(),
                control,
                Some(directory.path().to_owned()),
                query,
            )
            .unwrap();
            wait(|| {
                service
                    .0
                    .retained
                    .load(std::sync::atomic::Ordering::Relaxed)
                    > 0
            });
            assert!(!manager.status(id).unwrap().state.is_finished());
            if cancel {
                manager.cancel(id).unwrap();
            }
            drop(receiver);
            wait(|| manager.status(id).unwrap().state.is_finished());
            assert_eq!(
                manager.outcome(id).unwrap(),
                Some(if cancel {
                    JobOutcome::Canceled
                } else {
                    JobOutcome::Completed
                })
            );
            manager.remove(id).unwrap();
        }
    }
}
