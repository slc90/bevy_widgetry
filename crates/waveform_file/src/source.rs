use crate::Recording;
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_job::{JobContext, JobId, JobManager, JobOptions, JobOutcome};
use bevy_widgetry_waveform::{
    PlanarBuffer, WaveformMetadata, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
    WaveformReducedReadStatus, WaveformSource,
};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

type Reply = Result<Vec<bevy_widgetry_waveform::ReducedChannel>, WaveformReadError>;

struct Slot {
    request: WaveformReadRequest,
    result: Option<Reply>,
}

#[derive(Default)]
struct State {
    slots: BTreeMap<u64, Slot>,
    queued: VecDeque<u64>,
    shutdown: bool,
    stats: DiskSourceStats,
}

#[derive(Clone, Debug)]
pub struct DiskReadObservation {
    pub request: WaveformReadRequest,
    pub elapsed: std::time::Duration,
    pub read: crate::DiskReadStats,
}

#[derive(Clone, Debug, Default)]
pub struct DiskSourceStats {
    pub queries: u64,
    pub discarded_queries: u64,
    pub queued_readers: usize,
    pub last_read: Option<DiskReadObservation>,
}

pub struct DiskWaveformSource {
    metadata: WaveformMetadata,
    state: Arc<(Mutex<State>, Condvar)>,
    manager: JobManager,
    job: JobId,
}

impl DiskWaveformSource {
    pub fn stats(&self) -> Result<DiskSourceStats, WaveformReadError> {
        let state = self
            .state
            .0
            .lock()
            .map_err(|e| WaveformReadError(format!("disk reader lock poisoned: {e}")))?;
        Ok(DiskSourceStats {
            queued_readers: state.queued.len(),
            ..state.stats.clone()
        })
    }
    pub fn job_id(&self) -> JobId {
        self.job
    }
    /// Opening validates metadata synchronously, so applications should load recordings off the UI thread.
    pub fn open(path: &Path, manager: &JobManager) -> Result<Self, WaveformReadError> {
        let mut recording = Recording::open(path)?;
        let metadata = recording.metadata().clone();
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let shared = state.clone();
        let (job, receiver) = manager
            .spawn_thread::<(), _, String>(
                JobOptions {
                    label: Some("waveform disk reader".into()),
                    ..Default::default()
                },
                move |context| {
                    run(&context, &shared, |request| {
                        recording.query(
                            request.range.clone(),
                            request.output_len,
                            request.algorithm,
                        )
                    })
                },
            )
            .map_err(|e| WaveformReadError(format!("cannot start disk reader: {e}")))?;
        drop(receiver);
        Ok(Self {
            metadata,
            state,
            manager: manager.clone(),
            job,
        })
    }
}

impl WaveformSource for DiskWaveformSource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(self.metadata.clone())
    }
    fn provides_reduced(&self) -> bool {
        true
    }
    fn read(
        &self,
        _: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(WaveformReadError(
            "disk source requires reduced reads".into(),
        ))
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        let (lock, wake) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|e| WaveformReadError(format!("disk reader lock poisoned: {e}")))?;
        if state.shutdown {
            return Err(WaveformReadError("disk reader stopped".into()));
        }
        match self.manager.outcome(self.job) {
            Ok(Some(JobOutcome::Failed(error))) => {
                return Err(WaveformReadError(error.to_string()));
            }
            Ok(Some(_)) => return Err(WaveformReadError("disk reader stopped".into())),
            Err(error) => return Err(WaveformReadError(error.to_string())),
            Ok(None) => {}
        }
        if let Some(slot) = state
            .slots
            .get_mut(&request.reader_id)
            .filter(|slot| slot.request == *request)
        {
            if let Some(result) = slot.result.take() {
                state.slots.remove(&request.reader_id);
                return result.map(|channels| WaveformReducedReadStatus::Ready {
                    request: request.clone(),
                    channels,
                });
            }
        } else {
            state.slots.insert(
                request.reader_id,
                Slot {
                    request: request.clone(),
                    result: None,
                },
            );
            if !state.queued.contains(&request.reader_id) {
                state.queued.push_back(request.reader_id);
            }
            wake.notify_one();
        }
        Ok(WaveformReducedReadStatus::Pending)
    }
    fn cancel_reads(&self, reader_id: u64) {
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.slots.remove(&reader_id);
        state.queued.retain(|reader| *reader != reader_id);
    }
}

impl Drop for DiskWaveformSource {
    fn drop(&mut self) {
        if let Err(error) = self.manager.cancel(self.job) {
            widgetry_error!(%error, "取消磁盘 Waveform Reader Job 失败");
        }
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        self.state.1.notify_one();
    }
}

fn run(
    context: &JobContext<()>,
    shared: &(Mutex<State>, Condvar),
    mut query: impl FnMut(&WaveformReadRequest) -> Result<crate::DiskQuery, WaveformReadError>,
) -> Result<(), String> {
    let (lock, wake) = shared;
    loop {
        let mut state = lock.lock().map_err(lock_failed)?;
        // A Manager-wide cancellation has no business Condvar notification.
        // A bounded wait observes it without occupying any shared task-pool worker.
        while state.queued.is_empty() && !state.shutdown && !context.is_cancelled() {
            state = wake
                .wait_timeout(state, std::time::Duration::from_millis(10))
                .map_err(lock_failed)?
                .0;
        }
        if state.shutdown || context.is_cancelled() {
            state.shutdown = true;
            state.slots.clear();
            state.queued.clear();
            context.acknowledge_cancel();
            return Ok(());
        }
        let Some(reader) = state.queued.pop_front() else {
            continue;
        };
        let Some(slot) = state.slots.get(&reader) else {
            continue;
        };
        let request = slot.request.clone();
        drop(state);
        let start = std::time::Instant::now();
        let result = query(&request);
        let elapsed = start.elapsed();
        let mut state = lock.lock().map_err(lock_failed)?;
        state.stats.queries = state.stats.queries.saturating_add(1);
        if let Ok(query) = &result {
            state.stats.last_read = Some(DiskReadObservation {
                request: request.clone(),
                elapsed,
                read: query.stats,
            });
        }
        if let Some(slot) = state
            .slots
            .get_mut(&reader)
            .filter(|slot| slot.request == request)
        {
            slot.result = Some(result.map(|query| query.channels));
        } else {
            state.stats.discarded_queries = state.stats.discarded_queries.saturating_add(1);
        }
    }
}

fn lock_failed(error: impl std::fmt::Display) -> String {
    widgetry_error!(%error, "磁盘 Waveform Reader 队列锁损坏");
    format!("disk reader lock poisoned: {error}")
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_job::{JobState, ShutdownMessage};
    use bevy_widgetry_waveform::{WaveformAlgorithm, WaveformSessionKind, WaveformUnit};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn source(
        query: impl FnMut(&WaveformReadRequest) -> Result<crate::DiskQuery, WaveformReadError>
        + Send
        + 'static,
    ) -> DiskWaveformSource {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let shared = state.clone();
        let (job, receiver) = manager
            .spawn_thread::<(), _, String>(Default::default(), move |context| {
                run(&context, &shared, query)
            })
            .unwrap();
        drop(receiver);
        DiskWaveformSource {
            metadata: WaveformMetadata {
                channel_count: 1,
                channel_labels: vec!["A".into()],
                sample_rate: 1000,
                unit: WaveformUnit::Volts,
                kind: WaveformSessionKind::Replay { total_frames: 100 },
            },
            state,
            manager,
            job,
        }
    }
    fn request(id: u64, reader_id: u64) -> WaveformReadRequest {
        WaveformReadRequest {
            id,
            reader_id,
            session_generation: 1,
            target_generation: id,
            range: 0..100,
            viewport: 0..100,
            output_len: 4,
            algorithm: WaveformAlgorithm::MinMax,
        }
    }
    fn wait(mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    #[test]
    fn idle_reader_observes_global_cancel_and_reports_stopped_to_live_source() {
        let source = source(|_| panic!("idle reader must not query"));
        wait(|| source.manager.status(source.job).unwrap().state == JobState::Running);
        let shutdown = source
            .manager
            .begin_shutdown(Duration::from_secs(5))
            .unwrap();
        wait(|| {
            source
                .manager
                .status(source.job)
                .unwrap()
                .state
                .is_finished()
        });
        assert_eq!(
            source.manager.outcome(source.job).unwrap(),
            Some(JobOutcome::Canceled)
        );
        assert!(
            source
                .read_reduced(&request(1, 1))
                .err()
                .unwrap()
                .to_string()
                .contains("stopped")
        );
        wait(|| matches!(shutdown.try_recv(), Some(ShutdownMessage::Completed)));
    }

    #[test]
    fn source_drop_and_shutdown_do_not_finish_a_blocked_query_before_real_exit() {
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        let source = source(move |_| {
            entered.send(()).unwrap();
            gate.recv().unwrap();
            Ok(crate::DiskQuery {
                channels: vec![],
                stats: Default::default(),
            })
        });
        let manager = source.manager.clone();
        let id = source.job;
        source.read_reduced(&request(1, 1)).unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let (dropped, finished) = mpsc::channel();
        let dropper = std::thread::spawn(move || {
            drop(source);
            dropped.send(()).unwrap();
        });
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        dropper.join().unwrap();
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        let shutdown = manager.begin_shutdown(Duration::ZERO).unwrap();
        wait(|| matches!(shutdown.try_recv(), Some(ShutdownMessage::TimedOut { .. })));
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        release.send(()).unwrap();
        wait(|| manager.status(id).unwrap().state.is_finished());
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Canceled));
        wait(|| matches!(shutdown.try_recv(), Some(ShutdownMessage::Completed)));
        manager.remove(id).unwrap();
    }

    #[test]
    fn stale_blocked_error_is_discarded_without_interrupting_other_readers() {
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        let source = source(move |request| {
            if request.id == 1 {
                entered.send(()).unwrap();
                gate.recv().unwrap();
                Err(WaveformReadError("obsolete failure".into()))
            } else {
                Ok(crate::DiskQuery {
                    channels: vec![],
                    stats: Default::default(),
                })
            }
        });
        source.read_reduced(&request(1, 1)).unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        source.cancel_reads(1);
        let latest = request(2, 2);
        source.read_reduced(&latest).unwrap();
        release.send(()).unwrap();
        wait(
            || matches!(source.read_reduced(&latest).unwrap(), WaveformReducedReadStatus::Ready {request, ..} if request == latest),
        );
        assert_eq!(source.stats().unwrap().discarded_queries, 1);
        assert_eq!(source.manager.outcome(source.job).unwrap(), None);
    }

    #[test]
    fn reader_panic_is_failed_and_never_leaves_live_source_permanently_pending() {
        let source = source(|_| panic!("controlled reader failure"));
        source.read_reduced(&request(1, 1)).unwrap();
        wait(|| {
            source
                .manager
                .status(source.job)
                .unwrap()
                .state
                .is_finished()
        });
        assert!(matches!(
            source.manager.outcome(source.job).unwrap(),
            Some(JobOutcome::Failed(_))
        ));
        assert!(
            source
                .read_reduced(&request(2, 2))
                .err()
                .unwrap()
                .to_string()
                .contains("controlled reader failure")
        );
    }
}
