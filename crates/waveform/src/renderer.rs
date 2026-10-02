use crate::geometry::{empty_mesh, update_mesh};
use crate::view::WaveformViewport;
use crate::{Waveform, WaveformOutputLength, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::camera::{
    RenderTarget, ScalingMode,
    primitives::Aabb,
    visibility::{RenderLayers, VisibilitySystems},
};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::UiSystems;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 每实例独占的 layer；宿主应保留 1024 及以上的 Waveform layer 区间。
#[derive(Resource)]
pub(crate) struct RendererLayers {
    next: usize,
    free: Vec<usize>,
}

/// 构造失败跨 update 去重，由当前 root 持有，不新增每帧诊断 system。
#[derive(Component, Default)]
pub(crate) struct InitializationFailure(FailureState);

/// renderer 自有资源与内容版本；handle 数量不随 cursor 前进增长。
#[derive(Component)]
pub(crate) struct Renderer {
    viewport: Entity,
    scene: Entity,
    camera: Entity,
    mesh_entity: Entity,
    mesh: Handle<Mesh>,
    image: Handle<Image>,
    material: Handle<ColorMaterial>,
    layer: usize,
    size: UVec2,
    revision: u64,
    style_tick: Option<Tick>,
    failure: FailureState,
}

/// BSN entity references 只用于建立 ownership，camera 与 mesh 共用同一 isolated scene。
#[derive(Component, FromTemplate)]
struct RenderScene {
    camera: Entity,
    mesh: Entity,
}

/// 为 BSN Waveform 装配 renderer，并自动补齐纯 headless plugin；宿主提供 Asset/UI/2D plugins。
pub struct WaveformRenderPlugin;

/// 只初始化新 root，既有 renderer shell 和 handles 保持稳定。
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

/// asset 和 layer 在 BSN 成功前保持本地 ownership，失败时释放全部已创建资源。
fn initialize_root(world: &mut World, root: Entity) -> Result {
    let viewport = world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&entity| world.get::<WaveformViewport>(entity).is_some())
        })
        .ok_or_else(|| BevyError::error("Waveform viewport missing"))?;
    let background = world
        .get::<WaveformStyle>(root)
        .ok_or_else(|| BevyError::error("WaveformStyle missing"))?
        .background;
    let layer = {
        let mut layers = world.resource_mut::<RendererLayers>();
        if let Some(layer) = layers.free.pop() {
            layer
        } else {
            let layer = layers.next;
            layers.next = layers
                .next
                .checked_add(1)
                .ok_or_else(|| BevyError::error("Waveform layer overflow"))?;
            layer
        }
    };
    let mut image = Image::new_uninit(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image = world.resource_mut::<Assets<Image>>().add(image);
    let mesh = world.resource_mut::<Assets<Mesh>>().add(empty_mesh());
    let material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::from(Color::WHITE));
    let camera_image = image.clone();
    let mesh_handle = mesh.clone();
    let material_handle = material.clone();
    let scene = spawn_scene(
        world,
        bsn! {
            RenderScene { camera: #WaveformCamera, mesh: #WaveformMesh }
            Transform::default()
            Visibility::default()
            Children [(
                #WaveformCamera
                Camera2d
                Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(background) }
                template(move |_| Ok(RenderTarget::Image(camera_image.clone().into())))
                template(move |_| Ok(Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: 1.0, height: 1.0 }, ..OrthographicProjection::default_2d() })))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Msaa::Off
            ), (
                #WaveformMesh
                template(move |_| Ok(Mesh2d(mesh_handle.clone())))
                template(move |_| Ok(MeshMaterial2d(material_handle.clone())))
                template(move |_| Ok(RenderLayers::none().with(layer)))
                Transform::default()
            Visibility::default()
            template(|_| Ok(Aabb::from_min_max(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0))))
            Pickable::IGNORE
            )]
        },
    );
    let scene = match scene {
        Ok(scene) => scene,
        Err(error) => {
            world.resource_mut::<Assets<Image>>().remove(image.id());
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
            world.resource_mut::<RendererLayers>().free.push(layer);
            return Err(BevyError::error(error.to_string()));
        }
    };
    let members = world
        .get::<RenderScene>(scene)
        .ok_or_else(|| BevyError::error("Waveform RenderScene missing"))?;
    let camera = members.camera;
    let mesh_entity = members.mesh;
    world.entity_mut(root).add_child(scene);
    world.entity_mut(viewport).insert(ViewportNode::new(camera));
    world.entity_mut(root).insert(Renderer {
        viewport,
        scene,
        camera,
        mesh_entity,
        mesh,
        image,
        material,
        layer,
        size: UVec2::ZERO,
        revision: u64::MAX,
        style_tick: None,
        failure: FailureState::default(),
    });
    Ok(())
}

/// layout 之后消费真实 physical size；CPU path 在正常 Bevy render extraction 前完成。
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

/// 零尺寸关闭 camera；稳定 size/revision/style 不修改 mesh，不创建重复资源。
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
    let size = world
        .get::<ComputedNode>(viewport)
        .ok_or_else(|| BevyError::error("Waveform ComputedNode missing"))?
        .size()
        .as_uvec2();
    let active = size.x > 0
        && size.y > 0
        && world
            .get::<InheritedVisibility>(root)
            .is_some_and(|visibility| visibility.get());
    let mut camera_state = world
        .get_mut::<Camera>(camera)
        .ok_or_else(|| BevyError::error("Waveform camera missing"))?;
    if camera_state.is_active != active {
        camera_state.is_active = active;
    }
    if !active {
        return Ok(());
    }
    if camera_state
        .computed
        .target_info
        .as_ref()
        .is_some_and(|info| info.physical_size != size)
        && let Some(info) = &mut camera_state.computed.target_info
    {
        info.physical_size = size;
    }
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
    if old_revision == runtime.revision() && old_size == size && !changed_style {
        return Ok(());
    }
    style.validate()?;
    let background = style.background;
    let revision = runtime.revision();
    // ECS 分离 resource 与 component borrow，直接复用同一 Mesh allocation。
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
        update_mesh(&mut mesh, runtime, style, size.as_vec2())
    })?;
    if changed_style {
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
    renderer.style_tick = Some(style_tick);
    Ok(())
}

/// root lifecycle 显式释放 owned resources 和 layer，外部 source 永远不销毁。
pub(crate) fn release(
    event: On<Remove, Renderer>,
    renderers: Query<&Renderer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut layers: ResMut<RendererLayers>,
    mut commands: Commands,
) {
    if let Ok(renderer) = renderers.get(event.entity) {
        images.remove(renderer.image.id());
        meshes.remove(renderer.mesh.id());
        materials.remove(renderer.material.id());
        layers.free.push(renderer.layer);
        commands.entity(renderer.scene).try_despawn();
    }
}

impl Default for RendererLayers {
    fn default() -> Self {
        Self {
            next: 1024,
            free: Vec::new(),
        }
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
        if !app.world().contains_resource::<Assets<Mesh>>() {
            app.init_asset::<Mesh>();
        }
        if !app.world().contains_resource::<Assets<Image>>() {
            app.init_asset::<Image>();
        }
        if !app.world().contains_resource::<Assets<ColorMaterial>>() {
            app.init_asset::<ColorMaterial>();
        }
        app.init_resource::<RendererLayers>()
            .add_observer(release)
            .add_systems(
                PostUpdate,
                initialize
                    .in_set(WidgetryUiSystems::Build)
                    .before(bevy::camera::CameraUpdateSystems),
            )
            .add_systems(
                PostUpdate,
                render
                    .after(UiSystems::Layout)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .before(VisibilitySystems::CalculateBounds)
                    .before(VisibilitySystems::CheckVisibility)
                    .before(bevy::ui::widget::update_viewport_render_target_size),
            );
        widgetry_info!("WaveformRenderPlugin 注册完成");
    }
}
