use crate::window_root::{WindowInitialized, WindowRoot};
use bevy::prelude::*;
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_core::z_index;

#[derive(Component, Clone, Copy, Debug)]
pub struct WidgetryModalWindow {
    pub parent: Entity,
}

#[derive(Component, Default)]
pub(crate) struct ModalState {
    blocker: Option<Entity>,
}

#[derive(Component)]
struct ModalBlocker;

pub(crate) fn sync_modal_windows(world: &mut World) {
    let roots: Vec<_> = world
        .query_filtered::<(Entity, &WindowRoot), With<WindowInitialized>>()
        .iter(world)
        .map(|(entity, root)| (entity, root.target_window))
        .collect();
    let children: Vec<_> = world
        .query_filtered::<(Entity, &WidgetryModalWindow), (With<WindowRoot>, With<WindowInitialized>)>()
        .iter(world)
        .map(|(entity, modal)| (entity, modal.parent))
        .collect();
    for &(child, parent) in &children {
        if (!roots
            .iter()
            .any(|&(root, target)| root != child && target == parent)
            || world.get::<Window>(parent).is_none())
            && let Ok(child) = world.get_entity_mut(child)
        {
            child.despawn();
        }
    }
    for (root, parent) in roots {
        if world.get_entity(root).is_err() {
            continue;
        }
        let Some(state) = world.get::<ModalState>(parent) else {
            continue;
        };
        let blocker = state
            .blocker
            .filter(|&entity| world.get_entity(entity).is_ok());
        let needed = crate::input::active_root(world, parent).is_some_and(|active| active != root);
        let next = match (needed, blocker) {
            (true, None) => {
                let blocker = world.commands().spawn_scene_with_error_handler(bsn! {
                    template(|_| Ok(ModalBlocker))
                    Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), bottom: px(0) }
                    GlobalZIndex({z_index::MODAL})
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.28))
                    Pickable { should_block_lower: true, is_hoverable: false }
                    template(move |_| Ok(ChildOf(root)))
                }).id();
                Some(blocker)
            }
            (false, Some(blocker)) => {
                world.entity_mut(blocker).despawn();
                None
            }
            (_, blocker) => blocker,
        };
        if let Some(mut state) = world.get_mut::<ModalState>(parent) {
            state.blocker = next;
        }
    }
    world.flush();
}

pub(crate) fn modal_added(_event: On<Add, WidgetryModalWindow>, mut commands: Commands) {
    commands.queue(sync_modal_windows);
}

// Remove observer 的 query 仍含待移除的 modal relationship。
// 延后重新计算，避免把最后一个 child 错计为存活而残留 overlay。
pub(crate) fn modal_removed(_event: On<Remove, WidgetryModalWindow>, mut commands: Commands) {
    commands.queue(sync_modal_windows);
    commands.queue(crate::input::sync_focus);
}

pub(crate) fn root_removed(
    event: On<Remove, WindowRoot>,
    roots: Query<&WindowRoot>,
    states: Query<&ModalState>,
    mut commands: Commands,
) {
    if let Ok(root) = roots.get(event.entity)
        && let Ok(state) = states.get(root.target_window)
        && let Some(blocker) = state.blocker
    {
        commands.entity(blocker).try_despawn();
    }
    commands.queue(sync_modal_windows);
    commands.queue(crate::input::sync_focus);
}

pub(crate) fn parent_removed(
    event: On<Remove, Window>,
    states: Query<&ModalState>,
    mut commands: Commands,
) {
    if let Ok(state) = states.get(event.entity) {
        if let Some(blocker) = state.blocker {
            commands.entity(blocker).try_despawn();
        }
        commands.queue(sync_modal_windows);
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
        owned_widgetry_window, prepare_native_window, widgetry_window,
    };
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn modal_lifecycle_syncs_without_another_frame() {
        for end in 0..3 {
            let mut app = scene_app();
            app.add_plugins(WidgetryWindowPlugin);
            let parent_root = app.world_mut().commands().spawn_scene(bsn! {
                owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            }).id();
            let child = app.world_mut().commands().spawn_scene(bsn! {
                owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            }).id();
            app.update();
            let parent = app
                .world()
                .get::<WindowRoot>(parent_root)
                .unwrap()
                .target_window;
            app.world_mut()
                .entity_mut(child)
                .insert(WidgetryModalWindow { parent });
            app.world_mut().flush();
            let blocker = app
                .world()
                .get::<ModalState>(parent)
                .unwrap()
                .blocker
                .unwrap();
            match end {
                0 => {
                    app.world_mut().entity_mut(child).remove::<WindowRoot>();
                }
                1 => {
                    app.world_mut()
                        .entity_mut(parent_root)
                        .remove::<WindowRoot>();
                }
                _ => {
                    app.world_mut().entity_mut(parent).despawn();
                }
            }
            app.world_mut().flush();
            assert!(app.world().get_entity(blocker).is_err());
            if end != 0 {
                assert!(app.world().get_entity(child).is_err());
            }
        }
    }

    #[test]
    fn stray_modal_entity_does_not_keep_blocker() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        }).id();
        app.update();
        let parent = app.world().get::<WindowRoot>(root).unwrap().target_window;
        app.world_mut().spawn(WidgetryModalWindow { parent });
        app.update();
        assert!(
            app.world()
                .get::<ModalState>(parent)
                .unwrap()
                .blocker
                .is_none()
        );
        let child = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            template(move |_| Ok(WidgetryModalWindow { parent }))
        }).id();
        app.update();
        let blocker = app
            .world()
            .get::<ModalState>(parent)
            .unwrap()
            .blocker
            .unwrap();
        app.world_mut().entity_mut(child).despawn();
        app.world_mut().flush();
        assert!(app.world().get_entity(blocker).is_err());
        assert!(
            app.world()
                .get::<ModalState>(parent)
                .unwrap()
                .blocker
                .is_none()
        );
    }

    #[test]
    fn blocker_tracks_last_modal_child() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let parent = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                widgetry_window(parent, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            })
            .id();
        app.update();
        assert!(app.world().get::<ModalState>(parent).is_some());
        let children: Vec<_> = (0..2).map(|_| app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            template(move |_| Ok(WidgetryModalWindow { parent }))
        }).id()).collect();
        app.update();
        let blocker = app
            .world()
            .get::<ModalState>(parent)
            .unwrap()
            .blocker
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&ModalBlocker>()
                .iter(app.world())
                .count(),
            2
        );
        assert_eq!(app.world().get::<ChildOf>(blocker).unwrap().parent(), root);
        let node = app.world().get::<Node>(blocker).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!([node.left, node.right, node.top, node.bottom], [px(0); 4]);
        let picking = app.world().get::<Pickable>(blocker).unwrap();
        assert!(picking.should_block_lower && !picking.is_hoverable);
        assert_eq!(
            app.world().get::<GlobalZIndex>(blocker).unwrap().0,
            z_index::MODAL
        );
        app.world_mut().entity_mut(children[0]).despawn();
        app.world_mut().flush();
        assert!(app.world().get_entity(blocker).is_ok());
        app.world_mut()
            .entity_mut(children[1])
            .remove::<WidgetryModalWindow>();
        app.world_mut().flush();
        assert!(app.world().get_entity(blocker).is_err());
        assert!(
            app.world()
                .get::<ModalState>(parent)
                .unwrap()
                .blocker
                .is_none()
        );
    }

    #[test]
    fn invalid_parent_cleans_owned_child() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let parent = app.world_mut().spawn(Window::default()).id();
        let child = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            template(move |_| Ok(WidgetryModalWindow { parent }))
        }).id();
        app.update();
        assert!(app.world().get_entity(child).is_err());
        assert_eq!(
            app.world_mut().query::<&Window>().iter(app.world()).count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Camera>().iter(app.world()).count(),
            0
        );
    }
}
