mod navigation_readiness;

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
    pending: Option<(Instant, u64, UVec2, UVec2, String)>,
    captures: Vec<(u64, f64, bool, String)>,
    capture_until: u64,
    active_entities: u32,
    burst_started: bool,
    finished: bool,
    replay: bool,
    captured_seek: Option<Instant>,
}

pub(crate) fn install(app: &mut App) -> Result {
    let Some(output) = std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") else {
        return Ok(());
    };
    let output = PathBuf::from(output);
    fs::create_dir_all(&output)?;
    navigation_readiness::install(app, output.join("navigation"))?;
    let mut frames = BufWriter::new(File::create_new(output.join("frames.csv"))?);
    writeln!(
        frames,
        "frame,elapsed_s,interval_ms,main_update_ms,producer_ms,producer_frame,displayed_frame,backlog,dropped_samples,read_frames,width,height,raw_bytes,reduced_bytes,mesh_bytes,entity_indices,images,meshes,capture,burst,asset_extract_ms,render_schedule_ms,active_entities,algorithm,preview,file_queries,file_discarded,file_queued,file_query_ms,file_io_ms,file_query_cpu_ms,file_bytes,file_bucket,file_request,file_target_generation,file_output_len,file_range_end,file_algorithm"
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
        replay: std::env::var("GALLERY_WAVEFORM_BENCH_SCENARIO").as_deref() == Ok("Replay"),
        captured_seek: None,
    })
    .add_systems(First, begin)
    .add_systems(
        Last,
        measure.after(crate::pages::WaveformDemoSystems::Status),
    );
    Ok(())
}

fn begin(mut state: ResMut<Measurement>, mut redraw: MessageWriter<RequestRedraw>) {
    state.update_start = Instant::now();
    if state.replay && !state.finished {
        redraw.write(RequestRedraw);
    }
}

fn measure(world: &mut World) -> Result {
    let replay = world.resource::<Measurement>().replay;
    let stress = world
        .query::<(Entity, &Name, &WaveformDemoState)>()
        .iter(world)
        .find(|(_, name, _)| {
            name.as_str()
                == if replay {
                    "DiskReplayWaveform"
                } else {
                    "StressWaveform"
                }
        })
        .map(|(e, _, _)| e);
    let Some(root) = stress else {
        return Ok(());
    };
    if world
        .get::<WaveformDemoState>(root)
        .is_none_or(|state| state.output_width == 0)
        || replay
            && world
                .get::<WaveformDemoState>(root)
                .is_none_or(|state| state.sample_rate == 0)
        || world
            .get::<ComputedNode>(root)
            .is_none_or(|node| node.size().x <= 0.0)
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
            writeln!(captures, "source_frame,readback_upper_ms,pixels_nonempty,algorithm")?;
            for (source_frame, latency, nonempty, algorithm) in &state.captures { writeln!(captures, "{source_frame},{latency},{nonempty},{algorithm}")?; }
            captures.flush()?;
            fs::write(state.output.join("done.txt"), format!("frames={frame}\nelapsed_seconds={elapsed}\n"))?;
            state.finished = true;
            return Ok(());
        }
        if !state.replay && elapsed >= 45.0 && !state.burst_started {
            world.resource_mut::<WaveformDemoSources>().hold_until = Some(now + Duration::from_millis(100));
            state.burst_started = true;
        }
        let last_seek = world.resource::<WaveformDemoSources>().last_seek.filter(|(_,entity,_)| *entity == root).map(|(time,_,frame)|(time,frame));
        let displayed = world.get::<WaveformDemoState>(root).ok_or_else(|| BevyError::error("benchmark root missing"))?.displayed_frame;
        let file = if state.replay { world.resource::<WaveformDemoSources>().disk_stats().map_err(|error| BevyError::error(error.to_string()))? } else { None };
        let snapshot_ready = if state.replay {
            let runtime = world.get::<bevy_widgetry::waveform::WaveformRuntime>(root).ok_or_else(||BevyError::error("Replay runtime missing"))?;
            let cursor = world.get::<bevy_widgetry::waveform::WaveformCursor>(root).ok_or_else(||BevyError::error("Replay cursor missing"))?;
            let sample = world.get::<WaveformDemoState>(root).ok_or_else(||BevyError::error("Replay sample missing"))?;
            runtime.stats().kind != bevy_widgetry::waveform::WaveformUpdateKind::Pending && runtime.last_error().is_none()
                && file.as_ref().and_then(|stats|stats.last_read.as_ref()).is_some_and(|read|
                    read.request.range.end == displayed && read.request.target_generation == cursor.seek_generation
                    && read.request.session_generation == sample.session_generation && read.request.algorithm == runtime.algorithm()
                    && read.request.output_len == if sample.preview { sample.output_width.div_ceil(4) } else {sample.output_width})
        } else { true };
        let request_capture = if state.replay {
            last_seek.is_some_and(|(time, target)| snapshot_ready && target == displayed && state.captured_seek != Some(time)) && state.pending.is_none()
        } else { elapsed > 12.0 && frame.is_multiple_of(60) && state.pending.is_none() };
        let capture = state.pending.is_some() || frame <= state.capture_until || request_capture;
        let sample = world.get::<WaveformDemoState>(root).ok_or_else(|| BevyError::error("benchmark stress state missing"))?;
        let sources = world.resource::<WaveformDemoSources>();
        let producer_ms = sources.producer_ms;
        let burst = sources.burst;
        let generated = if state.replay { last_seek.map_or(sources.generated_at, |(time,_)|time) } else { sources.generated_at };
        let generated_frame = sample.displayed_frame;
        let capture_algorithm = sample.algorithm.clone();
        let mesh_bytes: usize = world.resource::<Assets<Mesh>>().iter().map(|(_, mesh)| mesh.attributes().map(|(_, values)| values.get_bytes().len()).sum::<usize>() + mesh.get_index_buffer_bytes().map_or(0, <[u8]>::len)).sum();
        if frame == 1 || frame.is_multiple_of(60) { state.active_entities = world.entities().count_spawned(); }
        let active_entities = state.active_entities;
        let timing = world.resource::<RenderTiming>();
        let extract_ms = timing.0.0.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let render_ms = timing.0.1.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let (queries, discarded, queued, query_ms, io_ms, cpu_ms, bytes, bucket) = file.as_ref().map_or((0,0,0,0.0,0.0,0.0,0,0), |stats| {
            let (query, io, bytes, bucket) = stats.last_read.as_ref().map_or((Duration::ZERO,Duration::ZERO,0,0), |read| (read.elapsed, read.read.io_time, read.read.bytes_read, read.read.bucket_size));
            (stats.queries,stats.discarded_queries,stats.queued_readers,query.as_secs_f64()*1000.0,io.as_secs_f64()*1000.0,query.saturating_sub(io).as_secs_f64()*1000.0,bytes,bucket)
        });
        let identity = file.as_ref().and_then(|stats|stats.last_read.as_ref()).map_or_else(|| "0,0,0,0,".to_owned(), |read|format!("{},{},{},{},{:?}",read.request.id,read.request.target_generation,read.request.output_len,read.request.range.end,read.request.algorithm));
        let line = format!("{frame},{elapsed},{interval},{update_ms},{producer_ms},{},{},{},{},{},{},{},{},{},{mesh_bytes},{},{},{},{capture},{burst},{extract_ms},{render_ms},{active_entities},{},{},{queries},{discarded},{queued},{query_ms},{io_ms},{cpu_ms},{bytes},{bucket},{identity}\n", sample.producer_frame, sample.displayed_frame, sample.producer_frame.saturating_sub(sample.displayed_frame), sample.dropped_samples, sample.read_frames, sample.output_width, sample.height, sample.raw_capacity_bytes, sample.reduced_capacity_bytes, world.entities().len(), world.resource::<Assets<Image>>().len(), world.resource::<Assets<Mesh>>().len(), sample.algorithm, sample.preview);
        state.frames.write_all(line.as_bytes())?;
        if let Some(store) = world.get_resource::<DiagnosticsStore>() {
            for diagnostic in store.iter() {
                if let Some(value) = diagnostic.value() { writeln!(state.diagnostics, "{frame},{},{}", diagnostic.path(), value)?; }
            }
        }
        // This is a conservative upper bound from generated data to pixels available through GPU readback.
        // Stop timing at the observer to exclude PNG encoding and the BRP round trip.
        // This upper bound does not measure OS presentation latency.
        // It also includes the GPU readback overhead of this capture.
        if request_capture {
            if state.replay { state.captured_seek = last_seek.map(|(time,_)|time); }
            state.capture_until = frame + 4;
            let capture_root = if state.replay {
                world.query::<(Entity, &Name)>().iter(world).find(|(entity,name)| {
                    if name.as_str() != "WaveformPlot" { return false; }
                    let mut current = Some(*entity);
                    while let Some(entity) = current { if entity == root { return true; } current = world.get::<ChildOf>(entity).map(ChildOf::parent); }
                    false
                }).map(|(entity,_)|entity).ok_or_else(|| BevyError::error("Replay benchmark plot missing"))?
            } else { root };
            let node = world.get::<ComputedNode>(capture_root).ok_or_else(|| BevyError::error("benchmark layout missing"))?;
            let transform = world.get::<UiGlobalTransform>(capture_root).ok_or_else(|| BevyError::error("benchmark transform missing"))?;
            let center = transform.to_scale_angle_translation().2;
            let min = (center - node.size() * 0.5).max(Vec2::ZERO).as_uvec2();
            let max = (center + node.size() * 0.5).max(Vec2::ZERO).as_uvec2();
            state.pending = Some((generated, generated_frame, min, max, capture_algorithm));
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
    if let Some((generated, source_frame, min, max, algorithm)) = state.pending.take() {
        state.capture_until = state.frame + 4;
        let upper_ms = generated.elapsed().as_secs_f64() * 1000.0;
        let nonempty = event.image.data.as_ref().is_some_and(|data| {
            let size = event.image.texture_descriptor.size;
            let pixels = data.as_chunks::<4>().0;
            let max = max.min(UVec2::new(size.width, size.height));
            if state.replay {
                return (min.y..max.y).any(|y| {
                    (min.x..max.x).any(|x| {
                        pixels
                            .get((y * size.width + x) as usize)
                            .is_some_and(|pixel| {
                                let brightest = pixel[0].max(pixel[1]).max(pixel[2]);
                                let darkest = pixel[0].min(pixel[1]).min(pixel[2]);
                                brightest > 60 && brightest.saturating_sub(darkest) > 20
                            })
                    })
                });
            }
            // Non-background pixels across the whole screenshot could come only from sidebar Text.
            // Check every lane in the Stress region so a missing Waveform cannot count as success.
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
        state
            .captures
            .push((source_frame, upper_ms, nonempty, algorithm));
    }
}
