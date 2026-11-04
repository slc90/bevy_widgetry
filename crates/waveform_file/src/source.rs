use crate::{DiskReaderError, DiskReaderFailure, ReaderStopReason, Recording};
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobOutcome, JobRead, JobReceiver,
};
use bevy_widgetry_waveform::{
    PlanarBuffer, WaveformMetadata, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
    WaveformReducedReadStatus, WaveformSource,
};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

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
    failure: Option<WaveformReadError>,
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
    path: PathBuf,
    metadata: WaveformMetadata,
    state: Arc<(Mutex<State>, Condvar)>,
    manager: JobManager,
    job: JobId,
    receiver: Option<JobReceiver<()>>,
}

impl DiskWaveformSource {
    pub fn stats(&self) -> Result<DiskSourceStats, WaveformReadError> {
        let state = lock_state(&self.state, || {
            reader_frame(
                &self.path,
                Some(self.job),
                None,
                "waveform_file.source.stats",
            )
        });
        if let Some(error) = &state.failure {
            return Err(error.clone().with_context(reader_frame(
                &self.path,
                Some(self.job),
                None,
                "waveform_file.source.stats",
            )));
        }
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
        let mut recording = Recording::open(path).map_err(|error| {
            error.with_context(reader_frame(path, None, None, "waveform_file.source.open"))
        })?;
        let metadata = recording.metadata().clone();
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let shared = state.clone();
        let worker_path = path.to_owned();
        let (job, receiver) = manager
            .spawn_thread::<(), _>(
                JobOptions {
                    label: Some("waveform disk reader".into()),
                    ..Default::default()
                },
                move |context| {
                    run(&context, &shared, &worker_path, |request| {
                        recording.query(
                            request.range.clone(),
                            request.output_len,
                            request.algorithm,
                        )
                    })
                },
            )
            .map_err(|error| {
                WaveformReadError::from(error).with_context(reader_frame(
                    path,
                    None,
                    None,
                    "waveform_file.source.submit",
                ))
            })?;
        Ok(Self {
            path: path.to_owned(),
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
        request: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(bevy_widgetry_diagnostics::WidgetryFailure::new(
            bevy_widgetry_waveform::WaveformError::UnsupportedRead { reduced: false },
            reader_frame(
                &self.path,
                Some(self.job),
                Some(request),
                "waveform_file.source.read",
            ),
        )
        .into())
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        let (_, wake) = &*self.state;
        let frame = || {
            reader_frame(
                &self.path,
                Some(self.job),
                Some(request),
                "waveform_file.source.read_reduced",
            )
        };
        let mut state = lock_state(&self.state, frame);
        if let Some(error) = &state.failure {
            return Err(error.clone().with_context(frame()));
        }
        if state.shutdown {
            return Err(DiskReaderFailure::new(
                DiskReaderError::Stopped {
                    reason: ReaderStopReason::Shutdown,
                },
                frame(),
            )
            .into());
        }
        match self.manager.outcome(self.job) {
            Ok(Some(JobOutcome::Failed(error))) => return Err(error.with_context(frame())),
            Ok(Some(outcome)) => {
                let reason = if matches!(outcome, JobOutcome::Canceled) {
                    ReaderStopReason::Canceled
                } else {
                    ReaderStopReason::Completed
                };
                return Err(
                    DiskReaderFailure::new(DiskReaderError::Stopped { reason }, frame()).into(),
                );
            }
            Err(error) => return Err(WaveformReadError::from(error).with_context(frame())),
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
                return result
                    .map(|channels| WaveformReducedReadStatus::Ready {
                        request: request.clone(),
                        channels,
                    })
                    .map_err(|error| error.with_context(frame()));
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
        let mut state = lock_state(&self.state, || {
            reader_frame(
                &self.path,
                Some(self.job),
                None,
                "waveform_file.source.cancel",
            )
            .field("reader_id", reader_id)
        });
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
        let mut state = lock_state(&self.state, || {
            reader_frame(
                &self.path,
                Some(self.job),
                None,
                "waveform_file.source.drop",
            )
        });
        state.shutdown = true;
        let queue_failure = state.failure.clone();
        let discarded = std::mem::take(&mut state.slots);
        state.queued.clear();
        self.state.1.notify_one();
        drop(state);
        for slot in discarded.into_values() {
            discard_slot(slot);
        }
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
                    if let Some(error) = queue_failure {
                        error.record_handled(
                            "放弃已销毁 Source 的队列终止诊断",
                            "Reader 已实际结束或结果通道已关闭",
                        );
                    }
                })
                .detach();
        }
    }
}

fn run(
    context: &JobContext<()>,
    shared: &(Mutex<State>, Condvar),
    path: &Path,
    mut query: impl FnMut(&WaveformReadRequest) -> Result<crate::DiskQuery, WaveformReadError>,
) -> Result<(), bevy_widgetry_job::JobFailure> {
    let (_, wake) = shared;
    loop {
        let mut state = lock_state(shared, || {
            reader_frame(path, Some(context.id()), None, "waveform_file.reader.wait")
        });
        // A Manager-wide cancellation has no business Condvar notification.
        // A bounded wait observes it without occupying any shared task-pool worker.
        while state.queued.is_empty() && !state.shutdown && !context.is_cancelled() {
            state = match wake.wait_timeout(state, std::time::Duration::from_millis(10)) {
                Ok((state, _)) => state,
                Err(error) => poisoned_queue(
                    shared,
                    error.into_inner().0,
                    reader_frame(path, Some(context.id()), None, "waveform_file.reader.wait"),
                ),
            };
        }
        if state.shutdown || context.is_cancelled() {
            return stop_queue(context, state);
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
            error.with_context(reader_frame(
                path,
                Some(context.id()),
                Some(&request),
                "waveform_file.query",
            ))
        });
        let elapsed = start.elapsed();
        let mut state = lock_state(shared, || {
            reader_frame(
                path,
                Some(context.id()),
                Some(&request),
                "waveform_file.reader.publish",
            )
        });
        if state.failure.is_some() {
            if let Err(error) = result
                && let Some(failure) = state.failure.take()
            {
                state.failure = Some(failure.combine(error));
            }
            return stop_queue(context, state);
        }
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

fn reader_frame(
    path: &Path,
    job: Option<JobId>,
    request: Option<&WaveformReadRequest>,
    operation: &'static str,
) -> ErrorFrame {
    let mut frame = ErrorFrame::new(operation).field("path", path.display());
    if let Some(job) = job {
        frame = frame.field("job_id", job.get());
    }
    if let Some(request) = request {
        frame = frame
            .field("reader_id", request.reader_id)
            .field("request_id", request.id)
            .field("session_generation", request.session_generation)
            .field("target_generation", request.target_generation)
            .field("range", format_args!("{:?}", request.range))
            .field("viewport", format_args!("{:?}", request.viewport))
            .field("output_len", request.output_len)
            .field("algorithm", format_args!("{:?}", request.algorithm));
    }
    frame
}

fn poisoned_queue<'a>(
    shared: &'a (Mutex<State>, Condvar),
    mut state: MutexGuard<'a, State>,
    frame: ErrorFrame,
) -> MutexGuard<'a, State> {
    if state.failure.is_none() {
        state.failure = Some(DiskReaderFailure::new(DiskReaderError::QueuePoisoned, frame).into());
    }
    state.shutdown = true;
    shared.0.clear_poison();
    shared.1.notify_all();
    state
}

fn lock_state<'a>(
    shared: &'a (Mutex<State>, Condvar),
    frame: impl FnOnce() -> ErrorFrame,
) -> MutexGuard<'a, State> {
    match shared.0.lock() {
        Ok(state) => state,
        Err(error) => poisoned_queue(shared, error.into_inner(), frame()),
    }
}

fn stop_queue(
    context: &JobContext<()>,
    mut state: MutexGuard<'_, State>,
) -> Result<(), WaveformReadError> {
    state.shutdown = true;
    let failure = state.failure.clone();
    let discarded = std::mem::take(&mut state.slots);
    state.queued.clear();
    drop(state);
    for slot in discarded.into_values() {
        discard_slot(slot);
    }
    match failure {
        Some(error) => Err(error),
        None => {
            context.acknowledge_cancel();
            Ok(())
        }
    }
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
                    run(&context, &shared, Path::new("test-recording.widx"), query)
                })
            })
            .unwrap();
        DiskWaveformSource {
            path: PathBuf::from("test-recording.widx"),
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

    fn has_field(error: &WaveformReadError, key: &str, expected: &str) -> bool {
        error
            .diagnostics()
            .flat_map(|d| d.contexts())
            .flat_map(|c| c.fields())
            .any(|(k, value)| k == key && value == expected)
    }

    fn cause<'a, T: std::error::Error + 'static>(
        mut error: &'a (dyn std::error::Error + 'static),
    ) -> Option<&'a T> {
        loop {
            if let Some(value) = error.downcast_ref::<T>() {
                return Some(value);
            }
            error = error.source()?;
        }
    }

    #[test]
    fn a_poisoned_queue_has_one_terminal_origin_with_reader_context() {
        let source = source(|_| panic!("a poisoned idle queue must not query"));
        let state = source.state.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = state.0.lock().unwrap();
                panic!("foreign queue poison");
            })
            .join()
            .is_err()
        );
        let first = source.stats().unwrap_err();
        assert!(has_field(&first, "path", "test-recording.widx"));
        assert!(has_field(&first, "job_id", &source.job.get().to_string()));
        let second = source.stats().unwrap_err();
        assert!(Arc::ptr_eq(
            first.diagnostics().next().unwrap().origin(),
            second.diagnostics().next().unwrap().origin()
        ));
        wait(|| {
            source
                .manager
                .status(source.job)
                .unwrap()
                .state
                .is_finished()
        });
        let Some(JobOutcome::Failed(terminal)) = source.manager.outcome(source.job).unwrap() else {
            panic!("expected reader failure");
        };
        assert!(Arc::ptr_eq(
            first.diagnostics().next().unwrap().origin(),
            terminal.diagnostics().next().unwrap().origin()
        ));
        let request = request(7, 9);
        let delivered = source.read_reduced(&request).err().unwrap();
        assert!(Arc::ptr_eq(
            first.diagnostics().next().unwrap().origin(),
            delivered.diagnostics().next().unwrap().origin()
        ));
        assert!(has_field(&delivered, "request_id", "7"));
        assert!(has_field(&delivered, "reader_id", "9"));
    }

    #[test]
    fn a_query_failure_keeps_origin_and_structured_request_across_threads() {
        let failure = WaveformReadError::from(std::io::Error::other("query failure"));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let source = source(move |_| Err(failure.clone()));
        let request = request(5, 11);
        let mut failure = None;
        wait(|| {
            failure = source.read_reduced(&request).err();
            failure.is_some()
        });
        let failure = failure.unwrap();
        assert!(Arc::ptr_eq(
            failure.diagnostics().next().unwrap().origin(),
            &origin
        ));
        for (key, value) in [
            ("path", "test-recording.widx"),
            ("request_id", "5"),
            ("reader_id", "11"),
            ("session_generation", "1"),
            ("target_generation", "5"),
            ("range", "0..100"),
            ("algorithm", "MinMax"),
        ] {
            assert!(has_field(&failure, key, value), "missing {key}");
        }
        assert!(has_field(&failure, "job_id", &source.job.get().to_string()));
    }

    #[test]
    fn a_worker_file_failure_reaches_runtime_with_the_original_origin_and_native_cause() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.widx");
        let worker_path = path.clone();
        let (sent, original) = mpsc::channel();
        let logs = LogCapture::default();
        let source = Arc::new(logs.run(|| {
            source(move |_| {
                let failure = Recording::open(&worker_path).err().unwrap();
                sent.send(failure.clone()).unwrap();
                Err(failure)
            })
        }));
        let runtime = bevy_widgetry_waveform::WaveformRuntime::new(
            Default::default(),
            source.clone(),
            &source.manager,
        )
        .unwrap();
        let mut world = bevy::prelude::World::new();
        let root = world
            .spawn((
                bevy_widgetry_waveform::Waveform,
                runtime,
                bevy_widgetry_waveform::WaveformCursor::default(),
            ))
            .id();
        bevy_widgetry_waveform::Waveform::set_running_in_world(&mut world, root, true).unwrap();
        let mut failure = None;
        wait(|| {
            failure = logs
                .run(|| {
                    world
                        .get_mut::<bevy_widgetry_waveform::WaveformRuntime>(root)
                        .unwrap()
                        .update(Duration::from_millis(50), 8)
                })
                .err();
            failure.is_some()
        });
        let failure = failure.unwrap();
        let original = original.recv_timeout(Duration::from_secs(5)).unwrap();
        let origin = original.diagnostics().next().unwrap().origin();
        assert!(Arc::ptr_eq(
            failure.diagnostics().next().unwrap().origin(),
            origin
        ));
        let recording =
            cause::<crate::RecordingFailure>(failure.diagnostics().next().unwrap()).unwrap();
        assert!(
            matches!(recording.kind(), crate::RecordingError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound)
        );
        assert!(has_field(&failure, "path", &path.display().to_string()));
        assert!(has_field(&failure, "job_id", &source.job.get().to_string()));
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
            !world
                .get::<bevy_widgetry_waveform::WaveformRuntime>(root)
                .unwrap()
                .is_running()
        );
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
    fn poisoning_during_a_failed_query_preserves_both_terminal_diagnostics() {
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        let query_failure =
            WaveformReadError::from(std::io::Error::other("query failed during poison"));
        let query_origin = query_failure.diagnostics().next().unwrap().origin().clone();
        let source = source(move |_| {
            entered.send(()).unwrap();
            gate.recv().unwrap();
            Err(query_failure.clone())
        });
        source.read_reduced(&request(1, 2)).unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let state = source.state.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = state.0.lock().unwrap();
                panic!("foreign queue poison during query");
            })
            .join()
            .is_err()
        );
        let poisoned = source.stats().unwrap_err();
        let poison_origin = poisoned.diagnostics().next().unwrap().origin().clone();
        release.send(()).unwrap();
        wait(|| {
            source
                .manager
                .status(source.job)
                .unwrap()
                .state
                .is_finished()
        });
        let Some(JobOutcome::Failed(terminal)) = source.manager.outcome(source.job).unwrap() else {
            panic!("expected combined reader failure");
        };
        assert_eq!(terminal.diagnostics().count(), 2);
        for origin in [&poison_origin, &query_origin] {
            assert!(
                terminal
                    .diagnostics()
                    .any(|d| Arc::ptr_eq(d.origin(), origin))
            );
        }
        let delivered = source.read_reduced(&request(2, 2)).err().unwrap();
        assert_eq!(delivered.diagnostics().count(), 2);
    }

    #[test]
    fn drop_consumes_poison_observed_after_the_worker_already_panicked() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        const CHILD: &str = "WIDGETRY_READER_POISON_DROP_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap()).env(CHILD, "1")
                .args(["--exact", "source::tests::drop_consumes_poison_observed_after_the_worker_already_panicked", "--nocapture"]).output().unwrap();
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
        let queue = Arc::new(std::sync::OnceLock::<Arc<(Mutex<State>, Condvar)>>::new());
        let worker_queue = queue.clone();
        let source = source(move |_| {
            let _guard = worker_queue.get().unwrap().0.lock().unwrap();
            panic!("reader panic while holding queue");
        });
        assert!(queue.set(source.state.clone()).is_ok());
        let manager = source.manager.clone();
        let job = source.job;
        source.read_reduced(&request(1, 1)).unwrap();
        wait(|| manager.status(job).unwrap().state.is_finished());
        let Some(JobOutcome::Failed(panic)) = manager.outcome(job).unwrap() else {
            panic!("expected worker panic");
        };
        let panic_origin = panic.diagnostics().next().unwrap().origin().clone();
        drop(source);
        let poison = queue
            .get()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .failure
            .clone()
            .unwrap();
        let poison_origin = poison.diagnostics().next().unwrap().origin().clone();
        wait(|| {
            manager.find_status(job).is_none()
                && logs
                    .diagnostic_records(poison_origin.id(), DiagnosticPhase::Handled)
                    .len()
                    == 1
        });
        for origin in [panic_origin, poison_origin] {
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .len(),
                1
            );
        }
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
                _ => logs.run(|| drop(source)),
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
            assert!(matches!(cause::<bevy_widgetry_job::JobAccessError>(diagnostic), Some(bevy_widgetry_job::JobAccessError::NotFound { job }) if *job == source.job));
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
