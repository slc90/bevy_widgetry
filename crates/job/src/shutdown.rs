use crate::manager::{ExecutionCompletion, Shared, lock};
use crate::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownPhase {
    ShuttingDown,
    TimedOut,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownStatus {
    pub phase: ShutdownPhase,
    pub remaining: Vec<JobStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownMessage {
    TimedOut { remaining: Vec<JobStatus> },
    Completed,
}

#[derive(Clone)]
pub struct ShutdownHandle {
    manager: JobManager,
    shared: Arc<Mutex<ShutdownData>>,
}

pub(crate) struct ShutdownData {
    started: Instant,
    timeout: Duration,
    pub status: ShutdownStatus,
    notifications: VecDeque<ShutdownMessage>,
    pub task: Option<bevy::tasks::Task<()>>,
    jobs: Vec<TrackedJob>,
}

struct TrackedJob {
    shared: Arc<Shared>,
    requested: JobStatus,
}

impl ShutdownData {
    pub fn new(timeout: Duration) -> Self {
        Self {
            started: Instant::now(),
            timeout,
            status: ShutdownStatus {
                phase: ShutdownPhase::ShuttingDown,
                remaining: Vec::new(),
            },
            notifications: VecDeque::new(),
            task: None,
            jobs: Vec::new(),
        }
    }

    pub fn track(&mut self, shared: Arc<Shared>) {
        let requested = lock(&shared.lifecycle).status.clone();
        self.jobs.push(TrackedJob { shared, requested });
    }

    pub fn advance(&mut self, now: Instant, remaining: Vec<JobStatus>) {
        self.status.remaining = remaining;
        if self.status.phase == ShutdownPhase::Finished {
            return;
        }
        if self.status.phase == ShutdownPhase::ShuttingDown
            && now.duration_since(self.started) >= self.timeout
        {
            let overdue: Vec<_> = self
                .jobs
                .iter()
                .filter(|job| match *lock(&job.shared.completion) {
                    ExecutionCompletion::Unfinished => true,
                    ExecutionCompletion::Finished(at) => {
                        at >= self.started && at.duration_since(self.started) >= self.timeout
                    }
                    ExecutionCompletion::Rejected => false,
                })
                .map(|job| job.requested.clone())
                .collect();
            if !overdue.is_empty() {
                self.status.phase = ShutdownPhase::TimedOut;
                self.notifications
                    .push_back(ShutdownMessage::TimedOut { remaining: overdue });
            }
        }
        if self.status.remaining.is_empty() {
            self.status.phase = ShutdownPhase::Finished;
            self.notifications.push_back(ShutdownMessage::Completed);
            self.jobs.clear();
        }
    }
}

impl ShutdownHandle {
    pub(crate) fn new(manager: JobManager, shared: Arc<Mutex<ShutdownData>>) -> Self {
        Self { manager, shared }
    }

    pub fn status(&self) -> ShutdownStatus {
        self.manager.update_shutdown();
        lock(&self.shared).status.clone()
    }

    pub fn try_recv(&self) -> Option<ShutdownMessage> {
        self.manager.update_shutdown();
        lock(&self.shared).notifications.pop_front()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn track(data: &mut ShutdownData, status: JobStatus, finished_at: Option<Instant>) {
        let shared = Arc::new(Shared {
            lifecycle: Mutex::new(crate::lifecycle::Lifecycle::new(status)),
            signal: Arc::default(),
            cancel: event_listener::Event::new(),
            completion: Mutex::new(finished_at.map_or(
                ExecutionCompletion::Unfinished,
                ExecutionCompletion::Finished,
            )),
        });
        data.track(shared);
    }

    #[test]
    fn global_deadline_and_notifications_are_monotonic() {
        let mut data = ShutdownData::new(Duration::from_secs(10));
        let active = vec![JobStatus {
            id: JobId(1),
            kind: JobKind::Io,
            state: JobState::Stopping,
            outcome: None,
            label: None,
        }];
        track(&mut data, active[0].clone(), None);
        data.advance(data.started + Duration::from_secs(9), active.clone());
        assert_eq!(data.status.phase, ShutdownPhase::ShuttingDown);
        assert!(data.notifications.is_empty());
        data.advance(data.started + Duration::from_secs(10), active.clone());
        assert_eq!(data.notifications.len(), 1);
        data.advance(data.started + Duration::from_secs(100), active);
        assert_eq!(data.notifications.len(), 1);
        data.advance(data.started + Duration::from_secs(101), Vec::new());
        data.advance(data.started + Duration::from_secs(102), Vec::new());
        assert_eq!(data.status.phase, ShutdownPhase::Finished);
        assert_eq!(data.notifications.len(), 2);
        assert!(matches!(
            data.notifications.pop_front(),
            Some(ShutdownMessage::TimedOut { .. })
        ));
        assert_eq!(
            data.notifications.pop_front(),
            Some(ShutdownMessage::Completed)
        );
    }

    #[test]
    fn late_poll_preserves_timeout_when_all_jobs_finished_after_deadline() {
        let mut data = ShutdownData::new(Duration::from_secs(10));
        let active = JobStatus {
            id: JobId(42),
            kind: JobKind::Thread,
            state: JobState::Stopping,
            outcome: None,
            label: None,
        };
        let late = data.started + Duration::from_secs(11);
        track(&mut data, active.clone(), Some(late));
        lock(&data.jobs[0].shared.lifecycle).finish(JobOutcome::Completed);
        data.advance(data.started + Duration::from_secs(20), Vec::new());
        assert_eq!(data.status.phase, ShutdownPhase::Finished);
        assert!(
            matches!(data.notifications.pop_front(), Some(ShutdownMessage::TimedOut { remaining }) if remaining == vec![active.clone()])
        );
        assert_eq!(
            data.notifications.pop_front(),
            Some(ShutdownMessage::Completed)
        );
        let mut on_time = ShutdownData::new(Duration::from_secs(10));
        let early = on_time.started + Duration::from_secs(9);
        track(&mut on_time, active, Some(early));
        on_time.advance(on_time.started + Duration::from_secs(20), Vec::new());
        assert_eq!(
            on_time.notifications.pop_front(),
            Some(ShutdownMessage::Completed)
        );
        assert!(on_time.notifications.is_empty());
    }

    #[test]
    fn rejected_thread_startup_is_not_reported_as_a_lingering_job() {
        let mut data = ShutdownData::new(Duration::from_secs(10));
        let active = JobStatus {
            id: JobId(42),
            kind: JobKind::Thread,
            state: JobState::Stopping,
            outcome: None,
            label: None,
        };
        track(&mut data, active, None);
        data.jobs[0].shared.reject_startup();
        data.advance(data.started + Duration::from_secs(20), Vec::new());
        assert_eq!(
            data.notifications.pop_front(),
            Some(ShutdownMessage::Completed)
        );
        assert!(data.notifications.is_empty());
    }

    #[test]
    fn coordinator_reports_timeout_and_completion_without_host_polling() {
        use bevy::tasks::{IoTaskPool, TaskPool};
        use std::sync::mpsc;
        IoTaskPool::get_or_init(TaskPool::new);
        let manager = JobManager::default();
        let release = event_listener::Event::new();
        let listener = release.listen();
        let (started, running) = mpsc::channel();
        let (_, receiver) = manager
            .spawn_io::<(), _, _, String>(JobOptions::default(), move |context| async move {
                started.send(()).unwrap();
                context.cancelled().await;
                listener.await;
                Ok(())
            })
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        let handle = manager.begin_shutdown(Duration::from_millis(10)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while lock(&handle.shared).status.phase != ShutdownPhase::TimedOut {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        release.notify(usize::MAX);
        while lock(&handle.shared).status.phase != ShutdownPhase::Finished {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(lock(&handle.shared).notifications.len(), 2);
        drop(receiver);
    }
}
