use crate::*;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_job::{JobContext, JobId, JobManager, JobOptions, JobOutcome, SubmitError};
use crossbeam_channel::{Receiver, Sender};
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, MutexGuard, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::Duration;

const CAPACITY: usize = 64;
const RETAINED_CAPACITY: usize = 256;
const DIRECTORY_RETAINED_CAPACITY: usize = RETAINED_CAPACITY - CAPACITY;

pub(crate) struct Retention {
    cpu: Sender<CpuWork>,
    cpu_receiver: Mutex<Option<Receiver<CpuWork>>>,
    cpu_admission: Mutex<bool>,
    pub(crate) capacity: event_listener::Event,
    pub(crate) shutdown: AtomicBool,
    pub(crate) live: AtomicUsize,
    pub(crate) retained: AtomicUsize,
    pub(crate) fault: Mutex<Option<String>>,
    manager: JobManager,
    id: OnceLock<JobId>,
}

#[derive(Clone)]
pub(crate) struct Control {
    pub(crate) root: Entity,
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) cancelled: Arc<AtomicBool>,
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
    service: Arc<Retention>,
}

impl Drop for CpuInbox {
    fn drop(&mut self) {
        self.service.close_cpu(&self.receiver);
    }
}

#[derive(Clone)]
pub(crate) enum Retained {
    Snapshot(Arc<WidgetryFileDialogSnapshot>),
    Selection(Arc<BTreeSet<WidgetryFileDialogEntryId>>),
    Paths(Arc<[PathBuf]>),
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
    service: &Retention,
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

fn run(
    context: &JobContext<()>,
    service: &Retention,
    receiver: &Receiver<CpuWork>,
) -> Result<(), String> {
    let mut retained = Vec::<Retained>::new();
    loop {
        if context.is_cancelled() {
            service.shutdown.store(true, Ordering::Release);
            service.capacity.notify(usize::MAX);
        }
        let before = retained.len();
        retained.retain(|item| !item.released());
        if retained.len() < before {
            service.capacity.notify(usize::MAX);
        }
        service.retained.store(retained.len(), Ordering::Relaxed);
        if service.shutdown.load(Ordering::Acquire) {
            while receiver.try_recv().is_ok() {}
            if retained.is_empty() {
                context.acknowledge_cancel();
                return Ok(());
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
                return Err("FileDialog retention channel disconnected".into());
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

impl Retention {
    pub(crate) fn start(manager: &JobManager) -> Result<Arc<Self>, SubmitError> {
        let service = Self::new(manager.clone());
        let inbox = service
            .take_cpu_receiver()
            .map_err(|error| SubmitError::ThreadSpawn(error.to_string()))?;
        let worker = service.clone();
        let (id, receiver) = manager.spawn_thread::<(), _, String>(
            JobOptions {
                label: Some("FileDialog snapshot lifetime".into()),
                ..Default::default()
            },
            move |context| {
                struct Running(Arc<Retention>);
                impl Drop for Running {
                    fn drop(&mut self) {
                        self.0.live.store(0, Ordering::Release);
                    }
                }
                worker.live.store(1, Ordering::Release);
                let _running = Running(worker.clone());
                run(&context, &worker, &inbox.receiver)
            },
        )?;
        drop(receiver);
        // Only the starter writes the identity, before publishing the service to callers.
        if service.id.set(id).is_err() {
            service.stop();
            return Err(SubmitError::ThreadSpawn(
                "retention identity already assigned".into(),
            ));
        }
        Ok(service)
    }

    fn new(manager: JobManager) -> Arc<Self> {
        let (cpu, cpu_receiver) = crossbeam_channel::bounded(CAPACITY);
        Arc::new(Self {
            cpu,
            cpu_receiver: Mutex::new(Some(cpu_receiver)),
            cpu_admission: Mutex::new(true),
            capacity: event_listener::Event::new(),
            shutdown: AtomicBool::new(false),
            live: AtomicUsize::new(0),
            retained: AtomicUsize::new(0),
            fault: Mutex::new(None),
            manager,
            id: OnceLock::new(),
        })
    }

    pub(crate) fn active(&self) -> bool {
        self.id.get().is_some_and(|id| {
            self.manager
                .status(*id)
                .is_ok_and(|status| !status.state.is_finished())
        })
    }
    pub(crate) fn error(&self) -> Result<Option<String>, String> {
        let fault = self
            .fault
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        if fault.is_some() {
            return Ok(fault);
        }
        match self.id.get().map(|id| self.manager.outcome(*id)) {
            Some(Ok(Some(JobOutcome::Failed(error)))) => Ok(Some(error.to_string())),
            Some(Err(error)) => Err(error.to_string()),
            _ => Ok(None),
        }
    }
    pub(crate) fn stop(&self) {
        if let Some(id) = self.id.get()
            && let Err(error) = self.manager.cancel(*id)
        {
            widgetry_error!(%error, "FileDialog 取消快照生命周期 Job 失败");
        }
        self.shutdown.store(true, Ordering::Release);
        self.capacity.notify(usize::MAX);
    }
    pub(crate) fn queued(&self) -> usize {
        self.cpu.len()
    }
    fn fail(&self, error: String) {
        match self.fault.lock() {
            Ok(mut fault) if fault.is_none() => {
                widgetry_error!(%error, "FileDialog 快照生命周期服务失败");
                *fault = Some(error);
            }
            Ok(_) => {}
            Err(error) => {
                widgetry_error!(%error, "FileDialog 快照错误通道损坏");
            }
        }
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
        // Receiver teardown excludes late producers before releasing their reply senders.
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
}

impl Control {
    pub(crate) fn stopped(&self, service: &Retention) -> bool {
        self.cancelled.load(Ordering::Acquire) || service.shutdown.load(Ordering::Acquire)
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
            // Release the final owner on the retention thread regardless of which projection held it.
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

    struct StopRetention(Arc<Retention>);
    impl Drop for StopRetention {
        fn drop(&mut self) {
            self.0.stop();
        }
    }

    fn test_manager() -> JobManager {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        JobManager::default()
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
    fn canceled_lifetime_job_keeps_the_last_owner_until_ui_handles_are_released() {
        let manager = test_manager();
        let service = Retention::start(&manager).unwrap();
        let stop = StopRetention(service.clone());
        let paths: Arc<[PathBuf]> = vec![PathBuf::from("C:/held")].into();
        let state = WidgetryFileDialogState::new(Default::default()).unwrap();
        let control = Control {
            root: Entity::PLACEHOLDER,
            token: state.token(),
            cancelled: Arc::default(),
        };
        let (reply, ready) = futures_channel::oneshot::channel();
        assert!(
            service
                .try_send_cpu(CpuWork::Retain(
                    control,
                    vec![Retained::Paths(paths.clone())],
                    reply
                ))
                .is_ok()
        );
        assert!(bevy::tasks::block_on(ready).unwrap());
        let id = *service.id.get().unwrap();
        manager.cancel(id).unwrap();
        assert_eq!(
            manager.status(id).unwrap().state,
            bevy_widgetry_job::JobState::Stopping
        );
        assert!(manager.outcome(id).unwrap().is_none());
        assert!(Arc::strong_count(&paths) >= 2);
        drop(paths);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Canceled));
        drop(stop);
        manager.remove(id).unwrap();
    }

    #[test]
    fn directory_retention_pressure_retries_and_leaves_room_for_session_projections() {
        use crate::directory::BatchReply;
        let service = Retention::start(&test_manager()).unwrap();
        let _stop = StopRetention(service.clone());
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
        let service = Retention::new(JobManager::default());
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
        let service = Retention::new(JobManager::default());
        let cpu = service.take_cpu_receiver().unwrap();
        assert!(cpu.receiver.is_empty());
        let (late, mut ready) = reply_batch();
        assert!(service.cpu.try_send(late).is_ok());
        drop(cpu);
        assert!(
            ready.try_recv().is_err(),
            "CPU exit must release queued reply senders before the surviving Retention sender is dropped"
        );
    }

    #[test]
    fn exited_cpu_worker_cannot_accept_a_late_batch_that_needs_a_reply() {
        let service = Retention::start(&test_manager()).unwrap();
        service.stop();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while service.active() {
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
            let service = Retention::new(JobManager::default());
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
}
