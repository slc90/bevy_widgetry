use crate::title_bar::bar::TitleBar;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{CompositeAlphaMode, WindowClosed, WindowRef},
};
use bevy_widgetry_core::ThemeChanged;

/// 关联 UI hierarchy 与真实 window，使 child node 上的交互作用于正确的 window。
#[derive(Component)]
#[require(Node = window_root_node())]
pub(crate) struct WindowRoot {
    /// 需要接收该 UI hierarchy 的 window 操作的真实 window entity。
    pub target_window: Entity,
    /// 最近一次读取的 winit 实际 maximized state，不从操作请求推断。
    pub maximized: bool,
}

/// window title bar 之外的内容容器，占据 root layout 的剩余空间。
#[derive(Component)]
#[require(Node = window_content_node(), Pickable::IGNORE)]
pub(crate) struct WindowContent;

/// 标记已通过绑定校验的 root，确保排队创建时第一个成功绑定者保留。
#[derive(Component)]
pub(crate) struct WindowInitialized;

/// ownership 只附着于 root；资源标识仍由 WindowRoot 和 UiTargetCamera 唯一保存。
#[derive(Component)]
pub(crate) struct OwnedWindow;

/// 按创建时 observer 的执行顺序记录 root，等 BSN 完成全部 subtree 和 relationship 后再处理。
#[derive(Resource, Default)]
pub(crate) struct PendingWindows(Vec<Entity>);

/// 使 window UI root 填满可用空间，并按纵向排列 title bar 和内容。
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

/// 让应用内容占据 title bar 之外的剩余区域。
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

/// 从交互 entity 或其 ancestor 查找 window 关联，非 window hierarchy 返回 None。
pub(crate) fn find_window_root<'a>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &'a Query<&WindowRoot>,
) -> Option<&'a WindowRoot> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| roots.get(ancestor).ok())
}

/// Add 发生时 subtree 可能尚未展开，此处只记录创建顺序。
pub(crate) fn queue_window_initialization(
    event: On<Add, WindowRoot>,
    mut pending: ResMut<PendingWindows>,
) {
    pending.0.push(event.entity);
}

/// 在完整 Scene 展开后校验绑定，无效 tree 统一清理且不影响已有实例。
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
        world.commands().queue(crate::modal::sync_modal_windows);
    }
}

/// 只在整体销毁 root 时回收资源，容许操作系统已经先行移除了 native window。
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

/// native close 通知只回收对应 UI tree，camera lifecycle 留给消费者。
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

/// theme event 只更新 window 表面与分隔线，系统按钮保留固定配色。
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
    use crate::{WidgetryWindowControlsConfig, WidgetryWindowPlugin, owned_widgetry_window};
    use bevy_widgetry_test_utils::scene_app;

    /// 单独移除 ownership marker 不等价于销毁 owned root，native window 和 camera 继续存在。
    #[test]
    fn removing_owned_marker_keeps_resources() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
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
