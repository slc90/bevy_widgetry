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
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};
use bevy_widgetry_core::{offscreen::OffscreenTarget, scene::spawn_scene};
use bevy_widgetry_internal_log::widgetry_error;
use std::path::{Path, PathBuf};
use std::{
    sync::{
        Arc,
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
            begin(world, document, path.to_owned(), snapshot, size)
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
        || w > f64::from(max_texture)
        || h > f64::from(max_texture)
    {
        return Err(CanvasExportError::InvalidResolution);
    }
    let size = UVec2::new(w as u32, h as u32);
    let pixels = u64::from(size.x) * u64::from(size.y);
    let padded = ((u64::from(size.x) * 4).div_ceil(256) * 256).saturating_mul(u64::from(size.y));
    // Include 4x MSAA, resolve/postprocess targets, GPU readback and CPU encoding copies.
    if padded > max_buffer || pixels.saturating_mul(80).saturating_add(padded) > 512 * 1024 * 1024 {
        return Err(CanvasExportError::InvalidResolution);
    }
    Ok(size)
}

enum Phase {
    Preparing,
    Capturing,
    Writing(Task<std::result::Result<(), CanvasExportError>>),
}

#[derive(Component)]
struct Job {
    document: Entity,
    path: PathBuf,
    size: UVec2,
    target: Option<OffscreenTarget>,
    scene: Entity,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<ColorMaterial>>,
    ready: Arc<AtomicBool>,
    started: Instant,
    phase: Phase,
    capture: Option<Entity>,
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
    app.insert_resource(PngRenderer)
        .add_plugins(ExtractComponentPlugin::<Probe>::default())
        .add_observer(release)
        .add_systems(Update, poll_jobs);
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
    size: UVec2,
) -> std::result::Result<Entity, CanvasExportError> {
    let target =
        OffscreenTarget::allocate(world).map_err(|e| CanvasExportError::Render(e.to_string()))?;
    let layer = target.layer();
    let image_handle = target.image().clone();
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
    if let Err(error) = world
        .resource_mut::<Assets<Image>>()
        .insert(image_handle.id(), image)
    {
        target.release(world);
        return Err(CanvasExportError::Render(error.to_string()));
    }
    let region = snapshot.region;
    let center = region.min + region.size() * 0.5;
    let scene = match spawn_scene(
        world,
        bsn! {
            Name("CanvasPngScene") Parts {camera:#Camera,content:#Content} Transform::default() Visibility::default()
            Children [
                #Camera Name("CanvasPngCamera") Camera2d
                Camera {order:-1,clear_color:ClearColorConfig::Custom(Color::NONE)}
                Transform::from_xyz(center.x,-center.y,0.0) Msaa::Sample4 crate::alpha::StraightAlpha::default()
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
    let entity = world
        .spawn(Job {
            document,
            path,
            size,
            target: Some(target),
            scene,
            meshes: Vec::new(),
            materials: Vec::new(),
            ready: ready.clone(),
            started: Instant::now(),
            phase: Phase::Preparing,
            capture: None,
        })
        .id();
    if let Err(error) = geometry(world, entity, content, layer, &snapshot) {
        world.despawn(entity);
        world.flush();
        return Err(error);
    }
    let job = world
        .get::<Job>(entity)
        .ok_or_else(|| CanvasExportError::Render("Export job missing".to_owned()))?;
    let probe = Probe {
        meshes: job.meshes.iter().map(Handle::id).collect(),
        materials: job.materials.iter().map(Handle::id).collect(),
        image: job
            .target
            .as_ref()
            .ok_or_else(|| CanvasExportError::Render("Export target missing".to_owned()))?
            .image()
            .id(),
        ready,
    };
    world.entity_mut(camera).insert(probe);
    Ok(entity)
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
        if let Some(color) = shape.paint.fill {
            paints.push((shape.transform.translation.z, shape, color, None));
        }
        if let Some(stroke) = shape.paint.stroke {
            paints.push((
                shape.transform.translation.z + 0.01,
                shape,
                stroke.color,
                Some(stroke.width),
            ));
        }
    }
    paints.sort_by(|a, b| a.0.total_cmp(&b.0));
    let count = paints.len().max(1) as f32;
    for (index, (_, shape, color, width)) in paints.into_iter().enumerate() {
        let mesh = match width {
            Some(width) => crate::geometry::stroke(shape.rectangle.size, width),
            None => crate::geometry::fill(shape.rectangle.size),
        };
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let material = world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial { color, ..default() });
        let mut owner = world
            .get_mut::<Job>(job)
            .ok_or_else(|| CanvasExportError::Render("Export job missing".to_owned()))?;
        owner.meshes.push(mesh.clone());
        owner.materials.push(material.clone());
        let mut transform = shape.transform;
        transform.scale.z = 1.0;
        // Model Z is paint order, not a depth that may clip geometry out of an export camera.
        transform.translation.z = index as f32 / count * 100.0;
        spawn_scene(
            world,
            bsn! {
                Name("CanvasPngPaint") template(move |_|Ok(ChildOf(content)))
                template(move |_|Ok(Mesh2d(mesh.clone())))
                template(move |_|Ok(MeshMaterial2d(material.clone())))
                template(move |_|Ok(RenderLayers::none().with(layer)))
                template(move |_|Ok(transform)) Visibility::default() Pickable::IGNORE
            },
        )
        .map_err(|error| CanvasExportError::Render(error.to_string()))?;
    }
    Ok(())
}

fn poll_jobs(world: &mut World) {
    let jobs = world
        .query_filtered::<Entity, With<Job>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in jobs {
        let Some(mut job) = world.get_mut::<Job>(entity) else {
            continue;
        };
        let timeout = job.started.elapsed() > Duration::from_secs(30);
        let ready = job.ready.load(Ordering::Acquire);
        let mut result = None;
        let mut capture = None;
        match &mut job.phase {
            Phase::Writing(task) => {
                result = block_on(future::poll_once(task));
            }
            Phase::Preparing if ready => {
                capture = job.target.as_ref().map(|target| target.image().clone());
                job.phase = Phase::Capturing;
            }
            _ if timeout => {
                result = Some(Err(CanvasExportError::Render(
                    "Timed out waiting for export GPU rendering/readback".to_owned(),
                )));
            }
            _ => {}
        }
        if let Some(image) = capture {
            let capture = request_capture(world, entity, image);
            if let Some(mut job) = world.get_mut::<Job>(entity) {
                job.capture = Some(capture);
            }
        }
        if let Some(result) = result {
            let Some(job) = world.get::<Job>(entity) else {
                continue;
            };
            let (document, path, size) = (job.document, job.path.clone(), job.size);
            if let Err(error) = &result {
                widgetry_error!(?entity,?document,?path,%error,"Canvas PNG 导出失败");
            }
            world.trigger(CanvasPngExportFinished {
                entity,
                document,
                path,
                result: result.map(|()| size),
            });
            if world.get_entity(entity).is_ok() {
                world.despawn(entity);
            }
        }
    }
}

fn request_capture(world: &mut World, entity: Entity, image: Handle<Image>) -> Entity {
    world
        .spawn(Screenshot::image(image))
        .observe(
            move |event: On<ScreenshotCaptured>, mut commands: Commands| {
                let image = event.image.clone();
                commands.queue(move |world: &mut World| {
                    let Some(mut job) = world.get_mut::<Job>(entity) else {
                        return;
                    };
                    let path = job.path.clone();
                    job.phase = Phase::Writing(AsyncComputeTaskPool::get().spawn(async move {
                        let image = image
                            .try_into_dynamic()
                            .map_err(|error| CanvasExportError::Render(error.to_string()))?;
                        image
                            .save_with_format(path, image::ImageFormat::Png)
                            .map_err(|error| match error {
                                image::ImageError::IoError(error) => CanvasExportError::Io(error),
                                error => CanvasExportError::Encoding(error),
                            })
                    }));
                });
            },
        )
        .id()
}

fn release(event: On<Discard<Job>>, mut jobs: Query<&mut Job>, mut commands: Commands) {
    let Ok(mut job) = jobs.get_mut(event.entity) else {
        return;
    };
    let target = job.target.take();
    let scene = job.scene;
    let capture = job.capture;
    let meshes = std::mem::take(&mut job.meshes);
    let materials = std::mem::take(&mut job.materials);
    commands.queue(move |world: &mut World| {
        if let Some(target) = target {
            target.release(world);
        }
        if world.get_entity(scene).is_ok() {
            world.despawn(scene);
        }
        if let Some(capture) = capture
            && world.get_entity(capture).is_ok()
            // Bevy owns submitted readbacks; its detached task still targets this entity.
            && world.get::<bevy::render::view::screenshot::Capturing>(capture).is_none()
        {
            world.despawn(capture);
        }
        for mesh in meshes {
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
        }
        for material in materials {
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
        }
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::render::view::screenshot::{
        Captured, CapturedScreenshots, Capturing, trigger_screenshots,
    };

    #[test]
    fn timed_out_export_allows_late_screenshot_delivery_without_writing_or_notifying_again() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_observer(release)
            .add_systems(First, |mut commands: Commands| {
                commands.despawn_all::<With<Captured>>()
            })
            .add_systems(Update, trigger_screenshots);
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        let (sender, receiver) = std::sync::mpsc::channel();
        app.insert_resource(CapturedScreenshots(Arc::new(std::sync::Mutex::new(
            receiver,
        ))));
        let notices = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = notices.clone();
        app.add_observer(move |event: On<CanvasPngExportFinished>| {
            assert!(matches!(event.result, Err(CanvasExportError::Render(_))));
            seen.fetch_add(1, Ordering::Relaxed);
        });
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let document = app.world_mut().spawn(CanvasDocument).id();
        let scene = app.world_mut().spawn_empty().id();
        let job = app
            .world_mut()
            .spawn(Job {
                document,
                path: file.path().to_owned(),
                size: UVec2::ONE,
                target: None,
                scene,
                meshes: Vec::new(),
                materials: Vec::new(),
                ready: Arc::new(AtomicBool::new(false)),
                started: Instant::now() - Duration::from_secs(31),
                phase: Phase::Capturing,
                capture: None,
            })
            .id();
        let capture = request_capture(app.world_mut(), job, Handle::default());
        app.world_mut().entity_mut(capture).insert(Capturing);
        app.world_mut().get_mut::<Job>(job).unwrap().capture = Some(capture);
        poll_jobs(app.world_mut());
        app.world_mut().flush();
        assert_eq!(notices.load(Ordering::Relaxed), 1);
        assert!(app.world().get_entity(job).is_err());
        assert!(app.world().get_entity(scene).is_err());
        assert!(app.world().get_entity(capture).is_ok());
        sender.send((capture, Image::default())).unwrap();
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        errors.run(|| {
            app.update();
            app.update();
        });
        assert!(errors.take().is_empty());
        assert!(app.world().get_entity(capture).is_err());
        assert_eq!(notices.load(Ordering::Relaxed), 1);
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
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
        assert!(dimensions(region, 1.0, 64, 1 << 30).is_err());
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
