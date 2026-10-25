use std::{
    collections::BTreeMap,
    fmt::Display,
    future::Future,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Wake, Waker},
};

use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, Task, TaskPool};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use event_listener::Event;
use futures_util::FutureExt;
use tracing::Instrument;

use crate::channel::{Data, ResultChannel};
use crate::lifecycle::Lifecycle;
use crate::shutdown::ShutdownData;
use crate::*;

#[derive(Clone, Default)]
pub struct JobManager {
    inner: Arc<Mutex<Registry>>,
}

struct Registry {
    last_id: u64,
    buffer_capacity: usize,
    records: BTreeMap<JobId, Record>,
    shutdown: Option<Arc<Mutex<ShutdownData>>>,
}

struct Record {
    shared: Arc<Shared>,
    task: Arc<Mutex<Option<Task<JobOutcome>>>>,
    channel: Arc<dyn ResultChannel>,
}

pub(crate) struct Shared {
    pub lifecycle: Mutex<Lifecycle>,
    pub signal: Arc<Signal>,
    pub cancel: Event,
    pub completion: Mutex<ExecutionCompletion>,
}

#[derive(Clone, Copy)]
pub(crate) enum ExecutionCompletion {
    Unfinished,
    Finished(std::time::Instant),
    Rejected,
}

impl Shared {
    pub(crate) fn reject_startup(&self) {
        *lock(&self.completion) = ExecutionCompletion::Rejected;
    }
}

#[derive(Default)]
pub(crate) struct Signal(pub Event);

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.0.notify(usize::MAX);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.notify(usize::MAX);
    }
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| {
        widgetry_error!("Job 内部锁中毒，恢复受保护状态");
        error.into_inner()
    })
}

impl JobManager {
    pub fn begin_shutdown(
        &self,
        timeout: std::time::Duration,
    ) -> Result<ShutdownHandle, SubmitError> {
        let mut registry = lock(&self.inner);
        if let Some(shared) = &registry.shutdown {
            return Ok(ShutdownHandle::new(self.clone(), shared.clone()));
        }
        let remaining = registry.remaining();
        let pool = if remaining.is_empty() {
            None
        } else {
            Some(
                IoTaskPool::try_get()
                    .map(|pool| &**pool)
                    .or_else(|| AsyncComputeTaskPool::try_get().map(|pool| &**pool))
                    .ok_or_else(|| {
                        widgetry_error!("Job 关闭协调所需任务池尚未初始化");
                        SubmitError::PoolUnavailable(JobKind::Io)
                    })?,
            )
        };
        let shared = Arc::new(Mutex::new(ShutdownData::new(timeout)));
        registry.shutdown = Some(shared.clone());
        for record in registry.records.values_mut() {
            if lock(&record.shared.lifecycle).cancel() == CancelResult::CancelRequested {
                record.shared.cancel.notify(usize::MAX);
            }
            if !lock(&record.shared.lifecycle).status.state.is_finished() {
                lock(&shared).track(record.shared.clone());
            }
        }
        registry.advance_shutdown(&shared, std::time::Instant::now);
        if let Some(pool) = pool {
            let manager = self.clone();
            let state = shared.clone();
            let task = pool.spawn(async move {
                loop {
                    manager.update_shutdown();
                    if lock(&state).status.phase == ShutdownPhase::Finished {
                        break;
                    }
                    futures_timer::Delay::new(std::time::Duration::from_millis(1)).await;
                }
            });
            lock(&shared).task = Some(task);
        }
        Ok(ShutdownHandle::new(self.clone(), shared))
    }

    pub(crate) fn update_shutdown(&self) {
        let mut registry = lock(&self.inner);
        let Some(shared) = registry.shutdown.clone() else {
            return;
        };
        registry.advance_shutdown(&shared, std::time::Instant::now);
        let mut shutdown = lock(&shared);
        if let Some(task) = &mut shutdown.task
            && task.is_finished()
            && Pin::new(task)
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_ready()
        {
            shutdown.task = None;
        }
    }

    pub fn spawn_thread<T, F, E>(
        &self,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Result<(), E> + Send + 'static,
        E: Display + Send + 'static,
    {
        let pool = IoTaskPool::try_get().ok_or_else(|| {
            widgetry_error!("Job 线程收割所需 I/O 任务池尚未初始化");
            SubmitError::PoolUnavailable(JobKind::Io)
        })?;
        self.spawn_thread_on(pool, std::thread::Builder::new(), options, work)
    }

    fn spawn_thread_on<T, F, E>(
        &self,
        pool: &TaskPool,
        builder: std::thread::Builder,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Result<(), E> + Send + 'static,
        E: Display + Send + 'static,
    {
        if options
            .label
            .as_ref()
            .is_some_and(|label| label.contains('\0'))
        {
            widgetry_error!("Job 线程名称包含空字符");
            return Err(SubmitError::ThreadSpawn(
                "thread name contains a nul byte".into(),
            ));
        }
        let (id, shared, slot, context, receiver) = {
            let mut registry = lock(&self.inner);
            let (record, context, receiver) =
                registry.register::<T>(self.clone(), JobKind::Thread, options)?;
            let id = lock(&record.shared.lifecycle).status.id;
            (
                id,
                record.shared.clone(),
                record.task.clone(),
                context,
                receiver,
            )
        };
        let name = lock(&shared.lifecycle)
            .status
            .label
            .clone()
            .unwrap_or_else(|| format!("widgetry-job-{}", id.get()));
        let exited = Arc::new(Event::new());
        let listener = exited.listen();
        let state = shared.clone();
        let span =
            tracing::info_span!(target: "bevy_widgetry", "job", job_id = id.get(), kind = "Thread");
        let thread = match builder.name(name).spawn(move || {
            let _exit = ThreadExit(exited);
            span.in_scope(|| {
                let run = lock(&state.lifecycle).start();
                let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
                    if run {
                        work(context).map_err(|error| error.to_string())
                    } else {
                        Ok(())
                    }
                }));
                lock(&state.lifecycle).end();
                job_outcome(result, id)
            })
        }) {
            Ok(thread) => thread,
            Err(error) => {
                shared.reject_startup();
                lock(&self.inner).records.remove(&id);
                widgetry_error!(job_id = id.get(), %error, "Job 线程创建失败");
                return Err(SubmitError::ThreadSpawn(error.to_string()));
            }
        };
        let keep_alive = self.inner.clone();
        let task = pool.spawn(async move {
            listener.await;
            let outcome = collect_thread(thread, id).await;
            *lock(&shared.completion) = ExecutionCompletion::Finished(std::time::Instant::now());
            drop(keep_alive);
            outcome
        });
        *lock(&slot) = Some(task);
        self.reap(id);
        Ok((id, receiver))
    }

    pub fn cancel(&self, id: JobId) -> Result<CancelResult, JobAccessError> {
        let mut registry = lock(&self.inner);
        let record = registry
            .records
            .get_mut(&id)
            .ok_or(JobAccessError::NotFound)?;
        record.reap();
        let result = lock(&record.shared.lifecycle).cancel();
        if result == CancelResult::CancelRequested {
            record.shared.cancel.notify(usize::MAX);
        }
        Ok(result)
    }

    pub fn with_buffer_capacity(buffer_capacity: usize) -> Result<Self, SubmitError> {
        if buffer_capacity == 0 {
            widgetry_error!("Job 缓冲容量必须大于零");
            return Err(SubmitError::ZeroCapacity);
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(Registry {
                buffer_capacity,
                ..Default::default()
            })),
        })
    }

    pub fn subscribe<T: Send + Sync + 'static>(
        &self,
        id: JobId,
    ) -> Result<JobReceiver<T>, JobAccessError> {
        let mut registry = lock(&self.inner);
        let record = registry
            .records
            .get_mut(&id)
            .ok_or(JobAccessError::NotFound)?;
        record.reap();
        let state = lock(&record.shared.lifecycle).status.state;
        if !matches!(state, JobState::Pending | JobState::Running) {
            return Err(JobAccessError::NotSubscribable(state));
        }
        let data = record
            .channel
            .as_any()
            .downcast_ref::<Data<T>>()
            .ok_or_else(|| {
                widgetry_error!(job_id = id.get(), "Job 订阅结果类型不匹配");
                JobAccessError::TypeMismatch
            })?;
        Ok(JobReceiver::new(
            self.clone(),
            record.shared.clone(),
            data.sender.new_receiver(),
        ))
    }

    pub fn spawn_io<T, F, Fut, E>(
        &self,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Display + Send + 'static,
    {
        self.spawn(JobKind::Io, options, work)
    }

    pub fn spawn_cpu<T, F, Fut, E>(
        &self,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Display + Send + 'static,
    {
        self.spawn(JobKind::Cpu, options, work)
    }

    fn spawn<T, F, Fut, E>(
        &self,
        kind: JobKind,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Display + Send + 'static,
    {
        let pool = match kind {
            JobKind::Io => IoTaskPool::try_get().map(|pool| &**pool),
            JobKind::Cpu => AsyncComputeTaskPool::try_get().map(|pool| &**pool),
            JobKind::Thread => None,
        }
        .ok_or_else(|| {
            widgetry_error!(?kind, "Job 所需任务池尚未初始化");
            SubmitError::PoolUnavailable(kind)
        })?;
        self.spawn_on(pool, kind, options, work)
    }

    fn spawn_on<T, F, Fut, E>(
        &self,
        pool: &TaskPool,
        kind: JobKind,
        options: JobOptions,
        work: F,
    ) -> Result<(JobId, JobReceiver<T>), SubmitError>
    where
        T: Send + Sync + 'static,
        F: FnOnce(JobContext<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Display + Send + 'static,
    {
        let mut registry = lock(&self.inner);
        let (record, context, receiver) = registry.register::<T>(self.clone(), kind, options)?;
        let shared = record.shared.clone();
        let id = lock(&shared.lifecycle).status.id;
        let span = tracing::info_span!(target: "bevy_widgetry", "job", job_id = id.get(), ?kind);
        let task = pool.spawn(
            async move {
                let run = lock(&shared.lifecycle).start();
                let result = AssertUnwindSafe(async move {
                    if run {
                        work(context).await.map_err(|error| error.to_string())
                    } else {
                        Ok(())
                    }
                })
                .catch_unwind()
                .await;
                lock(&shared.lifecycle).end();
                let outcome = job_outcome(result, id);
                *lock(&shared.completion) =
                    ExecutionCompletion::Finished(std::time::Instant::now());
                outcome
            }
            .instrument(span),
        );
        *lock(&record.task) = Some(task);
        record.reap();
        Ok((id, receiver))
    }

    pub fn status(&self, id: JobId) -> Result<JobStatus, JobAccessError> {
        let mut registry = lock(&self.inner);
        let record = registry
            .records
            .get_mut(&id)
            .ok_or(JobAccessError::NotFound)?;
        record.reap();
        Ok(lock(&record.shared.lifecycle).status.clone())
    }

    pub fn outcome(&self, id: JobId) -> Result<Option<JobOutcome>, JobAccessError> {
        Ok(self.status(id)?.outcome)
    }

    pub fn remove(&self, id: JobId) -> Result<JobStatus, JobAccessError> {
        let mut registry = lock(&self.inner);
        let record = registry
            .records
            .get_mut(&id)
            .ok_or(JobAccessError::NotFound)?;
        record.reap();
        let status = lock(&record.shared.lifecycle).status.clone();
        if !status.state.is_finished() {
            return Err(JobAccessError::NotFinished(status.state));
        }
        registry.records.remove(&id);
        Ok(status)
    }

    pub(crate) fn reap(&self, id: JobId) {
        if let Some(record) = lock(&self.inner).records.get_mut(&id) {
            record.reap();
        }
    }
}

impl Registry {
    fn advance_shutdown(
        &mut self,
        shared: &Mutex<ShutdownData>,
        now: impl FnOnce() -> std::time::Instant,
    ) {
        let remaining = self.remaining();
        let now = now();
        lock(shared).advance(now, remaining);
    }

    fn register<T: Send + Sync + 'static>(
        &mut self,
        manager: JobManager,
        kind: JobKind,
        options: JobOptions,
    ) -> Result<(&mut Record, JobContext<T>, JobReceiver<T>), SubmitError> {
        if self.shutdown.is_some() {
            return Err(SubmitError::ShuttingDown);
        }
        let capacity = options.buffer_capacity.unwrap_or(self.buffer_capacity);
        if capacity == 0 {
            widgetry_error!("Job 缓冲容量必须大于零");
            return Err(SubmitError::ZeroCapacity);
        }
        let id = self.allocate_id()?;
        let shared = Arc::new(Shared {
            lifecycle: Mutex::new(Lifecycle::new(JobStatus {
                id,
                kind,
                state: JobState::Pending,
                outcome: None,
                label: options.label,
            })),
            signal: Arc::default(),
            cancel: Event::new(),
            completion: Mutex::new(ExecutionCompletion::Unfinished),
        });
        let (data, initial) = Data::<T>::new(capacity);
        let receiver = JobReceiver::new(manager, shared.clone(), initial);
        let context = JobContext::new(shared.clone(), data.clone());
        let record = self.records.entry(id).or_insert(Record {
            shared,
            task: Arc::default(),
            channel: data,
        });
        Ok((record, context, receiver))
    }

    fn allocate_id(&mut self) -> Result<JobId, SubmitError> {
        self.last_id = self.last_id.checked_add(1).ok_or_else(|| {
            widgetry_error!("Job ID 已耗尽");
            SubmitError::IdExhausted
        })?;
        Ok(JobId(self.last_id))
    }

    fn remaining(&mut self) -> Vec<JobStatus> {
        self.records
            .values_mut()
            .filter_map(|record| {
                record.reap();
                let status = lock(&record.shared.lifecycle).status.clone();
                (!status.state.is_finished()).then_some(status)
            })
            .collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            last_id: 0,
            buffer_capacity: 32,
            records: BTreeMap::new(),
            shutdown: None,
        }
    }
}

impl Record {
    fn reap(&mut self) {
        let mut slot = lock(&self.task);
        let Some(task) = &mut *slot else {
            return;
        };
        let waker = Waker::from(self.shared.signal.clone());
        if let Poll::Ready(outcome) = Pin::new(task).poll(&mut Context::from_waker(&waker)) {
            *slot = None;
            self.channel.close();
            lock(&self.shared.lifecycle).finish(outcome);
            self.shared.signal.0.notify(usize::MAX);
        }
    }
}

struct ThreadExit(Arc<Event>);

async fn collect_thread(thread: std::thread::JoinHandle<JobOutcome>, id: JobId) -> JobOutcome {
    let mut failure = None;
    loop {
        let error = match win_custody::wait(&thread, Some(std::time::Duration::ZERO)) {
            Ok(win_custody::WaitOutcome::Signaled) => break,
            Ok(win_custody::WaitOutcome::TimedOut) => None,
            Ok(win_custody::WaitOutcome::Abandoned) => {
                Some("thread wait returned abandoned".to_owned())
            }
            Err(error) => Some(error.to_string()),
        };
        if let Some(error) = error
            && failure.is_none()
        {
            widgetry_error!(job_id = id.get(), %error, "Job 线程退出查询失败，保留句柄等待回收");
            failure = Some(error);
        }
        futures_timer::Delay::new(std::time::Duration::from_millis(1)).await;
    }
    let outcome = match thread.join() {
        Ok(outcome) => outcome,
        Err(payload) => job_outcome(Err(payload), id),
    };
    if let Some(error) = failure {
        widgetry_info!(job_id = id.get(), "Job 线程退出查询已恢复并完成回收");
        JobOutcome::Failed(JobError::Infrastructure(format!(
            "{error}; thread outcome: {outcome:?}"
        )))
    } else {
        outcome
    }
}

impl Drop for ThreadExit {
    fn drop(&mut self) {
        self.0.notify(usize::MAX);
    }
}

fn job_outcome(
    result: Result<Result<(), String>, Box<dyn std::any::Any + Send>>,
    id: JobId,
) -> JobOutcome {
    match result {
        Ok(Ok(())) => JobOutcome::Completed,
        Ok(Err(error)) => JobOutcome::Failed(JobError::Business(error)),
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_owned())
                })
                .unwrap_or_else(|| "non-string panic payload".to_owned());
            widgetry_error!(job_id = id.get(), error = %message, "Job 执行体发生 panic");
            JobOutcome::Failed(JobError::Panic(message))
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    struct TlsWait {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl Drop for TlsWait {
        #[allow(clippy::unwrap_used)]
        fn drop(&mut self) {
            self.entered.send(()).unwrap();
            self.release
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        }
    }

    thread_local! { static TLS_WAIT: std::cell::RefCell<Option<TlsWait>> = const { std::cell::RefCell::new(None) }; }

    #[test]
    fn tls_cleanup_does_not_block_a_single_worker_io_pool() {
        use std::{
            sync::mpsc,
            time::{Duration, Instant},
        };
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(1).build();
        let (entered, cleaning) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            TLS_WAIT.with(|slot| {
                *slot.borrow_mut() = Some(TlsWait {
                    entered,
                    release: released,
                })
            });
            JobOutcome::Completed
        });
        cleaning.recv_timeout(Duration::from_secs(5)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !thread.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let (polling, polled) = mpsc::channel();
        let collector = pool.spawn(async move {
            polling.send(()).unwrap();
            collect_thread(thread, JobId(1)).await
        });
        polled.recv_timeout(Duration::from_secs(5)).unwrap();
        let (progress, progressed) = mpsc::channel();
        let unblock = release.clone();
        let other_io = pool.spawn(async move {
            progress.send(()).unwrap();
            unblock.send(()).unwrap_or_default();
        });
        let independent_progress = progressed.recv_timeout(Duration::from_secs(1));
        if independent_progress.is_err() {
            release.send(()).unwrap();
        }
        bevy::tasks::block_on(other_io);
        assert_eq!(bevy::tasks::block_on(collector), JobOutcome::Completed);
        assert!(
            independent_progress.is_ok(),
            "TLS cleanup blocked the only I/O worker"
        );
    }

    #[test]
    fn exhausted_ids_never_wrap_or_reuse() {
        let mut registry = Registry {
            last_id: u64::MAX - 1,
            ..Default::default()
        };
        assert_eq!(registry.allocate_id().unwrap().get(), u64::MAX);
        assert_eq!(registry.allocate_id(), Err(SubmitError::IdExhausted));
        assert_eq!(registry.allocate_id(), Err(SubmitError::IdExhausted));
        assert_eq!(registry.last_id, u64::MAX);
    }

    #[test]
    fn shutdown_reads_the_clock_after_initial_collection() {
        use std::time::{Duration, Instant};
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(1).build();
        let manager = JobManager::default();
        let release = Event::new();
        let listener = release.listen();
        let (id, _receiver) = manager
            .spawn_on::<(), _, _, String>(
                &pool,
                JobKind::Io,
                JobOptions::default(),
                |_| async move {
                    listener.await;
                    Ok(())
                },
            )
            .unwrap();
        let mut registry = lock(&manager.inner);
        let shared = registry.records[&id].shared.clone();
        let shutdown = Mutex::new(ShutdownData::new(Duration::from_secs(1)));
        lock(&shared.lifecycle).cancel();
        lock(&shutdown).track(shared.clone());
        release.notify(usize::MAX);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !lock(&registry.records[&id].task)
            .as_ref()
            .is_some_and(|task| task.is_finished())
        {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        registry.advance_shutdown(&shutdown, || {
            assert!(
                lock(&shared.lifecycle).status.state.is_finished(),
                "clock read before collection"
            );
            Instant::now()
        });
    }

    #[test]
    fn pending_is_registered_before_executor_gets_a_chance() {
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(0).build();
        let manager = JobManager::default();
        let (id, mut receiver) = manager
            .spawn_on::<(), _, _, String>(&pool, JobKind::Io, JobOptions::default(), |_| async {
                panic!("this executor must not run");
            })
            .unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Pending);
        let mut second = manager.subscribe::<()>(id).unwrap();
        assert_eq!(second.try_recv().unwrap_err(), TryRecvError::Empty);
        assert_eq!(
            manager.remove(id),
            Err(JobAccessError::NotFinished(JobState::Pending))
        );
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Empty);
    }

    #[test]
    fn cancel_before_execution_skips_business_body_and_collects_handle() {
        use std::{
            sync::atomic::{AtomicBool, Ordering},
            sync::mpsc,
            time::Duration,
        };
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(1).build();
        let (occupied, waiting) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let blocker = pool.spawn(async move {
            occupied.send(()).unwrap();
            blocked.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let manager = JobManager::default();
        let invoked = Arc::new(AtomicBool::new(false));
        let observed = invoked.clone();
        let (id, mut receiver) = manager
            .spawn_on::<(), _, _, String>(
                &pool,
                JobKind::Io,
                JobOptions::default(),
                move |_| async move {
                    invoked.store(true, Ordering::SeqCst);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Pending);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::CancelRequested);
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        assert_eq!(manager.cancel(id).unwrap(), CancelResult::AlreadyStopping);
        release.send(()).unwrap();
        bevy::tasks::block_on(blocker);
        assert!(matches!(
            receiver.recv_blocking(),
            Ok(JobMessage::Finished(JobOutcome::Canceled))
        ));
        assert!(!observed.load(Ordering::SeqCst));
        assert!(lock(&lock(&manager.inner).records[&id].task).is_none());
    }

    #[test]
    fn thread_creation_failure_removes_pending_record_without_reusing_id() {
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(1).build();
        let manager = JobManager::default();
        let result = manager.spawn_thread_on::<(), _, String>(
            &pool,
            std::thread::Builder::new().stack_size(1usize << 63),
            JobOptions::default(),
            |_| panic!("must not run"),
        );
        assert!(matches!(result, Err(SubmitError::ThreadSpawn(_))));
        assert!(lock(&manager.inner).records.is_empty());
        assert_eq!(lock(&manager.inner).last_id, 1);
        let (id, mut receiver) = manager
            .spawn_thread_on::<(), _, String>(
                &pool,
                std::thread::Builder::new(),
                JobOptions::default(),
                |_| Ok(()),
            )
            .unwrap();
        assert_eq!(id.get(), 2);
        assert!(matches!(
            receiver.recv_blocking(),
            Ok(JobMessage::Finished(JobOutcome::Completed))
        ));
        assert!(lock(&lock(&manager.inner).records[&id].task).is_none());
    }

    #[test]
    fn thread_reaper_retains_core_after_last_external_handle_drop() {
        use std::{
            sync::mpsc,
            time::{Duration, Instant},
        };
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(1).build();
        let manager = JobManager::default();
        let weak = Arc::downgrade(&manager.inner);
        let (started, running) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let (_, receiver) = manager
            .spawn_thread_on::<(), _, String>(
                &pool,
                std::thread::Builder::new(),
                JobOptions::default(),
                move |_| {
                    started.send(()).unwrap();
                    blocked.recv_timeout(Duration::from_secs(5)).unwrap();
                    Ok(())
                },
            )
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(receiver);
        drop(manager);
        assert!(weak.upgrade().is_some());
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while weak.upgrade().is_some() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
}
