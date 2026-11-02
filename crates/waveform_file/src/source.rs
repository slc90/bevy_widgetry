use crate::Recording;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobOutcome, JobRead, JobReceiver,
};
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

fn discard_slot(slot: Slot) {
    if let Some(Err(error)) = slot.result {
        error.record_handled("放弃已取消或被替换的磁盘读取失败", "当前读取请求已失效");
    }
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
    receiver: Option<JobReceiver<()>>,
}

impl DiskWaveformSource {
    pub fn stats(&self) -> Result<DiskSourceStats, WaveformReadError> {
        let state = self.state.0.lock().map_err(lock_failed)?;
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
            .spawn_thread::<(), _>(
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
            .map_err(WaveformReadError::from)?;
        Ok(Self {
            metadata,
            state,
            manager: manager.clone(),
            job,
            receiver: Some(receiver),
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
        Err(bevy_widgetry_diagnostics::WidgetryFailure::new(
            bevy_widgetry_waveform::WaveformError::UnsupportedRead { reduced: false },
            bevy_widgetry_diagnostics::ErrorFrame::new("waveform_file.source.read"),
        )
        .into())
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().map_err(lock_failed)?;
        if state.shutdown {
            return Err(bevy_widgetry_job::JobFailure::external(
                ReaderStopped,
                bevy_widgetry_diagnostics::ErrorFrame::new("waveform_file.source.read_reduced"),
            ));
        }
        match self.manager.outcome(self.job) {
            Ok(Some(JobOutcome::Failed(error))) => {
                return Err(error);
            }
            Ok(Some(_)) => {
                return Err(bevy_widgetry_job::JobFailure::external(
                    ReaderStopped,
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform_file.source.read_reduced"),
                ));
            }
            Err(error) => return Err(WaveformReadError::from(error)),
            Ok(None) => {}
        }
        let mut discarded = None;
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
            discarded = state.slots.insert(
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
        drop(state);
        if let Some(slot) = discarded {
            discard_slot(slot);
        }
        Ok(WaveformReducedReadStatus::Pending)
    }
    fn cancel_reads(&self, reader_id: u64) {
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let discarded = state.slots.remove(&reader_id);
        state.queued.retain(|reader| *reader != reader_id);
        drop(state);
        if let Some(slot) = discarded {
            discard_slot(slot);
        }
    }
}

impl Drop for DiskWaveformSource {
    fn drop(&mut self) {
        let _ = self.manager.cancel_if_present(self.job);
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        self.state.1.notify_one();
        drop(state);
        if let Some(mut receiver) = self.receiver.take() {
            let manager = self.manager.clone();
            let job = self.job;
            // An entered disk query may still fail after its Source is gone.
            // Use the existing pool so cleanup also survives Manager shutdown.
            bevy::tasks::IoTaskPool::get()
                .spawn(async move {
                    loop {
                        match receiver.recv().await {
                            Ok(JobRead::Finished(JobOutcome::Failed(error))) => {
                                error.record_handled(
                                    "放弃已销毁磁盘 Source 的 Reader 失败结果",
                                    "终止过期读取操作",
                                );
                                if let Err(error) = manager.remove_if_present(job) {
                                    error.record_handled(
                                        "保留无法移除的磁盘 Reader Job 记录",
                                        "Reader 失败结果已放弃",
                                    );
                                }
                                break;
                            }
                            Ok(JobRead::Finished(_) | JobRead::Closed) => break,
                            Ok(JobRead::Item(_) | JobRead::Empty) => {}
                            Err(error) => error.record_handled(
                                "丢弃过期 Reader 交付结果并继续收割",
                                "磁盘 Source 已销毁",
                            ),
                        }
                    }
                })
                .detach();
        }
    }
}

fn run(
    context: &JobContext<()>,
    shared: &(Mutex<State>, Condvar),
    mut query: impl FnMut(&WaveformReadRequest) -> Result<crate::DiskQuery, WaveformReadError>,
) -> Result<(), bevy_widgetry_job::JobFailure> {
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
            let discarded = std::mem::take(&mut state.slots);
            state.queued.clear();
            drop(state);
            for slot in discarded.into_values() {
                discard_slot(slot);
            }
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
        let result = query(&request).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("waveform_file.query")
                    .field("request", format_args!("{request:?}")),
            )
        });
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
            drop(state);
            if let Err(error) = result {
                error.record_handled("放弃已过期的磁盘查询失败", "较新的读取请求继续执行");
            }
        }
    }
}

#[derive(Debug)]
struct ReaderStopped;
impl std::fmt::Display for ReaderStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("disk reader stopped")
    }
}
impl std::error::Error for ReaderStopped {}

#[derive(Debug)]
struct ReaderQueuePoisoned;
impl std::fmt::Display for ReaderQueuePoisoned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("disk reader queue lock poisoned")
    }
}
impl std::error::Error for ReaderQueuePoisoned {}

fn lock_failed(_error: impl std::fmt::Display) -> bevy_widgetry_job::JobFailure {
    bevy_widgetry_job::JobFailure::external(
        ReaderQueuePoisoned,
        bevy_widgetry_diagnostics::ErrorFrame::new("waveform_file.reader.queue"),
    )
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
        let dispatcher = bevy::log::tracing::dispatcher::get_default(Clone::clone);
        let (job, receiver) = manager
            .spawn_thread::<(), _>(Default::default(), move |context| {
                bevy::log::tracing::dispatcher::with_default(&dispatcher, || {
                    run(&context, &shared, query)
                })
            })
            .unwrap();
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
            receiver: Some(receiver),
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
    fn completed_failed_slots_are_consumed_when_canceled_replaced_or_dropped() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        for mode in 0..3 {
            let logs = LogCapture::default();
            let failure =
                logs.run(|| WaveformReadError::from(std::io::Error::other("stored failure")));
            let origin = failure.diagnostics().next().unwrap().origin().clone();
            let source = logs.run(|| {
                source(move |request| {
                    if request.id == 1 {
                        Err(failure.clone())
                    } else {
                        Ok(crate::DiskQuery {
                            channels: vec![],
                            stats: Default::default(),
                        })
                    }
                })
            });
            source.read_reduced(&request(1, 9)).unwrap();
            wait(|| {
                source
                    .state
                    .0
                    .lock()
                    .unwrap()
                    .slots
                    .get(&9)
                    .is_some_and(|slot| slot.result.is_some())
            });
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            match mode {
                0 => logs.run(|| source.cancel_reads(9)),
                1 => {
                    logs.run(|| source.read_reduced(&request(2, 9))).unwrap();
                }
                _ => drop(source),
            }
            wait(|| {
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .len()
                    == 1
            });
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
    }

    #[test]
    fn stale_blocked_error_is_discarded_without_interrupting_other_readers() {
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        let failure =
            logs.run(|| WaveformReadError::from(std::io::Error::other("obsolete failure")));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let source = logs.run(|| {
            source(move |request| {
                if request.id == 1 {
                    entered.send(()).unwrap();
                    gate.recv().unwrap();
                    Err(failure.clone())
                } else {
                    Ok(crate::DiskQuery {
                        channels: vec![],
                        stats: Default::default(),
                    })
                }
            })
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
        assert_eq!(
            logs.diagnostic_records(
                origin.id(),
                bevy_widgetry_test_utils::DiagnosticPhase::Handled
            )
            .len(),
            1
        );
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
        source.manager.remove(source.job).unwrap();
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        logs.run(|| {
            let error = source.read_reduced(&request(3, 3)).err().unwrap();
            let report = error.into_report();
            let diagnostic = report.diagnostics().next().unwrap();
            assert!(matches!(diagnostic.source().unwrap().downcast_ref::<bevy_widgetry_job::JobAccessError>(), Some(bevy_widgetry_job::JobAccessError::NotFound { job }) if *job == source.job));
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), bevy_widgetry_test_utils::DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(diagnostic.origin().id(), bevy_widgetry_test_utils::DiagnosticPhase::Handled).is_empty());
        });
    }

    #[test]
    fn dropped_source_consumes_a_reader_panic_after_cancellation() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        const CHILD: &str = "WIDGETRY_DROPPED_DISK_READER_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .env(CHILD, "1")
                .args([
                    "--exact",
                    "source::tests::dropped_source_consumes_a_reader_panic_after_cancellation",
                    "--nocapture",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let logs = LogCapture::default();
        bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
        let (entered, ready) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let source = source(move |_| {
            entered.send(()).unwrap();
            gate.recv().unwrap();
            panic!("controlled late reader failure");
        });
        let manager = source.manager.clone();
        let id = source.job;
        let mut observer = manager.subscribe::<()>(id).unwrap();
        source.read_reduced(&request(1, 1)).unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(source);
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        let shutdown = manager.begin_shutdown(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
        let failure = match observer.recv_blocking().unwrap() {
            bevy_widgetry_job::JobRead::Finished(JobOutcome::Failed(error)) => error,
            other => panic!("unexpected reader result: {other:?}"),
        };
        let diagnostic = failure.diagnostics().next().unwrap();
        wait(|| {
            !logs
                .diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
                && manager.find_status(id).is_none()
        });
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
        wait(|| matches!(shutdown.try_recv(), Some(ShutdownMessage::Completed)));
    }
}
