use crate::assets::GalleryWaveform;
use crate::waveform_data::{LiveSource, ReplayAsset, ReplaySource};
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use bevy_widgetry::waveform::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct WaveformDemoPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WaveformDemoSystems {
    Status,
}

#[derive(Resource)]
pub(crate) struct DemoSources {
    basic: Arc<LiveSource>,
    stress: Arc<LiveSource>,
    replay: Arc<ReplaySource>,
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

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct WaveformDemoState {
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_ms: u64,
    pub producer_frame: u64,
    pub displayed_frame: u64,
    pub buffered_start: u64,
    pub viewport_start: u64,
    pub output_width: usize,
    pub height: f32,
    pub envelope: bool,
    pub loop_count: u64,
    pub dropped_samples: u64,
    missing_until: u64,
    pub read_frames: usize,
    pub raw_capacity_bytes: usize,
    pub reduced_capacity_bytes: usize,
}

#[derive(Component)]
struct StressStatus;

impl FromWorld for DemoSources {
    fn from_world(world: &mut World) -> Self {
        Self {
            basic: Arc::new(LiveSource::new(4, 600)),
            stress: Arc::new(LiveSource::new(64, 704000)),
            replay: Arc::new(ReplaySource::default()),
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
    let basic = waveform(DemoKind::Basic, sources.basic.clone());
    let replay = waveform(DemoKind::Replay, sources.replay.clone());
    let stress = waveform(DemoKind::Stress, sources.stress.clone());
    bsn! {
        #WaveformDemo
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12) }
        Children [
            (Text("Waveform") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(24.0) }),
            (#BasicWaveforms Node { width: percent(100), height: px(230), flex_shrink: 0.0, column_gap: px(16) } Children [
                (Node { width: px(790), height: px(230), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: px(6) } Children [
                    Text("Basic Live · 100 Hz · 4 channels · 5 s · Polyline") bevy_widgetry::text::WidgetryText,
                    (#BasicLiveWaveform {basic}),
                ]),
                (Node { width: px(790), height: px(230), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: px(6) } Children [
                    Text("Basic Replay · 8 kHz · 4 channels · 5 s · fixed .wfrm · Envelope") bevy_widgetry::text::WidgetryText,
                    (#BasicReplayWaveform {replay}),
                ]),
            ]),
            (Text("Stress Live · 64 kHz · 64 channels · 10 s") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(20.0) }),
            (#StressWaveform {stress}),
            (#WaveformStatus Text("Waiting for Waveform page") bevy_widgetry::text::WidgetryText template(|_| Ok(StressStatus))),
        ]
    }
}

fn waveform(kind: DemoKind, adapter: Arc<dyn WaveformSource>) -> impl Scene {
    let (rate, channels, duration, width, height) = match kind {
        DemoKind::Basic => (100, 4, 5000, 790.0, 200.0),
        DemoKind::Replay => (8000, 4, 5000, 790.0, 200.0),
        DemoKind::Stress => (64000, 64, 10000, 1600.0, 640.0),
    };
    bsn! {
        @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { sample_rate: rate, visible_duration_ms: duration, channel_ranges: vec![-1.0..=1.0; channels] }} }
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
        if sources.replay_start.is_none()
            && let Some(asset) = world.resource::<Assets<ReplayAsset>>().get(&sources.asset)
        {
            if asset.0.rate != 8000 || asset.0.frames != 240000 || asset.0.channels.len() != 4 {
                return Err(BevyError::error(
                    "Basic Replay asset specification mismatch",
                ));
            }
            *sources
                .replay
                .0
                .write()
                .map_err(|_| BevyError::error("replay source lock poisoned"))? =
                Some(asset.0.clone());
            sources.replay_start = Some(sources.elapsed);
        }
        let replay_elapsed = sources
            .replay_start
            .map_or(Duration::ZERO, |start| sources.elapsed - start);
        let replay_total = sample_boundary(replay_elapsed, 8000)?;
        for (root, kind) in roots {
            let frame = match kind {
                DemoKind::Basic => basic_frame,
                DemoKind::Stress => stress_frame,
                // 文件已就绪后至少提交第一个 sample，避免 loop 恰好落在零边界时闪出整屏空白。
                DemoKind::Replay => {
                    if sources.replay_start.is_some() {
                        (replay_total % 240000).max(1)
                    } else {
                        0
                    }
                }
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
                DemoKind::Basic => Some(600),
                DemoKind::Stress => Some(704000),
                DemoKind::Replay => None,
            };
            let mut state = world
                .get_mut::<WaveformDemoState>(root)
                .ok_or_else(|| BevyError::error("demo state missing"))?;
            state.producer_frame = frame;
            state.loop_count = if matches!(kind, DemoKind::Replay) {
                replay_total / 240000
            } else {
                0
            };
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

fn status(
    mut roots: Query<(
        &DemoKind,
        &WaveformRuntime,
        &ComputedNode,
        &WaveformOutputLength,
        &mut WaveformDemoState,
    )>,
    sources: Res<DemoSources>,
    mut labels: Query<&mut Text, With<StressStatus>>,
) {
    if sources.last.is_none() {
        return;
    }
    for (kind, runtime, node, width, mut state) in &mut roots {
        state.sample_rate = runtime.config().sample_rate;
        state.channels = runtime.config().channel_ranges.len();
        state.duration_ms = u64::from(runtime.config().visible_duration_ms);
        state.displayed_frame = runtime.buffered_range().end;
        state.buffered_start = runtime.buffered_range().start;
        state.viewport_start = runtime.viewport_range().start;
        state.output_width = width.0;
        state.height = node.size().y;
        state.envelope = matches!(
            runtime.reduced_channels().first(),
            Some(ReducedChannel::Envelope(_))
        );
        state.read_frames = runtime.stats().read_frames;
        let (raw, _, reduced) = runtime.working_set_bytes();
        state.raw_capacity_bytes = raw;
        state.reduced_capacity_bytes = reduced;
        if matches!(kind, DemoKind::Stress) {
            for mut text in &mut labels {
                text.0 = format!(
                    "64 kHz · 64 ch · 10 s · producer {} / displayed {} · {:.1} FPS / {:.2} ms · dropped {}",
                    state.producer_frame,
                    state.displayed_frame,
                    if sources.frame_ms > 0.0 {
                        1000.0 / sources.frame_ms
                    } else {
                        0.0
                    },
                    sources.frame_ms,
                    state.dropped_samples
                );
            }
        }
    }
}

impl Plugin for WaveformDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WaveformRenderPlugin)
            .init_resource::<DemoSources>()
            .register_type::<WaveformDemoState>()
            .add_systems(Update, drive.before(WaveformSystems::Update))
            .add_systems(Last, status.in_set(WaveformDemoSystems::Status));
    }
}
