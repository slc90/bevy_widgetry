use crate::pages::{WaveformDemoSources, WaveformDemoState};
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::diagnostic::{
    MeshAllocatorDiagnosticPlugin, RenderAssetDiagnosticPlugin, RenderDiagnosticsPlugin,
};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::render::{
    ExtractSchedule, Render, RenderApp, RenderSystems, render_asset::AssetExtractionSystems,
};
use bevy::window::RequestRedraw;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Resource, Clone, Default)]
struct RenderTiming(Arc<(AtomicU64, AtomicU64)>);

#[derive(Resource)]
struct TimingStart {
    assets: Instant,
    render: Instant,
}

fn begin_assets(mut time: ResMut<TimingStart>) {
    time.assets = Instant::now();
}
fn end_assets(time: Res<TimingStart>, shared: Res<RenderTiming>) {
    shared.0.0.store(
        time.assets.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
        Ordering::Relaxed,
    );
}
fn begin_render(mut time: ResMut<TimingStart>) {
    time.render = Instant::now();
}
fn end_render(time: Res<TimingStart>, shared: Res<RenderTiming>) {
    shared.0.1.store(
        time.render.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
        Ordering::Relaxed,
    );
}

#[derive(Resource)]
struct Measurement {
    output: PathBuf,
    frames: BufWriter<File>,
    diagnostics: BufWriter<File>,
    start: Option<Instant>,
    update_start: Instant,
    previous: Option<Instant>,
    frame: u64,
    pending: Option<(Instant, u64, UVec2, UVec2)>,
    captures: Vec<(u64, f64, bool)>,
    capture_until: u64,
    active_entities: u32,
    burst_started: bool,
    finished: bool,
}

pub(crate) fn install(app: &mut App) -> Result {
    let Some(output) = std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") else {
        return Ok(());
    };
    let output = PathBuf::from(output);
    fs::create_dir_all(&output)?;
    let mut frames = BufWriter::new(File::create_new(output.join("frames.csv"))?);
    writeln!(
        frames,
        "frame,elapsed_s,interval_ms,main_update_ms,producer_ms,producer_frame,displayed_frame,backlog,dropped_samples,read_frames,width,height,raw_bytes,reduced_bytes,mesh_bytes,entity_indices,images,meshes,capture,burst,asset_extract_ms,render_schedule_ms,active_entities"
    )?;
    let mut diagnostics = BufWriter::new(File::create_new(output.join("render.csv"))?);
    writeln!(diagnostics, "frame,path,value")?;
    let timing = RenderTiming::default();
    app.insert_resource(timing.clone());
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .insert_resource(timing)
            .insert_resource(TimingStart {
                assets: Instant::now(),
                render: Instant::now(),
            })
            .add_systems(ExtractSchedule, begin_assets.before(AssetExtractionSystems))
            .add_systems(ExtractSchedule, end_assets.after(AssetExtractionSystems))
            .add_systems(Render, begin_render.before(RenderSystems::ExtractCommands))
            .add_systems(Render, end_render.after(RenderSystems::PostCleanup));
    }
    app.add_plugins((
        RenderDiagnosticsPlugin,
        MeshAllocatorDiagnosticPlugin,
        RenderAssetDiagnosticPlugin::<bevy::render::texture::GpuImage>::new(" images"),
    ))
    .insert_resource(Measurement {
        output,
        frames,
        diagnostics,
        start: None,
        update_start: Instant::now(),
        previous: None,
        frame: 0,
        pending: None,
        captures: Vec::new(),
        capture_until: 0,
        active_entities: 0,
        burst_started: false,
        finished: false,
    })
    .add_systems(First, begin)
    .add_systems(
        Last,
        measure.after(crate::pages::WaveformDemoSystems::Status),
    );
    Ok(())
}

fn begin(mut state: ResMut<Measurement>) {
    state.update_start = Instant::now();
}

fn measure(world: &mut World) -> Result {
    let stress = world
        .query::<(Entity, &Name, &WaveformDemoState)>()
        .iter(world)
        .find(|(_, name, _)| name.as_str() == "StressWaveform")
        .map(|(e, _, _)| e);
    let Some(root) = stress else {
        return Ok(());
    };
    if world
        .get::<WaveformDemoState>(root)
        .is_none_or(|state| state.output_width != 1600)
        || world
            .get::<ComputedNode>(root)
            .is_none_or(|node| node.size().x < 1600.0)
    {
        return Ok(());
    }
    let result = world.resource_scope(|world, mut state: Mut<Measurement>| -> Result {
        if state.finished { return Ok(()); }
        let now = Instant::now();
        let start = *state.start.get_or_insert(now);
        let elapsed = now.duration_since(start).as_secs_f64();
        let interval = state.previous.map_or(0.0, |previous| now.duration_since(previous).as_secs_f64() * 1000.0);
        state.previous = Some(now);
        state.frame += 1;
        let frame = state.frame;
        let update_ms = state.update_start.elapsed().as_secs_f64() * 1000.0;
        if elapsed >= 90.0 {
            state.frames.flush()?;
            state.diagnostics.flush()?;
            let mut captures = BufWriter::new(File::create_new(state.output.join("observable.csv"))?);
            writeln!(captures, "source_frame,readback_upper_ms,pixels_nonempty")?;
            for (source_frame, latency, nonempty) in &state.captures { writeln!(captures, "{source_frame},{latency},{nonempty}")?; }
            captures.flush()?;
            fs::write(state.output.join("done.txt"), format!("frames={frame}\nelapsed_seconds={elapsed}\n"))?;
            state.finished = true;
            return Ok(());
        }
        if elapsed >= 45.0 && !state.burst_started {
            world.resource_mut::<WaveformDemoSources>().hold_until = Some(now + Duration::from_millis(100));
            state.burst_started = true;
        }
        let request_capture = elapsed > 12.0 && frame.is_multiple_of(60) && state.pending.is_none();
        let capture = state.pending.is_some() || frame <= state.capture_until || request_capture;
        let sample = world.get::<WaveformDemoState>(root).ok_or_else(|| BevyError::error("benchmark stress state missing"))?;
        let sources = world.resource::<WaveformDemoSources>();
        let producer_ms = sources.producer_ms;
        let burst = sources.burst;
        let generated = sources.generated_at;
        let generated_frame = sample.displayed_frame;
        let mesh_bytes: usize = world.resource::<Assets<Mesh>>().iter().map(|(_, mesh)| mesh.attributes().map(|(_, values)| values.get_bytes().len()).sum::<usize>() + mesh.get_index_buffer_bytes().map_or(0, <[u8]>::len)).sum();
        if frame == 1 || frame.is_multiple_of(60) { state.active_entities = world.entities().count_spawned(); }
        let active_entities = state.active_entities;
        let timing = world.resource::<RenderTiming>();
        let extract_ms = timing.0.0.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let render_ms = timing.0.1.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let line = format!("{frame},{elapsed},{interval},{update_ms},{producer_ms},{},{},{},{},{},{},{},{},{},{mesh_bytes},{},{},{},{capture},{burst},{extract_ms},{render_ms},{active_entities}\n", sample.producer_frame, sample.displayed_frame, sample.producer_frame.saturating_sub(sample.displayed_frame), sample.dropped_samples, sample.read_frames, sample.output_width, sample.height, sample.raw_capacity_bytes, sample.reduced_capacity_bytes, world.entities().len(), world.resource::<Assets<Image>>().len(), world.resource::<Assets<Mesh>>().len());
        state.frames.write_all(line.as_bytes())?;
        if let Some(store) = world.get_resource::<DiagnosticsStore>() {
            for diagnostic in store.iter() {
                if let Some(value) = diagnostic.value() { writeln!(state.diagnostics, "{frame},{},{}", diagnostic.path(), value)?; }
            }
        }
        // 已生成数据→GPU readback 可读像素的保守上界。
        // 计时到 observer，排除 PNG 编码与 BRP 往返。
        // 不能把该上界当作 OS present latency。
        // 单次 capture 的 GPU readback 开销也包含在内。
        if request_capture {
            state.capture_until = frame + 4;
            let node = world.get::<ComputedNode>(root).ok_or_else(|| BevyError::error("benchmark stress layout missing"))?;
            let transform = world.get::<UiGlobalTransform>(root).ok_or_else(|| BevyError::error("benchmark stress transform missing"))?;
            let center = transform.to_scale_angle_translation().2;
            let min = (center - node.size() * 0.5).max(Vec2::ZERO).as_uvec2();
            let max = (center + node.size() * 0.5).max(Vec2::ZERO).as_uvec2();
            state.pending = Some((generated, generated_frame, min, max));
            world.spawn(Screenshot::primary_window()).observe(on_capture);
            world.write_message(RequestRedraw);
        }
        Ok(())
    });
    if let Err(ref failure) = result {
        error!(%failure, "Waveform GUI benchmark 记录失败");
    }
    result
}

fn on_capture(event: On<ScreenshotCaptured>, mut state: ResMut<Measurement>) {
    if let Some((generated, source_frame, min, max)) = state.pending.take() {
        state.capture_until = state.frame + 4;
        let upper_ms = generated.elapsed().as_secs_f64() * 1000.0;
        let nonempty = event.image.data.as_ref().is_some_and(|data| {
            let size = event.image.texture_descriptor.size;
            let pixels = data.as_chunks::<4>().0;
            let max = max.min(UVec2::new(size.width, size.height));
            // 整张截图的非背景像素可能只来自 sidebar Text。
            // 只检查 Stress 区的全部 lane，避免把缺失 Waveform 的画面判为成功。
            (0..64).all(|lane| {
                let begin = min.y + (max.y - min.y) * lane / 64;
                let end = min.y + (max.y - min.y) * (lane + 1) / 64;
                (begin..end).any(|y| {
                    (min.x..max.x).step_by(4).any(|x| {
                        pixels
                            .get((y * size.width + x) as usize)
                            .is_some_and(|pixel| pixel[0] > 50 && pixel[1] > 50 && pixel[2] > 50)
                    })
                })
            })
        });
        state.captures.push((source_frame, upper_ms, nonempty));
    }
}
