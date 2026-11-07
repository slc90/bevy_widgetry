use crate::gallery::GalleryPage;
use bevy::prelude::*;
use bevy_widgetry::progress_bar::{WidgetryProgressBarHeadless, WidgetryProgressState};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};

#[derive(Resource)]
struct Measurement {
    writer: BufWriter<File>,
    epoch: Instant,
    begin: Instant,
    previous: Instant,
    flush: Instant,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(First, begin.run_if(resource_exists::<Measurement>))
        .add_systems(Last, record.run_if(resource_exists::<Measurement>));
}

fn setup(mut commands: Commands) -> Result {
    let Some(path) = std::env::var_os("GALLERY_PROGRESS_BENCH_OUTPUT") else {
        return Ok(());
    };
    let mut writer = BufWriter::new(File::create_new(path).map_err(failure)?);
    writeln!(
        writer,
        "elapsed_ms,main_update_ms,interval_ms,page,indeterminate"
    )
    .map_err(failure)?;
    let now = Instant::now();
    commands.insert_resource(Measurement {
        writer,
        epoch: now,
        begin: now,
        previous: now,
        flush: now,
    });
    Ok(())
}
fn begin(mut measurement: ResMut<Measurement>) {
    measurement.begin = Instant::now();
}
fn record(world: &mut World) -> Result {
    let page = *world.resource::<State<GalleryPage>>().get();
    let roots = world
        .query_filtered::<Entity, With<WidgetryProgressBarHeadless>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut indeterminate = 0;
    for root in roots {
        if WidgetryProgressBarHeadless::state(world, root)
            .map_err(|error| error.into_bevy_error())?
            == WidgetryProgressState::Indeterminate
        {
            indeterminate += 1;
        }
    }
    let now = Instant::now();
    let exiting = !world.resource::<Messages<AppExit>>().is_empty();
    let mut measurement = world.resource_mut::<Measurement>();
    let elapsed = now.duration_since(measurement.epoch).as_secs_f64() * 1000.0;
    let update = now.duration_since(measurement.begin).as_secs_f64() * 1000.0;
    let interval = now.duration_since(measurement.previous).as_secs_f64() * 1000.0;
    measurement.previous = now;
    writeln!(
        measurement.writer,
        "{elapsed:.4},{update:.4},{interval:.4},{page:?},{indeterminate}"
    )
    .map_err(failure)?;
    if now.duration_since(measurement.flush).as_millis() >= 500 || exiting {
        measurement.writer.flush().map_err(failure)?;
        measurement.flush = now;
    }
    Ok(())
}
fn failure(error: std::io::Error) -> BevyError {
    crate::diagnostics::at(
        error,
        bevy_widgetry::diagnostics::ErrorFrame::new("ProgressBar benchmark 文件操作失败"),
    )
}
