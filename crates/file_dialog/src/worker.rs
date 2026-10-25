use crate::*;
use bevy::prelude::*;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_warn};
use bevy_widgetry_job::JobContext;
use crossbeam_channel::{Receiver, SendTimeoutError, Sender};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const CAPACITY: usize = 64;
const RETAINED_CAPACITY: usize = 256;
// Directory streams must leave room for legacy projections while consumers drain their snapshots.
const DIRECTORY_RETAINED_CAPACITY: usize = RETAINED_CAPACITY - CAPACITY;

pub(crate) struct Service {
    backend: WidgetryFileDialogBackend,
    pending: Mutex<BTreeMap<(Entity, u8), IoWork>>,
    ready: Sender<()>,
    ready_receiver: Receiver<()>,
    cpu: Sender<CpuWork>,
    cpu_receiver: Mutex<Option<Receiver<CpuWork>>>,
    cpu_admission: Mutex<bool>,
    pub(crate) capacity: event_listener::Event,
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

pub(crate) enum IoWork {
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
    DirectoryBatch(
        Box<crate::directory::Batch>,
        futures_channel::oneshot::Sender<crate::directory::BatchReply>,
    ),
    Retain(
        Control,
        Vec<Retained>,
        futures_channel::oneshot::Sender<bool>,
    ),
}

pub(crate) enum CpuSendError {
    Busy(Box<CpuWork>),
    Closed,
    Fault(String),
}

struct CpuInbox {
    receiver: Receiver<CpuWork>,
    service: Arc<Service>,
}

impl Drop for CpuInbox {
    fn drop(&mut self) {
        self.service.close_cpu(&self.receiver);
    }
}

pub(crate) enum Output {
    Reply(Entity, WidgetryFileDialogReply),
    Snapshot(
        Entity,
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    ),
    Locations(Control, Result<Vec<WidgetryFileDialogLocation>, String>),
    Loaded(Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, String>),
    Saved(BTreeMap<String, u64>, Result<(), String>),
}

#[derive(Clone)]
pub(crate) enum Retained {
    Snapshot(Arc<WidgetryFileDialogSnapshot>),
    Selection(Arc<BTreeSet<WidgetryFileDialogEntryId>>),
    Paths(Arc<[PathBuf]>),
}

fn interrupted() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "FileDialog session cancelled")
}

pub(crate) fn cancel_if_requested<T: Send + Sync + 'static>(context: &JobContext<T>) -> bool {
    if context.is_cancelled() {
        context.acknowledge_cancel();
        true
    } else {
        false
    }
}

pub(crate) async fn retain_result<T: Send + Sync + 'static>(
    context: &JobContext<T>,
    service: &Service,
    control: &Control,
    items: Vec<Retained>,
) -> Result<bool, String> {
    loop {
        if cancel_if_requested(context) {
            return Ok(false);
        }
        if control.stopped(service) {
            return Err("FileDialog result retention service stopped".into());
        }
        let capacity = service.capacity.listen();
        let (reply, ready) = futures_channel::oneshot::channel();
        let mut work = CpuWork::Retain(control.clone(), items.clone(), reply);
        loop {
            if cancel_if_requested(context) {
                return Ok(false);
            }
            if control.stopped(service) {
                return Err("FileDialog result retention service stopped".into());
            }
            match service.try_send_cpu(work) {
                Ok(()) => break,
                Err(CpuSendError::Busy(returned)) => {
                    work = *returned;
                    futures_timer::Delay::new(Duration::from_millis(1)).await;
                }
                Err(CpuSendError::Closed) => {
                    return Err("FileDialog result retention channel closed".into());
                }
                Err(CpuSendError::Fault(error)) => return Err(error),
            }
        }
        // Publish only after the background owner holds large outputs through UI and App teardown.
        match ready.await {
            Ok(true) => return Ok(!cancel_if_requested(context)),
            Ok(false) => {
                bevy::tasks::futures_lite::future::race(context.cancelled(), capacity).await;
            }
            Err(error) => {
                if cancel_if_requested(context) {
                    return Ok(false);
                }
                return Err(error.to_string());
            }
        }
    }
}

fn cpu_loop(service: &Service, receiver: &Receiver<CpuWork>) {
    let mut retained = Vec::<Retained>::new();
    loop {
        let before = retained.len();
        retained.retain(|item| !item.released());
        if retained.len() < before {
            service.capacity.notify(usize::MAX);
        }
        service.retained.store(retained.len(), Ordering::Relaxed);
        if service.shutdown.load(Ordering::Acquire) {
            while receiver.try_recv().is_ok() {}
            while service.replies.try_recv().is_ok() {}
            if retained.is_empty() {
                break;
            }
            // World Components may still hold handles when the App drops the runtime Resource first.
            // Keep the final owner on the worker until other handles are released so large payloads are not destroyed during App Drop.
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        let work = match receiver.recv_timeout(Duration::from_millis(10)) {
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
        if let CpuWork::DirectoryBatch(mut batch, reply) = work {
            let result = if retained.len() >= DIRECTORY_RETAINED_CAPACITY {
                crate::directory::BatchReply::Retry(batch)
            } else {
                match batch.prepare() {
                    Ok(event) => {
                        if let crate::directory::DirectoryEvent::Snapshot { snapshot, .. } = &event
                        {
                            retain_unique(&mut retained, Retained::Snapshot(snapshot.clone()));
                        }
                        crate::directory::BatchReply::Prepared(event)
                    }
                    Err(error) => crate::directory::BatchReply::Failed(error),
                }
            };
            // The producer may already have been canceled and dropped its reply future.
            drop(reply.send(result));
            continue;
        }
        if let CpuWork::Retain(_, items, reply) = work {
            let accepted = retained.len() < RETAINED_CAPACITY - 3;
            if accepted {
                for item in items {
                    retain_unique(&mut retained, item);
                }
            }
            reply.send(accepted).unwrap_or_default();
            continue;
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
            // The bounded channel retains notifications even if stop or admission occurs between the check and recv.
            // Release the pending lock before waiting so App admission through try_lock never waits.
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
        // The worker must be the sole receiver so its exit also rejects late reply-bearing work.
        let cpu = service.take_cpu_receiver()?;
        for (index, cpu) in [None, None, Some(cpu)].into_iter().enumerate() {
            let worker = service.clone();
            service.live.fetch_add(1, Ordering::Relaxed);
            let handle = match spawn(
                index,
                Box::new(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if let Some(cpu) = cpu {
                            cpu_loop(&worker, &cpu.receiver);
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
            cpu_receiver: Mutex::new(Some(cpu_receiver)),
            cpu_admission: Mutex::new(true),
            capacity: event_listener::Event::new(),
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

    fn take_cpu_receiver(self: &Arc<Self>) -> io::Result<CpuInbox> {
        let receiver = self
            .cpu_receiver
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .take()
            .ok_or_else(|| io::Error::other("FileDialog CPU receiver already assigned"))?;
        Ok(CpuInbox {
            receiver,
            service: self.clone(),
        })
    }

    fn close_cpu(&self, receiver: &Receiver<CpuWork>) {
        let mut accepting = self.cpu_admission.lock().unwrap_or_else(|error| {
            self.fail(format!("FileDialog CPU admission poisoned: {error}"));
            error.into_inner()
        });
        *accepting = false;
        // Receiver Drop only disconnects Crossbeam's bounded queue while senders survive.
        // Exclude producers and destroy pending replies here to release awaiting directory Jobs.
        while receiver.try_recv().is_ok() {}
        self.capacity.notify(usize::MAX);
    }

    pub(crate) fn try_send_cpu(&self, work: CpuWork) -> Result<(), CpuSendError> {
        let accepting = match self.cpu_admission.try_lock() {
            Ok(accepting) => accepting,
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err(CpuSendError::Busy(Box::new(work)));
            }
            Err(error) => {
                let error = format!("FileDialog CPU admission poisoned: {error}");
                self.fail(error.clone());
                return Err(CpuSendError::Fault(error));
            }
        };
        self.enqueue_cpu(&accepting, work)
    }

    pub(crate) fn send_directory_batch(&self, work: CpuWork) -> Result<(), CpuSendError> {
        // Directory Jobs run on managed native threads and can wait for this short admission gate.
        // ECS admission uses try_send_cpu instead so it never waits for another owner.
        let accepting = self.cpu_admission.lock().map_err(|error| {
            let error = format!("FileDialog CPU admission poisoned: {error}");
            self.fail(error.clone());
            CpuSendError::Fault(error)
        })?;
        self.enqueue_cpu(&accepting, work)
    }

    fn enqueue_cpu(
        &self,
        accepting: &MutexGuard<'_, bool>,
        work: CpuWork,
    ) -> Result<(), CpuSendError> {
        if !**accepting || self.shutdown.load(Ordering::Acquire) {
            return Err(CpuSendError::Closed);
        }
        match self.cpu.try_send(work) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::TrySendError::Full(work)) => {
                Err(CpuSendError::Busy(Box::new(work)))
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => Err(CpuSendError::Closed),
        }
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
        // A producer may enqueue before the flag clears and coalesce its own wake.
        // Recheck the queue after clearing to cover that race and replies remaining after a budget exit.
        let fault_pending = match self.fault.try_lock() {
            Ok(fault) => fault.is_some(),
            // If a producer is writing a fault, schedule another consumer observation.
            Err(_) => true,
        };
        if !self.replies.is_empty() || fault_pending {
            self.notify();
        }
    }

    pub(crate) fn stop(&self) {
        self.shutdown.store(true, Ordering::Release);
        self.capacity.notify(usize::MAX);
        // Retain one notification per I/O waiter so the main thread never waits on the pending mutex.
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
            // Wake even when the backend panics before its first reply so a reactive App can observe the fault.
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
            Self::Folder(control, ..) | Self::Locations(control) => Some(control),
            Self::Load(_) | Self::Save(..) => None,
        }
    }
}

impl CpuWork {
    fn control(&self) -> &Control {
        match self {
            Self::DirectoryBatch(batch, _) => &batch.control,
            Self::Retain(control, ..) => control,
        }
    }
}

impl Retained {
    fn released(&self) -> bool {
        match self {
            // State and background jobs both retain snapshots, and projections can share data across multiple retained owners.
            // Release the final owner on the CPU worker regardless of which projection held it.
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
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    struct StopService(Arc<Service>);
    impl Drop for StopService {
        fn drop(&mut self) {
            self.0.stop();
        }
    }

    fn reply_batch() -> (
        CpuWork,
        futures_channel::oneshot::Receiver<crate::directory::BatchReply>,
    ) {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let (reply, ready) = futures_channel::oneshot::channel();
        let late = CpuWork::DirectoryBatch(
            Box::new(crate::directory::Batch {
                control: Control {
                    root: Entity::PLACEHOLDER,
                    token: state.token(),
                    cancelled: Arc::default(),
                },
                previous: None,
                path: PathBuf::from("C:/fixture"),
                query: state.query(),
                data: vec![],
                finished: true,
                errors: 0,
                summary: vec![],
            }),
            reply,
        );
        (late, ready)
    }

    #[test]
    fn directory_retention_pressure_retries_and_leaves_room_for_session_projections() {
        use crate::directory::BatchReply;
        let service =
            Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(()))).unwrap();
        let _stop = StopService(service.clone());
        let mut held = Vec::new();
        let mut retried = None;
        for _ in 0..(8 * 32 + 1) {
            let (work, ready) = reply_batch();
            assert!(service.send_directory_batch(work).is_ok());
            match bevy::tasks::block_on(ready).unwrap() {
                BatchReply::Prepared(event) => held.push(event),
                BatchReply::Retry(batch) => {
                    retried = Some(batch);
                    break;
                }
                BatchReply::Failed(error) => {
                    panic!("normal snapshot pressure must backpressure rather than fail: {error}")
                }
            }
        }
        assert!(
            held.len() + 8 * 3 <= RETAINED_CAPACITY,
            "directory backpressure must leave room for the eight sessions' legacy CPU results"
        );
        let mut pending =
            retried.expect("bounded retention must eventually backpressure directory batches");
        drop(held.pop());
        loop {
            let capacity = service.capacity.listen();
            let (reply, ready) = futures_channel::oneshot::channel();
            assert!(
                service
                    .send_directory_batch(CpuWork::DirectoryBatch(pending, reply))
                    .is_ok()
            );
            match bevy::tasks::block_on(ready).unwrap() {
                BatchReply::Prepared(_) => break,
                BatchReply::Retry(batch) => {
                    pending = batch;
                    bevy::tasks::block_on(capacity);
                }
                BatchReply::Failed(error) => {
                    panic!("released capacity must resume the directory: {error}")
                }
            }
        }
        service.stop();
    }

    #[test]
    fn cpu_admission_retries_contention_and_rejects_before_receiver_destruction() {
        let service = Service::new(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())));
        let cpu = service.take_cpu_receiver().unwrap();
        let (work, mut ready) = reply_batch();
        let gate = service.cpu_admission.lock().unwrap();
        let work = match service.try_send_cpu(work) {
            Err(CpuSendError::Busy(work)) => *work,
            _ => panic!("CPU admission must not wait for another gate owner"),
        };
        drop(gate);
        service.close_cpu(&cpu.receiver);
        assert!(matches!(
            service.try_send_cpu(work),
            Err(CpuSendError::Closed)
        ));
        assert!(ready.try_recv().is_err());
        assert!(cpu.receiver.is_empty());
    }

    #[test]
    fn cpu_exit_drains_a_late_batch_even_while_the_service_sender_survives() {
        let service = Service::new(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())));
        let cpu = service.take_cpu_receiver().unwrap();
        assert!(cpu.receiver.is_empty());
        let (late, mut ready) = reply_batch();
        assert!(service.cpu.try_send(late).is_ok());
        drop(cpu);
        assert!(
            ready.try_recv().is_err(),
            "CPU exit must release queued reply senders before the surviving Service sender is dropped"
        );
    }

    #[test]
    fn exited_cpu_worker_cannot_accept_a_late_batch_that_needs_a_reply() {
        let service =
            Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(()))).unwrap();
        service.stop();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while service.live.load(Ordering::Acquire) != 0 {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        let (late, ready) = reply_batch();
        assert!(
            matches!(
                service.cpu.try_send(late),
                Err(crossbeam_channel::TrySendError::Disconnected(_))
            ),
            "a late batch must not retain an unanswered sender after CPU shutdown"
        );
        assert!(bevy::tasks::block_on(ready).is_err());
    }

    #[test]
    fn directory_job_waits_for_its_cpu_batch_before_acknowledging_cancel() {
        use bevy_widgetry_job::{JobManager, JobMessage, JobOutcome, JobState};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for canceled in [false, true] {
            let manager = JobManager::default();
            let service = Service::new(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())));
            let cpu = service.take_cpu_receiver().unwrap();
            let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let control = Control {
                root: Entity::PLACEHOLDER,
                token: state.token(),
                cancelled: Arc::default(),
            };
            let (id, mut receiver) = crate::directory::spawn(
                &manager,
                WidgetryFileDialogBackend::default(),
                service.clone(),
                control,
                Some(directory.path().to_owned()),
                state.query(),
            )
            .unwrap();
            let work = cpu.receiver.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(manager.status(id).unwrap().state, JobState::Running);
            assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(_))));
            assert!(matches!(
                receiver.try_recv(),
                Err(bevy_widgetry_job::TryRecvError::Empty)
            ));
            if canceled {
                manager.cancel(id).unwrap();
                assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
                drop(work);
                assert!(matches!(
                    receiver.recv_blocking(),
                    Ok(JobMessage::Finished(JobOutcome::Canceled))
                ));
            } else {
                let CpuWork::DirectoryBatch(mut batch, reply) = work else {
                    panic!("expected directory CPU work");
                };
                let result = match batch.prepare() {
                    Ok(event) => crate::directory::BatchReply::Prepared(event),
                    Err(error) => crate::directory::BatchReply::Failed(error),
                };
                assert!(reply.send(result).is_ok());
                assert!(
                    matches!(receiver.recv_blocking(), Ok(JobMessage::Item(event)) if matches!(*event, crate::directory::DirectoryEvent::Snapshot { finished: true, .. }))
                );
                assert!(matches!(
                    receiver.recv_blocking(),
                    Ok(JobMessage::Finished(JobOutcome::Completed))
                ));
            }
            manager.remove(id).unwrap();
        }
    }

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
    fn pending_operations_coalesce_and_admission_never_waits_for_lock() -> Result {
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
                    .admit((root, 0), IoWork::Load(path.into()))
                    .map_err(BevyError::error)?
            );
        }
        let pending = service
            .pending
            .lock()
            .map_err(|error| BevyError::error(error.to_string()))?;
        assert_eq!(pending.len(), 1);
        assert!(
            matches!(pending.get(&(root, 0)), Some(IoWork::Load(path)) if path == &PathBuf::from("C:/c"))
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
