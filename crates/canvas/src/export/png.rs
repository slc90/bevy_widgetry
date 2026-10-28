use super::CanvasExportError;
use crate::CanvasDocument;
use bevy::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode, visibility::RenderLayers},
    core_pipeline::fullscreen_material::FullscreenMaterialPipelineId,
    render::{
        Render, RenderApp, RenderSystems,
        erased_render_asset::ErasedRenderAssets,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::RenderMesh,
        render_asset::RenderAssets,
        render_resource::{
            Extent3d, PipelineCache, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        texture::GpuImage,
        view::{
            ViewTarget,
            screenshot::{Screenshot, ScreenshotCaptured},
        },
    },
    sprite_render::PreparedMaterial2d,
};
use bevy_widgetry_core::{offscreen::OffscreenTarget, scene::spawn_scene};
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_job::{
    JobAccessError, JobContext, JobId, JobManager, JobManagerPlugin, JobMessage, JobOptions,
    JobOutcome, JobReceiver, JobState, TryRecvError,
};
use event_listener::Event;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct CanvasPngOptions {
    pub region: Option<Rect>,
    pub scale: f32,
}

impl Default for CanvasPngOptions {
    fn default() -> Self {
        Self {
            region: None,
            scale: 1.0,
        }
    }
}

#[derive(EntityEvent)]
pub struct CanvasPngExportFinished {
    pub entity: Entity,
    pub job_id: JobId,
    pub document: Entity,
    pub path: PathBuf,
    pub result: std::result::Result<UVec2, CanvasExportError>,
}

impl CanvasDocument {
    pub fn export_png(
        world: &mut World,
        document: Entity,
        path: impl AsRef<Path>,
        options: CanvasPngOptions,
    ) -> std::result::Result<Entity, CanvasExportError> {
        let path = path.as_ref();
        let result = (|| {
            let snapshot = super::snapshot(world, document, options.region)?;
            if !options.scale.is_finite() || options.scale <= 0.0 {
                return Err(CanvasExportError::InvalidScale);
            }
            if !world.contains_resource::<PngRenderer>() {
                return Err(CanvasExportError::RendererUnavailable);
            }
            let device = world
                .get_resource::<RenderDevice>()
                .ok_or(CanvasExportError::RendererUnavailable)?;
            let limits = device.limits();
            let size = dimensions(
                snapshot.region,
                options.scale,
                limits.max_texture_dimension_2d,
                limits.max_buffer_size,
            )?;
            let tiles = Tiles::new(
                size,
                snapshot.region,
                limits.max_texture_dimension_2d,
                limits.max_buffer_size,
            )?;
            begin(world, document, path.to_owned(), snapshot, tiles)
        })();
        result
            .inspect_err(|error| widgetry_error!(?document,?path,%error,"Canvas PNG 导出请求失败"))
    }
}

fn dimensions(
    region: Rect,
    scale: f32,
    max_texture: u32,
    max_buffer: u64,
) -> std::result::Result<UVec2, CanvasExportError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(CanvasExportError::InvalidScale);
    }
    let w = (f64::from(region.width()) * f64::from(scale)).ceil();
    let h = (f64::from(region.height()) * f64::from(scale)).ceil();
    if !w.is_finite()
        || !h.is_finite()
        || w < 1.0
        || h < 1.0
        || w > 16_777_216.0
        || h > 16_777_216.0
    {
        return Err(CanvasExportError::InvalidResolution);
    }
    let size = UVec2::new(w as u32, h as u32);
    Tiles::new(size, region, max_texture, max_buffer)?.tile_region()?;
    Ok(size)
}

const BAND_BYTES: u64 = 32 * 1024 * 1024;
const GUARD: u32 = 2;

#[derive(Clone, Copy)]
struct Tiles {
    size: UVec2,
    region: Rect,
    step: UVec2,
    origin: UVec2,
}

#[derive(Clone, Copy)]
struct Tile {
    origin: UVec2,
    size: UVec2,
    render_origin: UVec2,
    render_size: UVec2,
}

impl Tiles {
    fn new(
        size: UVec2,
        region: Rect,
        max_texture: u32,
        max_buffer: u64,
    ) -> std::result::Result<Self, CanvasExportError> {
        let row_bytes = u64::from(size.x) * 4;
        // Bound total work as well as the row assembly buffer, independently of GPU limits.
        if size.x == 0
            || size.y == 0
            || size.max_element() > 16_777_216
            || row_bytes > BAND_BYTES
            || u64::from(size.x) * u64::from(size.y) > u64::from(u32::MAX)
            || max_texture <= 2 * GUARD
            || max_buffer < 256
        {
            return Err(CanvasExportError::InvalidResolution);
        }
        let mut step = UVec2::new(
            2048.min(max_texture - 2 * GUARD).min(size.x),
            512.min(max_texture - 2 * GUARD)
                .min(size.y)
                .min((BAND_BYTES / row_bytes) as u32),
        );
        loop {
            let render = (step + UVec2::splat(2 * GUARD)).min(size);
            let padded = (u64::from(render.x) * 4).div_ceil(256) * 256 * u64::from(render.y);
            if padded <= max_buffer
                && u64::from(render.x) * u64::from(render.y) * 80 + padded + BAND_BYTES
                    <= 512 * 1024 * 1024
            {
                break;
            }
            if step.x >= step.y && step.x > 1 {
                step.x = step.x.div_ceil(2);
            } else if step.y > 1 {
                step.y = step.y.div_ceil(2);
            } else {
                return Err(CanvasExportError::InvalidResolution);
            }
        }
        Ok(Self {
            size,
            region,
            step,
            origin: UVec2::ZERO,
        })
    }

    fn tile(&self) -> Tile {
        let size = self.step.min(self.size - self.origin);
        let render_origin = self.origin.saturating_sub(UVec2::splat(GUARD));
        let end = (self.origin + size + UVec2::splat(GUARD)).min(self.size);
        Tile {
            origin: self.origin,
            size,
            render_origin,
            render_size: end - render_origin,
        }
    }

    fn tile_region(&self) -> std::result::Result<Rect, CanvasExportError> {
        let tile = self.tile();
        let pixel = self.region.size().as_dvec2() / self.size.as_dvec2();
        let min = self.region.min.as_dvec2() + tile.render_origin.as_dvec2() * pixel;
        let extent = tile.render_size.as_dvec2() * pixel;
        let region = Rect::from_corners(min.as_vec2(), (min + extent).as_vec2());
        // Nonzero f32 extents can still shift large-coordinate tiles by whole pixels.
        // Bound the camera's worst edge displacement below a raster subpixel (1/256 pixel).
        let error = (region.center().as_dvec2() - (min + extent * 0.5)).abs()
            + (region.size().as_dvec2() - extent).abs() * 0.5;
        if !region.min.is_finite()
            || !region.max.is_finite()
            || region.width() <= 0.0
            || region.height() <= 0.0
            || !(error / pixel).is_finite()
            || (error / pixel).max_element() > 1.0 / 256.0
        {
            return Err(CanvasExportError::InvalidResolution);
        }
        Ok(region)
    }

    fn advance(&mut self) -> bool {
        self.origin.x += self.tile().size.x;
        if self.origin.x == self.size.x {
            self.origin.x = 0;
            self.origin.y += self.step.y.min(self.size.y - self.origin.y);
        }
        self.origin.y < self.size.y
    }
}

enum Phase {
    Queued(super::Snapshot),
    Preparing,
    Capturing,
    Writing,
    Finishing,
}

#[derive(Component)]
pub struct CanvasPngExport {
    id: JobId,
    phase: Phase,
    resources: Option<RenderResources>,
    tiles: Tiles,
}

impl CanvasPngExport {
    pub fn job_id(&self) -> JobId {
        self.id
    }
}

struct RenderResources {
    target: OffscreenTarget,
    scene: Entity,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<ColorMaterial>>,
    ready: Arc<AtomicBool>,
    started: Instant,
    capture: Option<Entity>,
}

#[derive(Resource, Default)]
struct Exports(BTreeMap<Entity, Operation>);

struct Operation {
    id: JobId,
    document: Entity,
    path: PathBuf,
    bridge: Arc<Bridge>,
    receiver: JobReceiver<()>,
    finished: Option<JobOutcome>,
}

#[derive(Resource, Default)]
struct CleanupQueue(Vec<(Arc<Bridge>, RenderResources)>);

#[derive(Default)]
struct Bridge {
    state: Mutex<Handoff>,
    changed: Event,
}

#[derive(Default)]
struct Handoff {
    input: Option<ExportInput>,
    result: Option<std::result::Result<UVec2, CanvasExportError>>,
    released: bool,
    consumed: bool,
}

enum ExportInput {
    Image(Image),
    Failed(CanvasExportError),
    Canceled,
}

impl Bridge {
    fn lock(&self) -> MutexGuard<'_, Handoff> {
        self.state.lock().unwrap_or_else(|error| {
            widgetry_error!("Canvas PNG 交接锁中毒，恢复状态以完成收尾");
            error.into_inner()
        })
    }

    fn deliver(&self, input: ExportInput) {
        let mut handoff = self.lock();
        // A later cancellation must not erase a failure the encoder has yet to consume.
        if !matches!(handoff.input, Some(ExportInput::Failed(_))) {
            handoff.input = Some(input);
        }
        drop(handoff);
        self.changed.notify(usize::MAX);
    }

    async fn input(&self) -> ExportInput {
        loop {
            let changed = self.changed.listen();
            if let Some(input) = self.lock().input.take() {
                return input;
            }
            changed.await;
        }
    }

    async fn released(&self) {
        loop {
            let changed = self.changed.listen();
            if self.lock().released {
                return;
            }
            changed.await;
        }
    }
}

#[derive(Component, FromTemplate)]
struct Parts {
    camera: Entity,
    content: Entity,
}

#[derive(Component, Clone, ExtractComponent)]
#[extract_app(RenderApp)]
struct Probe {
    meshes: Vec<AssetId<Mesh>>,
    materials: Vec<AssetId<ColorMaterial>>,
    image: AssetId<Image>,
    ready: Arc<AtomicBool>,
}

#[derive(Resource)]
struct PngRenderer;

pub(super) fn install(app: &mut App) {
    if !app.is_plugin_added::<JobManagerPlugin>() {
        app.add_plugins(JobManagerPlugin);
    }
    app.insert_resource(PngRenderer)
        .init_resource::<Exports>()
        .init_resource::<CleanupQueue>()
        .add_plugins(ExtractComponentPlugin::<Probe>::default())
        .add_observer(release)
        .add_systems(Update, poll_jobs)
        .add_systems(Last, redraw_exports);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(
            Render,
            mark_ready
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
    }
}

fn mark_ready(
    probes: Query<(
        &Probe,
        &FullscreenMaterialPipelineId<crate::alpha::StraightAlpha>,
        &ViewTarget,
    )>,
    meshes: Res<RenderAssets<RenderMesh>>,
    materials: Res<ErasedRenderAssets<PreparedMaterial2d>>,
    images: Res<RenderAssets<GpuImage>>,
    pipelines: Res<PipelineCache>,
) {
    if pipelines.waiting_pipelines().next().is_some() {
        return;
    }
    for (probe, pipeline, _) in &probes {
        if pipelines.get_render_pipeline(pipeline.0).is_some()
            && images.get(probe.image).is_some()
            && probe.meshes.iter().all(|id| meshes.get(*id).is_some())
            && probe
                .materials
                .iter()
                .all(|id| materials.get(*id).is_some())
        {
            probe.ready.store(true, Ordering::Release);
        }
    }
}

fn begin(
    world: &mut World,
    document: Entity,
    path: PathBuf,
    snapshot: super::Snapshot,
    tiles: Tiles,
) -> std::result::Result<Entity, CanvasExportError> {
    let manager = world
        .get_resource::<JobManager>()
        .ok_or(CanvasExportError::RendererUnavailable)?;
    let bridge = Arc::new(Bridge::default());
    let handoff = bridge.clone();
    let output = path.clone();
    let (id, receiver) = manager
        .spawn_cpu(
            JobOptions {
                label: Some("Canvas PNG export".into()),
                ..default()
            },
            move |context| encode(context, handoff, output, tiles),
        )
        .map_err(CanvasExportError::Job)?;
    let entity = world
        .spawn(CanvasPngExport {
            id,
            phase: Phase::Queued(snapshot),
            resources: None,
            tiles,
        })
        .id();
    world.resource_mut::<Exports>().0.insert(
        entity,
        Operation {
            id,
            document,
            path,
            bridge,
            receiver,
            finished: None,
        },
    );
    Ok(entity)
}

async fn encode(
    context: JobContext<()>,
    bridge: Arc<Bridge>,
    path: PathBuf,
    tiles: Tiles,
) -> std::result::Result<(), String> {
    let result = encode_tiles(&context, &bridge, &path, tiles)
        .await
        .map(|()| tiles.size);
    let canceled = matches!(result, Err(CanvasExportError::Canceled));
    let failure = result.as_ref().err().map(ToString::to_string);
    bridge.lock().result = Some(result);
    bridge.released().await;
    if canceled {
        context.acknowledge_cancel();
        Ok(())
    } else {
        failure.map_or(Ok(()), Err)
    }
}

async fn encode_tiles(
    context: &JobContext<()>,
    bridge: &Bridge,
    path: &Path,
    mut tiles: Tiles,
) -> std::result::Result<(), CanvasExportError> {
    let mut input = bridge.input().await;
    let first = tile_image(input, context)?;
    let file = std::fs::File::create(path).map_err(CanvasExportError::Io)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), tiles.size.x, tiles.size.y);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder.write_header().map_err(encoding_error)?;
    let mut stream = writer.stream_writer().map_err(encoding_error)?;
    let row_bytes = tiles.size.x as usize * 4;
    let mut band = vec![0; row_bytes * tiles.step.y as usize];
    let mut image = first;
    loop {
        let tile = tiles.tile();
        copy_tile(&mut band, row_bytes, &image, tile)?;
        drop(image);
        if tile.origin.x + tile.size.x == tiles.size.x {
            for row in band[..row_bytes * tile.size.y as usize].chunks_exact(row_bytes) {
                if context.is_cancelled() {
                    return Err(CanvasExportError::Canceled);
                }
                stream.write_all(row).map_err(CanvasExportError::Io)?;
            }
        }
        if !tiles.advance() {
            break;
        }
        bridge.lock().consumed = true;
        input = bridge.input().await;
        image = tile_image(input, context)?;
    }
    stream.finish().map_err(encoding_error)?;
    writer.finish().map_err(encoding_error)
}

fn tile_image(
    input: ExportInput,
    context: &JobContext<()>,
) -> std::result::Result<image::RgbaImage, CanvasExportError> {
    match input {
        ExportInput::Image(_) if context.is_cancelled() => Err(CanvasExportError::Canceled),
        ExportInput::Image(image) => image
            .try_into_dynamic()
            .map(|image| image.into_rgba8())
            .map_err(|error| CanvasExportError::Render(error.to_string())),
        ExportInput::Failed(error) => Err(error),
        ExportInput::Canceled => Err(CanvasExportError::Canceled),
    }
}

fn copy_tile(
    band: &mut [u8],
    row_bytes: usize,
    image: &image::RgbaImage,
    tile: Tile,
) -> std::result::Result<(), CanvasExportError> {
    if image.dimensions() != (tile.render_size.x, tile.render_size.y) {
        return Err(CanvasExportError::Render(
            "Readback tile dimensions do not match the request".into(),
        ));
    }
    let crop = tile.origin - tile.render_origin;
    for y in 0..tile.size.y as usize {
        let src = ((crop.y as usize + y) * tile.render_size.x as usize + crop.x as usize) * 4;
        let dst = y * row_bytes + tile.origin.x as usize * 4;
        let count = tile.size.x as usize * 4;
        band[dst..dst + count].copy_from_slice(&image.as_raw()[src..src + count]);
    }
    Ok(())
}

fn encoding_error(error: png::EncodingError) -> CanvasExportError {
    match error {
        png::EncodingError::IoError(error) => CanvasExportError::Io(error),
        error => CanvasExportError::Encoding(image::ImageError::Encoding(
            image::error::EncodingError::new(
                image::error::ImageFormatHint::Exact(image::ImageFormat::Png),
                error,
            ),
        )),
    }
}

fn next_tile(world: &mut World, entity: Entity) -> std::result::Result<(), CanvasExportError> {
    let Some(mut job) = world.get_mut::<CanvasPngExport>(entity) else {
        return Ok(());
    };
    if !job.tiles.advance() {
        return Ok(());
    }
    let region = job.tiles.tile_region()?;
    let size = job.tiles.tile().render_size;
    job.phase = Phase::Preparing;
    let owner = job
        .resources
        .as_mut()
        .ok_or_else(|| CanvasExportError::Render("Export tile resources missing".into()))?;
    let image = owner.target.image().clone();
    let scene = owner.scene;
    let ready = Arc::new(AtomicBool::new(false));
    owner.ready = ready.clone();
    owner.started = Instant::now();
    owner.capture = None;
    let camera = world
        .get::<Parts>(scene)
        .ok_or_else(|| CanvasExportError::Render("Export scene parts missing".into()))?
        .camera;
    set_image(world, image.clone(), size)?;
    let center = region.center();
    world.entity_mut(camera).insert((
        Transform::from_xyz(center.x, -center.y, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: region.width(),
                height: region.height(),
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    if let Some(mut probe) = world.get_mut::<Probe>(camera) {
        probe.ready = ready;
    } else {
        return Err(CanvasExportError::Render(
            "Export readiness probe missing".into(),
        ));
    }
    Ok(())
}

fn set_image(
    world: &mut World,
    image_handle: Handle<Image>,
    size: UVec2,
) -> std::result::Result<(), CanvasExportError> {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    world
        .resource_mut::<Assets<Image>>()
        .insert(image_handle.id(), image)
        .map_err(|error| CanvasExportError::Render(error.to_string()))
}

fn prepare(
    world: &mut World,
    entity: Entity,
    snapshot: super::Snapshot,
    size: UVec2,
) -> std::result::Result<(), CanvasExportError> {
    let target =
        OffscreenTarget::allocate(world).map_err(|e| CanvasExportError::Render(e.to_string()))?;
    let layer = target.layer();
    let image_handle = target.image().clone();
    if let Err(error) = set_image(world, image_handle.clone(), size) {
        target.release(world);
        return Err(error);
    }
    let region = snapshot.region;
    let center = region.center();
    let scene = match spawn_scene(
        world,
        bsn! {
            Name("CanvasPngScene") Parts {camera:#Camera,content:#Content} Transform::default() Visibility::default()
            Children [
                #Camera Name("CanvasPngCamera") Camera2d
                Camera {order:-1,clear_color:ClearColorConfig::Custom(Color::NONE)}
                Transform::from_xyz(center.x,-center.y,0.0) Msaa::Sample4 crate::alpha::StraightAlpha::default()
                // Keep translucent MSAA samples linear until the final sRGB output conversion.
                bevy::camera::Hdr
                bevy::core_pipeline::tonemapping::Tonemapping::None
                bevy::core_pipeline::tonemapping::DebandDither::Disabled
                template(move |_|Ok(RenderLayers::none().with(layer)))
                template(move |_|Ok(RenderTarget::Image(image_handle.clone().into())))
                template(move |_|Ok(Projection::Orthographic(OrthographicProjection {scaling_mode:ScalingMode::Fixed {width:region.width(),height:region.height()},..OrthographicProjection::default_2d()})))
            --
                #Content Name("CanvasPngContent") Transform::from_scale(Vec3::new(1.0,-1.0,1.0)) Visibility::default()
            ]
        },
    ) {
        Ok(scene) => scene,
        Err(error) => {
            target.release(world);
            return Err(CanvasExportError::Render(error.to_string()));
        }
    };
    let Some(parts) = world.get::<Parts>(scene) else {
        target.release(world);
        world.despawn(scene);
        return Err(CanvasExportError::Render(
            "Export scene parts missing".to_owned(),
        ));
    };
    let (camera, content) = (parts.camera, parts.content);
    let ready = Arc::new(AtomicBool::new(false));
    world
        .entity_mut(entity)
        .get_mut::<CanvasPngExport>()
        .ok_or_else(|| CanvasExportError::Render("Export operation missing".into()))?
        .resources = Some(RenderResources {
        target,
        scene,
        meshes: Vec::new(),
        materials: Vec::new(),
        ready: ready.clone(),
        started: Instant::now(),
        capture: None,
    });
    geometry(world, entity, content, layer, &snapshot)?;
    let job = world
        .get::<CanvasPngExport>(entity)
        .and_then(|job| job.resources.as_ref())
        .ok_or_else(|| CanvasExportError::Render("Export job missing".to_owned()))?;
    let probe = Probe {
        meshes: job.meshes.iter().map(Handle::id).collect(),
        materials: job.materials.iter().map(Handle::id).collect(),
        image: job.target.image().id(),
        ready,
    };
    world.entity_mut(camera).insert(probe);
    Ok(())
}

fn geometry(
    world: &mut World,
    job: Entity,
    content: Entity,
    layer: usize,
    snapshot: &super::Snapshot,
) -> std::result::Result<(), CanvasExportError> {
    let mut paints = Vec::new();
    for shape in &snapshot.shapes {
        let (fill_mesh, stroke_mesh) = crate::geometry::meshes(&shape.geometry, shape.paint)
            .map_err(|error| CanvasExportError::InvalidGeometry(error.to_string()))?;
        if let Some(color) = shape.paint.fill
            && fill_mesh.count_vertices() > 0
        {
            paints.push((shape, color, fill_mesh));
        }
        if let Some(stroke) = shape.paint.stroke
            && stroke_mesh.count_vertices() > 0
        {
            paints.push((shape, stroke.color, stroke_mesh));
        }
    }
    let count = paints.len().max(1) as f32;
    for (index, (shape, color, mesh)) in paints.into_iter().enumerate() {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let material = world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial { color, ..default() });
        let mut export = world
            .get_mut::<CanvasPngExport>(job)
            .ok_or_else(|| CanvasExportError::Render("Export job missing".to_owned()))?;
        let owner = export
            .resources
            .as_mut()
            .ok_or_else(|| CanvasExportError::Render("Export resources missing".to_owned()))?;
        owner.meshes.push(mesh.clone());
        owner.materials.push(material.clone());
        let mut transform = shape.transform;
        transform.scale.z = 1.0;
        // Map document paint order to bounded export depth independently of model Z.
        transform.translation.z = index as f32 / count * 100.0;
        spawn_scene(
            world,
            bsn! {
                Name("CanvasPngPaint") template(move |_|Ok(ChildOf(content)))
                template(move |_|Ok(Mesh2d(mesh.clone())))
                template(move |_|Ok(MeshMaterial2d(material.clone())))
                template(move |_|Ok(RenderLayers::none().with(layer)))
                template(move |_|Ok(transform)) Visibility::default() Pickable::IGNORE
                bevy::camera::visibility::NoFrustumCulling
            },
        )
        .map_err(|error| CanvasExportError::Render(error.to_string()))?;
    }
    Ok(())
}

fn poll_jobs(world: &mut World) -> Result {
    let manager = world.resource::<JobManager>().clone();
    let entities: Vec<_> = world.resource::<Exports>().0.keys().copied().collect();
    for entity in entities {
        let Some(operation) = world.resource::<Exports>().0.get(&entity) else {
            continue;
        };
        let (id, bridge) = (operation.id, operation.bridge.clone());
        let state = match manager.status(id) {
            Ok(status) => Some(status.state),
            Err(JobAccessError::NotFound) => None,
            Err(error) => return Err(BevyError::error(error.to_string())),
        };
        let capture_busy = world.get::<CanvasPngExport>(entity).is_some_and(|job| {
            capture_in_flight(
                world,
                job.resources
                    .as_ref()
                    .and_then(|resources| resources.capture),
            )
        });
        if let Some(mut job) = world.get_mut::<CanvasPngExport>(entity) {
            if matches!(job.phase, Phase::Queued(_)) && state == Some(JobState::Stopping) {
                fail_export(world, entity, &bridge, ExportInput::Canceled);
            } else if matches!(job.phase, Phase::Queued(_)) && state == Some(JobState::Running) {
                let Phase::Queued(mut snapshot) =
                    std::mem::replace(&mut job.phase, Phase::Preparing)
                else {
                    continue;
                };
                snapshot.region = match job.tiles.tile_region() {
                    Ok(region) => region,
                    Err(error) => {
                        fail_export(world, entity, &bridge, ExportInput::Failed(error));
                        continue;
                    }
                };
                let size = job.tiles.tile().render_size;
                if let Err(error) = prepare(world, entity, snapshot, size) {
                    fail_export(world, entity, &bridge, ExportInput::Failed(error));
                }
            } else {
                let preparing = matches!(job.phase, Phase::Preparing);
                let capturing = matches!(job.phase, Phase::Capturing);
                let resources = job.resources.as_ref();
                let timed_out = (preparing || capturing)
                    && resources.is_some_and(|resources| {
                        resources.started.elapsed() > Duration::from_secs(30)
                    });
                let ready =
                    resources.is_some_and(|resources| resources.ready.load(Ordering::Acquire));
                if timed_out {
                    fail_export(
                        world,
                        entity,
                        &bridge,
                        ExportInput::Failed(CanvasExportError::Render(
                            "Timed out waiting for export GPU rendering/readback".into(),
                        )),
                    );
                } else if state == Some(JobState::Stopping)
                    && (preparing
                        || capturing && !capture_busy
                        || matches!(job.phase, Phase::Writing))
                {
                    fail_export(world, entity, &bridge, ExportInput::Canceled);
                } else if matches!(job.phase, Phase::Writing)
                    && bridge.lock().consumed
                    && !capture_busy
                {
                    bridge.lock().consumed = false;
                    if let Err(error) = next_tile(world, entity) {
                        fail_export(world, entity, &bridge, ExportInput::Failed(error));
                    }
                } else if preparing && ready {
                    let image = resources.map(|resources| resources.target.image().clone());
                    if let Some(image) = image {
                        job.phase = Phase::Capturing;
                        let capture = request_capture(world, entity, id, image);
                        if let Some(mut job) = world.get_mut::<CanvasPngExport>(entity)
                            && let Some(resources) = &mut job.resources
                        {
                            resources.capture = Some(capture);
                        }
                    }
                }
            }
        }
        let finished = {
            let mut exports = world.resource_mut::<Exports>();
            let Some(operation) = exports.0.get_mut(&entity) else {
                continue;
            };
            if operation.finished.is_none() {
                match operation.receiver.try_recv() {
                    Ok(JobMessage::Finished(outcome)) => operation.finished = Some(outcome),
                    Ok(JobMessage::Item(_)) | Err(TryRecvError::Empty) => {}
                    Err(error) => {
                        widgetry_error!(job_id=id.get(), %error, "Canvas PNG Job 结果接收失败");
                        return Err(BevyError::error(error.to_string()));
                    }
                }
            }
            operation.finished.clone()
        };
        if let Some(outcome) = finished {
            let mut handoff = bridge.lock();
            if handoff.result.is_none() {
                handoff.result = Some(Err(match outcome {
                    JobOutcome::Canceled => CanvasExportError::Canceled,
                    JobOutcome::Failed(error) => CanvasExportError::Render(error.to_string()),
                    JobOutcome::Completed => {
                        CanvasExportError::Render("PNG Job completed without a result".into())
                    }
                }));
            }
        }
        if bridge.lock().result.is_some()
            && world
                .get::<CanvasPngExport>(entity)
                .is_some_and(|job| !matches!(job.phase, Phase::Finishing))
        {
            begin_cleanup(world, entity, &bridge);
        }
    }
    cleanup_resources(world);
    let completed: Vec<_> = world
        .resource::<Exports>()
        .0
        .iter()
        .filter_map(|(entity, operation)| {
            (operation.finished.is_some() && operation.bridge.lock().released).then_some(*entity)
        })
        .collect();
    for entity in completed {
        let Some(operation) = world.resource_mut::<Exports>().0.remove(&entity) else {
            continue;
        };
        let result = operation
            .bridge
            .lock()
            .result
            .take()
            .ok_or_else(|| BevyError::error("Canvas PNG completion result missing"))?;
        if let Err(error) = &result
            && !matches!(error, CanvasExportError::Canceled)
        {
            widgetry_error!(job_id=operation.id.get(), ?entity, document=?operation.document, path=?operation.path, %error, "Canvas PNG 导出失败");
        }
        world.trigger(CanvasPngExportFinished {
            entity,
            job_id: operation.id,
            document: operation.document,
            path: operation.path,
            result,
        });
        if world.get_entity(entity).is_ok() {
            world.despawn(entity);
        }
        match manager.remove(operation.id) {
            Ok(_) | Err(JobAccessError::NotFound) => {}
            Err(error) => return Err(BevyError::error(error.to_string())),
        }
    }
    Ok(())
}

fn fail_export(world: &mut World, entity: Entity, bridge: &Bridge, input: ExportInput) {
    if let Some(mut job) = world.get_mut::<CanvasPngExport>(entity) {
        job.phase = Phase::Writing;
    }
    bridge.deliver(input);
}

fn request_capture(world: &mut World, entity: Entity, id: JobId, image: Handle<Image>) -> Entity {
    world
        .spawn(Screenshot::image(image))
        .observe(
            move |event: On<ScreenshotCaptured>,
                  jobs: Query<&CanvasPngExport>,
                  mut commands: Commands| {
                let capture = event.entity;
                let Ok(job) = jobs.get(entity) else {
                    return;
                };
                if job.id != id
                    || !matches!(job.phase, Phase::Capturing)
                    || job
                        .resources
                        .as_ref()
                        .and_then(|resources| resources.capture)
                        != Some(capture)
                {
                    return;
                }
                let image = event.image.clone();
                commands.queue(move |world: &mut World| {
                    let Some(mut job) = world.get_mut::<CanvasPngExport>(entity) else {
                        return;
                    };
                    if job.id != id
                        || !matches!(job.phase, Phase::Capturing)
                        || job
                            .resources
                            .as_ref()
                            .and_then(|resources| resources.capture)
                            != Some(capture)
                    {
                        return;
                    }
                    job.phase = Phase::Writing;
                    if let Some(operation) = world.resource::<Exports>().0.get(&entity)
                        && operation.id == id
                    {
                        operation.bridge.deliver(ExportInput::Image(image));
                    }
                });
            },
        )
        .id()
}

fn capture_in_flight(world: &World, capture: Option<Entity>) -> bool {
    capture.is_some_and(|capture| {
        world
            .get::<bevy::render::view::screenshot::Capturing>(capture)
            .is_some()
            && world
                .get::<bevy::render::view::screenshot::Captured>(capture)
                .is_none()
    })
}

fn begin_cleanup(world: &mut World, entity: Entity, bridge: &Arc<Bridge>) {
    let resources = world
        .get_mut::<CanvasPngExport>(entity)
        .and_then(|mut job| {
            job.phase = Phase::Finishing;
            job.resources.take()
        });
    queue_cleanup(world, bridge.clone(), resources);
}

fn queue_cleanup(world: &mut World, bridge: Arc<Bridge>, resources: Option<RenderResources>) {
    if let Some(resources) = resources {
        world
            .resource_mut::<CleanupQueue>()
            .0
            .push((bridge, resources));
    } else {
        bridge.lock().released = true;
        bridge.changed.notify(usize::MAX);
    }
}

fn cleanup_resources(world: &mut World) {
    let pending = std::mem::take(&mut world.resource_mut::<CleanupQueue>().0);
    for (bridge, resources) in pending {
        // A submitted readback still owns its target until Bevy delivers Captured.
        // Keep the image, layer and scene alive even if the request entity was removed.
        if capture_in_flight(world, resources.capture) {
            world
                .resource_mut::<CleanupQueue>()
                .0
                .push((bridge, resources));
            continue;
        }
        resources.target.release(world);
        if world.get_entity(resources.scene).is_ok() {
            world.despawn(resources.scene);
        }
        if let Some(capture) = resources.capture
            && world.get_entity(capture).is_ok()
            && world
                .get::<bevy::render::view::screenshot::Captured>(capture)
                .is_none()
        {
            world.despawn(capture);
        }
        for mesh in resources.meshes {
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
        }
        for material in resources.materials {
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
        }
        bridge.lock().released = true;
        bridge.changed.notify(usize::MAX);
    }
}

fn release(
    event: On<Discard<CanvasPngExport>>,
    mut jobs: Query<&mut CanvasPngExport>,
    exports: Res<Exports>,
    manager: Res<JobManager>,
    mut commands: Commands,
) -> Result {
    let Some(operation) = exports.0.get(&event.entity) else {
        return Ok(());
    };
    let Ok(mut job) = jobs.get_mut(event.entity) else {
        return Ok(());
    };
    let bridge = operation.bridge.clone();
    let resources = job.resources.take();
    if !matches!(job.phase, Phase::Finishing) {
        match manager.cancel(operation.id) {
            Ok(_) | Err(JobAccessError::NotFound) => {}
            Err(error) => return Err(BevyError::error(error.to_string())),
        }
        let mut handoff = bridge.lock();
        if handoff.result.is_none() && handoff.input.is_none() {
            handoff.input = Some(ExportInput::Canceled);
        }
        drop(handoff);
        bridge.changed.notify(usize::MAX);
        commands.queue(move |world: &mut World| queue_cleanup(world, bridge, resources));
    }
    Ok(())
}

fn redraw_exports(exports: Res<Exports>, mut redraw: MessageWriter<bevy::window::RequestRedraw>) {
    if !exports.0.is_empty() {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::render::view::screenshot::{
        Captured, CapturedScreenshots, Capturing, trigger_screenshots,
    };

    #[test]
    fn high_resolution_dimensions_are_not_limited_by_a_full_frame_allocation() {
        let region = Rect::from_corners(Vec2::ZERO, Vec2::new(1920.0, 1080.0));
        assert_eq!(
            dimensions(region, 4.0, 4096, 256 * 1024 * 1024).unwrap(),
            UVec2::new(7680, 4320)
        );
    }

    #[test]
    fn tile_projection_rejects_nonzero_extents_that_distort_pixel_mapping() {
        for offset in [
            Vec2::new(100_000_000.0, 0.0),
            Vec2::new(0.0, -100_000_000.0),
        ] {
            let region = Rect::from_corners(offset, offset + Vec2::splat(4096.0));
            assert!(matches!(
                dimensions(region, 1.0, 16384, 1 << 30),
                Err(CanvasExportError::InvalidResolution)
            ));
            let mut tiles = Tiles::new(UVec2::splat(4096), region, 16384, 1 << 30).unwrap();
            loop {
                assert!(matches!(
                    tiles.tile_region(),
                    Err(CanvasExportError::InvalidResolution)
                ));
                if !tiles.advance() {
                    break;
                }
            }
            assert_eq!(
                dimensions(region, 1.0 / 128.0, 16384, 1 << 30).unwrap(),
                UVec2::splat(32)
            );
        }
    }

    #[test]
    fn tile_guards_are_cropped_once_and_bands_cover_the_exact_output() {
        let size = UVec2::new(17, 11);
        let mut tiles = Tiles::new(
            size,
            Rect::from_corners(Vec2::ZERO, size.as_vec2()),
            64,
            1024 * 1024,
        )
        .unwrap();
        tiles.step = UVec2::new(5, 4);
        let mut band = vec![0; size.x as usize * 4 * tiles.step.y as usize];
        let mut output = Vec::new();
        loop {
            let tile = tiles.tile();
            let image =
                image::RgbaImage::from_fn(tile.render_size.x, tile.render_size.y, |x, y| {
                    image::Rgba([
                        (tile.render_origin.x + x) as u8,
                        (tile.render_origin.y + y) as u8,
                        17,
                        128,
                    ])
                });
            copy_tile(&mut band, size.x as usize * 4, &image, tile).unwrap();
            if tile.origin.x + tile.size.x == size.x {
                output.extend_from_slice(&band[..size.x as usize * 4 * tile.size.y as usize]);
            }
            if !tiles.advance() {
                break;
            }
        }
        let expected = image::RgbaImage::from_fn(size.x, size.y, |x, y| {
            image::Rgba([x as u8, y as u8, 17, 128])
        });
        assert_eq!(output, expected.into_raw());
        for (size, max_texture, max_buffer) in [
            (UVec2::new(7680, 4320), 4096, 256 * 1024 * 1024),
            (UVec2::new(130, 90), 64, 4096),
        ] {
            let plan = Tiles::new(
                size,
                Rect::from_corners(Vec2::ZERO, size.as_vec2()),
                max_texture,
                max_buffer,
            )
            .unwrap();
            assert!(u64::from(size.x) * 4 * u64::from(plan.step.y) <= BAND_BYTES);
            let render = plan.tile().render_size;
            assert!(render.max_element() <= max_texture);
            assert!(
                (u64::from(render.x) * 4).div_ceil(256) * 256 * u64::from(render.y) <= max_buffer
            );
        }
        let region = Rect::from_corners(Vec2::splat(1e12), Vec2::splat(1e12 + 65536.0));
        let plan = Tiles::new(UVec2::splat(32768), region, 4096, 1 << 30).unwrap();
        assert!(matches!(
            plan.tile_region(),
            Err(CanvasExportError::InvalidResolution)
        ));
    }

    #[test]
    #[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
    fn dx12_forced_tiles_match_full_frame_curves_holes_strokes_and_transparency() {
        use crate::{
            CanvasFillRule, CanvasGeometry, CanvasPaint, CanvasPath, CanvasPathCommand::*,
            CanvasStroke,
        };
        use bevy::render::{
            RenderPlugin,
            pipelined_rendering::PipelinedRenderingPlugin,
            settings::{Backends, RenderCreation, WgpuSettings},
        };
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(Backends::DX12),
                        ..default()
                    })),
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .disable::<PipelinedRenderingPlugin>(),
        )
        .add_plugins(crate::CanvasPlugin)
        .init_resource::<Notices>()
        .add_observer(
            |event: On<CanvasPngExportFinished>,
             mut notices: ResMut<Notices>,
             manager: Res<JobManager>| {
                notices.0.push(Notice {
                    entity: event.entity,
                    id: event.job_id,
                    result: event.result.as_ref().copied().map_err(ToString::to_string),
                    outcome: manager.outcome(event.job_id).unwrap().unwrap(),
                });
            },
        );
        while app.plugins_state() != bevy::app::PluginsState::Ready {
            bevy::tasks::tick_global_task_pools_on_main_thread();
            std::thread::sleep(Duration::from_millis(1));
        }
        app.finish();
        app.cleanup();
        for _ in 0..4 {
            app.update();
        }
        let document = app.world_mut().spawn(CanvasDocument).id();
        let paint = CanvasPaint {
            fill: Some(Color::srgba(0.1, 0.5, 0.8, 0.4)),
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.8, 0.1, 0.2, 0.5),
                width: 19.0,
                cap: crate::CanvasLineCap::Round,
                join: crate::CanvasLineJoin::Round,
            }),
        };
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::new(10.0, 30.0)),
                    CubicTo {
                        control1: Vec2::new(40.0, -20.0),
                        control2: Vec2::new(180.0, 190.0),
                        to: Vec2::new(235.0, 15.0),
                    },
                    QuadTo {
                        control: Vec2::new(270.0, 180.0),
                        to: Vec2::new(25.0, 170.0),
                    },
                    Close,
                    MoveTo(Vec2::new(80.0, 70.0)),
                    LineTo(Vec2::new(135.0, 75.0)),
                    LineTo(Vec2::new(110.0, 120.0)),
                    Close,
                    MoveTo(Vec2::new(20.0, 90.0)),
                    ArcTo {
                        radii: Vec2::new(80.0, 45.0),
                        rotation: 0.3,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(225.0, 150.0),
                    },
                ],
            },
            Transform::from_xyz(7.0, 5.0, 0.0)
                .with_rotation(Quat::from_rotation_z(0.07))
                .with_scale(Vec3::new(0.95, 1.1, 1.0)),
            paint,
        )
        .unwrap();
        CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            crate::CanvasRectangle {
                size: Vec2::new(90.0, 70.0),
                corner_radius: 10.0,
            },
            Transform::from_xyz(145.0, 95.0, 0.0).with_rotation(Quat::from_rotation_z(-0.4)),
            paint,
        )
        .unwrap();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<CanvasGeometry>>()
                .iter(app.world())
                .count()
                > 0
        );
        let directory = tempfile::tempdir().unwrap();
        for scale in [1, 2, 4] {
            let region = Rect::from_corners(Vec2::new(-3.25, -7.5), Vec2::new(253.75, 185.5));
            let size = UVec2::new(257 * scale, 193 * scale);
            for tiled in [false, true] {
                let mut tiles = Tiles::new(size, region, 16384, 1 << 30).unwrap();
                tiles.step = if tiled { UVec2::new(64, 47) } else { size };
                let snapshot =
                    super::super::snapshot(app.world_mut(), document, Some(region)).unwrap();
                let before = app.world().resource::<Notices>().0.len();
                let path = directory.path().join(format!("{scale}-{tiled}.png"));
                begin(app.world_mut(), document, path, snapshot, tiles).unwrap();
                let deadline = Instant::now() + Duration::from_secs(90);
                while app.world().resource::<Notices>().0.len() == before {
                    assert!(Instant::now() < deadline);
                    app.update();
                    std::thread::sleep(Duration::from_millis(1));
                }
                assert_eq!(
                    app.world().resource::<Notices>().0.last().unwrap().result,
                    Ok(size)
                );
            }
            let full = image::open(directory.path().join(format!("{scale}-false.png")))
                .unwrap()
                .into_rgba8();
            let tiled = image::open(directory.path().join(format!("{scale}-true.png")))
                .unwrap()
                .into_rgba8();
            let max = full
                .as_raw()
                .iter()
                .zip(tiled.as_raw())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            let different = full
                .pixels()
                .zip(tiled.pixels())
                .filter(|(a, b)| a != b)
                .count();
            eprintln!("scale={scale} different_pixels={different} max_channel_difference={max}");
            // Tile projection rounding can move a grazing edge across an MSAA sample.
            assert!(
                max <= 4 && different <= (size.x * size.y / 1000) as usize,
                "scale {scale}: {different} different pixels, maximum channel difference {max}"
            );
        }
    }

    fn tiled_request(app: &mut App, path: PathBuf) -> (Entity, JobId) {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            crate::CanvasRectangle {
                size: Vec2::splat(10.0),
                corner_radius: 0.0,
            },
            Transform::default(),
            crate::CanvasPaint::default(),
        )
        .unwrap();
        let snapshot = super::super::snapshot(app.world_mut(), document, None).unwrap();
        let mut tiles = Tiles::new(UVec2::splat(10), snapshot.region, 64, 1 << 20).unwrap();
        tiles.step = UVec2::new(4, 4);
        let entity = begin(app.world_mut(), document, path, snapshot, tiles).unwrap();
        let id = app.world().get::<CanvasPngExport>(entity).unwrap().id;
        pump(app, |world| {
            world
                .get::<CanvasPngExport>(entity)
                .is_some_and(|job| matches!(job.phase, Phase::Preparing))
        });
        (entity, id)
    }

    fn deliver_tile(app: &mut App, entity: Entity, id: JobId) {
        let tile = app
            .world()
            .get::<CanvasPngExport>(entity)
            .unwrap()
            .tiles
            .tile();
        let capture = capturing(app, entity, id);
        app.world_mut().entity_mut(capture).insert(Captured);
        let image = Image::new_fill(
            Extent3d {
                width: tile.render_size.x,
                height: tile.render_size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[12, 34, 56, 128],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        );
        app.world_mut().trigger(ScreenshotCaptured {
            entity: capture,
            image,
        });
        app.world_mut().flush();
    }

    #[test]
    fn multiple_tiles_use_one_job_notify_once_and_reclaim_all_assets() {
        let (mut app, _) = test_app();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tiles.png");
        let before = app.world().resource::<Assets<Image>>().len();
        let (entity, id) = tiled_request(&mut app, path.clone());
        for index in 0..9 {
            assert_eq!(app.world().get::<CanvasPngExport>(entity).unwrap().id, id);
            deliver_tile(&mut app, entity, id);
            if index != 8 {
                pump(&mut app, |world| {
                    world
                        .get::<CanvasPngExport>(entity)
                        .is_some_and(|job| matches!(job.phase, Phase::Preparing))
                });
            }
        }
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert!(
            image::open(path)
                .unwrap()
                .to_rgba8()
                .pixels()
                .all(|p| p.0 == [12, 34, 56, 128])
        );
        assert_eq!(app.world().resource::<Assets<Image>>().len(), before);
        assert!(app.world().resource::<Exports>().0.is_empty());
        app.update();
        assert_eq!(app.world().resource::<Notices>().0.len(), 1);
    }

    #[test]
    fn cancel_between_tiles_wakes_the_encoder_and_reclaims_the_scene() {
        let (mut app, _) = test_app();
        let directory = tempfile::tempdir().unwrap();
        let (entity, id) = tiled_request(&mut app, directory.path().join("canceled.png"));
        deliver_tile(&mut app, entity, id);
        let bridge = app.world().resource::<Exports>().0[&entity].bridge.clone();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !bridge.lock().consumed {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        app.world().resource::<JobManager>().cancel(id).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Canceled
        );
        assert!(app.world().resource::<CleanupQueue>().0.is_empty());
        assert!(app.world().get_entity(entity).is_err());
    }

    #[test]
    fn cancel_does_not_overwrite_a_render_failure_waiting_for_the_encoder() {
        let (mut app, _) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let path = file.path().to_owned();
        let tiles = Tiles::new(
            UVec2::splat(10),
            Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0)),
            64,
            1 << 20,
        )
        .unwrap();
        let bridge = Arc::new(Bridge::default());
        let handoff = bridge.clone();
        let gate = Event::new();
        let listener = gate.listen();
        let output = path.clone();
        let (id, receiver) = app
            .world()
            .resource::<JobManager>()
            .spawn_cpu(JobOptions::default(), move |context| async move {
                listener.await;
                encode(context, handoff, output, tiles).await
            })
            .unwrap();
        let document = app.world_mut().spawn(CanvasDocument).id();
        let entity = app
            .world_mut()
            .spawn(CanvasPngExport {
                id,
                phase: Phase::Writing,
                resources: None,
                tiles,
            })
            .id();
        app.world_mut().resource_mut::<Exports>().0.insert(
            entity,
            Operation {
                id,
                document,
                path,
                bridge: bridge.clone(),
                receiver,
                finished: None,
            },
        );
        pump(&mut app, |world| {
            world.resource::<JobManager>().status(id).unwrap().state == JobState::Running
        });
        bridge.deliver(ExportInput::Failed(CanvasExportError::Render(
            "original GPU failure".into(),
        )));
        app.world().resource::<JobManager>().cancel(id).unwrap();
        poll_jobs(app.world_mut()).unwrap();
        gate.notify(usize::MAX);
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        let notices = app.world().resource::<Notices>();
        assert!(matches!(notices.0[0].outcome, JobOutcome::Failed(_)));
        assert!(
            notices.0[0]
                .result
                .as_ref()
                .unwrap_err()
                .contains("original GPU failure")
        );
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
        assert!(app.world().resource::<Exports>().0.is_empty());
        app.update();
        assert_eq!(app.world().resource::<Notices>().0.len(), 1);
    }

    #[test]
    fn shutdown_rejects_an_export_before_allocating_its_render_resources() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(bevy_widgetry_core::offscreen::WidgetryOffscreenPlugin)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>();
        let manager = bevy_widgetry_job::JobManager::default();
        manager.begin_shutdown(Duration::from_secs(1)).unwrap();
        app.insert_resource(manager);
        let document = app.world_mut().spawn(CanvasDocument).id();
        crate::CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            crate::CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::splat(10.0),
            },
            Transform::default(),
            crate::CanvasPaint::default(),
        )
        .unwrap();
        let snapshot = super::super::snapshot(app.world_mut(), document, None).unwrap();
        let before = app.world().entities().len();
        let tiles = Tiles::new(UVec2::splat(10), snapshot.region, 4096, 1 << 30).unwrap();
        let result = begin(
            app.world_mut(),
            document,
            "unused.png".into(),
            snapshot,
            tiles,
        );
        assert!(matches!(
            result,
            Err(CanvasExportError::Job(
                bevy_widgetry_job::SubmitError::ShuttingDown
            ))
        ));
        assert_eq!(app.world().entities().len(), before);
    }

    #[derive(Resource, Default)]
    struct Notices(Vec<Notice>);

    struct Notice {
        entity: Entity,
        id: JobId,
        result: std::result::Result<UVec2, String>,
        outcome: JobOutcome,
    }

    fn test_app() -> (App, std::sync::mpsc::Sender<(Entity, Image)>) {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins((
            JobManagerPlugin,
            bevy_widgetry_core::offscreen::WidgetryOffscreenPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .init_resource::<Exports>()
        .init_resource::<CleanupQueue>()
        .init_resource::<Notices>()
        .add_message::<bevy::window::RequestRedraw>()
        .add_observer(release)
        .add_observer(
            |event: On<CanvasPngExportFinished>,
             manager: Res<JobManager>,
             mut notices: ResMut<Notices>| {
                let outcome = manager.outcome(event.job_id).unwrap().unwrap();
                if event.result.is_ok() {
                    assert_eq!(image::image_dimensions(&event.path).unwrap(), (10, 10));
                    assert_eq!(outcome, JobOutcome::Completed);
                } else if matches!(event.result, Err(CanvasExportError::Canceled)) {
                    assert_eq!(outcome, JobOutcome::Canceled);
                } else {
                    assert!(matches!(outcome, JobOutcome::Failed(_)));
                }
                notices.0.push(Notice {
                    entity: event.entity,
                    id: event.job_id,
                    result: event.result.as_ref().copied().map_err(ToString::to_string),
                    outcome,
                });
            },
        )
        .add_systems(First, |mut commands: Commands| {
            commands.despawn_all::<With<Captured>>()
        })
        .add_systems(Update, (trigger_screenshots, poll_jobs).chain());
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        let (sender, receiver) = std::sync::mpsc::channel();
        app.insert_resource(CapturedScreenshots(Arc::new(Mutex::new(receiver))));
        (app, sender)
    }

    fn pump(app: &mut App, done: impl Fn(&World) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done(app.world()) {
            assert!(Instant::now() < deadline, "PNG lifecycle did not settle");
            let errors = bevy_widgetry_test_utils::ErrorCapture::default();
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            std::thread::yield_now();
        }
    }

    fn request(app: &mut App, path: PathBuf) -> (Entity, JobId) {
        let (entity, id) = queued(app, path);
        pump(app, |world| {
            world
                .get::<CanvasPngExport>(entity)
                .is_some_and(|job| matches!(job.phase, Phase::Preparing))
        });
        (entity, id)
    }

    fn queued(app: &mut App, path: PathBuf) -> (Entity, JobId) {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            crate::CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::splat(10.0),
            },
            Transform::default(),
            crate::CanvasPaint::default(),
        )
        .unwrap();
        let snapshot = super::super::snapshot(app.world_mut(), document, None).unwrap();
        let tiles = Tiles::new(UVec2::splat(10), snapshot.region, 4096, 1 << 30).unwrap();
        let entity = begin(app.world_mut(), document, path, snapshot, tiles).unwrap();
        let id = app.world().get::<CanvasPngExport>(entity).unwrap().job_id();
        (entity, id)
    }

    #[test]
    fn cancel_running_job_before_gpu_preparation_never_allocates_a_target() {
        let (mut app, _) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let images = app.world().resource::<Assets<Image>>().len();
        let (entity, id) = queued(&mut app, file.path().to_owned());
        let manager = app.world().resource::<JobManager>().clone();
        let deadline = Instant::now() + Duration::from_secs(5);
        while manager.status(id).unwrap().state != JobState::Running {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        manager.cancel(id).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Canceled
        );
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(app.world().resource::<Assets<Image>>().len(), images);
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    }

    #[test]
    fn global_shutdown_during_readback_waits_for_cleanup_before_completion() {
        let (mut app, sender) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let (entity, id) = request(&mut app, file.path().to_owned());
        let capture = capturing(&mut app, entity, id);
        let manager = app.world().resource::<JobManager>().clone();
        let shutdown = manager.begin_shutdown(Duration::from_secs(5)).unwrap();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(manager.status(id).unwrap().state, JobState::Stopping);
        assert!(shutdown.try_recv().is_none());
        sender.send((capture, pixels())).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Canceled
        );
        assert_eq!(
            shutdown.try_recv(),
            Some(bevy_widgetry_job::ShutdownMessage::Completed)
        );
        assert!(app.world().resource::<CleanupQueue>().0.is_empty());
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    }

    fn capturing(app: &mut App, entity: Entity, id: JobId) -> Entity {
        let image = app
            .world()
            .get::<CanvasPngExport>(entity)
            .unwrap()
            .resources
            .as_ref()
            .unwrap()
            .target
            .image()
            .clone();
        let capture = request_capture(app.world_mut(), entity, id, image);
        app.world_mut().entity_mut(capture).insert(Capturing);
        let mut job = app.world_mut().get_mut::<CanvasPngExport>(entity).unwrap();
        job.phase = Phase::Capturing;
        job.resources.as_mut().unwrap().capture = Some(capture);
        capture
    }

    fn pixels() -> Image {
        Image::new_fill(
            Extent3d {
                width: 10,
                height: 10,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[12, 34, 56, 128],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        )
    }

    #[test]
    fn timed_out_export_waits_for_real_readback_and_ignores_the_late_image() {
        let (mut app, sender) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let (entity, id) = request(&mut app, file.path().to_owned());
        let capture = capturing(&mut app, entity, id);
        let scene = {
            let mut job = app.world_mut().get_mut::<CanvasPngExport>(entity).unwrap();
            let resources = job.resources.as_mut().unwrap();
            resources.started = Instant::now() - Duration::from_secs(31);
            resources.scene
        };
        pump(&mut app, |world| {
            world
                .get::<CanvasPngExport>(entity)
                .is_some_and(|job| matches!(job.phase, Phase::Finishing))
        });
        assert!(app.world().get_entity(scene).is_ok());
        assert!(app.world().resource::<Notices>().0.is_empty());
        assert!(
            !app.world()
                .resource::<JobManager>()
                .status(id)
                .unwrap()
                .state
                .is_finished()
        );
        sender.send((capture, pixels())).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert!(app.world().get_entity(entity).is_err());
        assert!(app.world().get_entity(scene).is_err());
        assert!(app.world().resource::<CleanupQueue>().0.is_empty());
        assert_eq!(app.world().resource::<Notices>().0[0].entity, entity);
        assert_eq!(app.world().resource::<Notices>().0[0].id, id);
        assert!(
            app.world().resource::<Notices>().0[0]
                .result
                .as_ref()
                .unwrap_err()
                .contains("Timed out")
        );
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
        app.update();
        assert!(app.world().get_entity(capture).is_err());
        assert_eq!(app.world().resource::<Notices>().0.len(), 1);
    }

    #[test]
    fn cancel_before_capture_releases_resources_and_reports_canceled_once() {
        let (mut app, _) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let (entity, id) = request(&mut app, file.path().to_owned());
        let scene = app
            .world()
            .get::<CanvasPngExport>(entity)
            .unwrap()
            .resources
            .as_ref()
            .unwrap()
            .scene;
        app.world().resource::<JobManager>().cancel(id).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Canceled
        );
        assert!(app.world().get_entity(scene).is_err());
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    }

    #[test]
    fn removed_request_retains_a_submitted_readback_until_delivery_without_writing() {
        let (mut app, sender) = test_app();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let (entity, id) = request(&mut app, file.path().to_owned());
        let capture = capturing(&mut app, entity, id);
        let scene = app
            .world()
            .get::<CanvasPngExport>(entity)
            .unwrap()
            .resources
            .as_ref()
            .unwrap()
            .scene;
        app.world_mut().despawn(entity);
        app.world_mut().flush();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<JobManager>()
                .status(id)
                .unwrap()
                .state,
            JobState::Stopping
        );
        assert!(app.world().get_entity(scene).is_ok());
        assert!(app.world().resource::<Notices>().0.is_empty());
        sender.send((capture, pixels())).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Canceled
        );
        assert!(app.world().get_entity(scene).is_err());
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    }

    #[test]
    fn duplicate_capture_writes_once_and_late_cancel_after_saved_file_remains_completed() {
        let (mut app, _) = test_app();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("saved.png");
        let (entity, id) = request(&mut app, path.clone());
        let capture = capturing(&mut app, entity, id);
        app.world_mut().entity_mut(capture).insert(Captured);
        for image in [pixels(), Image::default()] {
            app.world_mut().trigger(ScreenshotCaptured {
                entity: capture,
                image,
            });
            app.world_mut().flush();
        }
        let bridge = app.world().resource::<Exports>().0[&entity].bridge.clone();
        let deadline = Instant::now() + Duration::from_secs(5);
        while bridge.lock().result.is_none() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(image::image_dimensions(&path).unwrap(), (10, 10));
        app.world().resource::<JobManager>().cancel(id).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        assert_eq!(
            app.world().resource::<Notices>().0[0].outcome,
            JobOutcome::Completed
        );
        assert_eq!(
            app.world().resource::<Notices>().0[0].result,
            Ok(UVec2::splat(10))
        );
        assert!(app.world().resource::<Exports>().0.is_empty());
    }

    #[test]
    fn captured_image_disk_error_keeps_the_business_error_and_failed_job_identity() {
        let (mut app, sender) = test_app();
        let directory = tempfile::tempdir().unwrap();
        let (entity, id) = request(&mut app, directory.path().to_owned());
        let capture = capturing(&mut app, entity, id);
        sender.send((capture, pixels())).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        let notices = &app.world().resource::<Notices>().0;
        assert!(notices[0].result.as_ref().unwrap_err().contains("I/O"));
        assert_eq!(notices[0].id, id);
        assert!(matches!(notices[0].outcome, JobOutcome::Failed(_)));
    }

    #[test]
    fn reactive_redraw_continues_through_readback_and_stops_after_notification() {
        let (mut app, sender) = test_app();
        app.add_systems(Last, redraw_exports);
        let directory = tempfile::tempdir().unwrap();
        let (entity, id) = request(&mut app, directory.path().join("redraw.png"));
        let capture = capturing(&mut app, entity, id);
        app.world_mut()
            .resource_mut::<Messages<bevy::window::RequestRedraw>>()
            .clear();
        app.update();
        assert!(
            !app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
        sender.send((capture, pixels())).unwrap();
        pump(&mut app, |world| world.resource::<Notices>().0.len() == 1);
        app.world_mut()
            .resource_mut::<Messages<bevy::window::RequestRedraw>>()
            .clear();
        app.update();
        assert!(
            app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
    }

    #[test]
    fn png_dimensions_round_up_independently_and_validate_gpu_and_memory_limits() {
        let region = Rect::from_corners(Vec2::new(-65.0, -45.0), Vec2::new(65.0, 45.0));
        for scale in [1.0, 2.0, 4.0] {
            assert_eq!(
                dimensions(region, scale, 16384, 1 << 30).unwrap(),
                UVec2::new((130.0 * scale) as u32, (90.0 * scale) as u32)
            );
        }
        assert_eq!(
            dimensions(
                Rect::from_corners(Vec2::ZERO, Vec2::new(13.25, 9.1)),
                2.0,
                16384,
                1 << 30
            )
            .unwrap(),
            UVec2::new(27, 19)
        );
        for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                dimensions(region, scale, 16384, 1 << 30),
                Err(CanvasExportError::InvalidScale)
            ));
        }
        assert!(dimensions(region, 1000.0, 16384, 1 << 30).is_err());
        assert_eq!(
            dimensions(region, 1.0, 64, 1 << 30).unwrap(),
            UVec2::new(130, 90)
        );
        assert!(dimensions(region, 1.0, 4, 1 << 30).is_err());
        assert!(dimensions(region, 1.0, 16384, 128).is_err());
        assert!(
            dimensions(
                Rect::from_corners(Vec2::ZERO, Vec2::splat(4_294_967_040.0)),
                1.0,
                u32::MAX,
                u64::MAX
            )
            .is_err()
        );
    }
}
