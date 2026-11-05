use crate::error::CanvasFailureState;
use crate::error::CanvasResult as RawResult;
use crate::view::ViewState;
use crate::{
    CanvasDocument, CanvasGeometry, CanvasPaint, CanvasRectangle, CanvasShape, CanvasView,
};
use bevy::camera::{CameraProjection, ScalingMode, primitives::Aabb, visibility::RenderLayers};
use bevy::prelude::*;
use bevy_widgetry_core::offscreen::{
    OffscreenTarget, OffscreenUpdate, OffscreenViewport, WidgetryOffscreenPlugin,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, WidgetryEcsError},
};
use bevy_widgetry_internal_log::widgetry_debug;
use std::collections::HashMap;
mod nodes;
mod outline;
mod pen;
mod preview;

pub struct CanvasPlugin;

#[derive(Component, Default)]
pub(crate) struct RenderFailure {
    initialize: CanvasFailureState,
    geometry: CanvasFailureState,
    view: CanvasFailureState,
    outline: CanvasFailureState,
    preview: CanvasFailureState,
    pen: CanvasFailureState,
    nodes: CanvasFailureState,
    shapes: HashMap<Entity, CanvasFailureState>,
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Initialize,
    Geometry,
    View,
    Outline,
    Preview,
    Pen,
    Nodes,
}

#[derive(Component)]
struct Renderer {
    camera: Entity,
    content: Entity,
    layer: usize,
    proxies: HashMap<Entity, Entity>,
    outline: Option<Entity>,
    preview: Option<Entity>,
    pen: Option<Entity>,
    nodes: Option<Entity>,
}

#[derive(Component, FromTemplate)]
struct SceneParts {
    camera: Entity,
    content: Entity,
}

#[derive(Component, FromTemplate)]
struct ProxyParts {
    fill: Entity,
    stroke: Entity,
}

#[derive(Component, Clone)]
struct ShapeRender {
    shape: Entity,
    fill_material: Handle<ColorMaterial>,
    stroke_material: Handle<ColorMaterial>,
}

fn initialize(world: &mut World) -> bevy::prelude::Result {
    let roots = world
        .query_filtered::<Entity, (With<CanvasView>, Without<Renderer>)>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = Vec::new();
    let mut diagnostics = DiagnosticBatch::default();
    for root in roots {
        let result = initialize_root(world, root);
        if let Err(error) = observe(world, root, Stage::Initialize, result) {
            collect_error(error, &mut failure, &mut diagnostics);
        }
    }
    finish_errors(world, failure, diagnostics)
}

fn observe<T>(
    world: &mut World,
    root: Entity,
    stage: Stage,
    result: RawResult<T>,
) -> std::result::Result<T, BevyError> {
    let frame = ErrorFrame::new("canvas.renderer")
        .field("view", root)
        .field("stage", format_args!("{stage:?}"));
    if let Some(mut failure) = world.get_mut::<RenderFailure>(root) {
        let state = match stage {
            Stage::Initialize => &mut failure.initialize,
            Stage::Geometry => &mut failure.geometry,
            Stage::View => &mut failure.view,
            Stage::Outline => &mut failure.outline,
            Stage::Preview => &mut failure.preview,
            Stage::Pen => &mut failure.pen,
            Stage::Nodes => &mut failure.nodes,
        };
        state.observe(result.map_err(|error| error.at("view", root)), frame)
    } else {
        result.map_err(|error| error.into_bevy_error(frame))
    }
}

fn initialize_root(world: &mut World, root: Entity) -> RawResult {
    let target = OffscreenTarget::allocate(world, root).map_err(|error| {
        error
            .with_context(ErrorFrame::new("canvas.allocate").field("root", root))
            .into_bevy_error()
    })?;
    let layer = target.layer();
    let scene = spawn_scene(
        world,
        bsn! {
            SceneParts { camera: #Camera, content: #Content }
            Transform::default()
            Visibility::default()
            Children [
                #Camera Name("CanvasCamera") Camera2d
                Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(Color::NONE) }
                Msaa::Sample4
            crate::alpha::StraightAlpha::default()
                template(move |_| Ok(RenderLayers::none().with(layer)))
                template(|_| Ok(Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: 1.0, height: 1.0 }, ..OrthographicProjection::default_2d() })))
            --
                #Content Name("CanvasContent")
                Transform::from_scale(Vec3::new(1.0, -1.0, 1.0))
                Visibility::default()
            ]
        },
    );
    let scene = match scene {
        Ok(scene) => scene,
        Err(error) => {
            if let Err(cleanup) = target.release(world) {
                world
                    .commands()
                    .queue(move |_: &mut World| -> bevy::prelude::Result {
                        Err(cleanup.into_bevy_error())
                    });
            }
            return Err(error.into_bevy_error().into());
        }
    };
    let Some(parts) = world.get::<SceneParts>(scene) else {
        if let Err(cleanup) = target.release(world) {
            world
                .commands()
                .queue(move |_: &mut World| -> bevy::prelude::Result {
                    Err(cleanup.into_bevy_error())
                });
        }
        world.despawn(scene);
        return Err(crate::error::missing(crate::CanvasPart::SceneParts));
    };
    let (camera, content) = (parts.camera, parts.content);
    let viewport = target.bind(world, root, camera, scene).map_err(|error| {
        error
            .with_context(ErrorFrame::new("canvas.bind").field("root", root))
            .into_bevy_error()
    })?;
    world.entity_mut(root).insert((
        viewport,
        Renderer {
            camera,
            content,
            layer,
            proxies: HashMap::new(),
            outline: None,
            preview: None,
            pen: None,
            nodes: None,
        },
    ));
    Ok(())
}

fn render(world: &mut World) -> bevy::prelude::Result {
    let roots = world
        .query_filtered::<Entity, With<Renderer>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = Vec::new();
    let mut diagnostics = DiagnosticBatch::default();
    for root in roots {
        let result = render_root(world, root);
        if matches!(result, Ok(None)) {
            continue;
        }
        if let Err(error) = observe(world, root, Stage::View, result.map(|_| ())) {
            collect_error(error, &mut failure, &mut diagnostics);
        }
    }
    finish_errors(world, failure, diagnostics)
}

fn render_root(world: &mut World, root: Entity) -> RawResult<Option<()>> {
    let Some(size) = OffscreenViewport::update(world, root).map_err(|error| {
        error
            .with_context(ErrorFrame::new("canvas.update").field("root", root))
            .into_bevy_error()
    })?
    else {
        return Ok(None);
    };
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?;
    let camera = renderer.camera;
    let node = world
        .get::<ComputedNode>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Layout))?;
    let logical_size = node.size() * node.inverse_scale_factor() / CanvasView::_zoom(world, root)?;
    let mut projection = world
        .get_mut::<Projection>(camera)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Projection))?;
    if let Projection::Orthographic(projection) = &mut *projection {
        let mode = ScalingMode::Fixed {
            width: logical_size.x,
            height: logical_size.y,
        };
        if !matches!(&projection.scaling_mode, ScalingMode::Fixed { width, height } if *width == logical_size.x && *height == logical_size.y)
        {
            projection.scaling_mode = mode;
        }
        // CameraUpdateSystems precedes UI layout; update the current frame's projection explicitly.
        projection.update(size.x as f32, size.y as f32);
    }
    let clip_from_view = projection.get_clip_from_view();
    world
        .get_mut::<Camera>(camera)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Camera))?
        .computed
        .clip_from_view = clip_from_view;
    Ok(Some(()))
}

fn prepare(world: &mut World) -> bevy::prelude::Result {
    let roots = world
        .query_filtered::<Entity, With<Renderer>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = Vec::new();
    let mut diagnostics = DiagnosticBatch::default();
    for root in roots {
        if world.get::<Renderer>(root).is_none() {
            continue;
        }
        let result = prepare_root(world, root);
        if let Err(error) = observe(world, root, Stage::Geometry, result) {
            collect_error(error, &mut failure, &mut diagnostics);
        }
    }
    let used = world
        .query::<&ShapeRender>()
        .iter(world)
        .map(|proxy| proxy.shape)
        .collect();
    crate::mesh_cache::retain(world, &used);
    finish_errors(world, failure, diagnostics)
}

fn prepare_root(world: &mut World, root: Entity) -> RawResult {
    if crate::selection::repair(world, root)? {
        world.flush();
        if world.get::<Renderer>(root).is_none() {
            return Ok(());
        }
    }
    let document = CanvasView::_document(world, root)?;
    if document.is_some_and(|document| world.get::<CanvasDocument>(document).is_none()) {
        CanvasView::_set_document_in_world(world, root, None)?;
        world.flush();
        if world.get::<Renderer>(root).is_none() {
            return Ok(());
        }
    }
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?;
    let (content, layer) = (renderer.content, renderer.layer);
    let camera = renderer.camera;
    let center = CanvasView::_center(world, root)?;
    let mut transform = world
        .get_mut::<Transform>(camera)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::CameraTransform))?;
    let translation = Vec3::new(center.x, -center.y, transform.translation.z);
    if transform.translation != translation {
        transform.translation = translation;
    }
    let document = CanvasView::_document(world, root)?;
    let shapes = if let Some(document) =
        document.filter(|document| world.get::<CanvasDocument>(*document).is_some())
    {
        CanvasDocument::_shape_order(world, document)?
            .into_iter()
            .map(|entity| {
                let geometry = world
                    .get::<CanvasGeometry>(entity)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry))?;
                let transform = world
                    .get::<Transform>(entity)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Transform))?;
                let paint = world
                    .get::<CanvasPaint>(entity)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Paint))?;
                Ok((entity, geometry.clone(), *transform, *paint))
            })
            .collect::<RawResult<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let stale = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
        .proxies
        .iter()
        .filter(|(shape, _)| !shapes.iter().any(|(entity, ..)| entity == *shape))
        .map(|(shape, proxy)| (*shape, *proxy))
        .collect::<Vec<_>>();
    for (shape, proxy) in stale {
        world.despawn(proxy);
        world
            .get_mut::<Renderer>(root)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
            .proxies
            .remove(&shape);
    }
    let spacing = 100.0 / shapes.len().max(1) as f32;
    observe(world, root, Stage::Geometry, Ok(()))?;
    let active = shapes
        .iter()
        .map(|(shape, ..)| *shape)
        .collect::<std::collections::HashSet<_>>();
    if let Some(mut failure) = world.get_mut::<RenderFailure>(root) {
        failure.shapes.retain(|shape, _| active.contains(shape));
    }
    let mut failure = Vec::new();
    let mut diagnostics = DiagnosticBatch::default();
    for (index, (shape, geometry, transform, paint)) in shapes.into_iter().enumerate() {
        let result = prepare_shape(
            world, root, content, layer, index, spacing, shape, geometry, transform, paint,
        )
        .map_err(|error| {
            let error = error.at("view", root).at("shape", shape);
            match document {
                Some(document) => error.at("document", document),
                None => error,
            }
        });
        let frame = ErrorFrame::new("canvas.renderer.shape")
            .field("view", root)
            .field("shape", shape);
        let result = if let Some(mut states) = world.get_mut::<RenderFailure>(root) {
            states
                .shapes
                .entry(shape)
                .or_default()
                .observe(result, frame)
        } else {
            result.map_err(|error| error.into_bevy_error(frame))
        };
        if let Err(error) = result {
            collect_error(error, &mut failure, &mut diagnostics);
        }
    }

    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
        .outline;
    let result = outline::sync(world, root, content, layer, current);
    match observe(world, root, Stage::Outline, result) {
        Ok(value) => {
            if let Some(mut renderer) = world.get_mut::<Renderer>(root) {
                renderer.outline = value;
            }
        }
        Err(error) => collect_error(error, &mut failure, &mut diagnostics),
    }
    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
        .preview;
    let result = preview::sync(world, root, content, layer, current);
    match observe(world, root, Stage::Preview, result) {
        Ok(value) => {
            if let Some(mut renderer) = world.get_mut::<Renderer>(root) {
                renderer.preview = value;
            }
        }
        Err(error) => collect_error(error, &mut failure, &mut diagnostics),
    }
    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
        .pen;
    let result = pen::sync(world, root, content, layer, current);
    match observe(world, root, Stage::Pen, result) {
        Ok(value) => {
            if let Some(mut renderer) = world.get_mut::<Renderer>(root) {
                renderer.pen = value;
            }
        }
        Err(error) => collect_error(error, &mut failure, &mut diagnostics),
    }
    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
        .nodes;
    let result = nodes::sync(world, root, content, layer, current);
    match observe(world, root, Stage::Nodes, result) {
        Ok(value) => {
            if let Some(mut renderer) = world.get_mut::<Renderer>(root) {
                renderer.nodes = value;
            }
        }
        Err(error) => collect_error(error, &mut failure, &mut diagnostics),
    }
    finish_errors(world, failure, diagnostics).map_err(crate::error::CanvasFault::from)
}

fn prepare_shape(
    world: &mut World,
    root: Entity,
    content: Entity,
    layer: usize,
    index: usize,
    spacing: f32,
    shape: Entity,
    geometry: CanvasGeometry,
    transform: Transform,
    paint: CanvasPaint,
) -> RawResult {
    geometry._validate(&transform, &paint)?;
    let handles = crate::mesh_cache::geometry(world, shape, &geometry, paint)?;
    let proxy = world
        .get::<Renderer>(root)
        .and_then(|renderer| renderer.proxies.get(&shape))
        .copied();
    let proxy = if let Some(proxy) = proxy {
        proxy
    } else {
        let proxy = create_proxy(world, content, layer, shape, paint, &handles)?;
        world
            .get_mut::<Renderer>(root)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::Renderer))?
            .proxies
            .insert(shape, proxy);
        proxy
    };
    let mut projected = transform;
    projected.translation.z = index as f32 * spacing;
    update_proxy(world, proxy, projected, paint, &handles)?;
    let stroke = world
        .get::<ProxyParts>(proxy)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::ProxyParts))?
        .stroke;
    world
        .get_mut::<Transform>(stroke)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::StrokeTransform))?
        .set_if_neq(Transform::from_xyz(0.0, 0.0, spacing * 0.5));

    Ok(())
}
fn create_proxy(
    world: &mut World,
    content: Entity,
    layer: usize,
    shape: Entity,
    paint: CanvasPaint,
    handles: &crate::mesh_cache::ShapeMeshes,
) -> RawResult<Entity> {
    let assets = ShapeRender {
        shape,
        fill_material: world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial {
                color: paint.fill.unwrap_or(Color::NONE),
                ..default()
            }),
        stroke_material: world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial {
                color: paint.stroke.map_or(Color::NONE, |stroke| stroke.color),
                ..default()
            }),
    };
    let (fill, stroke) = (handles.fill.clone(), handles.stroke.clone());
    let (fill_material, stroke_material) =
        (assets.fill_material.clone(), assets.stroke_material.clone());
    let result = spawn_scene(
        world,
        bsn! {
            Name("CanvasShapeProxy") ProxyParts { fill: #Fill, stroke: #Stroke }
            Transform::default() Visibility::default()
            template(move |_| Ok(ChildOf(content)))
            Children [
                #Fill Name("CanvasFill")
                template(move |_| Ok(Mesh2d(fill.clone())))
                template(move |_| Ok(MeshMaterial2d(fill_material.clone())))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Pickable::IGNORE Transform::default() Visibility::default()
            --
                #Stroke Name("CanvasStroke")
                template(move |_| Ok(Mesh2d(stroke.clone())))
                template(move |_| Ok(MeshMaterial2d(stroke_material.clone())))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Pickable::IGNORE Transform::from_xyz(0.0, 0.0, 0.01) Visibility::default()
            ]
        },
    );
    match result {
        Ok(proxy) => {
            if matches!(
                world.get::<CanvasGeometry>(shape),
                Some(CanvasGeometry::Path(_))
            ) {
                let parts = world
                    .get::<ProxyParts>(proxy)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::ProxyParts))?;
                let parts = [parts.fill, parts.stroke];
                // A distant subpath can round the shared AABB away from a nearby visible contour.
                // Path meshes rely on GPU clipping within the dedicated View render layer.
                for part in parts {
                    world
                        .entity_mut(part)
                        .insert(bevy::camera::visibility::NoFrustumCulling);
                }
            }
            world.entity_mut(proxy).insert(assets);
            Ok(proxy)
        }
        Err(error) => {
            release_assets(world, &assets);
            Err(error.into_bevy_error().into())
        }
    }
}

fn update_proxy(
    world: &mut World,
    proxy: Entity,
    transform: Transform,
    paint: CanvasPaint,
    handles: &crate::mesh_cache::ShapeMeshes,
) -> RawResult {
    let old = world
        .get::<ShapeRender>(proxy)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::ProxyAssets))?
        .clone();
    let parts = world
        .get::<ProxyParts>(proxy)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::ProxyParts))?;
    let (fill, stroke) = (parts.fill, parts.stroke);
    for (entity, color, handle, bounds, region) in [
        (
            fill,
            paint.fill,
            &old.fill_material,
            handles.fill_bounds,
            handles.fill_region,
        ),
        (
            stroke,
            paint.stroke.map(|stroke| stroke.color),
            &old.stroke_material,
            handles.stroke_bounds,
            handles.stroke_region,
        ),
    ] {
        {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            let mut material = materials
                .get_mut(handle)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::Material))?;
            if material.color != color.unwrap_or(Color::NONE) {
                material.color = color.unwrap_or(Color::NONE);
            }
        }
        // CPU-only empty updates do not replace a previously uploaded GPU mesh.
        set_mesh_visibility(world, entity, color.is_some() && region.is_some())?;
        let aabb = bounds;
        if world.get::<Aabb>(entity) != Some(&aabb) {
            world.entity_mut(entity).insert(aabb);
        }
    }
    let mut projected_transform = transform;
    // Model Z scale has no planar geometry meaning and must not reverse or collapse paint ordering.
    projected_transform.scale.z = 1.0;
    world
        .get_mut::<Transform>(proxy)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::ProxyTransform))?
        .set_if_neq(projected_transform);
    Ok(())
}

fn set_mesh_visibility(world: &mut World, entity: Entity, visible: bool) -> RawResult {
    world
        .get_mut::<Visibility>(entity)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::MeshVisibility))?
        .set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    Ok(())
}

fn release_assets(world: &mut World, assets: &ShapeRender) {
    world
        .resource_mut::<Assets<ColorMaterial>>()
        .remove(assets.fill_material.id());
    world
        .resource_mut::<Assets<ColorMaterial>>()
        .remove(assets.stroke_material.id());
}

fn release_shape(
    event: On<Discard<ShapeRender>>,
    shapes: Query<&ShapeRender>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(assets) = shapes.get(event.entity) {
        materials.remove(assets.fill_material.id());
        materials.remove(assets.stroke_material.id());
    }
}

fn release_renderer(event: On<Remove<Renderer>>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .try_remove::<OffscreenViewport>();
}

impl Plugin for CanvasPlugin {
    fn build(&self, app: &mut App) {
        crate::alpha::install(app);
        #[cfg(feature = "png_export")]
        crate::export::install(app);
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryOffscreenPlugin>() {
            app.add_plugins(WidgetryOffscreenPlugin);
        }
        crate::input::install(app);
        crate::pen::install(app);
        outline::install(app);
        preview::install(app);
        pen::install(app);
        nodes::install(app);
        if !app.world().contains_resource::<Assets<Mesh>>() {
            app.init_asset::<Mesh>();
        }
        if !app.world().contains_resource::<Assets<ColorMaterial>>() {
            app.init_asset::<ColorMaterial>();
        }
        app.init_resource::<crate::mesh_cache::MeshCache>();
        app.register_type::<CanvasDocument>()
            .register_type::<CanvasShape>()
            .register_type::<CanvasGeometry>()
            .register_type::<CanvasRectangle>()
            .register_type::<crate::CanvasEllipse>()
            .register_type::<crate::CanvasPath>()
            .register_type::<crate::CanvasPathCommand>()
            .register_type::<CanvasPaint>()
            .register_type::<ViewState>()
            .add_observer(release_shape)
            .add_observer(release_renderer)
            .add_systems(
                PostUpdate,
                initialize
                    .in_set(WidgetryUiSystems::Build)
                    .before(bevy::camera::CameraUpdateSystems),
            )
            .add_systems(
                PostUpdate,
                prepare
                    .in_set(WidgetryUiSystems::Build)
                    .after(initialize)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                render
                    .in_set(OffscreenUpdate)
                    .before(bevy::camera::visibility::VisibilitySystems::UpdateFrusta),
            );
        widgetry_debug!("CanvasPlugin 注册完成");
    }
}

fn collect_error(error: BevyError, native: &mut Vec<BevyError>, diagnostics: &mut DiagnosticBatch) {
    if error.severity() == bevy::ecs::error::Severity::Error
        && let Some(managed) = error.downcast_ref::<WidgetryEcsError>()
    {
        for diagnostic in managed.shared_diagnostics() {
            diagnostics.push_shared(diagnostic);
        }
    } else {
        native.push(error);
    }
}

fn finish_errors(
    world: &mut World,
    mut native: Vec<BevyError>,
    diagnostics: DiagnosticBatch,
) -> bevy::prelude::Result {
    let error = diagnostics.into_bevy_error().or_else(|| native.pop());
    for error in native {
        world
            .commands()
            .queue(move |_: &mut World| -> bevy::prelude::Result { Err(error) });
    }
    error.map_or(Ok(()), Err)
}
