use crate::title_bar::bar::TitleBar;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{CompositeAlphaMode, WindowClosed, WindowRef},
};
use bevy_widgetry_core::ThemeChanged;

/// 关联 UI 层级与真实窗口，使子节点上的交互作用于正确的窗口。
#[derive(Component)]
#[require(Node = window_root_node())]
pub(crate) struct WindowRoot {
    /// 需要接收该 UI 层级窗口操作的真实窗口实体。
    pub target_window: Entity,
    /// 最近一次读取的 winit 实际最大化状态，不从操作请求推断。
    pub maximized: bool,
}

/// 窗口标题栏之外的内容容器，占据根布局的剩余空间。
#[derive(Component)]
#[require(Node = window_content_node(), Pickable::IGNORE)]
pub(crate) struct WindowContent;

/// 标记已通过绑定校验的根，确保排队创建时第一个成功绑定者保留。
#[derive(Component)]
pub(crate) struct WindowInitialized;

/// 所有权只附着于根；资源标识仍由 WindowRoot 和 UiTargetCamera 唯一保存。
#[derive(Component)]
pub(crate) struct OwnedWindow;

/// 按创建观察顺序记录根，等 BSN 完成全部子树和关系后再处理。
#[derive(Resource, Default)]
pub(crate) struct PendingWindows(Vec<Entity>);

/// 使窗口 UI 根填满可用空间，并按纵向排列标题栏和内容。
fn window_root_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(8)),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// 让应用内容占据标题栏之外的剩余区域。
fn window_content_node() -> Node {
    Node {
        width: percent(100),
        flex_grow: 1.0,
        min_height: px(0),
        border_radius: BorderRadius::bottom(px(8)),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// 从交互实体或其祖先查找窗口关联，非窗口层级返回 None。
pub(crate) fn find_window_root<'a>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &'a Query<&WindowRoot>,
) -> Option<&'a WindowRoot> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| roots.get(ancestor).ok())
}

/// Add 发生时子树可能尚未展开，此处只记录创建顺序。
pub(crate) fn queue_window_initialization(
    event: On<Add, WindowRoot>,
    mut pending: ResMut<PendingWindows>,
) {
    pending.0.push(event.entity);
}

/// 在完整场景展开后校验绑定，错误树统一清理且不影响已有实例。
pub(crate) fn initialize_windows(world: &mut World) {
    let pending = std::mem::take(&mut world.resource_mut::<PendingWindows>().0);
    for entity in pending {
        let Some(root) = world.get::<WindowRoot>(entity) else {
            continue;
        };
        let target = root.target_window;
        let camera = world.get::<UiTargetCamera>(entity).map(|camera| camera.0);
        let valid_window = world.get::<Window>(target).is_some();
        let valid_properties = world.get::<Window>(target).is_some_and(|window| {
            window.transparent
                && !window.decorations
                && window.composite_alpha_mode == CompositeAlphaMode::PreMultiplied
        });
        let valid_camera = camera.is_some_and(|camera| world.get::<Camera>(camera).is_some());
        let duplicate_window = world
            .query_filtered::<(Entity, &WindowRoot), With<WindowInitialized>>()
            .iter(world)
            .any(|(other, root)| other != entity && root.target_window == target);
        let duplicate_camera = camera.is_some_and(|camera| {
            world
                .query_filtered::<(Entity, &UiTargetCamera), (With<WindowRoot>, With<WindowInitialized>)>()
                .iter(world)
                .any(|(other, bound)| other != entity && bound.0 == camera)
        });
        if !valid_window
            || !valid_properties
            || !valid_camera
            || duplicate_window
            || duplicate_camera
        {
            world.entity_mut(entity).despawn();
            continue;
        }
        if let Some(camera) = camera {
            if let Some(mut config) = world.get_mut::<Camera>(camera) {
                config.viewport = None;
                config.clear_color = ClearColorConfig::Custom(Color::NONE);
            }
            world
                .entity_mut(camera)
                .insert(RenderTarget::Window(WindowRef::Entity(target)));
        }
        world.entity_mut(entity).insert(WindowInitialized);
        world
            .entity_mut(target)
            .entry::<crate::modal::ModalState>()
            .or_default();
    }
}

/// 只在整体销毁根时回收资源，容许系统已经先行移除了原生窗口。
pub(crate) fn cleanup_owned_window(
    event: On<Despawn, OwnedWindow>,
    roots: Query<(&WindowRoot, &UiTargetCamera)>,
    mut commands: Commands,
) {
    if let Ok((root, camera)) = roots.get(event.entity) {
        let target = root.target_window;
        let camera = camera.0;
        commands.queue(move |world: &mut World| {
            for entity in [camera, target] {
                if let Ok(entity) = world.get_entity_mut(entity) {
                    entity.despawn();
                }
            }
        });
    }
}

/// 原生关闭通知只回收对应 UI 树，相机生命周期留给消费者。
pub(crate) fn cleanup_closed_windows(
    mut closed: MessageReader<WindowClosed>,
    roots: Query<(Entity, &WindowRoot)>,
    mut commands: Commands,
) {
    for event in closed.read() {
        for (entity, root) in &roots {
            if root.target_window == event.window {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

/// 主题事件只更新窗口表面与分隔线，系统按钮保留固定配色。
pub(crate) fn refresh_window_theme(
    event: On<ThemeChanged>,
    mut roots: Query<(&mut BackgroundColor, &mut BorderColor), With<WindowRoot>>,
    mut bars: Query<&mut BorderColor, (With<TitleBar>, Without<WindowRoot>)>,
) {
    let colors = event.mode.colors();
    for (mut background, mut border) in &mut roots {
        background.0 = colors.window_background;
        *border = BorderColor::all(colors.window_border);
    }
    for mut border in &mut bars {
        *border = BorderColor::all(colors.title_bar_border);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WindowControlsConfig, WindowPlugin, owned_window};
    use bevy_widgetry_test_utils::scene_app;

    /// 单独移除所有权 marker 不等价于销毁 owned 根，原生窗口和相机继续存在。
    #[test]
    fn removing_owned_marker_keeps_resources() {
        let mut app = scene_app();
        app.add_plugins(WindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
        }).id();
        app.update();
        let target = app.world().get::<WindowRoot>(root).unwrap().target_window;
        let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
        app.world_mut().entity_mut(root).remove::<OwnedWindow>();
        app.world_mut().flush();
        for entity in [root, target, camera] {
            assert!(app.world().get_entity(entity).is_ok());
        }
    }
}
