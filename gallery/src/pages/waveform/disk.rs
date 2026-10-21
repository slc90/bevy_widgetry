use super::*;
use crate::i18n::localized_binding;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry_waveform_file::DiskWaveformSource;
use std::path::PathBuf;

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
    task: Option<Task<Result<DiskWaveformSource, WaveformReadError>>>,
    path: PathBuf,
    state: LoadState,
    revision: u64,
}

#[derive(Component)]
struct DiskStatus;
#[derive(Component)]
struct Reload;

impl DiskReplay {
    pub(super) fn new() -> Self {
        let path = std::env::var_os("GALLERY_WAVEFORM_REPLAY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/waveform/replay.widx")
            });
        let mut state = Self {
            source: None,
            task: None,
            path,
            state: LoadState::Missing,
            revision: 0,
        };
        state.reload();
        state
    }
    fn reload(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.task = None;
        if !self.path.is_file() {
            self.state = LoadState::Missing;
            return;
        }
        let path = self.path.clone();
        self.state = LoadState::Loading;
        self.task = Some(IoTaskPool::get().spawn(async move { DiskWaveformSource::open(&path) }));
    }
    pub(super) fn loading(&self) -> bool {
        self.task.is_some()
    }
    pub(super) fn poll(&mut self) -> Option<Arc<DiskWaveformSource>> {
        let result = block_on(poll_once(self.task.as_mut()?))?;
        self.revision = self.revision.wrapping_add(1);
        self.task = None;
        match result {
            Ok(source) => {
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
                let source = Arc::new(source);
                self.source = Some(source.clone());
                self.state = LoadState::Ready {
                    rate,
                    channels,
                    seconds,
                };
                info!(path = %self.path.display(), "本地 Waveform 回放记录加载完成");
                Some(source)
            }
            Err(error) => {
                warn!(path = %self.path.display(), %error, "本地 Waveform 回放记录加载失败，保留当前会话");
                self.state = LoadState::Failed(error.to_string());
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
