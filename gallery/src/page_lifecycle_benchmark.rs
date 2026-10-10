use crate::gallery::GalleryPage;
use bevy::prelude::*;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Resource)]
struct Measurement {
    output: PathBuf,
    samples: BufWriter<File>,
    start: Instant,
    previous: GalleryPage,
}

pub(crate) fn install(app: &mut App) -> Result {
    let Some(output) = std::env::var_os("GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT") else {
        return Ok(());
    };
    let output = PathBuf::from(output);
    let samples = (|| -> std::io::Result<_> {
        fs::create_dir_all(&output)?;
        let mut samples = BufWriter::new(File::create_new(output.join("switches.csv"))?);
        writeln!(samples, "from,to,main_update_ms,active_entities")?;
        samples.flush()?;
        Ok(samples)
    })()
    .map_err(|error| {
        error!(path = %output.display(), %error, "无法准备页面 lifecycle 测量输出");
        BevyError::error(error)
    })?;
    app.insert_resource(Measurement {
        output,
        samples,
        start: Instant::now(),
        previous: GalleryPage::Initializing,
    })
    .add_systems(First, begin)
    .add_systems(
        Last,
        measure.after(crate::pages::WaveformDemoSystems::Status),
    );
    Ok(())
}

fn begin(mut measurement: ResMut<Measurement>) {
    measurement.start = Instant::now();
}

fn measure(world: &mut World) -> Result {
    let current = *world.resource::<State<GalleryPage>>().get();
    let measurement = world.resource::<Measurement>();
    if measurement.previous == current {
        return Ok(());
    }
    let elapsed = measurement.start.elapsed().as_secs_f64() * 1000.0;
    let previous = measurement.previous;
    let entities = world.entities().count_spawned();
    let mut measurement = world.resource_mut::<Measurement>();
    measurement.previous = current;
    (|| -> std::io::Result<()> {
        writeln!(
            measurement.samples,
            "{previous:?},{current:?},{elapsed},{entities}"
        )?;
        measurement.samples.flush()
    })()
    .map_err(|error| {
        error!(path = %measurement.output.display(), %error, "无法写入页面 lifecycle 测量结果");
        BevyError::error(error)
    })
}
