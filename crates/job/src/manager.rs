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
use bevy_widgetry_internal_log::widgetry_error;
use event_listener::Event;
use futures_util::FutureExt;
use tracing::Instrument;

use crate::*;

#[derive(Clone, Default)]
pub struct JobManager {
    inner: Arc<Mutex<Registry>>,
}

#[derive(Default)]
struct Registry {
    last_id: u64,
    records: BTreeMap<JobId, Record>,
}

struct Record {
    shared: Arc<Shared>,
    task: Option<Task<JobOutcome>>,
}

pub(crate) struct Shared {
    pub status: Mutex<JobStatus>,
    pub signal: Arc<Signal>,
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
        let id = registry.allocate_id()?;
        let shared = Arc::new(Shared {
            status: Mutex::new(JobStatus {
                id,
                kind,
                state: JobState::Pending,
                outcome: None,
                label: options.label,
            }),
            signal: Arc::default(),
        });
        let receiver = JobReceiver::new(self.clone(), shared.clone());
        let context = JobContext::new(shared.clone());
        let record = registry.records.entry(id).or_insert(Record {
            shared: shared.clone(),
            task: None,
        });
        let span = tracing::info_span!(target: "bevy_widgetry", "job", job_id = id.get(), ?kind);
        let task = pool.spawn(async move {
            lock(&shared.status).state = JobState::Running;
            match AssertUnwindSafe(async move { work(context).await.map_err(|error| error.to_string()) })
                .catch_unwind().await {
                Ok(Ok(())) => JobOutcome::Completed,
                Ok(Err(error)) => JobOutcome::Failed(JobError::Business(error)),
                Err(payload) => {
                    let message = payload.downcast_ref::<String>().cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|message| (*message).to_owned()))
                        .unwrap_or_else(|| "non-string panic payload".to_owned());
                    widgetry_error!(job_id = id.get(), error = %message, "Job 执行体发生 panic");
                    JobOutcome::Failed(JobError::Panic(message))
                }
            }
        }.instrument(span));
        record.task = Some(task);
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
        Ok(lock(&record.shared.status).clone())
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
        let status = lock(&record.shared.status).clone();
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
    fn allocate_id(&mut self) -> Result<JobId, SubmitError> {
        self.last_id = self.last_id.checked_add(1).ok_or_else(|| {
            widgetry_error!("Job ID 已耗尽");
            SubmitError::IdExhausted
        })?;
        Ok(JobId(self.last_id))
    }
}

impl Record {
    fn reap(&mut self) {
        let Some(task) = &mut self.task else {
            return;
        };
        let waker = Waker::from(self.shared.signal.clone());
        if let Poll::Ready(outcome) = Pin::new(task).poll(&mut Context::from_waker(&waker)) {
            self.task = None;
            let mut status = lock(&self.shared.status);
            status.state = match &outcome {
                JobOutcome::Completed => JobState::Completed,
                JobOutcome::Failed(_) => JobState::Failed,
                JobOutcome::Canceled => JobState::Canceled,
            };
            status.outcome = Some(outcome);
            self.shared.signal.0.notify(usize::MAX);
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

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
    fn pending_is_registered_before_executor_gets_a_chance() {
        let pool = bevy::tasks::TaskPoolBuilder::new().num_threads(0).build();
        let manager = JobManager::default();
        let (id, mut receiver) = manager
            .spawn_on::<(), _, _, String>(&pool, JobKind::Io, JobOptions::default(), |_| async {
                panic!("this executor must not run");
            })
            .unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Pending);
        assert_eq!(
            manager.remove(id),
            Err(JobAccessError::NotFinished(JobState::Pending))
        );
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Empty);
    }
}
