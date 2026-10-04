use crate::*;
use bevy::prelude::*;
use bevy_widgetry_log::{widgetry_error, widgetry_warn};
use crossbeam_channel::{Receiver, SendTimeoutError, Sender};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const CAPACITY: usize = 64;
const RETAINED_CAPACITY: usize = 256;

pub(crate) struct Service {
    backend: WidgetryFileDialogBackend,
    pending: Mutex<BTreeMap<(Entity, u8), IoWork>>,
    ready: Sender<()>,
    ready_receiver: Receiver<()>,
    pub(crate) cpu: Sender<CpuWork>,
    cpu_receiver: Receiver<CpuWork>,
    output: Sender<Output>,
    pub(crate) replies: Receiver<Output>,
    pub(crate) shutdown: AtomicBool,
    wake_pending: AtomicBool,
    wake: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    pub(crate) live: AtomicUsize,
    pub(crate) active_io: AtomicUsize,
    pub(crate) retained: AtomicUsize,
    pub(crate) fault: Mutex<Option<String>>,
    handles: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub(crate) struct Control {
    pub(crate) root: Entity,
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) cancelled: Arc<AtomicBool>,
}

pub(crate) struct Batch {
    control: Control,
    path: PathBuf,
    query: WidgetryFileDialogQuery,
    data: Vec<WidgetryFileDialogEntryData>,
    finished: bool,
    errors: usize,
    summary: Vec<String>,
}

pub(crate) enum IoWork {
    Directory(Control, Option<PathBuf>, WidgetryFileDialogQuery),
    Validate(Control, WidgetryFileDialogCandidate),
    Folder(Control, WidgetryFileDialogFolderRequest),
    Locations(Control),
    Load(PathBuf),
    Save(
        PathBuf,
        BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
        BTreeMap<String, u64>,
    ),
}

pub(crate) enum CpuWork {
    Batch(Batch),
    Projection(
        Control,
        Arc<WidgetryFileDialogState>,
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
        u64,
    ),
    Selection(Control, WidgetryFileDialogSelectionJob),
    Candidate(Control, WidgetryFileDialogValidationJob),
}

pub(crate) enum Output {
    Reply(Entity, WidgetryFileDialogReply),
    Snapshot(
        Entity,
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    ),
    Candidate(Control, WidgetryFileDialogCandidate),
    Projection(Control, u64, WidgetryFileDialogReply),
    Locations(Control, Result<Vec<WidgetryFileDialogLocation>, String>),
    Loaded(Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, String>),
    Saved(BTreeMap<String, u64>, Result<(), String>),
}

enum Retained {
    Snapshot(Arc<WidgetryFileDialogSnapshot>),
    Selection(Arc<BTreeSet<WidgetryFileDialogEntryId>>),
    Paths(Arc<[PathBuf]>),
}

fn interrupted() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "FileDialog session cancelled")
}

fn cpu_loop(service: &Service) {
    let mut cache: BTreeMap<
        (Entity, WidgetryFileDialogSessionId, u64),
        (Control, Arc<WidgetryFileDialogSnapshot>),
    > = BTreeMap::new();
    let mut retained = Vec::<Retained>::new();
    loop {
        cache.retain(|_, (control, _)| !control.stopped(service));
        retained.retain(|item| !item.released());
        service.retained.store(retained.len(), Ordering::Relaxed);
        if service.shutdown.load(Ordering::Acquire) {
            while service.cpu_receiver.try_recv().is_ok() {}
            while service.replies.try_recv().is_ok() {}
            if retained.is_empty() {
                break;
            }
            // App 先释放 runtime Resource 时，World 的 Component 仍可能持有 handle。
            // worker 保留最后 ownership，等待其它 handle 自然释放，避免大块数据在 App Drop 中析构。
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        let work = match service.cpu_receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(work) => work,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                service.fail("CPU work channel disconnected".into());
                break;
            }
        };
        let control = work.control().clone();
        if control.stopped(service) {
            continue;
        }
        if retained.len() >= RETAINED_CAPACITY - 3 {
            let reply = Output::Reply(
                control.root,
                WidgetryFileDialogReply::Failed {
                    token: control.token,
                    error: "FileDialog retained snapshot capacity exceeded".into(),
                },
            );
            if let Err(error) = service.send(&service.output, reply, Some(&control))
                && error.kind() != io::ErrorKind::Interrupted
            {
                service.fail(error.to_string());
            }
            control.cancelled.store(true, Ordering::Release);
            continue;
        }
        let outcome: Result<Output, String> = (|| match work {
            CpuWork::Batch(batch) => {
                let key = (
                    control.root,
                    control.token.session,
                    control.token.generation,
                );
                let previous = match cache.get(&key) {
                    Some((_, previous)) => previous.clone(),
                    None => Arc::new(
                        WidgetryFileDialogSnapshot::prepare(
                            control.token,
                            batch.path,
                            vec![],
                            &batch.query,
                            None,
                        )
                        .map_err(|error| error.to_string())?,
                    ),
                };
                let mut next = previous
                    .append_batch(batch.data, &batch.query)
                    .map_err(|error| error.to_string())?;
                if batch.finished {
                    next = next
                        .reproject(control.token, &batch.query)
                        .map_err(|error| error.to_string())?;
                }
                let snapshot = Arc::new(next);
                retain_unique(&mut retained, Retained::Snapshot(snapshot.clone()));
                cache.insert(key, (control.clone(), snapshot.clone()));
                let state = if batch.errors > 0 {
                    WidgetryFileDialogDirectoryState::Partial {
                        errors: batch.errors,
                        summary: batch.summary.into(),
                    }
                } else if batch.finished {
                    WidgetryFileDialogDirectoryState::Ready
                } else {
                    WidgetryFileDialogDirectoryState::Loading
                };
                Ok(Output::Snapshot(control.root, snapshot, state))
            }
            CpuWork::Projection(_, state, source, status, version) => {
                let snapshot = if source.token().query_revision != state.token().query_revision {
                    Arc::new(
                        source
                            .reproject(state.token(), &state.query())
                            .map_err(|error| error.to_string())?,
                    )
                } else {
                    source
                };
                let selection = state
                    .projection_selection_job(snapshot.clone())
                    .map_err(|error| error.to_string())?
                    .prepare()
                    .map_err(|error| error.to_string())?;
                retain_unique(&mut retained, Retained::Snapshot(snapshot.clone()));
                retain_unique(
                    &mut retained,
                    Retained::Selection(selection.selected.clone()),
                );
                Ok(Output::Projection(
                    control.clone(),
                    version,
                    WidgetryFileDialogReply::Snapshot {
                        snapshot,
                        state: status,
                        selection,
                    },
                ))
            }
            CpuWork::Selection(_, job) => {
                let prepared = job.prepare().map_err(|error| error.to_string())?;
                retain_unique(
                    &mut retained,
                    Retained::Selection(prepared.selected.clone()),
                );
                Ok(Output::Reply(
                    control.root,
                    WidgetryFileDialogReply::Selection(prepared),
                ))
            }
            CpuWork::Candidate(_, job) => match job.prepare() {
                Ok(candidate) => {
                    if let WidgetryFileDialogResult::Files(paths)
                    | WidgetryFileDialogResult::Directories(paths) = candidate.result()
                    {
                        retain_unique(&mut retained, Retained::Paths(paths.clone()));
                    }
                    Ok(Output::Candidate(control.clone(), candidate))
                }
                Err(error) => Ok(Output::Reply(
                    control.root,
                    WidgetryFileDialogReply::ValidationFailed {
                        token: control.token,
                        error,
                    },
                )),
            },
        })();
        let output = outcome.unwrap_or_else(|error| {
            Output::Reply(
                control.root,
                WidgetryFileDialogReply::Failed {
                    token: control.token,
                    error,
                },
            )
        });
        if let Err(error) = service.send(&service.output, output, Some(&control))
            && error.kind() != io::ErrorKind::Interrupted
        {
            service.fail(error.to_string());
        }
    }
}

fn io_loop(service: &Service) {
    loop {
        let work = {
            let mut pending = match service.pending.lock() {
                Ok(pending) => pending,
                Err(error) => {
                    service.fail(error.to_string());
                    return;
                }
            };
            if service.shutdown.load(Ordering::Acquire) {
                return;
            }
            pending.pop_first().map(|(_, work)| work)
        };
        let Some(work) = work else {
            // 通知保留在 bounded channel 中；stop/admission 即使发生在检查与 recv
            // 之间也不会丢失。等待前释放 pending lock，App 的 try_lock admission 不等待。
            if let Err(error) = service.ready_receiver.recv() {
                service.fail(error.to_string());
                return;
            }
            continue;
        };
        if work
            .control()
            .is_some_and(|control| control.stopped(service))
        {
            continue;
        }
        service.active_io.fetch_add(1, Ordering::Relaxed);
        let outcome = execute_io(service, work);
        service.active_io.fetch_sub(1, Ordering::Relaxed);
        if let Err(error) = outcome
            && error.kind() != io::ErrorKind::Interrupted
        {
            service.fail(error.to_string());
        }
    }
}

fn execute_io(service: &Service, work: IoWork) -> io::Result<()> {
    match work {
        IoWork::Directory(control, requested, query) => {
            if let Err(error) = enumerate(service, &control, requested, query) {
                if control.stopped(service) {
                    return Ok(());
                }
                return service.send(
                    &service.output,
                    Output::Reply(
                        control.root,
                        WidgetryFileDialogReply::Failed {
                            token: control.token,
                            error: error.to_string(),
                        },
                    ),
                    Some(&control),
                );
            }
            Ok(())
        }
        IoWork::Validate(control, candidate) => {
            let reply = match service.backend.0.validate(&candidate) {
                Ok(exists) => WidgetryFileDialogReply::Validated { candidate, exists },
                Err(error) => WidgetryFileDialogReply::ValidationFailed {
                    token: candidate.token(),
                    error: error.to_string(),
                },
            };
            service.send(
                &service.output,
                Output::Reply(control.root, reply),
                Some(&control),
            )
        }
        IoWork::Folder(control, request) => {
            let outcome = service
                .backend
                .0
                .create_directory(request.path())
                .map_err(|error| error.to_string());
            service.send(
                &service.output,
                Output::Reply(
                    control.root,
                    WidgetryFileDialogReply::FolderCreated { request, outcome },
                ),
                Some(&control),
            )
        }
        IoWork::Locations(control) => {
            let outcome = service
                .backend
                .0
                .locations()
                .and_then(|locations| {
                    if locations.len() > 128 {
                        Err(io::Error::other("location capacity exceeded"))
                    } else {
                        Ok(locations)
                    }
                })
                .map_err(|error| error.to_string());
            service.send(
                &service.output,
                Output::Locations(control.clone(), outcome),
                Some(&control),
            )
        }
        IoWork::Load(path) => {
            let outcome = service
                .backend
                .0
                .load_preferences(&path)
                .map_err(|error| error.to_string());
            service.send(&service.output, Output::Loaded(outcome), None)
        }
        IoWork::Save(path, scopes, revisions) => {
            let outcome = service
                .backend
                .0
                .save_preferences(&path, &scopes)
                .map_err(|error| error.to_string());
            service.send(&service.output, Output::Saved(revisions, outcome), None)
        }
    }
}

fn enumerate(
    service: &Service,
    control: &Control,
    requested: Option<PathBuf>,
    query: WidgetryFileDialogQuery,
) -> io::Result<()> {
    let path = service.backend.0.resolve_directory(requested.as_deref())?;
    if control.stopped(service) {
        return Err(interrupted());
    }
    let entries = service.backend.0.read_directory(&path)?;
    service.send(
        &service.output,
        Output::Reply(
            control.root,
            WidgetryFileDialogReply::Started {
                token: control.token,
                path: path.clone(),
            },
        ),
        Some(control),
    )?;
    let mut data = Vec::with_capacity(128);
    let mut errors = 0;
    let mut summary = Vec::new();
    let mut total = 0;
    let mut last_flush = Instant::now();
    for entry in entries {
        if control.stopped(service) {
            return Err(interrupted());
        }
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
        if data.len() == 128
            || (!data.is_empty() && last_flush.elapsed() >= Duration::from_millis(8))
        {
            service.send(
                &service.cpu,
                CpuWork::Batch(Batch {
                    control: control.clone(),
                    path: path.clone(),
                    query: query.clone(),
                    data: std::mem::take(&mut data),
                    finished: false,
                    errors,
                    summary: summary.clone(),
                }),
                Some(control),
            )?;
            last_flush = Instant::now();
        }
    }
    service.send(
        &service.cpu,
        CpuWork::Batch(Batch {
            control: control.clone(),
            path,
            query,
            data,
            finished: true,
            errors,
            summary,
        }),
        Some(control),
    )
}

impl Service {
    pub(crate) fn start(
        backend: WidgetryFileDialogBackend,
        wake: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    ) -> io::Result<Arc<Self>> {
        Self::start_with(backend, wake, |index, job| {
            thread::Builder::new()
                .name(format!("FileDialog-{index}"))
                .spawn(job)
        })
    }

    fn start_with(
        backend: WidgetryFileDialogBackend,
        wake: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
        mut spawn: impl FnMut(usize, Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> io::Result<Arc<Self>> {
        let service = Self::new(backend, wake);
        for index in 0..3 {
            let worker = service.clone();
            service.live.fetch_add(1, Ordering::Relaxed);
            let handle = match spawn(
                index,
                Box::new(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if index == 2 {
                            cpu_loop(&worker);
                        } else {
                            io_loop(&worker);
                        }
                    }));
                    if result.is_err() {
                        worker.fail("FileDialog worker terminated unexpectedly".into());
                        worker.stop();
                    }
                    worker.live.fetch_sub(1, Ordering::Release);
                }),
            ) {
                Ok(handle) => handle,
                Err(error) => {
                    service.live.fetch_sub(1, Ordering::Relaxed);
                    service.stop();
                    return Err(error);
                }
            };
            match service.handles.lock() {
                Ok(mut handles) => handles.push(handle),
                Err(error) => {
                    service.stop();
                    return Err(io::Error::other(error.to_string()));
                }
            }
        }
        Ok(service)
    }

    fn new(
        backend: WidgetryFileDialogBackend,
        wake: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    ) -> Arc<Self> {
        let (cpu, cpu_receiver) = crossbeam_channel::bounded(CAPACITY);
        let (output, replies) = crossbeam_channel::bounded(CAPACITY);
        let (ready, ready_receiver) = crossbeam_channel::bounded(2);
        Arc::new(Self {
            backend,
            pending: Mutex::new(BTreeMap::new()),
            ready,
            ready_receiver,
            cpu,
            cpu_receiver,
            output,
            replies,
            shutdown: AtomicBool::new(false),
            wake_pending: AtomicBool::new(false),
            wake,
            live: AtomicUsize::new(0),
            active_io: AtomicUsize::new(0),
            retained: AtomicUsize::new(0),
            fault: Mutex::new(None),
            handles: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn admit(&self, key: (Entity, u8), work: IoWork) -> Result<bool, String> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err("FileDialog service closed".into());
        }
        let mut pending = match self.pending.try_lock() {
            Ok(pending) => pending,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
            Err(error) => return Err(error.to_string()),
        };
        pending.retain(|_, pending_work| {
            !pending_work
                .control()
                .is_some_and(|control| control.stopped(self))
        });
        if pending.len() >= CAPACITY && !pending.contains_key(&key) {
            return Err("FileDialog service busy".into());
        }
        pending.insert(key, work);
        self.wake_io()?;
        Ok(true)
    }

    pub(crate) fn admit_cpu(&self, work: CpuWork) -> Result<bool, String> {
        match self.cpu.try_send(work) {
            Ok(()) => Ok(true),
            Err(crossbeam_channel::TrySendError::Full(_)) => Ok(false),
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                Err("active FileDialog CPU channel disconnected".into())
            }
        }
    }

    fn send<T>(
        &self,
        sender: &Sender<T>,
        mut value: T,
        control: Option<&Control>,
    ) -> io::Result<()> {
        loop {
            if self.shutdown.load(Ordering::Acquire)
                || control.is_some_and(|control| control.stopped(self))
            {
                return Err(interrupted());
            }
            match sender.send_timeout(value, Duration::from_millis(10)) {
                Ok(()) => {
                    self.notify();
                    return Ok(());
                }
                Err(SendTimeoutError::Timeout(returned)) => value = returned,
                Err(SendTimeoutError::Disconnected(_)) => {
                    return Err(io::Error::other("active FileDialog channel disconnected"));
                }
            }
        }
    }

    pub(crate) fn notify(&self) {
        if !self.wake_pending.swap(true, Ordering::AcqRel)
            && let Err(error) = (self.wake)()
        {
            self.fail(error);
        }
    }

    pub(crate) fn finish_poll(&self) {
        self.wake_pending.store(false, Ordering::Release);
        // 生产者可能在标记清除前入队但合并掉自己的 wake。
        // 清除后再次检查队列，既覆盖这个 race，也覆盖预算退出后的剩余 replies。
        let fault_pending = match self.fault.try_lock() {
            Ok(fault) => fault.is_some(),
            // producer 正在写 fault 时，必须安排消费者再次观察。
            Err(_) => true,
        };
        if !self.replies.is_empty() || fault_pending {
            self.notify();
        }
    }

    pub(crate) fn stop(&self) {
        self.shutdown.store(true, Ordering::Release);
        // 两个 I/O waiter 各保留一个通知；不会在主线程等待 pending mutex。
        for _ in 0..2 {
            if let Err(error) = self.wake_io() {
                self.fail(error);
            }
        }
    }

    fn wake_io(&self) -> Result<(), String> {
        match self.ready.try_send(()) {
            Ok(()) | Err(crossbeam_channel::TrySendError::Full(())) => Ok(()),
            Err(crossbeam_channel::TrySendError::Disconnected(())) => {
                Err("FileDialog I/O wake channel disconnected".into())
            }
        }
    }

    fn fail(&self, error: String) {
        let first = match self.fault.lock() {
            Ok(mut fault) => {
                if fault.is_none() {
                    *fault = Some(error.clone());
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                widgetry_error!(error = %error, "FileDialog 服务错误通道损坏");
                false
            }
        };
        if first {
            widgetry_error!(error, "FileDialog 后台服务失败");
            // 包括 backend panic 在首个回复之前发生的情况，让 reactive App 能观察 fault。
            self.notify();
        }
    }

    pub(crate) fn queued(&self) -> usize {
        self.pending
            .try_lock()
            .map_or(CAPACITY, |pending| pending.len())
            + self.cpu.len()
            + self.replies.len()
    }

    pub(crate) fn diagnose_shutdown(&self) {
        if self.active_io.load(Ordering::Relaxed) > 0 {
            widgetry_warn!(
                active = self.active_io.load(Ordering::Relaxed),
                "FileDialog 关闭时仍有在途 I/O，不等待 OS 返回"
            );
        }
    }
}

impl Control {
    pub(crate) fn stopped(&self, service: &Service) -> bool {
        self.cancelled.load(Ordering::Acquire) || service.shutdown.load(Ordering::Acquire)
    }
}

impl IoWork {
    fn control(&self) -> Option<&Control> {
        match self {
            Self::Directory(control, ..)
            | Self::Validate(control, ..)
            | Self::Folder(control, ..)
            | Self::Locations(control) => Some(control),
            Self::Load(_) | Self::Save(..) => None,
        }
    }
}

impl CpuWork {
    fn control(&self) -> &Control {
        match self {
            Self::Batch(batch) => &batch.control,
            Self::Projection(control, ..)
            | Self::Selection(control, ..)
            | Self::Candidate(control, ..) => control,
        }
    }
}

impl Retained {
    fn released(&self) -> bool {
        match self {
            // State 与后台 job 都持有 snapshot；不同 projection 的共享 data 可由
            // 多个 retained owner 持有，最后一个 owner 仍在 CPU worker 上释放。
            Self::Snapshot(snapshot) => Arc::strong_count(snapshot) == 1,
            Self::Selection(selection) => Arc::strong_count(selection) == 1,
            Self::Paths(paths) => Arc::strong_count(paths) == 1,
        }
    }
}

fn retain_unique(retained: &mut Vec<Retained>, item: Retained) {
    let duplicate = retained.iter().any(|existing| match (existing, &item) {
        (Retained::Snapshot(left), Retained::Snapshot(right)) => Arc::ptr_eq(left, right),
        (Retained::Selection(left), Retained::Selection(right)) => Arc::ptr_eq(left, right),
        (Retained::Paths(left), Retained::Paths(right)) => Arc::ptr_eq(left, right),
        _ => false,
    });
    if !duplicate {
        retained.push(item);
    }
}

#[cfg(test)]
// 测试断言用于保护容量和并发 contract，不适用生产 macro 禁令。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! state 为空/满 admission、wake 合并/重发与启动成功/失败。
    //! stimuli 为重复导航、producer 通知、预算退出、取消与 channel 断开。
    //! invariants 为容量固定、最新 pending 覆盖、App admission 不等待锁与失败 worker 不遗留。
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn stop_preserves_wakes_for_both_late_waiters_without_pending_lock() -> Result {
        let service = Service::new(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())));
        let _pending = service
            .pending
            .lock()
            .map_err(|error| BevyError::error(error.to_string()))?;
        service.stop();
        assert!(service.shutdown.load(Ordering::Acquire));
        assert!(service.ready_receiver.try_recv().is_ok());
        assert!(service.ready_receiver.try_recv().is_ok());
        assert!(service.ready_receiver.try_recv().is_err());
        Ok(())
    }

    #[test]
    fn worker_fault_notifies_without_a_reply_and_preserves_first_error() -> Result {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let service = Service::new(
            WidgetryFileDialogBackend::default(),
            Box::new(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }),
        );
        service.fail("first".into());
        service.fail("second".into());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert!(service.replies.is_empty());
        assert_eq!(
            service
                .fault
                .lock()
                .map_err(|error| BevyError::error(error.to_string()))?
                .as_deref(),
            Some("first")
        );
        service.finish_poll();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        Ok(())
    }

    #[test]
    fn pending_navigation_coalesces_and_admission_never_waits_for_lock() -> Result {
        let service = Service::new(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())));
        let root = Entity::from_bits(1);
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default())?;
        let control = Control {
            root,
            token: state.token(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        for path in ["C:/a", "C:/b", "C:/c"] {
            assert!(
                service
                    .admit(
                        (root, 0),
                        IoWork::Directory(control.clone(), Some(path.into()), state.query())
                    )
                    .map_err(BevyError::error)?
            );
        }
        let pending = service
            .pending
            .lock()
            .map_err(|error| BevyError::error(error.to_string()))?;
        assert_eq!(pending.len(), 1);
        assert!(
            matches!(pending.get(&(root, 0)), Some(IoWork::Directory(_, Some(path), _)) if path == &PathBuf::from("C:/c"))
        );
        assert!(
            !service
                .admit((root, 1), IoWork::Locations(control))
                .map_err(BevyError::error)?
        );
        Ok(())
    }

    #[test]
    fn pending_reply_rearms_coalesced_wake_and_cancel_differs_from_disconnect() -> io::Result<()> {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let service = Service::new(
            WidgetryFileDialogBackend::default(),
            Box::new(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }),
        );
        service
            .output
            .try_send(Output::Loaded(Ok(BTreeMap::new())))
            .map_err(io::Error::other)?;
        service.notify();
        service.notify();
        assert_eq!(count.load(Ordering::Relaxed), 1);
        service.finish_poll();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        service.replies.try_recv().map_err(io::Error::other)?;
        service.finish_poll();
        service.notify();
        assert_eq!(count.load(Ordering::Relaxed), 3);
        let (sender, receiver) = crossbeam_channel::bounded::<usize>(1);
        drop(receiver);
        assert_ne!(
            service
                .send(&sender, 1, None)
                .err()
                .map(|error| error.kind()),
            Some(io::ErrorKind::Interrupted)
        );
        service.stop();
        assert_eq!(
            service
                .send(&sender, 1, None)
                .err()
                .map(|error| error.kind()),
            Some(io::ErrorKind::Interrupted)
        );
        Ok(())
    }

    #[test]
    fn failed_second_worker_start_stops_the_first_worker() -> io::Result<()> {
        let (completed, receiver) = mpsc::channel();
        let result = Service::start_with(
            WidgetryFileDialogBackend::default(),
            Box::new(|| Ok(())),
            |index, job| {
                if index == 1 {
                    return Err(io::Error::other("controlled spawn failure"));
                }
                let completed = completed.clone();
                thread::Builder::new().spawn(move || {
                    job();
                    if let Err(error) = completed.send(()) {
                        eprintln!("completion receiver ended: {error}");
                    }
                })
            },
        );
        assert!(result.is_err());
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(io::Error::other)?;
        Ok(())
    }
}
