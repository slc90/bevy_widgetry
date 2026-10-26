use crate::view::ViewState;
use crate::{
    CanvasDocument, CanvasGeometry, CanvasPaint, CanvasRectangle, CanvasShape, CanvasView,
};
use bevy::camera::{CameraProjection, ScalingMode, primitives::Aabb, visibility::RenderLayers};
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::offscreen::{
    OffscreenTarget, OffscreenUpdate, OffscreenViewport, WidgetryOffscreenPlugin,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use std::collections::HashMap;
mod outline;
mod preview;

pub struct CanvasPlugin;

#[derive(Component, Default)]
pub(crate) struct RenderFailure {
    initialize: FailureState,
    geometry: FailureState,
    view: FailureState,
}

#[derive(Clone, Copy)]
enum Stage {
    Initialize,
    Geometry,
    View,
}

#[derive(Component)]
struct Renderer {
    camera: Entity,
    content: Entity,
    layer: usize,
    proxies: HashMap<Entity, Entity>,
    outline: Option<Entity>,
    preview: Option<Entity>,
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
    geometry: CanvasGeometry,
    paint: CanvasPaint,
    fill_material: Handle<ColorMaterial>,
    stroke_material: Handle<ColorMaterial>,
}

fn initialize(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, (With<CanvasView>, Without<Renderer>)>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        let result = initialize_root(world, root);
        if let Err(error) = observe(world, root, Stage::Initialize, result)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn observe(world: &mut World, root: Entity, stage: Stage, result: Result) -> Result {
    if let Some(mut failure) = world.get_mut::<RenderFailure>(root) {
        let state = match stage {
            Stage::Initialize => &mut failure.initialize,
            Stage::Geometry => &mut failure.geometry,
            Stage::View => &mut failure.view,
        };
        state.observe(
            result,
            |error| widgetry_error!(?root, %error, "Canvas 渲染失败"),
            || widgetry_info!(?root, "Canvas 渲染恢复正常"),
        )
    } else {
        result
    }
}

fn initialize_root(world: &mut World, root: Entity) -> Result {
    let target = OffscreenTarget::allocate(world)?;
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
            target.release(world);
            return Err(BevyError::error(error));
        }
    };
    let Some(parts) = world.get::<SceneParts>(scene) else {
        target.release(world);
        world.despawn(scene);
        return Err(BevyError::error("Canvas scene parts missing"));
    };
    let (camera, content) = (parts.camera, parts.content);
    let viewport = target.bind(world, root, camera, scene)?;
    world.entity_mut(root).insert((
        viewport,
        Renderer {
            camera,
            content,
            layer,
            proxies: HashMap::new(),
            outline: None,
            preview: None,
        },
    ));
    Ok(())
}

fn render(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<Renderer>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        let result = render_root(world, root);
        if let Err(error) = observe(world, root, Stage::View, result)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn render_root(world: &mut World, root: Entity) -> Result {
    let Some(size) = OffscreenViewport::update(world, root)? else {
        return Ok(());
    };
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?;
    let camera = renderer.camera;
    let node = world
        .get::<ComputedNode>(root)
        .ok_or_else(|| BevyError::error("Canvas layout missing"))?;
    let logical_size = node.size() * node.inverse_scale_factor() / CanvasView::zoom(world, root)?;
    let mut projection = world
        .get_mut::<Projection>(camera)
        .ok_or_else(|| BevyError::error("Canvas projection missing"))?;
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
        .ok_or_else(|| BevyError::error("Canvas camera missing"))?
        .computed
        .clip_from_view = clip_from_view;
    Ok(())
}

fn prepare(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<Renderer>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if world.get::<Renderer>(root).is_none() {
            continue;
        }
        let result = prepare_root(world, root);
        if let Err(error) = observe(world, root, Stage::Geometry, result)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    let used = world
        .query::<&ShapeRender>()
        .iter(world)
        .map(|proxy| proxy.shape)
        .collect();
    crate::mesh_cache::retain(world, &used);
    failure.map_or(Ok(()), Err)
}

fn prepare_root(world: &mut World, root: Entity) -> Result {
    if crate::selection::repair(world, root)? {
        world.flush();
        if world.get::<Renderer>(root).is_none() {
            return Ok(());
        }
    }
    let document = CanvasView::document(world, root)?;
    if document.is_some_and(|document| world.get::<CanvasDocument>(document).is_none()) {
        CanvasView::set_document_in_world(world, root, None)?;
        world.flush();
        if world.get::<Renderer>(root).is_none() {
            return Ok(());
        }
    }
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?;
    let (content, layer) = (renderer.content, renderer.layer);
    let camera = renderer.camera;
    let center = CanvasView::center(world, root)?;
    let mut transform = world
        .get_mut::<Transform>(camera)
        .ok_or_else(|| BevyError::error("Canvas camera transform missing"))?;
    let translation = Vec3::new(center.x, -center.y, transform.translation.z);
    if transform.translation != translation {
        transform.translation = translation;
    }
    let document = CanvasView::document(world, root)?;
    let shapes = if let Some(document) =
        document.filter(|document| world.get::<CanvasDocument>(*document).is_some())
    {
        CanvasDocument::shape_order(world, document)?
            .into_iter()
            .map(|entity| {
                let geometry = world
                    .get::<CanvasGeometry>(entity)
                    .ok_or_else(|| BevyError::error("Canvas geometry missing"))?;
                let transform = world
                    .get::<Transform>(entity)
                    .ok_or_else(|| BevyError::error("Canvas transform missing"))?;
                let paint = world
                    .get::<CanvasPaint>(entity)
                    .ok_or_else(|| BevyError::error("Canvas paint missing"))?;
                Ok((entity, *geometry, *transform, *paint))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let stale = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
        .proxies
        .iter()
        .filter(|(shape, _)| !shapes.iter().any(|(entity, ..)| entity == *shape))
        .map(|(shape, proxy)| (*shape, *proxy))
        .collect::<Vec<_>>();
    for (shape, proxy) in stale {
        world.despawn(proxy);
        world
            .get_mut::<Renderer>(root)
            .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
            .proxies
            .remove(&shape);
    }
    let spacing = 100.0 / shapes.len().max(1) as f32;
    for (index, (shape, geometry, transform, paint)) in shapes.into_iter().enumerate() {
        geometry.validate(&transform, &paint)?;
        let handles = crate::mesh_cache::geometry(world, shape, geometry, paint)?;
        let proxy = world
            .get::<Renderer>(root)
            .and_then(|renderer| renderer.proxies.get(&shape))
            .copied();
        let proxy = if let Some(proxy) = proxy {
            proxy
        } else {
            let proxy = create_proxy(world, content, layer, shape, geometry, paint, &handles)?;
            world
                .get_mut::<Renderer>(root)
                .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
                .proxies
                .insert(shape, proxy);
            proxy
        };
        let mut projected = transform;
        projected.translation.z = index as f32 * spacing;
        update_proxy(world, proxy, geometry, projected, paint, &handles)?;
        let stroke = world
            .get::<ProxyParts>(proxy)
            .ok_or_else(|| BevyError::error("Canvas proxy parts missing"))?
            .stroke;
        world
            .get_mut::<Transform>(stroke)
            .ok_or_else(|| BevyError::error("Canvas stroke transform missing"))?
            .set_if_neq(Transform::from_xyz(0.0, 0.0, spacing * 0.5));
    }
    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
        .outline;
    let outline = outline::sync(world, root, content, layer, current)?;
    world
        .get_mut::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
        .outline = outline;
    let current = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
        .preview;
    let preview = preview::sync(world, root, content, layer, current)?;
    world
        .get_mut::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Canvas renderer missing"))?
        .preview = preview;
    Ok(())
}

fn create_proxy(
    world: &mut World,
    content: Entity,
    layer: usize,
    shape: Entity,
    geometry: CanvasGeometry,
    paint: CanvasPaint,
    handles: &crate::mesh_cache::ShapeMeshes,
) -> Result<Entity> {
    let assets = ShapeRender {
        shape,
        geometry,
        paint,
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
            world.entity_mut(proxy).insert(assets);
            Ok(proxy)
        }
        Err(error) => {
            release_assets(world, &assets);
            Err(BevyError::error(error))
        }
    }
}

fn update_proxy(
    world: &mut World,
    proxy: Entity,
    geometry: CanvasGeometry,
    transform: Transform,
    paint: CanvasPaint,
    handles: &crate::mesh_cache::ShapeMeshes,
) -> Result {
    let old = world
        .get::<ShapeRender>(proxy)
        .ok_or_else(|| BevyError::error("Canvas proxy assets missing"))?
        .clone();
    let parts = world
        .get::<ProxyParts>(proxy)
        .ok_or_else(|| BevyError::error("Canvas proxy parts missing"))?;
    let (fill, stroke) = (parts.fill, parts.stroke);
    for (entity, color, handle, bounds) in [
        (fill, paint.fill, &old.fill_material, handles.fill_bounds),
        (
            stroke,
            paint.stroke.map(|stroke| stroke.color),
            &old.stroke_material,
            handles.stroke_bounds,
        ),
    ] {
        {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            let mut material = materials
                .get_mut(handle)
                .ok_or_else(|| BevyError::error("Canvas material missing"))?;
            if material.color != color.unwrap_or(Color::NONE) {
                material.color = color.unwrap_or(Color::NONE);
            }
        }
        let mut visibility = world
            .get_mut::<Visibility>(entity)
            .ok_or_else(|| BevyError::error("Canvas mesh visibility missing"))?;
        visibility.set_if_neq(if color.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
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
        .ok_or_else(|| BevyError::error("Canvas proxy transform missing"))?
        .set_if_neq(projected_transform);
    if old.geometry != geometry || old.paint != paint {
        let mut assets = world
            .get_mut::<ShapeRender>(proxy)
            .ok_or_else(|| BevyError::error("Canvas proxy assets missing"))?;
        assets.geometry = geometry;
        assets.paint = paint;
    }
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
        crate::export::install(app);
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryOffscreenPlugin>() {
            app.add_plugins(WidgetryOffscreenPlugin);
        }
        crate::input::install(app);
        outline::install(app);
        preview::install(app);
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
        widgetry_info!("CanvasPlugin 注册完成");
    }
}
