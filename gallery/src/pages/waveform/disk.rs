use super::*;
use crate::i18n::localized_binding;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::job::{
    JobId, JobManager, JobMessage, JobOptions, JobOutcome, JobReceiver, SendError, TryRecvError,
};
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry_waveform_file::DiskWaveformSource;
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
    receiver: JobReceiver<Weak<DiskWaveformSource>>,
    pending: Option<Weak<DiskWaveformSource>>,
    // Dropping an unread broadcast Item must not cancel its Reader while the channel lock is held.
    owner: Arc<OnceLock<Arc<DiskWaveformSource>>>,
}

#[derive(Component)]
struct DiskStatus;
#[derive(Component)]
struct Reload;

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
        state.reload();
        state
    }
    fn reload(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.cancel_open();
        if !self.path.is_file() {
            self.state = LoadState::Missing;
            return;
        }
        let path = self.path.clone();
        self.state = LoadState::Loading;
        let manager = self.manager.clone();
        let owner = Arc::new(OnceLock::new());
        let thread_owner = owner.clone();
        match self
            .manager
            .spawn_thread::<Weak<DiskWaveformSource>, _, String>(
                JobOptions {
                    label: Some("Waveform open recording".into()),
                    ..default()
                },
                move |context| {
                    if context.is_cancelled() {
                        context.acknowledge_cancel();
                        return Ok(());
                    }
                    let source = Arc::new(
                        DiskWaveformSource::open(&path, &manager)
                            .map_err(|error| error.to_string())?,
                    );
                    if thread_owner.set(source.clone()).is_err() {
                        error!("本地 Waveform 加载源所有权重复初始化");
                        return Err("recording source owner already initialized".into());
                    }
                    match context.send_blocking(Arc::downgrade(&source)) {
                        Ok(()) | Err(SendError::Canceled) => Ok(()),
                        Err(error) => Err(error.to_string()),
                    }
                },
            ) {
            Ok((id, receiver)) => {
                self.task = Some(OpenJob {
                    id,
                    revision: self.revision,
                    receiver,
                    pending: None,
                    owner,
                })
            }
            Err(error) => self.failed(error.to_string()),
        }
    }
    fn cancel_open(&mut self) {
        if let Some(task) = self.task.take()
            && let Err(error) = self.manager.cancel(task.id)
        {
            warn!(%error, "取消本地 Waveform 加载失败");
        }
    }
    fn failed(&mut self, error: String) {
        warn!(path = %self.path.display(), %error, "本地 Waveform 回放记录加载失败，保留当前会话");
        self.state = LoadState::Failed(error);
    }
    pub(super) fn loading(&self) -> bool {
        self.task.is_some()
    }
    pub(super) fn poll(&mut self) -> Option<Arc<DiskWaveformSource>> {
        let task = self.task.as_mut()?;
        let result = match task.receiver.try_recv() {
            Ok(JobMessage::Item(source)) => {
                task.pending = Some((*source).clone());
                return None;
            }
            Ok(JobMessage::Finished(outcome)) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(error) => {
                self.cancel_open();
                self.revision = self.revision.wrapping_add(1);
                self.failed(error.to_string());
                return None;
            }
        };
        let task = self.task.take()?;
        if let Err(error) = self.manager.remove(task.id) {
            warn!(%error, "移除本地 Waveform 加载记录失败");
        }
        if task.revision != self.revision {
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
                    self.failed("recording open completed without a delivered source".into());
                    return None;
                };
                let details = source.metadata().and_then(|metadata| {
                    metadata
                        .total_duration()
                        .map_err(|error| WaveformReadError(error.to_string()))
                        .and_then(|total| {
                            total.ok_or_else(|| WaveformReadError("Replay duration missing".into()))
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
                        warn!(%error, "本地 Waveform 回放元信息无效");
                        self.state = LoadState::Failed(error.to_string());
                        return None;
                    }
                };
                self.source = Some(source.clone());
                self.state = LoadState::Ready {
                    rate,
                    channels,
                    seconds,
                };
                info!(path = %self.path.display(), "本地 Waveform 回放记录加载完成");
                Some(source)
            }
            JobOutcome::Failed(error) => {
                self.failed(error.to_string());
                None
            }
            JobOutcome::Canceled => {
                self.failed("recording open canceled".into());
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
        self.cancel_open();
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
                error!("Waveform 重载按钮缺少页面数据源");
                BevyError::error("Waveform demo sources missing")
            })?;
            sources.disk.reload();
            info!("请求重新加载本地 Waveform 回放记录");
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
            .spawn_thread::<Weak<DiskWaveformSource>, _, String>(
                Default::default(),
                move |context| {
                    let source = Arc::new(
                        DiskWaveformSource::open(&path, &manager)
                            .map_err(|error| error.to_string())?,
                    );
                    assert!(thread_owner.set(source.clone()).is_ok());
                    context
                        .send_blocking(Arc::downgrade(&source))
                        .map_err(|error| error.to_string())?;
                    entered.send(source.job_id()).unwrap();
                    gate.recv().unwrap();
                    context.acknowledge_cancel();
                    Ok(())
                },
            )
            .unwrap();
        let reader = ready.recv_timeout(Duration::from_secs(5)).unwrap();
        disk.task = Some(OpenJob {
            id,
            revision: 0,
            receiver,
            pending: None,
            owner,
        });
        (id, reader, release)
    }

    #[test]
    fn unread_open_item_has_an_owner_outside_the_broadcast_queue() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(path);
        disk.reload();
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
                    disk.reload();
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
            assert!(matches!(
                manager.outcome(id).unwrap(),
                Some(JobOutcome::Completed | JobOutcome::Canceled)
            ));
            assert_eq!(manager.outcome(reader).unwrap(), Some(JobOutcome::Canceled));
        }
    }

    #[test]
    fn open_commits_only_after_finished_and_failed_reload_keeps_the_previous_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.widx");
        recording(&path);
        let mut disk = empty(path);
        disk.reload();
        let id = disk.task.as_ref().unwrap().id;
        wait(|| disk.manager.status(id).unwrap().state.is_finished());
        assert!(disk.poll().is_none());
        assert!(disk.source.is_none());
        assert!(matches!(disk.state, LoadState::Loading));
        let source = disk.poll().unwrap();
        assert!(Arc::ptr_eq(disk.source.as_ref().unwrap(), &source));
        assert!(disk.manager.status(id).is_err());
        let reader = source.job_id();
        assert!(!disk.manager.status(reader).unwrap().state.is_finished());
        let invalid = directory.path().join("invalid.widx");
        std::fs::write(&invalid, "invalid recording").unwrap();
        disk.path = invalid;
        disk.reload();
        wait(|| {
            assert!(disk.poll().is_none());
            !disk.loading()
        });
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
        wait(|| {
            drop(disk.poll());
            disk.task.as_ref().unwrap().pending.is_some()
        });
        assert!(disk.source.is_none());
        disk.reload();
        assert!(matches!(disk.state, LoadState::Missing));
        assert!(disk.task.is_none());
        assert!(disk.source.is_none());
        assert_eq!(
            disk.manager.status(id).unwrap().state,
            bevy_widgetry::job::JobState::Stopping
        );
        release.send(()).unwrap();
        wait(|| {
            disk.manager.status(id).unwrap().state.is_finished()
                && disk.manager.status(reader).unwrap().state.is_finished()
        });
        assert_eq!(
            disk.manager.outcome(id).unwrap(),
            Some(JobOutcome::Canceled)
        );
        assert_eq!(
            disk.manager.outcome(reader).unwrap(),
            Some(JobOutcome::Canceled)
        );
        assert!(disk.source.is_none());
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
