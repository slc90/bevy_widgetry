mod render;

use super::{SplashSession, SplashStatus, presentation::SplashPresentation};
use crate::startup_cursor::GalleryFirstFrameReady;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::atomic::Ordering,
    time::Instant,
};

#[derive(Resource)]
struct SplashMeasurement {
    writer: BufWriter<File>,
    started: Option<Instant>,
    frame: Instant,
    previous: Instant,
    count: u64,
    gpu: BufWriter<File>,
    gpu_time: Option<bevy::platform::time::Instant>,
    epoch: Instant,
    drive: bool,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashBenchmarkStatus {
    updates: u64,
    presents: u64,
    splash_active: bool,
    entities: u32,
    images: usize,
    materials: usize,
}

pub(super) fn install(app: &mut App) -> Result {
    let Some(path) = std::env::var_os("GALLERY_SPLASH_BENCH_OUTPUT") else {
        return Ok(());
    };
    let mut writer = BufWriter::new(File::create(&path).map_err(|error| {
        error!(path = ?path, error = %error, "无法创建 Splash benchmark 文件");
        BevyError::error(error)
    })?);
    writeln!(
        writer,
        "elapsed_ms,update_ms,frame_interval_ms,present_count,present_interval_ms,startup_ms,splash_active,animation_ms,cpu_assets_ready,source_ready,presented,entities,images,materials"
    )
    .map_err(measurement_error)?;
    let gpu_path = std::path::PathBuf::from(&path).with_extension("gpu.csv");
    let mut gpu = BufWriter::new(File::create(&gpu_path).map_err(|error| {
        error!(path = ?gpu_path, error = %error, "无法创建 Splash GPU benchmark 文件");
        BevyError::error(error)
    })?);
    writeln!(gpu, "elapsed_ms,path,gpu_ms").map_err(measurement_error)?;
    let now = Instant::now();
    app.insert_resource(SplashMeasurement {
        writer,
        started: None,
        frame: now,
        previous: now,
        count: 0,
        gpu,
        gpu_time: None,
        epoch: now,
        drive: std::env::var_os("GALLERY_SPLASH_BENCH_IDLE").is_none(),
    })
    .init_resource::<SplashBenchmarkStatus>()
    .register_type::<SplashBenchmarkStatus>()
    .add_systems(First, begin)
    .add_systems(Last, finish);
    render::install(app)
}

fn begin(mut measurement: ResMut<SplashMeasurement>, mut status: ResMut<SplashBenchmarkStatus>) {
    measurement.frame = Instant::now();
    status.updates += 1;
}

fn finish(
    mut measurement: ResMut<SplashMeasurement>,
    presentation: Option<Res<SplashPresentation>>,
    ready: Option<Res<GalleryFirstFrameReady>>,
    session: Option<Res<SplashSession>>,
    status: Option<Res<SplashStatus>>,
    channel: Res<render::BenchmarkPresent>,
    mut benchmark: ResMut<SplashBenchmarkStatus>,
    entities: &bevy::ecs::entity::Entities,
    images: Res<Assets<Image>>,
    materials: Option<Res<Assets<super::SplashRevealMaterial>>>,
    diagnostics: Res<DiagnosticsStore>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut exit: ResMut<Messages<AppExit>>,
) -> Result {
    if measurement.drive {
        redraw.write(RequestRedraw);
    }
    let now = Instant::now();
    let count = channel.count.load(Ordering::Acquire);
    let presented = presentation
        .as_ref()
        .map_or(ready.is_some(), |p| p.presented.load(Ordering::Acquire));
    let first = presented && measurement.started.is_none();
    if first {
        measurement.started = Some(now);
        measurement.previous = now;
        measurement.count = count;
    }
    let elapsed = measurement.started.map_or(-1.0, |start| {
        now.duration_since(start).as_secs_f64() * 1000.0
    });
    let startup = now.duration_since(measurement.epoch).as_secs_f64() * 1000.0;
    let update = now.duration_since(measurement.frame).as_secs_f64() * 1000.0;
    let interval = if first {
        0.0
    } else {
        now.duration_since(measurement.previous).as_secs_f64() * 1000.0
    };
    measurement.previous = now;
    let present = if count != measurement.count {
        channel.interval_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0
    } else {
        0.0
    };
    measurement.count = count;
    benchmark.presents = count;
    benchmark.splash_active = session.is_some();
    benchmark.entities = entities.count_spawned();
    benchmark.images = images.len();
    benchmark.materials = materials.as_ref().map_or(0, |assets| assets.len());
    let gpu_time = diagnostics
        .iter()
        .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
        .filter_map(|d| d.measurement().map(|m| m.time))
        .max();
    if gpu_time != measurement.gpu_time {
        measurement.gpu_time = gpu_time;
        for diagnostic in diagnostics
            .iter()
            .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
        {
            if let Some(sample) = diagnostic
                .measurement()
                .filter(|m| Some(m.time) == gpu_time)
            {
                writeln!(
                    measurement.gpu,
                    "{elapsed:.4},{},{:.5}",
                    diagnostic.path(),
                    sample.value
                )
                .map_err(measurement_error)?;
            }
        }
    }
    writeln!(
        measurement.writer,
        "{elapsed:.4},{update:.4},{interval:.4},{count},{present:.4},{startup:.4},{},{:.4},{},{},{},{},{},{}",
        benchmark.splash_active,
        status.as_ref().map_or(0.0, |status| status.elapsed_ms),
        status.as_ref().is_some_and(|status| status.cpu_assets_ready),
        status.as_ref().is_some_and(|status| status.source_ready),
        presented,
        benchmark.entities,
        benchmark.images,
        benchmark.materials,
    )
    .map_err(measurement_error)?;
    if measurement.drive && elapsed >= 22000.0 {
        exit.write(AppExit::Success);
    }
    if !exit.is_empty() {
        measurement.writer.flush().map_err(measurement_error)?;
        measurement.gpu.flush().map_err(measurement_error)?;
    }
    Ok(())
}

fn measurement_error(error: std::io::Error) -> BevyError {
    error!(error = %error, "Splash benchmark 写入失败");
    BevyError::error(error)
}
