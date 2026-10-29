use super::{CursorPreview, CursorPreviewAssets};
use crate::gallery::GalleryPage;
use bevy::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};

#[derive(Resource)]
struct Measurement {
    writer: BufWriter<File>,
    epoch: Instant,
    frame: Instant,
    previous: Instant,
    flush: Instant,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(First, begin.run_if(resource_exists::<Measurement>))
        .add_systems(Last, record.run_if(resource_exists::<Measurement>));
}

fn setup(mut commands: Commands) -> Result {
    let Some(path) = std::env::var_os("GALLERY_CURSOR_BENCH_OUTPUT") else {
        return Ok(());
    };
    let mut writer = BufWriter::new(File::create_new(path).map_err(failure)?);
    writeln!(
        writer,
        "elapsed_ms,page,update_ms,interval_ms,previews,ready,page_assets,layouts,entities,indices"
    )
    .map_err(failure)?;
    let now = Instant::now();
    commands.insert_resource(Measurement {
        writer,
        epoch: now,
        frame: now,
        previous: now,
        flush: now,
    });
    Ok(())
}

fn begin(mut measurement: ResMut<Measurement>) {
    measurement.frame = Instant::now();
}

fn record(
    mut measurement: ResMut<Measurement>,
    page: Res<State<GalleryPage>>,
    assets: Option<Res<CursorPreviewAssets>>,
    previews: Query<(&CursorPreview, &ImageNode)>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    entities: &bevy::ecs::entity::Entities,
    exit: Res<Messages<AppExit>>,
) -> Result {
    let now = Instant::now();
    let elapsed = now.duration_since(measurement.epoch).as_secs_f64() * 1000.0;
    let update = now.duration_since(measurement.frame).as_secs_f64() * 1000.0;
    let interval = now.duration_since(measurement.previous).as_secs_f64() * 1000.0;
    measurement.previous = now;
    let mut ready = 0;
    let mut indices = Vec::new();
    for (preview, node) in &previews {
        if node.image.path() == Some(&preview.0.path()) {
            ready += 1;
            if let Some(atlas) = &node.texture_atlas {
                indices.push(format!("{:?}:{}", preview.0, atlas.index));
            }
        }
    }
    indices.sort();
    writeln!(
        measurement.writer,
        "{elapsed:.4},{:?},{update:.4},{interval:.4},{},{ready},{},{},{},{}",
        page.get(),
        previews.iter().len(),
        assets.is_some(),
        layouts.len(),
        entities.count_spawned(),
        indices.join(";")
    )
    .map_err(failure)?;
    if now.duration_since(measurement.flush).as_millis() >= 500 || !exit.is_empty() {
        measurement.writer.flush().map_err(failure)?;
        measurement.flush = now;
    }
    Ok(())
}

fn failure(error: std::io::Error) -> BevyError {
    error!(%error, "Cursor benchmark 文件操作失败");
    BevyError::error(error)
}
