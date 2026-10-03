use crate::background::ThemeWindowBackground;
use crate::title_bar::bar::TitleBar;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{CompositeAlphaMode, WindowClosed, WindowRef},
};
use bevy_widgetry_core::ThemeChanged;

#[derive(Component)]
#[require(Node = window_root_node())]
pub(crate) struct WindowRoot {
    pub target_window: Entity,
    pub maximized: bool,
}

#[derive(Component)]
#[require(Node = window_content_node(), Pickable::IGNORE)]
pub(crate) struct WindowContent;

#[derive(Component)]
pub(crate) struct WindowInitialized;

#[derive(Component)]
pub(crate) struct OwnedWindow;

#[derive(Resource, Default)]
pub(crate) struct PendingWindows(Vec<Entity>);

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

pub(crate) fn find_window_root<'a>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &'a Query<&WindowRoot>,
) -> Option<&'a WindowRoot> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| roots.get(ancestor).ok())
}

// Scene 的 Add observer 触发时 subtree 还可能未展开。
// 这里只记录顺序，避免在构造中间态校验绑定而误删合法 tree。
pub(crate) fn queue_window_initialization(
    event: On<Add, WindowRoot>,
    mut pending: ResMut<PendingWindows>,
) {
    pending.0.push(event.entity);
}

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
        // Scene template 完成后 Node 仍会自动插入透明 BackgroundColor。
        // 在完整 root 初始化时移除该默认层，确保 Image 模式不保留纯色背景。
        if world.get::<ImageNode>(entity).is_some() {
            world.entity_mut(entity).remove::<BackgroundColor>();
        }
        world.entity_mut(entity).insert(WindowInitialized);
        world
            .entity_mut(target)
            .entry::<crate::modal::ModalState>()
            .or_default();
        world.commands().queue(crate::modal::sync_modal_windows);
    }
}

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

pub(crate) fn refresh_window_theme(
    event: On<ThemeChanged>,
    mut backgrounds: Query<&mut BackgroundColor, (With<WindowRoot>, With<ThemeWindowBackground>)>,
    mut roots: Query<&mut BorderColor, With<WindowRoot>>,
    mut bars: Query<&mut BorderColor, (With<TitleBar>, Without<WindowRoot>)>,
) {
    let colors = event.mode.colors();
    for mut background in &mut backgrounds {
        background.0 = colors.window_background;
    }
    for mut border in &mut roots {
        *border = BorderColor::all(colors.window_border);
    }
    for mut border in &mut bars {
        *border = BorderColor::all(colors.title_bar_border);
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        owned_widgetry_window,
    };
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn removing_owned_marker_keeps_resources() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
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
