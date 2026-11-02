use crate::error::{directory_failure, directory_io, is_managed_io};
use crate::retention::{Control, CpuSendError, CpuWork, Retention};
use crate::*;
use bevy_widgetry_job::JobFailure;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobReceiver, SendStatus, SubmitFailure,
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
    Failed(FileDialogFailure),
}

pub(crate) fn spawn(
    manager: &JobManager,
    backend: WidgetryFileDialogBackend,
    service: Arc<Retention>,
    control: Control,
    requested: Option<PathBuf>,
    query: WidgetryFileDialogQuery,
) -> Result<(JobId, JobReceiver<DirectoryEvent>), SubmitFailure> {
    manager.spawn_thread(
        JobOptions {
            label: Some("FileDialog directory".into()),
            ..Default::default()
        },
        move |context| {
            bevy::tasks::block_on(async move {
                let result = enumerate(
                    &context,
                    backend,
                    &service,
                    control.clone(),
                    requested,
                    query,
                )
                .await;
                match result {
                    Ok(()) => Ok(()),
                    Err(DirectoryError::Canceled) => {
                        context.acknowledge_cancel();
                        Ok(())
                    }
                    Err(DirectoryError::Failed(error)) => Err(error),
                }
            })
        },
    )
}

enum DirectoryError {
    Canceled,
    Failed(JobFailure),
}

impl From<FileDialogFailure> for DirectoryError {
    fn from(error: FileDialogFailure) -> Self {
        Self::Failed(error.into())
    }
}

fn directory_context(
    control: &Control,
    operation: FileDialogDirectoryOperation,
    directory: Option<PathBuf>,
) -> FileDialogDirectoryContext {
    FileDialogDirectoryContext {
        operation,
        root: Some(control.root),
        token: Some(control.token),
        directory,
        entry: None,
        ordinal: None,
    }
}

fn io_failure(
    context: &JobContext<DirectoryEvent>,
    service: &Retention,
    control: &Control,
    details: FileDialogDirectoryContext,
    error: io::Error,
) -> DirectoryError {
    if !is_managed_io(&error)
        && error.kind() == io::ErrorKind::Interrupted
        && (context.is_cancelled() || control.stopped(service))
    {
        DirectoryError::Canceled
    } else {
        DirectoryError::Failed(directory_io(details, error))
    }
}

fn check(
    context: &JobContext<DirectoryEvent>,
    service: &Retention,
    control: &Control,
) -> Result<(), DirectoryError> {
    if context.is_cancelled() || control.stopped(service) {
        Err(DirectoryError::Canceled)
    } else {
        Ok(())
    }
}

async fn publish(
    context: &JobContext<DirectoryEvent>,
    event: DirectoryEvent,
    details: FileDialogDirectoryContext,
) -> Result<(), DirectoryError> {
    match context.send(event).await.map_err(|error| {
        DirectoryError::Failed(JobFailure::from(error).with_context(details.frame()))
    })? {
        SendStatus::Sent => Ok(()),
        SendStatus::Canceled => Err(DirectoryError::Canceled),
    }
}

async fn enumerate(
    context: &JobContext<DirectoryEvent>,
    backend: WidgetryFileDialogBackend,
    service: &Retention,
    control: Control,
    requested: Option<PathBuf>,
    query: WidgetryFileDialogQuery,
) -> Result<(), DirectoryError> {
    check(context, service, &control)?;
    let path = backend
        .0
        .resolve_directory(requested.as_deref())
        .map_err(|error| {
            io_failure(
                context,
                service,
                &control,
                directory_context(
                    &control,
                    FileDialogDirectoryOperation::ResolveDirectory,
                    requested.clone(),
                ),
                error,
            )
        })?;
    check(context, service, &control)?;
    let entries = backend.0.read_directory(&path).map_err(|error| {
        io_failure(
            context,
            service,
            &control,
            directory_context(
                &control,
                FileDialogDirectoryOperation::ReadDirectory,
                Some(path.clone()),
            ),
            error,
        )
    })?;
    check(context, service, &control)?;
    publish(
        context,
        DirectoryEvent::Started(path.clone()),
        directory_context(
            &control,
            FileDialogDirectoryOperation::ReadDirectory,
            Some(path.clone()),
        ),
    )
    .await?;
    let mut data = Vec::with_capacity(128);
    let mut previous = None;
    let mut errors = 0;
    let mut summary = Vec::new();
    let mut total = 0;
    let mut last_flush = Instant::now();
    for (ordinal, entry) in entries.enumerate() {
        match entry {
            Ok(entry) => {
                data.push(entry);
                total += 1;
            }
            Err(error) => {
                let mut details = directory_context(
                    &control,
                    FileDialogDirectoryOperation::ReadEntry,
                    Some(path.clone()),
                );
                details.ordinal = Some(ordinal);
                let error = match io_failure(context, service, &control, details, error) {
                    DirectoryError::Canceled => return Err(DirectoryError::Canceled),
                    DirectoryError::Failed(error) => error,
                };
                errors += 1;
                if summary.len() < 8 {
                    summary.push(error.to_string());
                }
                error.record_handled("跳过读取失败的目录条目", "继续枚举，目录结果标记为 Partial");
            }
        }
        if total > 100_000 {
            return Err(directory_failure(
                directory_context(
                    &control,
                    FileDialogDirectoryOperation::ReadEntry,
                    Some(path.clone()),
                ),
                FileDialogDirectoryFault::Capacity {
                    actual: total,
                    limit: 100_000,
                },
            )
            .into());
        }
        check(context, service, &control)?;
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
    service: &Retention,
    control: &Control,
    previous: Option<Arc<WidgetryFileDialogSnapshot>>,
    path: PathBuf,
    query: WidgetryFileDialogQuery,
    data: Vec<WidgetryFileDialogEntryData>,
    finished: bool,
    errors: usize,
    summary: Vec<String>,
) -> Result<Arc<WidgetryFileDialogSnapshot>, DirectoryError> {
    let details = directory_context(
        control,
        FileDialogDirectoryOperation::CpuSend,
        Some(path.clone()),
    );
    let mut pending = Box::new(Batch {
        control: control.clone(),
        previous,
        path,
        query,
        data,
        finished,
        errors,
        summary,
    });
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
                    return Err(directory_failure(
                        details.clone(),
                        FileDialogDirectoryFault::CpuDisconnected,
                    )
                    .into());
                }
                Err(CpuSendError::Fault(error)) => {
                    return Err(DirectoryError::Failed(error.with_context(details.frame())));
                }
            }
        }
        // Wait for the CPU owner even after cancellation so Finished cannot precede its last batch.
        match ready.await {
            Ok(BatchReply::Prepared(event)) => break event,
            Ok(BatchReply::Retry(batch)) => {
                pending = batch;
                bevy::tasks::futures_lite::future::race(context.cancelled(), capacity).await;
            }
            Ok(BatchReply::Failed(error)) => {
                return Err(DirectoryError::Failed(
                    JobFailure::from(error).with_context(details.frame()),
                ));
            }
            Err(error) => {
                check(context, service, control)?;
                let mut details = details.clone();
                details.operation = FileDialogDirectoryOperation::CpuReply;
                return Err(DirectoryError::Failed(JobFailure::external(
                    error,
                    details.frame(),
                )));
            }
        }
    };
    check(context, service, control)?;
    let DirectoryEvent::Snapshot { snapshot, .. } = &event else {
        return Err(
            directory_failure(details.clone(), FileDialogDirectoryFault::InvalidCpuEvent).into(),
        );
    };
    let snapshot = snapshot.clone();
    publish(context, event, details).await?;
    Ok(snapshot)
}

impl Batch {
    pub fn prepare(&mut self) -> Result<DirectoryEvent, FileDialogFailure> {
        let previous = match self.previous.take() {
            Some(snapshot) => snapshot,
            None => Arc::new(WidgetryFileDialogSnapshot::prepare(
                self.control.token,
                self.path.clone(),
                vec![],
                &self.query,
                None,
            )?),
        };
        let mut snapshot = previous.append_batch(std::mem::take(&mut self.data), &self.query)?;
        if self.finished {
            snapshot = snapshot.reproject(self.control.token, &self.query)?;
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
    use bevy_widgetry_job::{JobOutcome, JobRead};

    fn domain_error(failure: &JobFailure) -> &FileDialogError {
        let mut cause = failure.diagnostics().next().unwrap().source();
        while let Some(error) = cause {
            if let Some(domain) = error.downcast_ref::<FileDialogError>() {
                return domain;
            }
            cause = error.source();
        }
        panic!("directory domain error missing from source chain")
    }

    struct ManagedBackend {
        error: std::sync::Mutex<Option<io::Error>>,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
        interrupt: bool,
    }
    impl WidgetryFileDialogFileSystem for ManagedBackend {
        fn resolve_directory(&self, _: Option<&std::path::Path>) -> io::Result<PathBuf> {
            if self.interrupt {
                self.cancelled
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            Err(self.error.lock().unwrap().take().unwrap())
        }
        fn read_directory(
            &self,
            _: &std::path::Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            Ok(Box::new(std::iter::empty()))
        }
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            Ok(true)
        }
        fn create_directory(&self, _: &std::path::Path) -> io::Result<()> {
            Ok(())
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    #[test]
    fn managed_send_failure_keeps_its_origin_and_single_source_log_through_backend_io() {
        const CHILD: &str = "WIDGETRY_DIRECTORY_SEND_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "directory::tests::managed_send_failure_keeps_its_origin_and_single_source_log_through_backend_io", "--nocapture"]).env(CHILD, "1").output().unwrap();
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
        for interrupt in [false, true] {
            let (manager, service, control, directory) = setup(1);
            let (sent, ready) = std::sync::mpsc::channel();
            let (closed_id, mut closed_receiver) = manager
                .spawn_thread::<DirectoryEvent, _>(Default::default(), move |context| {
                    sent.send(context).unwrap();
                    Ok(())
                })
                .unwrap();
            let closed_context = ready.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(matches!(
                closed_receiver.recv_blocking().unwrap(),
                JobRead::Finished(JobOutcome::Completed)
            ));
            let error = closed_context
                .send_blocking(DirectoryEvent::Started(directory.path().to_owned()))
                .unwrap_err();
            let origin = error.origin().clone();
            let kind = if interrupt {
                io::ErrorKind::Interrupted
            } else {
                io::ErrorKind::Other
            };
            let backend = WidgetryFileDialogBackend(Arc::new(ManagedBackend {
                error: std::sync::Mutex::new(Some(io::Error::new(kind, error))),
                cancelled: control.cancelled.clone(),
                interrupt,
            }));
            let query = WidgetryFileDialogState::new(Default::default())
                .unwrap()
                .query();
            let (id, mut receiver) = spawn(
                &manager,
                backend,
                service.0.clone(),
                control,
                Some(directory.path().to_owned()),
                query,
            )
            .unwrap();
            let JobRead::Finished(JobOutcome::Failed(failure)) = receiver.recv_blocking().unwrap()
            else {
                panic!("managed SendFailure was lost")
            };
            let diagnostics: Vec<_> = failure.diagnostics().collect();
            assert_eq!(diagnostics.len(), 1);
            assert!(Arc::ptr_eq(diagnostics[0].origin(), &origin));
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            assert!(
                diagnostics[0]
                    .contexts()
                    .iter()
                    .any(|frame| frame.operation() == "file_dialog.directory")
            );
            manager.remove(id).unwrap();
            manager.remove(closed_id).unwrap();
        }
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.phase() == Some(DiagnosticPhase::Source))
                .count(),
            2
        );
    }

    struct FaultyEntries;
    impl WidgetryFileDialogFileSystem for FaultyEntries {
        fn resolve_directory(&self, path: Option<&std::path::Path>) -> io::Result<PathBuf> {
            Ok(path.unwrap().to_owned())
        }
        fn read_directory(
            &self,
            _: &std::path::Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>
        {
            Ok(Box::new(
                (0..12).map(|_| Err(io::Error::from_raw_os_error(5))),
            ))
        }
        fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
            Ok(true)
        }
        fn create_directory(&self, _: &std::path::Path) -> io::Result<()> {
            Ok(())
        }
        fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
            Ok(vec![])
        }
    }

    #[test]
    fn every_skipped_entry_failure_is_logged_even_after_summary_limit() {
        const CHILD: &str = "WIDGETRY_DIRECTORY_PARTIAL_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "directory::tests::every_skipped_entry_failure_is_logged_even_after_summary_limit", "--nocapture"]).env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
        let (manager, service, control, directory) = setup(1);
        let query = WidgetryFileDialogState::new(Default::default())
            .unwrap()
            .query();
        let (id, mut receiver) = spawn(
            &manager,
            WidgetryFileDialogBackend(Arc::new(FaultyEntries)),
            service.0.clone(),
            control,
            Some(directory.path().to_owned()),
            query,
        )
        .unwrap();
        let mut partial = false;
        loop {
            match receiver.recv_blocking().unwrap() {
                JobRead::Item(event) => {
                    if let DirectoryEvent::Snapshot {
                        state: WidgetryFileDialogDirectoryState::Partial { errors, summary },
                        finished: true,
                        ..
                    } = event.as_ref()
                    {
                        assert_eq!(*errors, 12);
                        assert_eq!(summary.len(), 8);
                        partial = true;
                    }
                }
                JobRead::Finished(outcome) => {
                    assert_eq!(outcome, JobOutcome::Completed);
                    break;
                }
                JobRead::Empty | JobRead::Closed => {
                    panic!("unexpected empty or closed directory stream")
                }
            }
        }
        assert!(partial);
        manager.remove(id).unwrap();
        use bevy_widgetry_test_utils::DiagnosticPhase;
        let records = logs.records();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.phase() == Some(DiagnosticPhase::Source))
                .count(),
            12
        );
        for source in records
            .iter()
            .filter(|record| record.phase() == Some(DiagnosticPhase::Source))
        {
            assert_eq!(source.level, bevy::log::tracing::Level::ERROR);
            assert!(!source.fields.contains_key("backtrace"));
            let id = &source.fields["error_id"];
            assert_eq!(
                records
                    .iter()
                    .filter(|record| record.phase() == Some(DiagnosticPhase::Handled)
                        && record.fields.get("error_id") == Some(id))
                    .count(),
                1
            );
            assert!(source.fields["context"].contains("ordinal: Some("));
            assert!(source.fields["context"].contains("entry: None"));
        }
        assert_eq!(
            records
                .iter()
                .filter(|record| record.phase() == Some(DiagnosticPhase::Handled))
                .count(),
            12
        );
    }

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
                        Ok(JobRead::Item(event)) => {
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
                        Ok(JobRead::Finished(outcome)) => {
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
                        Ok(JobRead::Empty) => {}
                        Ok(JobRead::Closed) => panic!("closed before Finished"),
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
        failure: Option<io::ErrorKind>,
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
            match self.failure {
                Some(kind) => Err(io::Error::new(kind, "gated directory failure")),
                None => WidgetryFileDialogNativeFileSystem.read_directory(path),
            }
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
        let backend = WidgetryFileDialogBackend(Arc::new(Gated {
            started,
            gate,
            failure: None,
        }));
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
            while !matches!(receiver.recv_blocking().unwrap(), JobRead::Finished(_)) {}
            manager.remove(id).unwrap();
        }
        assert!(
            progressed.is_ok(),
            "blocking directory calls occupied every I/O pool worker"
        );
    }

    struct Stop(Arc<Retention>);

    #[test]
    fn late_cancel_preserves_real_io_failure_but_acknowledges_interrupted() {
        for kind in [io::ErrorKind::PermissionDenied, io::ErrorKind::Interrupted] {
            let (manager, service, control, directory) = setup(1);
            let token = control.token;
            let state = WidgetryFileDialogState::new(Default::default()).unwrap();
            let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
            let release = Release(gate.clone());
            let (started, entered) = std::sync::mpsc::channel();
            let backend = WidgetryFileDialogBackend(Arc::new(Gated {
                started,
                gate,
                failure: Some(kind),
            }));
            let (id, mut receiver) = spawn(
                &manager,
                backend,
                service.0.clone(),
                control,
                Some(directory.path().to_owned()),
                state.query(),
            )
            .unwrap();
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            manager.cancel(id).unwrap();
            drop(release);
            let JobRead::Finished(outcome) = receiver.recv_blocking().unwrap() else {
                panic!("expected Finished")
            };
            if kind == io::ErrorKind::Interrupted {
                assert_eq!(outcome, JobOutcome::Canceled);
            } else {
                let JobOutcome::Failed(error) = outcome else {
                    panic!("late cancel lost I/O failure")
                };
                let domain = domain_error(&error);
                assert!(
                    matches!(domain, FileDialogError::DirectoryIo { context, source } if context.operation == FileDialogDirectoryOperation::ReadDirectory && context.token == Some(token) && source.kind() == kind)
                );
            }
            manager.remove(id).unwrap();
        }
    }

    #[test]
    fn native_resolve_failure_preserves_phase_path_token_and_raw_io_cause() {
        let (manager, service, control, directory) = setup(1);
        let token = control.token;
        let missing = directory.path().join("missing");
        let query = WidgetryFileDialogState::new(Default::default())
            .unwrap()
            .query();
        let (id, mut receiver) = spawn(
            &manager,
            Default::default(),
            service.0.clone(),
            control,
            Some(missing.clone()),
            query,
        )
        .unwrap();
        let JobRead::Finished(JobOutcome::Failed(error)) = receiver.recv_blocking().unwrap() else {
            panic!("expected native failure")
        };
        let diagnostic = error.diagnostics().next().unwrap();
        let domain = domain_error(&error);
        assert!(
            matches!(domain, FileDialogError::DirectoryIo { context, source } if context.operation == FileDialogDirectoryOperation::Canonicalize && context.directory.as_ref() == Some(&missing) && context.token == Some(token) && source.kind() == io::ErrorKind::NotFound)
        );
        let mut cause = diagnostic.source();
        let mut raw = false;
        while let Some(error) = cause {
            if let Some(error) = error.downcast_ref::<io::Error>() {
                raw |= error.raw_os_error().is_some();
            }
            cause = error.source();
        }
        assert!(raw);
        assert_eq!(
            diagnostic.origin().backtrace().status(),
            std::backtrace::BacktraceStatus::Captured
        );
        manager.remove(id).unwrap();
    }

    #[test]
    fn an_already_managed_interrupted_failure_is_not_canceled_again() {
        let (manager, service, control, directory) = setup(1);
        let details = directory_context(
            &control,
            FileDialogDirectoryOperation::ReadDirectory,
            Some(directory.path().to_owned()),
        );
        let failure = directory_io(details.clone(), io::Error::from_raw_os_error(5));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        control
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        let worker = service.0.clone();
        let (id, mut receiver) = manager
            .spawn_thread(Default::default(), move |context| {
                let error = io::Error::new(io::ErrorKind::Interrupted, failure);
                match io_failure(&context, &worker, &control, details, error) {
                    DirectoryError::Failed(error) => Err(error),
                    DirectoryError::Canceled => {
                        context.acknowledge_cancel();
                        Ok(())
                    }
                }
            })
            .unwrap();
        let JobRead::Finished(JobOutcome::Failed(failure)) = receiver.recv_blocking().unwrap()
        else {
            panic!("already captured failure lost to cancellation")
        };
        assert!(Arc::ptr_eq(
            failure.diagnostics().next().unwrap().origin(),
            &origin
        ));
        manager.remove(id).unwrap();
    }
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.stop();
        }
    }

    fn setup(capacity: usize) -> (JobManager, Stop, Control, tempfile::TempDir) {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::with_buffer_capacity(capacity).unwrap();
        let service = Retention::start(&manager).unwrap();
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let control = Control {
            root: bevy::prelude::Entity::PLACEHOLDER,
            token: state.token(),
            cancelled: Arc::default(),
        };
        (
            manager,
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
            matches!(receiver.recv_blocking().unwrap(), JobRead::Item(event) if matches!(*event, DirectoryEvent::Started(_)))
        );
        let mut batches = 0;
        let mut last_count = 0;
        let mut final_seen = false;
        loop {
            match receiver.recv_blocking().unwrap() {
                JobRead::Item(event) => {
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
                JobRead::Empty | JobRead::Closed => panic!("expected Item or Finished"),
                JobRead::Finished(outcome) => {
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
