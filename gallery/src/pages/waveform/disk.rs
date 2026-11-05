use super::*;
use crate::i18n::localized_binding;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::job::{
    JobContext, JobFailure, JobId, JobManager, JobOptions, JobOutcome, JobRead, JobReceiver,
    SendStatus,
};
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry_waveform_file::DiskWaveformSource;

#[derive(Debug)]
enum OpenError {
    OwnerAlreadyInitialized,
    ClosedBeforeFinished,
    CompletedWithoutSource,
}
impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "recording open failed: {self:?}")
    }
}
impl std::error::Error for OpenError {}

fn open_frame(
    path: &std::path::Path,
    job: Option<JobId>,
    revision: u64,
) -> bevy_widgetry::diagnostics::ErrorFrame {
    let frame = bevy_widgetry::diagnostics::ErrorFrame::new("gallery.waveform.open")
        .field("path", path.display())
        .field("revision", revision);
    match job {
        Some(job) => frame.field("job_id", job.get()),
        None => frame,
    }
}

fn open_failure(kind: OpenError, path: &std::path::Path, job: JobId, revision: u64) -> JobFailure {
    bevy_widgetry::diagnostics::WidgetryFailure::new(kind, open_frame(path, Some(job), revision))
        .into()
}

fn deliver_source(
    context: &JobContext<Weak<DiskWaveformSource>>,
    source: &Arc<DiskWaveformSource>,
) -> std::result::Result<(), JobFailure> {
    match context.send_blocking(Arc::downgrade(source)) {
        Ok(SendStatus::Sent) => Ok(()),
        Ok(SendStatus::Canceled) => {
            context.acknowledge_cancel();
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

use std::path::PathBuf;
use std::sync::{OnceLock, Weak};

enum LoadState {
    Missing,
    Loading,
    Ready {
        rate: u32,
        channels: usize,
        seconds: u64,
    },
    Failed(String),
}

pub(super) struct DiskReplay {
    pub(super) source: Option<Arc<DiskWaveformSource>>,
    task: Option<OpenJob>,
    manager: JobManager,
    path: PathBuf,
    state: LoadState,
    revision: u64,
}

struct OpenJob {
    id: JobId,
    revision: u64,
    origin: OpenOrigin,
    receiver: JobReceiver<Weak<DiskWaveformSource>>,
    pending: Option<Weak<DiskWaveformSource>>,
    // Dropping an unread broadcast Item must not cancel its Reader while the channel lock is held.
    owner: Arc<OnceLock<Arc<DiskWaveformSource>>>,
}

#[derive(Component)]
struct DiskStatus;
#[derive(Component)]
struct Reload;

#[derive(Clone, Copy, Debug)]
enum OpenOrigin {
    Automatic,
    User,
}

#[derive(Clone, Copy, Debug)]
enum OpenStop {
    Replaced,
    PageLeft,
    DeliveryFailed,
}

impl DiskReplay {
    pub(super) fn new(manager: JobManager) -> Self {
        let path = std::env::var_os("GALLERY_WAVEFORM_REPLAY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/waveform/replay.widx")
            });
        let mut state = Self {
            source: None,
            task: None,
            manager,
            path,
            state: LoadState::Missing,
            revision: 0,
        };
        state.reload(OpenOrigin::Automatic);
        state
    }
    fn reload(&mut self, origin: OpenOrigin) {
        self.revision = self.revision.wrapping_add(1);
        info!(path = %self.path.display(), revision = self.revision, ?origin, "请求打开本地 Waveform 录制文件");
        self.cancel_open(OpenStop::Replaced);
        if !self.path.is_file() {
            self.state = LoadState::Missing;
            info!(path = %self.path.display(), revision = self.revision, ?origin, outcome = "missing", "本地 Waveform 录制文件未找到，未启动打开任务");
            return;
        }
        let path = self.path.clone();
        let revision = self.revision;
        self.state = LoadState::Loading;
        let manager = self.manager.clone();
        let owner = Arc::new(OnceLock::new());
        let thread_owner = owner.clone();
        match self.manager.spawn_thread::<Weak<DiskWaveformSource>, _>(
            JobOptions {
                label: Some("Waveform open recording".into()),
                ..default()
            },
            move |context| {
                if context.is_cancelled() {
                    context.acknowledge_cancel();
                    return Ok(());
                }
                let source =
                    Arc::new(DiskWaveformSource::open(&path, &manager).map_err(|error| {
                        error.with_context(open_frame(&path, Some(context.id()), revision))
                    })?);
                if thread_owner.set(source.clone()).is_err() {
                    return Err(open_failure(
                        OpenError::OwnerAlreadyInitialized,
                        &path,
                        context.id(),
                        revision,
                    ));
                }
                deliver_source(&context, &source)
            },
        ) {
            Ok((id, receiver)) => {
                info!(path = %self.path.display(), job_id = id.get(), revision = self.revision, ?origin, outcome = "submitted", "本地 Waveform 录制文件打开任务已提交");
                self.task = Some(OpenJob {
                    id,
                    revision: self.revision,
                    origin,
                    receiver,
                    pending: None,
                    owner,
                })
            }
            Err(error) => {
                let error = JobFailure::from(error).with_context(open_frame(
                    &self.path,
                    None,
                    self.revision,
                ));
                info!(path = %self.path.display(), revision = self.revision, ?origin, outcome = "failed", "本地 Waveform 录制文件打开请求未提交");
                self.reject(error, "提交失败");
            }
        }
    }
    fn cancel_open(&mut self, reason: OpenStop) {
        if let Some(mut task) = self.task.take() {
            if !matches!(reason, OpenStop::DeliveryFailed) {
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, ?reason, outcome = "discarded", "本地 Waveform 录制文件打开结果已放弃");
            }
            let cancel_result = self.manager.cancel_if_present(task.id);
            debug!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, ?reason, ?cancel_result, "录制文件打开请求已转交后台收尾");
            let manager = self.manager.clone();
            let path = self.path.clone();
            // This owner also runs during shutdown, when new managed jobs are rejected.
            bevy::tasks::IoTaskPool::get()
                .spawn(async move {
                    let outcome = loop {
                        match task.receiver.recv().await {
                            Ok(JobRead::Finished(JobOutcome::Failed(error))) => {
                                error.record_handled(
                                    "放弃已重载或离页的录制文件打开结果",
                                    "终止过期打开操作",
                                );
                                break "failed";
                            }
                            Ok(JobRead::Finished(JobOutcome::Completed)) => break "completed",
                            Ok(JobRead::Finished(JobOutcome::Canceled)) => break "canceled",
                            Ok(JobRead::Closed) => break "closed",
                            Ok(JobRead::Item(_) | JobRead::Empty) => {}
                            Err(error) => error
                                .record_handled("丢弃过期录制打开结果并继续收割", "回放会话已放弃"),
                        }
                    };
                    let removed = match manager.remove_if_present(task.id) {
                        Ok(status) => status.is_some(),
                        Err(error) => {
                            error.record_handled(
                                "保留无法移除的录制打开 Job 记录",
                                "过期打开结果已放弃",
                            );
                            false
                        }
                    };
                    debug!(path = %path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, ?reason, outcome, removed, "录制文件打开结果已排空并完成回收检查");
                    // Keep the strong source owner alive until the weak-item queue is drained.
                    drop(task);
                })
                .detach();
        }
    }
    fn failed(&mut self, error: String) {
        self.state = LoadState::Failed(error);
    }
    fn reject(&mut self, error: JobFailure, outcome: &str) {
        self.failed(error.to_string());
        error.record_handled("停止打开录制文件并保留当前回放会话", outcome);
    }
    pub(super) fn loading(&self) -> bool {
        self.task.is_some()
    }
    pub(super) fn poll(&mut self) -> Option<Arc<DiskWaveformSource>> {
        let task = self.task.as_mut()?;
        let result = match task.receiver.try_recv() {
            Ok(JobRead::Item(source)) => {
                task.pending = Some((*source).clone());
                return None;
            }
            Ok(JobRead::Finished(outcome)) => outcome,
            Ok(JobRead::Empty) => return None,
            Ok(JobRead::Closed) => {
                let error = open_failure(
                    OpenError::ClosedBeforeFinished,
                    &self.path,
                    task.id,
                    task.revision,
                );
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "failed", "本地 Waveform 录制文件打开结果交付失败");
                self.cancel_open(OpenStop::DeliveryFailed);
                self.reject(error, "结果通道提前关闭");
                return None;
            }
            Err(error) => {
                let error = JobFailure::from(error).with_context(open_frame(
                    &self.path,
                    Some(task.id),
                    task.revision,
                ));
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "failed", "本地 Waveform 录制文件打开结果交付失败");
                self.cancel_open(OpenStop::DeliveryFailed);
                self.revision = self.revision.wrapping_add(1);
                self.reject(error, "结果交付失败");
                return None;
            }
        };
        let task = self.task.take()?;
        if let Err(error) = self.manager.remove_if_present(task.id) {
            error.record_handled("保留 Job 记录并继续交付已打开的回放", "记录清理失败");
        }
        if task.revision != self.revision {
            info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "discarded", "本地 Waveform 录制文件打开结果已过期，未提交回放");
            if let JobOutcome::Failed(error) = result {
                error.record_handled("放弃过期录制文件打开结果", "回放请求版本已改变");
            }
            return None;
        }
        self.revision = self.revision.wrapping_add(1);
        match result {
            JobOutcome::Completed => {
                let Some(source) =
                    task.pending
                        .and_then(|source| source.upgrade())
                        .filter(|source| {
                            task.owner
                                .get()
                                .is_some_and(|owner| Arc::ptr_eq(owner, source))
                        })
                else {
                    let failure = open_failure(
                        OpenError::CompletedWithoutSource,
                        &self.path,
                        task.id,
                        task.revision,
                    );
                    info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "failed", "本地 Waveform 录制文件打开未交付有效数据源");
                    self.reject(failure, "后台完成但未交付数据源");
                    return None;
                };
                let details = source.metadata().and_then(|metadata| {
                    metadata
                        .total_duration()
                        .map_err(WaveformReadError::from)
                        .and_then(|total| {
                            total.ok_or_else(|| {
                                bevy_widgetry::diagnostics::WidgetryFailure::new(
                                    bevy_widgetry::waveform::WaveformError::ReplayRequired {
                                        kind: metadata.kind,
                                    },
                                    bevy_widgetry::diagnostics::ErrorFrame::new(
                                        "gallery.waveform.replay.duration",
                                    ),
                                )
                                .into()
                            })
                        })
                        .map(|total| {
                            (
                                metadata.sample_rate,
                                metadata.channel_count,
                                total.as_secs(),
                            )
                        })
                });
                let (rate, channels, seconds) = match details {
                    Ok(details) => details,
                    Err(error) => {
                        info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "failed", "本地 Waveform 录制文件回放元信息无效");
                        self.reject(
                            error.with_context(open_frame(
                                &self.path,
                                Some(task.id),
                                task.revision,
                            )),
                            "回放元信息无效",
                        );
                        return None;
                    }
                };
                self.source = Some(source.clone());
                self.state = LoadState::Ready {
                    rate,
                    channels,
                    seconds,
                };
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, rate, channels, seconds, outcome = "loaded", "本地 Waveform 回放记录加载完成");
                Some(source)
            }
            JobOutcome::Failed(error) => {
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "failed", "本地 Waveform 录制文件打开失败");
                self.reject(
                    error.with_context(open_frame(&self.path, Some(task.id), task.revision)),
                    "后台打开失败",
                );
                None
            }
            JobOutcome::Canceled => {
                self.failed("recording open canceled".into());
                info!(path = %self.path.display(), job_id = task.id.get(), revision = task.revision, origin = ?task.origin, outcome = "canceled", "本地 Waveform 录制文件打开已取消");
                None
            }
        }
    }
    fn binding(&self) -> LocalizedText {
        let mut args = vec![("path", self.path.display().to_string())];
        let key = match &self.state {
            LoadState::Missing => WaveformKey::DiskMissing,
            LoadState::Loading => WaveformKey::DiskLoading,
            LoadState::Failed(error) => {
                args.push(("error", error.clone()));
                WaveformKey::DiskFailed
            }
            LoadState::Ready {
                rate,
                channels,
                seconds,
            } => {
                args.extend([
                    ("rate", rate.to_string()),
                    ("channels", channels.to_string()),
                    ("seconds", seconds.to_string()),
                ]);
                WaveformKey::DiskReady
            }
        };
        LocalizedText::with_args(TextKey::Waveform(key), args)
    }
}

impl Drop for DiskReplay {
    fn drop(&mut self) {
        self.cancel_open(OpenStop::PageLeft);
    }
}

pub(super) fn scene(disk: &DiskReplay) -> impl Scene + use<> {
    let binding = disk.binding();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), width: percent(100) }
        Children [
            @localized_binding(binding) WidgetryText Name::new("WaveformDiskStatus") template(|_|Ok(DiskStatus))--
            @WidgetryButton Node { align_self: AlignSelf::Start } Name::new("WaveformDiskReload") template(|_|Ok(Reload))
            Children [@localized(TextKey::Waveform(WaveformKey::DiskReload)) WidgetryText]
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.add_observer(
        |event: On<bevy::ui_widgets::Activate>,
         buttons: Query<(), With<Reload>>,
         sources: Option<ResMut<DemoSources>>|
         -> Result {
            if !buttons.contains(event.entity) {
                return Ok(());
            }
            let mut sources = sources.ok_or_else(|| {
                crate::diagnostics::at(
                    "Waveform demo sources missing",
                    bevy_widgetry::diagnostics::ErrorFrame::new("Waveform 重载按钮缺少页面数据源"),
                )
            })?;
            sources.disk.reload(OpenOrigin::User);
            Ok(())
        },
    )
    .add_systems(
        Update,
        project_status
            .after(drive)
            .run_if(in_state(GalleryPage::Waveform)),
    );
}

fn project_status(
    sources: Res<DemoSources>,
    mut texts: Query<(&mut LocalizedText, Ref<DiskStatus>)>,
    mut revision: Local<Option<u64>>,
) {
    if *revision == Some(sources.disk.revision)
        && texts.iter().all(|(_, status)| !status.is_added())
    {
        return;
    }
    *revision = Some(sources.disk.revision);
    for (mut text, _) in &mut texts {
        *text = sources.disk.binding();
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn recording(path: &std::path::Path) {
        bevy_widgetry_waveform_file::build_recording(
            path,
            WaveformMetadata {
                channel_count: 1,
                channel_labels: vec!["A".into()],
                sample_rate: 1000,
                unit: WaveformUnit::Volts,
                kind: WaveformSessionKind::Replay { total_frames: 100 },
            },
            |_, range, out| {
                out.extend(range.map(|_| Some(1.0)));
                Ok(())
            },
        )
        .unwrap();
    }
    fn empty(path: PathBuf) -> DiskReplay {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        DiskReplay {
            source: None,
            task: None,
            manager: JobManager::default(),
            path,
            state: LoadState::Missing,
            revision: 0,
        }
    }
    fn wait(mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    fn delayed_open(
        disk: &mut DiskReplay,
        path: PathBuf,
    ) -> (JobId, JobId, std::sync::mpsc::Sender<()>) {
        let manager = disk.manager.clone();
        let (release, gate) = std::sync::mpsc::channel();
        let (entered, ready) = std::sync::mpsc::channel();
        let owner = Arc::new(OnceLock::new());
        let thread_owner = owner.clone();
        let (id, receiver) = disk
            .manager
            .spawn_thread::<Weak<DiskWaveformSource>, _>(Default::default(), move |context| {
                let source = Arc::new(DiskWaveformSource::open(&path, &manager)?);
                assert!(thread_owner.set(source.clone()).is_ok());
                assert_eq!(
                    context.send_blocking(Arc::downgrade(&source))?,
                    SendStatus::Sent
                );
                entered.send(source.job_id()).unwrap();
                gate.recv().unwrap();
                context.acknowledge_cancel();
                Ok(())
            })
            .unwrap();
        let reader = ready.recv_timeout(Duration::from_secs(5)).unwrap();
        disk.task = Some(OpenJob {
            origin: OpenOrigin::User,
            id,
            revision: 0,
            receiver,
            pending: None,
            owner,
        });
        (id, reader, release)
    }

    #[test]
    fn a_missing_completed_source_is_diagnosed_and_normal_cancellation_is_not() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        for cancel in [false, true] {
            let mut disk = empty(PathBuf::from("completed-without-source.widx"));
            let (release, gate) = std::sync::mpsc::channel();
            let (id, receiver) = disk
                .manager
                .spawn_thread::<Weak<DiskWaveformSource>, _>(Default::default(), move |context| {
                    gate.recv().unwrap();
                    if context.is_cancelled() {
                        context.acknowledge_cancel();
                    }
                    Ok(())
                })
                .unwrap();
            disk.task = Some(OpenJob {
                origin: OpenOrigin::User,
                id,
                revision: disk.revision,
                receiver,
                pending: None,
                owner: Arc::new(OnceLock::new()),
            });
            if cancel {
                disk.manager.cancel(id).unwrap();
            }
            release.send(()).unwrap();
            wait(|| disk.manager.status(id).unwrap().state.is_finished());
            let logs = LogCapture::with_targets([
                "bevy_widgetry",
                "widget_gallery::pages::waveform::disk",
            ]);
            assert!(logs.run(|| disk.poll()).is_none());
            assert!(matches!(disk.state, LoadState::Failed(_)));
            let records = logs.records();
            let results: Vec<_> = records
                .iter()
                .filter(|record| record.level == bevy::log::Level::INFO)
                .collect();
            assert_eq!(results.len(), 1);
            assert!(results[0].matches_field("job_id", &id.get().to_string()));
            assert!(results[0].matches_field("revision", "0"));
            assert!(results[0].matches_field("origin", "User"));
            assert!(
                results[0].matches_field("outcome", if cancel { "canceled" } else { "failed" })
            );
            assert!(logs.run(|| disk.poll()).is_none());
            assert_eq!(logs.records().len(), records.len());
            let sources: Vec<_> = records
                .iter()
                .filter(|r| r.phase() == Some(DiagnosticPhase::Source))
                .collect();
            if cancel {
                assert!(sources.is_empty());
            } else {
                assert_eq!(sources.len(), 1);
                let id = sources[0].fields.get("error_id").unwrap();
                assert!(
                    sources[0]
                        .fields
                        .get("context")
                        .unwrap()
                        .contains("completed-without-source.widx")
                );
                assert_eq!(
                    records
                        .iter()
                        .filter(|r| r.phase() == Some(DiagnosticPhase::Handled)
                            && r.matches_field("error_id", id))
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn global_cancel_before_source_delivery_finishes_canceled_without_a_false_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(path.clone());
        let source = Arc::new(DiskWaveformSource::open(&path, &disk.manager).unwrap());
        let owner = Arc::new(OnceLock::new());
        assert!(owner.set(source.clone()).is_ok());
        let (release, gate) = std::sync::mpsc::channel();
        let (entered, ready) = std::sync::mpsc::channel();
        let (id, receiver) = disk
            .manager
            .spawn_thread::<Weak<DiskWaveformSource>, _>(Default::default(), move |context| {
                entered.send(()).unwrap();
                gate.recv().unwrap();
                deliver_source(&context, &source)
            })
            .unwrap();
        disk.task = Some(OpenJob {
            origin: OpenOrigin::User,
            id,
            revision: disk.revision,
            receiver,
            pending: None,
            owner,
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let shutdown = disk.manager.begin_shutdown(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
        wait(|| disk.manager.status(id).unwrap().state.is_finished());
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        assert!(logs.run(|| disk.poll()).is_none());
        assert!(
            logs.records()
                .iter()
                .all(|r| r.phase() != Some(bevy_widgetry_test_utils::DiagnosticPhase::Source))
        );
        assert!(
            matches!(disk.state, LoadState::Failed(ref message) if message == "recording open canceled")
        );
        wait(|| {
            matches!(
                shutdown.try_recv(),
                Some(bevy_widgetry::job::ShutdownMessage::Completed)
            )
        });
    }

    #[test]
    fn unread_open_item_has_an_owner_outside_the_broadcast_queue() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(path);
        disk.reload(OpenOrigin::User);
        let task = disk.task.as_ref().unwrap();
        wait(|| disk.manager.status(task.id).unwrap().state.is_finished());
        assert!(task.pending.is_none());
        assert!(
            task.owner.get().is_some(),
            "unread source must have a strong owner outside the broadcast queue to avoid reentrant Manager cancellation during receiver teardown"
        );
    }

    #[test]
    fn unread_open_item_can_be_discarded_while_global_shutdown_finishes_the_open() {
        for leaving_page in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("recording.widx");
            recording(&path);
            let mut disk = empty(directory.path().join("missing.widx"));
            let (id, reader, release) = delayed_open(&mut disk, path);
            assert!(disk.task.as_ref().unwrap().pending.is_none());
            let manager = disk.manager.clone();
            let mut observer = manager.subscribe::<Weak<DiskWaveformSource>>(id).unwrap();
            let closing_manager = manager.clone();
            let barrier = Arc::new(std::sync::Barrier::new(3));
            let closing_barrier = barrier.clone();
            let discarding_barrier = barrier.clone();
            let (closed, shutdown) = std::sync::mpsc::channel();
            let (discarded, done) = std::sync::mpsc::channel();
            let closer = std::thread::spawn(move || {
                closing_barrier.wait();
                closed
                    .send(
                        closing_manager
                            .begin_shutdown(Duration::from_secs(5))
                            .unwrap(),
                    )
                    .unwrap();
            });
            let discarder = std::thread::spawn(move || {
                discarding_barrier.wait();
                if !leaving_page {
                    disk.reload(OpenOrigin::User);
                    assert!(matches!(disk.state, LoadState::Missing));
                }
                drop(disk);
                discarded.send(()).unwrap();
            });
            barrier.wait();
            release.send(()).unwrap();
            done.recv_timeout(Duration::from_secs(5)).unwrap();
            let shutdown = shutdown.recv_timeout(Duration::from_secs(5)).unwrap();
            closer.join().unwrap();
            discarder.join().unwrap();
            wait(|| {
                matches!(
                    shutdown.try_recv(),
                    Some(bevy_widgetry::job::ShutdownMessage::Completed)
                )
            });
            loop {
                match observer.recv_blocking().unwrap() {
                    JobRead::Item(_) => {}
                    JobRead::Finished(JobOutcome::Completed | JobOutcome::Canceled) => break,
                    other => panic!("unexpected open result: {other:?}"),
                }
            }
            wait(|| manager.find_status(id).is_none());
            assert_eq!(manager.outcome(reader).unwrap(), Some(JobOutcome::Canceled));
        }
    }

    #[test]
    fn open_commits_only_after_finished_and_failed_reload_keeps_the_previous_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(path);
        let logs = bevy_widgetry_test_utils::LogCapture::with_targets([
            "widget_gallery::pages::waveform::disk",
        ]);
        logs.run(|| disk.reload(OpenOrigin::User));
        let id = disk.task.as_ref().unwrap().id;
        let revision = disk.task.as_ref().unwrap().revision;
        wait(|| disk.manager.status(id).unwrap().state.is_finished());
        assert!(logs.run(|| disk.poll()).is_none());
        assert!(disk.source.is_none());
        assert!(matches!(disk.state, LoadState::Loading));
        assert!(
            logs.records()
                .iter()
                .all(|record| !record.matches_field("outcome", "loaded"))
        );
        let source = logs.run(|| disk.poll()).unwrap();
        let loaded: Vec<_> = logs
            .records()
            .into_iter()
            .filter(|record| record.matches_field("outcome", "loaded"))
            .collect();
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].matches_field("job_id", &id.get().to_string()));
        assert!(loaded[0].matches_field("revision", &revision.to_string()));
        assert!(loaded[0].matches_field("origin", "User"));
        assert!(Arc::ptr_eq(disk.source.as_ref().unwrap(), &source));
        assert!(disk.manager.status(id).is_err());
        let reader = source.job_id();
        assert!(!disk.manager.status(reader).unwrap().state.is_finished());
        let invalid = directory.path().join("invalid.widx");
        std::fs::write(&invalid, "invalid recording").unwrap();
        disk.path = invalid;
        logs.run(|| disk.reload(OpenOrigin::User));
        let failed_id = disk.task.as_ref().unwrap().id;
        wait(|| {
            assert!(logs.run(|| disk.poll()).is_none());
            !disk.loading()
        });
        let failed: Vec<_> = logs
            .records()
            .into_iter()
            .filter(|record| record.matches_field("outcome", "failed"))
            .collect();
        assert_eq!(failed.len(), 1);
        assert!(failed[0].matches_field("job_id", &failed_id.get().to_string()));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("outcome", "loaded"))
                .count(),
            1
        );
        assert!(matches!(disk.state, LoadState::Failed(_)));
        assert!(Arc::ptr_eq(disk.source.as_ref().unwrap(), &source));
        let manager = disk.manager.clone();
        drop(source);
        drop(disk);
        wait(|| manager.status(reader).unwrap().state.is_finished());
        assert_eq!(manager.outcome(reader).unwrap(), Some(JobOutcome::Canceled));
    }

    #[test]
    fn reload_discards_unfinished_open_item_and_cancels_its_reader() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(directory.path().join("missing.widx"));
        let (id, reader, release) = delayed_open(&mut disk, path);
        let mut observer = disk
            .manager
            .subscribe::<Weak<DiskWaveformSource>>(id)
            .unwrap();
        wait(|| {
            drop(disk.poll());
            disk.task.as_ref().unwrap().pending.is_some()
        });
        assert!(disk.source.is_none());
        let logs = bevy_widgetry_test_utils::LogCapture::with_targets([
            "widget_gallery::pages::waveform::disk",
        ]);
        logs.run(|| disk.reload(OpenOrigin::User));
        let discarded: Vec<_> = logs
            .records()
            .into_iter()
            .filter(|record| record.matches_field("outcome", "discarded"))
            .collect();
        assert_eq!(discarded.len(), 1);
        assert!(discarded[0].matches_field("job_id", &id.get().to_string()));
        assert!(discarded[0].matches_field("reason", "Replaced"));
        assert!(
            logs.records()
                .iter()
                .all(|record| !record.matches_field("outcome", "canceled")
                    && !record.matches_field("outcome", "loaded"))
        );
        assert!(matches!(disk.state, LoadState::Missing));
        assert!(disk.task.is_none());
        assert!(disk.source.is_none());
        assert_eq!(
            disk.manager.status(id).unwrap().state,
            bevy_widgetry::job::JobState::Stopping
        );
        release.send(()).unwrap();
        wait(|| {
            disk.manager.find_status(id).is_none()
                && disk.manager.status(reader).unwrap().state.is_finished()
        });
        loop {
            match observer.recv_blocking().unwrap() {
                JobRead::Item(_) => {}
                JobRead::Finished(JobOutcome::Canceled) => break,
                other => panic!("unexpected open result: {other:?}"),
            }
        }
        assert_eq!(
            disk.manager.outcome(reader).unwrap(),
            Some(JobOutcome::Canceled)
        );
        assert!(disk.source.is_none());
    }

    #[test]
    fn abandoned_open_failure_is_consumed_after_reload_or_page_exit() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        const CHILD: &str = "GALLERY_ABANDONED_WAVEFORM_OPEN_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .env(CHILD, "1")
                .args(["--exact", "pages::waveform::disk::tests::abandoned_open_failure_is_consumed_after_reload_or_page_exit", "--nocapture"])
                .output().unwrap();
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
        for leaving in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let invalid = directory.path().join("invalid.widx");
            std::fs::write(&invalid, "invalid recording").unwrap();
            let mut disk = empty(directory.path().join("missing.widx"));
            let manager = disk.manager.clone();
            let worker_manager = manager.clone();
            let (entered, ready) = std::sync::mpsc::channel();
            let (release, gate) = std::sync::mpsc::channel();
            let (id, receiver) = manager
                .spawn_thread::<Weak<DiskWaveformSource>, _>(Default::default(), move |_| {
                    entered.send(()).unwrap();
                    gate.recv().unwrap();
                    DiskWaveformSource::open(&invalid, &worker_manager)?;
                    Ok(())
                })
                .unwrap();
            disk.task = Some(OpenJob {
                origin: OpenOrigin::User,
                id,
                revision: 0,
                receiver,
                pending: None,
                owner: Arc::new(OnceLock::new()),
            });
            let mut observer = manager.subscribe::<Weak<DiskWaveformSource>>(id).unwrap();
            ready.recv_timeout(Duration::from_secs(5)).unwrap();
            if leaving {
                drop(disk);
            } else {
                disk.reload(OpenOrigin::User);
                drop(disk);
            }
            assert_eq!(
                manager.status(id).unwrap().state,
                bevy_widgetry::job::JobState::Stopping
            );
            release.send(()).unwrap();
            let failure = match observer.recv_blocking().unwrap() {
                JobRead::Finished(JobOutcome::Failed(failure)) => failure,
                other => panic!("unexpected result: {other:?}"),
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
        }
    }

    #[test]
    fn recreated_status_projects_ready_metadata_when_revision_repeats() {
        let mut app = App::new();
        app.add_systems(Update, project_status);
        let sources = || {
            let live = Arc::new(LiveSource::new(vec!["A".into()], 1, 1, false));
            DemoSources {
                basic: live.clone(),
                stress: live,
                replay: None,
                disk: DiskReplay {
                    source: None,
                    task: None,
                    manager: JobManager::default(),
                    path: "reentry.widx".into(),
                    state: LoadState::Ready {
                        rate: 8000,
                        channels: 16,
                        seconds: 1800,
                    },
                    revision: 2,
                },
                mode: DemoMode::Disk,
                last_seek: None,
                gestures: HashMap::new(),
                asset: Handle::default(),
                asset_failure: None,
                elapsed: Duration::ZERO,
                last: None,
                replay_start: None,
                generated_at: Instant::now(),
                frame_ms: 0.0,
                producer_ms: 0.0,
                burst: false,
                hold_until: None,
            }
        };
        let mut previous = None;
        for _ in 0..2 {
            if let Some(entity) = previous.take() {
                app.world_mut().despawn(entity);
                app.world_mut().remove_resource::<DemoSources>();
            }
            app.insert_resource(sources());
            let entity = app
                .world_mut()
                .spawn((
                    DiskStatus,
                    LocalizedText::new(TextKey::Waveform(WaveformKey::DiskLoading)),
                ))
                .id();
            app.update();
            let rendered = app
                .world()
                .get::<LocalizedText>(entity)
                .unwrap()
                .render(Language::English);
            assert!(rendered.contains("8000 Hz"), "{rendered}");
            assert!(rendered.contains("16 channels"), "{rendered}");
            previous = Some(entity);
        }
    }

    #[test]
    fn unrelated_activation_without_waveform_page_does_not_require_its_resource() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        install(&mut app);
        let unrelated = app.world_mut().spawn_empty().id();
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        errors.run(|| {
            app.world_mut()
                .trigger(bevy::ui_widgets::Activate { entity: unrelated });
            app.world_mut().flush();
        });
        assert!(errors.take().is_empty());
    }
}
