use super::presentation::SplashPresentation;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
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
        "elapsed_ms,update_ms,frame_interval_ms,present_count,present_interval_ms"
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
    })
    .add_plugins(RenderDiagnosticsPlugin)
    .add_systems(First, begin)
    .add_systems(Last, finish);
    Ok(())
}

fn begin(mut measurement: ResMut<SplashMeasurement>) {
    measurement.frame = Instant::now();
}

fn finish(
    mut measurement: ResMut<SplashMeasurement>,
    presentation: Option<Res<SplashPresentation>>,
    diagnostics: Res<DiagnosticsStore>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut exit: MessageWriter<AppExit>,
) -> Result {
    redraw.write(RequestRedraw);
    let Some(presentation) = presentation.filter(|p| p.presented.load(Ordering::Acquire)) else {
        return Ok(());
    };
    let now = Instant::now();
    let Some(started) = measurement.started else {
        measurement.started = Some(now);
        measurement.previous = now;
        measurement.count = presentation.present_count.load(Ordering::Acquire);
        return Ok(());
    };
    let elapsed = now.duration_since(started).as_secs_f64() * 1000.0;
    let update = now.duration_since(measurement.frame).as_secs_f64() * 1000.0;
    let interval = now.duration_since(measurement.previous).as_secs_f64() * 1000.0;
    measurement.previous = now;
    let count = presentation.present_count.load(Ordering::Acquire);
    let present = if count != measurement.count {
        presentation.present_interval_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0
    } else {
        0.0
    };
    measurement.count = count;
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
        "{elapsed:.4},{update:.4},{interval:.4},{count},{present:.4}"
    )
    .map_err(measurement_error)?;
    if elapsed >= 22000.0 {
        measurement.writer.flush().map_err(measurement_error)?;
        measurement.gpu.flush().map_err(measurement_error)?;
        exit.write(AppExit::Success);
    }
    Ok(())
}

fn measurement_error(error: std::io::Error) -> BevyError {
    error!(error = %error, "Splash benchmark 写入失败");
    BevyError::error(error)
}
