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
    size: UVec2,
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
        self.lock().input = Some(input);
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
    size: UVec2,
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
            move |context| encode(context, handoff, output, size),
        )
        .map_err(CanvasExportError::Job)?;
    let entity = world
        .spawn(CanvasPngExport {
            id,
            phase: Phase::Queued(snapshot),
            resources: None,
        })
        .id();
    world.resource_mut::<Exports>().0.insert(
        entity,
        Operation {
            id,
            document,
            path,
            size,
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
    size: UVec2,
) -> std::result::Result<(), String> {
    let result = match bridge.input().await {
        ExportInput::Image(_) if context.is_cancelled() => Err(CanvasExportError::Canceled),
        ExportInput::Image(image) => encode_image(image, &path).map(|()| size),
        ExportInput::Failed(error) => Err(error),
        ExportInput::Canceled => Err(CanvasExportError::Canceled),
    };
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

fn encode_image(image: Image, path: &Path) -> std::result::Result<(), CanvasExportError> {
    let image = image
        .try_into_dynamic()
        .map_err(|error| CanvasExportError::Render(error.to_string()))?;
    image
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|error| match error {
            image::ImageError::IoError(error) => CanvasExportError::Io(error),
            error => CanvasExportError::Encoding(error),
        })
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
        let (fill_mesh, stroke_mesh) = crate::geometry::meshes(
            &crate::CanvasGeometry::Rectangle(shape.rectangle),
            shape.paint,
        )
        .map_err(|error| CanvasExportError::InvalidGeometry(error.to_string()))?;
        if let Some(color) = shape.paint.fill {
            paints.push((shape, color, fill_mesh));
        }
        if let Some(stroke) = shape.paint.stroke {
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
        let (id, size, bridge) = (operation.id, operation.size, operation.bridge.clone());
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
                let Phase::Queued(snapshot) = std::mem::replace(&mut job.phase, Phase::Preparing)
                else {
                    continue;
                };
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
                    && (preparing || capturing && !capture_busy)
                {
                    fail_export(world, entity, &bridge, ExportInput::Canceled);
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
        let result = begin(
            app.world_mut(),
            document,
            "unused.png".into(),
            snapshot,
            UVec2::splat(10),
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
        let entity = begin(app.world_mut(), document, path, snapshot, UVec2::splat(10)).unwrap();
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
