use crate::assets::GalleryWaveform;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::NavigationKey;
use crate::i18n::WaveformKey;
use crate::i18n::{Language, LocalizedText, TextKey, localized};
use crate::waveform_data::{LiveSource, ReplayAsset, ReplaySource};
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, block_on, poll_once};
use bevy::window::RequestRedraw;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry::list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry::waveform::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct WaveformDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let source = world.resource::<DemoSources>().basic.clone();
    let view = move || {
        bsn! {
            @Waveform { @source: {Some(source.clone() as Arc<dyn WaveformSource>)},
                @config: {WaveformConfig { visible_duration_ms: 5000, amplitude_range: WaveformAmplitude { value: 1.0, unit: WaveformUnit::Volts } }} }
            template(|_| Ok(ColorWaveform))
            Node { width: percent(100), height: px(160) }
        }
    };
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::Waveform),
        TextKey::Waveform(WaveformKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D),
        view(),
        view(),
        |world, entity, apply| {
            if !apply {
                return WidgetryWaveformColorOverrides::clear_in_world(world, entity);
            }
            let mut colors = WidgetryWaveformColorOverrides::default();
            colors.normal.palette = Some(vec![
                crate::color_showcase::PURPLE,
                crate::color_showcase::TEAL,
                Color::srgb_u8(180, 83, 9),
                Color::srgb_u8(190, 24, 93),
            ]);
            WidgetryWaveformColorOverrides::set_in_world(world, entity, colors)
        },
    )
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WaveformDemoSystems {
    Status,
}

#[derive(Resource)]
pub(crate) struct DemoSources {
    basic: Arc<LiveSource>,
    stress: Arc<LiveSource>,
    replay: Option<Arc<dyn WaveformSource>>,
    disk_load: Option<Task<Result<Arc<dyn WaveformSource>, WaveformReadError>>>,
    disk_requested: bool,
    gestures: HashMap<Entity, WaveformPlayback>,
    asset: Handle<ReplayAsset>,
    elapsed: Duration,
    last: Option<Instant>,
    replay_start: Option<Duration>,
    pub generated_at: Instant,
    pub frame_ms: f64,
    pub producer_ms: f64,
    pub burst: bool,
    pub hold_until: Option<Instant>,
}

#[derive(Component, Clone, Copy)]
enum DemoKind {
    Basic,
    Replay,
    Stress,
}

#[derive(Component)]
struct ColorWaveform;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct WaveformDemoState {
    pub sample_rate: u32,
    pub channels: usize,
    pub session_generation: u64,
    pub running: bool,
    pub duration_ms: u64,
    pub amplitude_volts: f64,
    pub producer_frame: u64,
    pub displayed_frame: u64,
    pub buffered_start: u64,
    pub viewport_start: u64,
    pub output_width: usize,
    pub height: f32,
    pub row_height: f32,
    pub scroll_offset: f32,
    pub envelope: bool,
    pub algorithm: String,
    pub playback_seconds: f64,
    pub playback_speed: f64,
    pub preview: bool,
    pub loop_count: u64,
    pub dropped_samples: u64,
    missing_until: u64,
    pub read_frames: usize,
    pub raw_capacity_bytes: usize,
    pub reduced_capacity_bytes: usize,
}

#[derive(Component)]
struct StressStatus;

#[derive(Clone, Copy)]
struct AlgorithmChoice(WaveformAlgorithm);

#[derive(Component, FromTemplate)]
struct AlgorithmParts {
    model: Entity,
    combo: Entity,
}

fn algorithm_selector() -> impl Scene {
    bsn! {
        AlgorithmParts { model: #AlgorithmModel, combo: #AlgorithmCombo }
        Node { align_items: AlignItems::Center, column_gap: px(12) }
        Children [
            #AlgorithmModel
            template(|_| {
                let mut model = WidgetryListModel::default();
                model.push(AlgorithmChoice(WaveformAlgorithm::MinMax))?;
                model.push(AlgorithmChoice(WaveformAlgorithm::Lttb))?;
                Ok(model)
            })--
            @localized(TextKey::Waveform(WaveformKey::Algorithm)) bevy_widgetry::text::WidgetryText--
            #AlgorithmCombo
            Name::new("WaveformAlgorithm") Node { width: px(140) } on(change_algorithm)--
            @localized(TextKey::Waveform(WaveformKey::AlgorithmHint)) bevy_widgetry::text::WidgetryText
        ]
    }
}

fn change_algorithm(
    event: On<bevy::ui_widgets::ValueChange<Option<WidgetryListItemId>>>,
    combos: Query<&WidgetryComboBox<AlgorithmChoice>>,
    models: Query<&WidgetryListModel<AlgorithmChoice>>,
    roots: Query<Entity, With<DemoKind>>,
    mut commands: Commands,
) -> Result {
    let Some(id) = event.value else {
        return Ok(());
    };
    let combo = combos.get(event.source)?;
    let choice = models
        .get(combo.source())?
        .get_by_id(id)
        .ok_or_else(|| BevyError::error("algorithm choice missing"))?;
    for root in &roots {
        Waveform::set_algorithm(&mut commands, root, choice.0);
    }
    Ok(())
}

impl FromWorld for DemoSources {
    fn from_world(world: &mut World) -> Self {
        Self {
            basic: Arc::new(LiveSource::new(
                vec![
                    "Sine".into(),
                    "Square".into(),
                    "Sawtooth".into(),
                    "Triangle".into(),
                ],
                3100,
                100,
                true,
            )),
            stress: Arc::new(LiveSource::new(
                (1..=64).map(|index| format!("Electrode {index}")).collect(),
                704000,
                64000,
                false,
            )),
            replay: None,
            disk_requested: std::env::var_os("GALLERY_WAVEFORM_REPLAY").is_some(),
            disk_load: std::env::var_os("GALLERY_WAVEFORM_REPLAY").map(|path| {
                IoTaskPool::get().spawn(async move {
                    bevy_widgetry_waveform_file::DiskWaveformSource::open(std::path::Path::new(
                        &path,
                    ))
                    .map(|source| Arc::new(source) as Arc<dyn WaveformSource>)
                })
            }),
            gestures: HashMap::new(),
            asset: world
                .resource::<AssetServer>()
                .load(GalleryWaveform::BasicReplay.path()),
            elapsed: Duration::ZERO,
            last: None,
            replay_start: None,
            generated_at: Instant::now(),
            frame_ms: 0.0,
            producer_ms: 0.0,
            burst: false,
            hold_until: None,
        }
    }
}

pub(crate) fn scene(sources: &DemoSources) -> impl Scene + use<> {
    let basic = waveform(DemoKind::Basic, Some(sources.basic.clone()));
    let replay = waveform(
        DemoKind::Replay,
        sources
            .replay
            .clone()
            .map(|source| source as Arc<dyn WaveformSource>),
    );
    let stress = waveform(DemoKind::Stress, Some(sources.stress.clone()));
    bsn! {
        #WaveformDemo
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12) }
        Children [
            Node { column_gap: px(20), align_items: AlignItems::Center }
            Children [
                @localized(TextKey::Navigation(NavigationKey::Waveform)) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(24.0) }--
                @localized(TextKey::Waveform(WaveformKey::MeasurementHint)) bevy_widgetry::text::WidgetryText
            ]--
            #WaveformGridToggle
            @bevy_widgetry::check_box::WidgetryCheckBox bevy::ui::Checked on(toggle_grid)
            Children [@localized(TextKey::Waveform(WaveformKey::ShowTimeGrid)) bevy_widgetry::text::WidgetryText]--
            @{algorithm_selector()}--
            #BasicWaveforms Node { width: percent(100), height: px(270), flex_shrink: 0.0, column_gap: px(16) } Children [
                Node { width: px(790), height: px(230), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: px(6) } Children [
                    @localized(TextKey::Waveform(WaveformKey::BasicLive100Hz4Channels5)) bevy_widgetry::text::WidgetryText--
                    #BasicLiveWaveform @{basic}
                ]--
                Node { width: px(790), height: px(270), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: px(6) } Children [
                    @localized(TextKey::Waveform(WaveformKey::BasicReplay8Khz4Channels5)) bevy_widgetry::text::WidgetryText--
                    #BasicReplayWaveform @{replay}
                ]
            ]--
            @localized(TextKey::Waveform(WaveformKey::StressLive64Khz64Channels10)) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(20.0) }--
            #StressWaveform @{stress}--
            #WaveformStatus @localized(TextKey::Waveform(WaveformKey::WaitingForWaveformPage)) bevy_widgetry::text::WidgetryText template(|_| Ok(StressStatus))
        ]
    }
}

fn waveform(kind: DemoKind, adapter: Option<Arc<dyn WaveformSource>>) -> impl Scene {
    let (duration, width, height) = match kind {
        DemoKind::Basic => (5000, 790.0, 200.0),
        DemoKind::Replay => (5000, 790.0, 240.0),
        DemoKind::Stress => (10000, 1600.0, 640.0),
    };
    let choices = match kind {
        DemoKind::Basic | DemoKind::Replay => vec![1000, 5000, 10000, 30000],
        DemoKind::Stress => vec![1000, 5000, 10000],
    };
    bsn! {
        @Waveform { @source: {adapter}, @config: {WaveformConfig { visible_duration_ms: duration, amplitude_range: WaveformAmplitude { value: 1.0, unit: WaveformUnit::Volts } }}, @time_ranges_ms: {choices}, @channels_per_screen: {if matches!(kind, DemoKind::Stress) { WaveformChannelCount::Count(16) } else { WaveformChannelCount::All }},
            @channel_counts: {vec![WaveformChannelCount::Count(8), WaveformChannelCount::Count(16), WaveformChannelCount::Count(32), WaveformChannelCount::All]}, @amplitude_ranges: {vec![WaveformAmplitude { value: 0.25, unit: WaveformUnit::Volts }, WaveformAmplitude::default(), WaveformAmplitude { value: 2.0, unit: WaveformUnit::Volts }]} }
        template(move |_| Ok(kind))
        template(|_| Ok(WaveformDemoState::default()))
        Node { width: px(width), height: px(height), flex_shrink: 0.0 }
    }
}

fn visible(world: &World, root: Entity) -> bool {
    let mut current = Some(root);
    while let Some(entity) = current {
        if world
            .get::<Node>(entity)
            .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    true
}

fn drive(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query::<(Entity, &DemoKind)>()
        .iter(world)
        .map(|(e, kind)| (e, *kind))
        .collect();
    if !roots.iter().any(|(e, _)| visible(world, *e)) {
        world.resource_mut::<DemoSources>().last = None;
        return Ok(());
    }
    let result = world.resource_scope(|world, mut sources: Mut<DemoSources>| -> Result {
        let now = Instant::now();
        let delta = sources
            .last
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        sources.last = Some(now);
        sources.elapsed += delta;
        sources.frame_ms = delta.as_secs_f64() * 1000.0;
        sources.burst = sources.hold_until.is_some();
        if let Some(until) = sources.hold_until {
            if now < until {
                world.write_message(RequestRedraw);
                return Ok(());
            }
            sources.hold_until = None;
        }
        let start = Instant::now();
        let basic_frame = sample_boundary(sources.elapsed, 100)?;
        let stress_frame = sample_boundary(sources.elapsed, 64000)?;
        sources.basic.produce(basic_frame, 100, true)?;
        sources.stress.produce(stress_frame, 64000, false)?;
        sources.generated_at = start;
        sources.producer_ms = start.elapsed().as_secs_f64() * 1000.0;
        sources.gestures.retain(|root, saved| {
            world
                .get::<WaveformRuntime>(*root)
                .is_some_and(|runtime| runtime.session_generation() == saved.session_generation)
        });
        if let Some(task) = &mut sources.disk_load
            && let Some(loaded) = block_on(poll_once(task))
        {
            sources.disk_load = None;
            match loaded {
                Ok(source) => {
                    sources.replay = Some(source);
                    info!("本地 Waveform 回放记录加载完成");
                }
                Err(error) => {
                    warn!(%error, "本地 Waveform 回放记录加载失败");
                }
            }
        }
        if !sources.disk_requested
            && sources.replay_start.is_none()
            && let Some(asset) = world.resource::<Assets<ReplayAsset>>().get(&sources.asset)
        {
            if asset.0.rate != 8000 || asset.0.frames != 240000 || asset.0.channels.len() != 4 {
                return Err(BevyError::error(
                    "Basic Replay asset specification mismatch",
                ));
            }
            sources.replay = Some(Arc::new(ReplaySource(
                asset.0.clone(),
                vec![
                    "Sine".into(),
                    "Square".into(),
                    "Sawtooth".into(),
                    "Triangle".into(),
                ],
            )));
            sources.replay_start = Some(sources.elapsed);
        }
        for (root, kind) in roots {
            if matches!(kind, DemoKind::Replay) {
                if world
                    .get::<WaveformRuntime>(root)
                    .is_some_and(|runtime| runtime.metadata().is_none())
                    && let Some(source) = &sources.replay
                {
                    Waveform::set_session_in_world(world, root, source.clone())?;
                }
                if let Some(mut playback) = Waveform::playback(world, root)? {
                    if playback.playing {
                        let total = world
                            .get::<WaveformRuntime>(root)
                            .and_then(WaveformRuntime::metadata)
                            .ok_or_else(|| BevyError::error("Replay metadata missing"))?
                            .total_duration()?
                            .ok_or_else(|| BevyError::error("Replay duration missing"))?;
                        let advance =
                            Duration::try_from_secs_f64(delta.as_secs_f64() * playback.speed)
                                .map_err(|error| BevyError::error(error.to_string()))?;
                        playback.position = playback.position.saturating_add(advance).min(total);
                        playback.playing = playback.position < total;
                        Waveform::set_playback_in_world(world, root, playback)?;
                    }
                    let rate = world
                        .get::<WaveformRuntime>(root)
                        .and_then(WaveformRuntime::metadata)
                        .map_or(8000, |metadata| metadata.sample_rate);
                    let mut state = world
                        .get_mut::<WaveformDemoState>(root)
                        .ok_or_else(|| BevyError::error("Replay demo state missing"))?;
                    state.playback_seconds = playback.position.as_secs_f64();
                    state.playback_speed = playback.speed;
                    state.preview = playback.preview;
                    state.producer_frame = sample_boundary(playback.position, rate)?;
                }
                continue;
            }
            let frame = match kind {
                DemoKind::Basic => basic_frame,
                DemoKind::Stress => stress_frame,
                DemoKind::Replay => 0,
            };
            let rate = match kind {
                DemoKind::Basic => 100,
                DemoKind::Stress => 64000,
                DemoKind::Replay => 8000,
            };
            let old = world
                .get::<WaveformRuntime>(root)
                .ok_or_else(|| BevyError::error("demo runtime missing"))?
                .buffered_range()
                .end;
            let capacity = match kind {
                DemoKind::Basic => Some(3100),
                DemoKind::Stress => Some(704000),
                DemoKind::Replay => None,
            };
            let mut state = world
                .get_mut::<WaveformDemoState>(root)
                .ok_or_else(|| BevyError::error("demo state missing"))?;
            state.producer_frame = frame;
            state.loop_count = 0;
            if let Some(capacity) = capacity {
                let evicted = frame.saturating_sub(capacity);
                state.dropped_samples += evicted.saturating_sub(old.max(state.missing_until));
                state.missing_until = evicted;
            }
            world
                .get_mut::<WaveformCursor>(root)
                .ok_or_else(|| BevyError::error("demo cursor missing"))?
                .position = duration_from_frames(frame, rate)?;
        }
        world.write_message(RequestRedraw);
        Ok(())
    });
    if let Err(ref failure) = result {
        error!(%failure, "Waveform demo 输入推进失败");
    }
    result
}

fn sync_color_cursor(
    demos: Query<(&DemoKind, &WaveformCursor), Without<ColorWaveform>>,
    mut examples: Query<&mut WaveformCursor, With<ColorWaveform>>,
) {
    if examples.is_empty() {
        return;
    }
    if let Some((_, source)) = demos
        .iter()
        .find(|(kind, _)| matches!(kind, DemoKind::Basic))
    {
        for mut cursor in &mut examples {
            if cursor.position != source.position {
                cursor.position = source.position;
            }
        }
    }
}

fn status(
    language: Res<Language>,
    mut roots: Query<(
        &DemoKind,
        &WaveformRuntime,
        &WaveformChannelLayout,
        &ComputedNode,
        &WaveformOutputLength,
        &mut WaveformDemoState,
    )>,
    sources: Res<DemoSources>,
    mut labels: Query<(&mut Text, &mut LocalizedText), With<StressStatus>>,
) {
    if sources.last.is_none() {
        return;
    }
    for (kind, runtime, layout, node, width, mut state) in &mut roots {
        state.sample_rate = runtime
            .metadata()
            .map_or(0, |metadata| metadata.sample_rate);
        state.channels = runtime.channel_count();
        state.session_generation = runtime.session_generation();
        state.running = runtime.is_running();
        state.duration_ms = u64::from(runtime.config().visible_duration_ms);
        state.amplitude_volts = runtime.config().amplitude_range.volts();
        state.displayed_frame = runtime.buffered_range().end;
        state.buffered_start = runtime.buffered_range().start;
        state.viewport_start = runtime.viewport_range().start;
        state.output_width = width.0;
        state.height = layout.viewport_height / node.inverse_scale_factor();
        state.row_height = layout.row_height / node.inverse_scale_factor();
        state.scroll_offset = layout.scroll_offset / node.inverse_scale_factor();
        state.envelope = matches!(
            runtime.reduced_channels().first(),
            Some(ReducedChannel::Envelope(_))
        );
        state.read_frames = runtime.stats().read_frames;
        state.algorithm = format!("{:?}", runtime.algorithm());
        let (raw, _, reduced) = runtime.working_set_bytes();
        state.raw_capacity_bytes = raw;
        state.reduced_capacity_bytes = reduced;
        if matches!(kind, DemoKind::Stress) {
            for (mut text, mut binding) in &mut labels {
                let args = vec![
                    ("producer", state.producer_frame.to_string()),
                    ("displayed", state.displayed_frame.to_string()),
                    (
                        "fps",
                        format!(
                            "{:.1}",
                            if sources.frame_ms > 0.0 {
                                1000.0 / sources.frame_ms
                            } else {
                                0.0
                            }
                        ),
                    ),
                    ("ms", format!("{:.2}", sources.frame_ms)),
                    ("dropped", state.dropped_samples.to_string()),
                ];
                *binding =
                    LocalizedText::with_args(TextKey::Waveform(WaveformKey::WaveformStatus), args);
                text.0 = binding.render(*language);
            }
        }
    }
}

impl Plugin for WaveformDemoPlugin {
    fn build(&self, app: &mut App) {
        if let Err(error) = app.register_widgetry_combo_box::<AlgorithmChoice>() {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
        app.add_plugins(WaveformRenderPlugin)
            .add_observer(|event: On<WaveformAlgorithmChanged>| {
                info!(entity = ?event.entity, algorithm = ?event.algorithm, "切换 Waveform 降采样算法");
            })
            .add_observer(log_grid_change)
            .add_observer(playback_requested)
            .add_observer(log_amplitude_change)
            .add_observer(log_channel_count_change)
            .register_type::<WaveformDemoState>()
            .add_systems(OnEnter(GalleryPage::Waveform), enter)
            .add_systems(OnExit(GalleryPage::Waveform), exit)
            .add_systems(
                Update,
                drive
                    .before(WaveformSystems::Update)
                    .run_if(in_state(GalleryPage::Waveform)),
            )
            .add_systems(
                Update,
                sync_color_cursor
                    .after(drive)
                    .before(WaveformSystems::Update)
                    .run_if(in_state(GalleryPage::Waveform)),
            )
            .add_systems(
                Last,
                status
                    .in_set(WaveformDemoSystems::Status)
                    .run_if(in_state(GalleryPage::Waveform)),
            );
    }
}

fn playback_requested(
    event: On<WaveformPlaybackRequested>,
    roots: Query<(), With<DemoKind>>,
    mut commands: Commands,
) {
    if !roots.contains(event.entity) {
        return;
    }
    let event = *event.event();
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<Waveform>(event.entity).is_none() { return Ok(()); }
        let Some(mut playback) = Waveform::playback(world, event.entity)? else { return Ok(()); };
        if playback.session_generation != event.session_generation { return Ok(()); }
        match event.action {
            WaveformPlaybackAction::Playing(playing) => {
                let total = world.get::<WaveformRuntime>(event.entity).and_then(WaveformRuntime::metadata).ok_or_else(|| BevyError::error("Replay metadata missing"))?.total_duration()?.ok_or_else(|| BevyError::error("Replay duration missing"))?;
                if playing && playback.position == total { playback.position = Duration::ZERO; }
                playback.playing = playing;
            }
            WaveformPlaybackAction::Speed(speed) => playback.speed = speed,
            WaveformPlaybackAction::SeekStarted => {
                world.resource_mut::<DemoSources>().gestures.entry(event.entity).or_insert(playback);
                playback.playing = false;
                playback.preview = true;
            }
            WaveformPlaybackAction::Seek(position) => {
                playback.position = position;
                let total = world.get::<WaveformRuntime>(event.entity).and_then(WaveformRuntime::metadata).ok_or_else(|| BevyError::error("Replay metadata missing"))?.total_duration()?.ok_or_else(|| BevyError::error("Replay duration missing"))?;
                if position == total { playback.playing = false; }
            },
            WaveformPlaybackAction::SeekEnded { position, .. } => {
                let Some(saved) = world.resource_mut::<DemoSources>().gestures.remove(&event.entity) else { return Ok(()); };
                if saved.session_generation != playback.session_generation { return Ok(()); }
                playback.position = position;
                playback.preview = false;
                playback.speed = saved.speed;
                let total = world.get::<WaveformRuntime>(event.entity).and_then(WaveformRuntime::metadata).ok_or_else(|| BevyError::error("Replay metadata missing"))?.total_duration()?.ok_or_else(|| BevyError::error("Replay duration missing"))?;
                playback.playing = saved.playing && position < total;
            }
        }
        Waveform::set_playback_in_world(world, event.entity, playback)?;
        if matches!(event.action, WaveformPlaybackAction::Seek(_)) && playback.playing {
            world.get_mut::<WaveformCursor>(event.entity).ok_or_else(|| BevyError::error("Replay cursor missing"))?.seek(playback.position);
        }
        if !matches!(event.action, WaveformPlaybackAction::Seek(_) | WaveformPlaybackAction::SeekStarted) {
            info!(entity = ?event.entity, playing = playback.playing, speed = playback.speed, "更新 Waveform 回放控制");
        }
        Ok(())
    });
}

fn toggle_grid(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    roots: Query<Entity, With<DemoKind>>,
    mut commands: Commands,
) {
    for root in &roots {
        Waveform::set_grid_visible(&mut commands, root, event.value);
    }
}

fn log_grid_change(event: On<WaveformGridChanged>, roots: Query<(), With<DemoKind>>) {
    if roots.contains(event.entity) {
        info!(entity = ?event.entity, visible = event.visible, "切换 Waveform 时间网格");
    }
}

fn log_amplitude_change(event: On<WaveformAmplitudeChanged>, roots: Query<(), With<DemoKind>>) {
    if roots.contains(event.entity) {
        info!(entity = ?event.entity, amplitude_volts = event.amplitude.volts(), "切换 Waveform 幅值上限");
    }
}

fn enter(world: &mut World) -> Result {
    world.init_resource::<DemoSources>();
    let content = scene(world.resource::<DemoSources>());
    if let Err(error) = mount_page(world, GalleryPage::Waveform, bsn_list! { @content }) {
        world.remove_resource::<DemoSources>();
        return Err(error);
    }
    let controls: Vec<_> = world
        .query::<&AlgorithmParts>()
        .iter(world)
        .map(|parts| (parts.model, parts.combo))
        .collect();
    for (model, combo) in controls {
        world.entity_mut(combo).apply_scene(bsn! {
            @WidgetryComboBox::<AlgorithmChoice> { @source: model,
                @renderer: {WidgetryListViewRenderer::new(|choice: &AlgorithmChoice| {
                    let label = match choice.0 { WaveformAlgorithm::MinMax => "Min/Max", WaveformAlgorithm::Lttb => "LTTB" };
                    bsn_list! { Text(label) bevy_widgetry::text::WidgetryText }
                })}
            }
        }).map_err(|error| BevyError::error(error.to_string()))?;
        let index = if std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT").is_some() {
            match std::env::var("GALLERY_WAVEFORM_BENCH_ALGORITHM").as_deref() {
                Ok("Lttb") => 1,
                Ok("MinMax") | Err(std::env::VarError::NotPresent) => 0,
                _ => return Err(BevyError::error("Invalid waveform benchmark algorithm")),
            }
        } else {
            0
        };
        WidgetryComboBox::<AlgorithmChoice>::select_index(&mut world.commands(), combo, index);
    }
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Waveform)?;
    world.remove_resource::<DemoSources>();
    Ok(())
}

fn log_channel_count_change(
    event: On<WaveformChannelCountChanged>,
    roots: Query<(), With<DemoKind>>,
) {
    if roots.contains(event.entity) {
        info!(entity = ?event.entity, count = ?event.count, "切换 Waveform 每屏通道数");
    }
}
