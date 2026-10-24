use crate::geometry::{empty_mesh, recolor_mesh, update_mesh};
use crate::{Waveform, WaveformOutputLength, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::camera::{
    ScalingMode,
    primitives::Aabb,
    visibility::{RenderLayers, VisibilitySystems},
};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::offscreen::{
    OffscreenTarget, OffscreenUpdate, OffscreenViewport, WidgetryOffscreenPlugin,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

#[derive(Component, Default)]
pub(crate) struct InitializationFailure(FailureState);

#[derive(Component)]
pub(crate) struct Renderer {
    viewport: Entity,
    camera: Entity,
    mesh_entity: Entity,
    mesh: Handle<Mesh>,
    grid: Handle<Mesh>,
    grid_key: Option<(u64, UVec2, Color, bool)>,
    material: Handle<ColorMaterial>,
    grid_material: Handle<ColorMaterial>,
    size: UVec2,
    revision: u64,
    amplitude: Option<crate::WaveformAmplitude>,
    lanes: Vec2,
    style_tick: Option<Tick>,
    color_tick: Option<Tick>,
    failure: FailureState,
}

#[derive(Component, FromTemplate)]
struct RenderScene {
    camera: Entity,
    mesh: Entity,
}

pub struct WaveformRenderPlugin;

pub(crate) fn initialize(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, (With<Waveform>, Without<Renderer>)>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = initialize_root(world, root);
        let result = if let Some(mut state) = world.get_mut::<InitializationFailure>(root) {
            state.0.observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform renderer 初始化失败"),
                || widgetry_info!(?root, "Waveform renderer 初始化恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn initialize_root(world: &mut World, root: Entity) -> Result {
    let viewport = world
        .get::<crate::view::ChannelParts>(root)
        .map(|parts| parts.plot)
        .ok_or_else(|| BevyError::error("Waveform viewport missing"))?;
    let background = world
        .get::<crate::colors::ResolvedColors>(root)
        .ok_or_else(|| BevyError::error("Waveform colors missing"))?
        .background;
    let target = OffscreenTarget::allocate(world)?;
    let layer = target.layer();
    let mesh = world.resource_mut::<Assets<Mesh>>().add(empty_mesh());
    let grid = world.resource_mut::<Assets<Mesh>>().add(
        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0; 4]; 3])
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2])),
    );
    let material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::from(Color::WHITE));
    let mesh_handle = mesh.clone();
    let grid_handle = grid.clone();
    let material_handle = material.clone();
    let grid_material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::default());
    let grid_material_handle = grid_material.clone();
    let scene = spawn_scene(
        world,
        bsn! {
            RenderScene { camera: #WaveformCamera, mesh: #WaveformMesh }
            Transform::default()
            Visibility::default()
            Children [
                #WaveformCamera
                Camera2d
                Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(background) }
                template(move |_| Ok(Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: 1.0, height: 1.0 }, ..OrthographicProjection::default_2d() })))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Msaa::Off
            --
                #WaveformMesh
                Name::new("WaveformMesh")
                template(move |_| Ok(Mesh2d(mesh_handle.clone())))
                template(move |_| Ok(MeshMaterial2d(material_handle.clone())))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Transform::default()
            Visibility::default()
            template(|_| Ok(Aabb::from_min_max(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0))))
            Pickable::IGNORE
            --
                Name::new("WaveformTimeGrid")
                template(move |_| Ok(Mesh2d(grid_handle.clone())))
                template(move |_| Ok(MeshMaterial2d(grid_material_handle.clone())))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Transform::from_xyz(0.0, 0.0, -0.1)
                Visibility::default()
                template(|_| Ok(Aabb::from_min_max(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0))))
                Pickable::IGNORE
            ]
        },
    );
    let scene = match scene {
        Ok(scene) => scene,
        Err(error) => {
            target.release(world);
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
            world.resource_mut::<Assets<Mesh>>().remove(grid.id());
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(grid_material.id());
            return Err(BevyError::error(error.to_string()));
        }
    };
    let members = world
        .get::<RenderScene>(scene)
        .ok_or_else(|| BevyError::error("Waveform RenderScene missing"))?;
    let camera = members.camera;
    let mesh_entity = members.mesh;
    let offscreen = target.bind(world, viewport, camera, scene)?;
    world.entity_mut(root).insert(offscreen);
    world.entity_mut(root).insert(Renderer {
        viewport,
        camera,
        mesh_entity,
        mesh,
        grid,
        grid_key: None,
        material,
        grid_material,
        size: UVec2::ZERO,
        revision: u64::MAX,
        amplitude: None,
        lanes: Vec2::ZERO,
        style_tick: None,
        color_tick: None,
        failure: FailureState::default(),
    });
    Ok(())
}

pub(crate) fn render(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<Renderer>>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = render_root(world, root);
        let result = if let Some(mut renderer) = world.get_mut::<Renderer>(root) {
            renderer.failure.observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform renderer 更新失败"),
                || widgetry_info!(?root, "Waveform renderer 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn render_root(world: &mut World, root: Entity) -> Result {
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Waveform renderer missing"))?;
    let (viewport, camera, mesh_entity, mesh_handle, old_size, old_revision) = (
        renderer.viewport,
        renderer.camera,
        renderer.mesh_entity,
        renderer.mesh.clone(),
        renderer.size,
        renderer.revision,
    );
    if world.get::<Mesh2d>(mesh_entity).is_none() || world.get::<ViewportNode>(viewport).is_none() {
        return Err(BevyError::error("Waveform renderer hierarchy broken"));
    }
    let Some(size) = OffscreenViewport::update(world, root)? else {
        return Ok(());
    };
    update_grid(world, root, size)?;
    let width_changed = world
        .get::<WaveformOutputLength>(root)
        .ok_or_else(|| BevyError::error("Waveform output length missing"))?
        .0
        != size.x as usize;
    if width_changed {
        world
            .get_mut::<WaveformOutputLength>(root)
            .ok_or_else(|| BevyError::error("Waveform output length missing"))?
            .0 = size.x as usize;
        world
            .get_mut::<WaveformRuntime>(root)
            .ok_or_else(|| BevyError::error("Waveform runtime missing"))?
            .resize_reduction(size.x as usize);
    }
    let runtime = world
        .get::<WaveformRuntime>(root)
        .ok_or_else(|| BevyError::error("Waveform runtime missing"))?;
    let style = world
        .entity(root)
        .get_ref::<WaveformStyle>()
        .ok_or_else(|| BevyError::error("WaveformStyle missing"))?;
    let style_tick = style.last_changed();
    let changed_style = world
        .get::<Renderer>(root)
        .is_none_or(|renderer| renderer.style_tick != Some(style_tick));
    let colors = world
        .entity(root)
        .get_ref::<crate::colors::ResolvedColors>()
        .ok_or_else(|| BevyError::error("Waveform colors missing"))?;
    let color_tick = colors.last_changed();
    let changed_colors = world
        .get::<Renderer>(root)
        .is_none_or(|renderer| renderer.color_tick != Some(color_tick));
    let amplitude = runtime.config().amplitude_range;
    let channel_layout = Waveform::channel_layout(world, root)?;
    let plot_node = world
        .get::<ComputedNode>(viewport)
        .ok_or_else(|| BevyError::error("Waveform plot layout missing"))?;
    let texture_scale = size.y as f32 / (plot_node.size().y * plot_node.inverse_scale_factor());
    let lanes = Vec2::new(channel_layout.row_height, channel_layout.scroll_offset) * texture_scale;
    let changed_lanes = world
        .get::<Renderer>(root)
        .is_none_or(|renderer| renderer.lanes != lanes);
    let geometry_changed = changed_lanes
        || old_revision != runtime.revision()
        || old_size != size
        || changed_style
        || world
            .get::<Renderer>(root)
            .is_none_or(|renderer| renderer.amplitude != Some(amplitude));
    if !geometry_changed && !changed_colors {
        return Ok(());
    }
    style.validate()?;
    let background = colors.background;
    let revision = runtime.revision();
    // Mesh updates need mutable access to both a resource and a Component.
    // Use resource_scope to separate the borrows without copying the Mesh or violating World borrowing rules.
    world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| -> Result {
        let mut mesh = meshes
            .get_mut(&mesh_handle)
            .ok_or_else(|| BevyError::error("Waveform Mesh asset missing"))?;
        let runtime = world
            .get::<WaveformRuntime>(root)
            .ok_or_else(|| BevyError::error("Waveform runtime missing"))?;
        let style = world
            .get::<WaveformStyle>(root)
            .ok_or_else(|| BevyError::error("WaveformStyle missing"))?;
        let colors = world
            .get::<crate::colors::ResolvedColors>(root)
            .ok_or_else(|| BevyError::error("Waveform colors missing"))?;
        if geometry_changed {
            update_mesh(
                &mut mesh,
                runtime,
                style,
                size.as_vec2(),
                lanes,
                &colors.palette,
            )
        } else {
            recolor_mesh(&mut mesh, &colors.palette)
        }
    })?;
    if changed_colors {
        world
            .get_mut::<Camera>(camera)
            .ok_or_else(|| BevyError::error("Waveform camera missing"))?
            .clear_color = ClearColorConfig::Custom(background);
    }
    let mut renderer = world
        .get_mut::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Waveform renderer missing"))?;
    renderer.size = size;
    renderer.revision = revision;
    renderer.amplitude = Some(amplitude);
    renderer.lanes = lanes;
    renderer.style_tick = Some(style_tick);
    renderer.color_tick = Some(color_tick);
    Ok(())
}

fn update_grid(world: &mut World, root: Entity, size: UVec2) -> Result {
    let (revision, positions) = crate::time_axis::grid_snapshot(world, root)?;
    let visible = crate::Waveform::grid_visible(world, root)?;
    let theme = world.resource::<bevy_widgetry_theme::WidgetryThemeMode>();
    let disabled = world.get::<bevy::ui::InteractionDisabled>(root).is_some();
    let color = if disabled {
        theme.colors().text.disabled.foreground
    } else {
        theme.colors().text.normal.foreground
    }
    .with_alpha(0.12);
    let key = (revision, size, color, visible);
    let renderer = world
        .get::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Waveform renderer missing"))?;
    if renderer.grid_key == Some(key) {
        return Ok(());
    }
    let handle = renderer.grid.clone();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    if visible {
        let half = 0.5 / size.x as f32;
        for fraction in positions {
            let start = vertices.len() as u32;
            let x = fraction - 0.5;
            let left = (x - half).max(-0.5);
            let right = (x + half).min(0.5);
            vertices.extend([
                [left, -0.5, 0.0],
                [right, -0.5, 0.0],
                [right, 0.5, 0.0],
                [left, 0.5, 0.0],
            ]);
            indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        }
    }
    let colors = if vertices.is_empty() {
        vertices.resize(3, [0.0; 3]);
        indices.extend([0, 1, 2]);
        vec![[0.0; 4]; 3]
    } else {
        vec![color.to_linear().to_f32_array(); vertices.len()]
    };
    {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mut mesh = meshes
            .get_mut(&handle)
            .ok_or_else(|| BevyError::error("Waveform grid asset missing"))?;
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    }
    world
        .get_mut::<Renderer>(root)
        .ok_or_else(|| BevyError::error("Waveform renderer missing"))?
        .grid_key = Some(key);
    Ok(())
}

pub(crate) fn release(
    event: On<Remove<Renderer>>,
    renderers: Query<&Renderer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    if let Ok(renderer) = renderers.get(event.entity) {
        meshes.remove(renderer.mesh.id());
        meshes.remove(renderer.grid.id());
        materials.remove(renderer.material.id());
        materials.remove(renderer.grid_material.id());
        commands
            .entity(event.entity)
            .try_remove::<OffscreenViewport>();
    }
}

impl Plugin for WaveformRenderPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::WaveformPlugin>() {
            app.add_plugins(crate::WaveformPlugin);
        }
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryOffscreenPlugin>() {
            app.add_plugins(WidgetryOffscreenPlugin);
        }
        crate::controls::install(app);
        crate::playback::install(app);
        crate::channels::install(app);
        crate::measurement::install(app);
        app.add_systems(
            PostUpdate,
            crate::time_axis::prepare.in_set(WidgetryUiSystems::Build),
        )
        .add_systems(
            PostUpdate,
            crate::time_axis::finish
                .after(UiSystems::Layout)
                .before(VisibilitySystems::VisibilityPropagate)
                .before(render),
        );
        if !app.world().contains_resource::<Assets<Mesh>>() {
            app.init_asset::<Mesh>();
        }
        if !app.world().contains_resource::<Assets<ColorMaterial>>() {
            app.init_asset::<ColorMaterial>();
        }
        app.add_systems(
            PostUpdate,
            crate::colors::update.in_set(WidgetryUiSystems::Colors),
        );
        app.add_observer(crate::colors::refresh);
        app.add_observer(release)
            .add_systems(
                PostUpdate,
                initialize
                    .in_set(WidgetryUiSystems::Build)
                    .before(bevy::camera::CameraUpdateSystems),
            )
            .add_systems(PostUpdate, render.in_set(OffscreenUpdate));
        widgetry_info!("WaveformRenderPlugin 注册完成");
    }
}
