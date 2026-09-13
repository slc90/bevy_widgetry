use crate::window_root::{WindowInitialized, WindowRoot};
use bevy::prelude::*;

/// 将子 Widgetry UI 根设为父原生窗口的指针模态窗口。
/// parent 必须是已绑定 Widgetry 根的原生 Window 实体，否则子根会被清理。
/// 只遮挡父 UI 指针交互，不捕获键盘焦点，也不建立 OS 模态关系。
#[derive(Component, Clone, Copy, Debug)]
pub struct ModalWindow {
    /// 父原生 Window 实体，不是父 UI 根。
    pub parent: Entity,
}

/// 原生父窗口只保存唯一遮罩；子窗口关系从 ECS 推导。
#[derive(Component, Default)]
pub(crate) struct ModalState {
    /// 当前父 UI 根上的遮罩，无子窗口时为空。
    blocker: Option<Entity>,
}

/// 覆盖整个父根并阻断底层拾取，不接收自身悬停。
#[derive(Component)]
struct ModalBlocker;

/// 场景与绑定完成后协调父子关系，移除最后一个子根也走同一路径。
pub(crate) fn sync_modal_windows(world: &mut World) {
    let roots: Vec<_> = world
        .query_filtered::<(Entity, &WindowRoot), With<WindowInitialized>>()
        .iter(world)
        .map(|(entity, root)| (entity, root.target_window))
        .collect();
    let children: Vec<_> = world
        .query_filtered::<(Entity, &ModalWindow), (With<WindowRoot>, With<WindowInitialized>)>()
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
        let needed = world
            .query_filtered::<&ModalWindow, (With<WindowRoot>, With<WindowInitialized>)>()
            .iter(world)
            .any(|modal| modal.parent == parent);
        let next = match (needed, blocker) {
            (true, None) => {
                let blocker = world.commands().spawn_scene(bsn! {
                    template(|_| Ok(ModalBlocker))
                    Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), bottom: px(0) }
                    GlobalZIndex(100_000)
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

/// 已初始化根补加模态关系时，在组件插入完成后协调遮罩。
pub(crate) fn modal_added(_event: On<Add, ModalWindow>, mut commands: Commands) {
    commands.queue(sync_modal_windows);
}

/// Remove 的查询仍含旧组件，延后到命令应用阶段重新计算关系。
pub(crate) fn modal_removed(_event: On<Remove, ModalWindow>, mut commands: Commands) {
    commands.queue(sync_modal_windows);
}

/// 根解除绑定时释放其父窗口遮罩，并重新校验仍存活的模态子根。
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
}

/// 原生父窗口结束生命周期时，遮罩和模态子根不能继续存活。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WindowControlsConfig, WindowPlugin, owned_window, widgetry_window, window};
    use bevy_widgetry_test_utils::scene_app;

    /// 已初始化根补加模态关系后立即建立遮罩，移除子根或结束父生命周期立即释放关系。
    #[test]
    fn modal_lifecycle_syncs_without_another_frame() {
        for end in 0..3 {
            let mut app = scene_app();
            app.add_plugins(WindowPlugin);
            let parent_root = app.world_mut().commands().spawn_scene(bsn! {
                owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            }).id();
            let child = app.world_mut().commands().spawn_scene(bsn! {
                owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            }).id();
            app.update();
            let parent = app
                .world()
                .get::<WindowRoot>(parent_root)
                .unwrap()
                .target_window;
            app.world_mut()
                .entity_mut(child)
                .insert(ModalWindow { parent });
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

    /// 普通实体误挂模态关系既不能创建遮罩，也不能在最后一个有效子根退出后维持遮罩。
    #[test]
    fn stray_modal_entity_does_not_keep_blocker() {
        let mut app = scene_app();
        app.add_plugins(WindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
        }).id();
        app.update();
        let parent = app.world().get::<WindowRoot>(root).unwrap().target_window;
        app.world_mut().spawn(ModalWindow { parent });
        app.update();
        assert!(
            app.world()
                .get::<ModalState>(parent)
                .unwrap()
                .blocker
                .is_none()
        );
        let child = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            template(move |_| Ok(ModalWindow { parent }))
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

    /// 外部父窗口也能承载唯一遮罩，多个模态子窗口按最后引用释放遮罩。
    #[test]
    fn blocker_tracks_last_modal_child() {
        let mut app = scene_app();
        app.add_plugins(WindowPlugin);
        let parent = app
            .world_mut()
            .spawn(widgetry_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                window(parent, camera, WindowControlsConfig::default(), bsn_list![], bsn_list![])
            })
            .id();
        app.update();
        assert!(app.world().get::<ModalState>(parent).is_some());
        let children: Vec<_> = (0..2).map(|_| app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            template(move |_| Ok(ModalWindow { parent }))
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
            1
        );
        assert_eq!(app.world().get::<ChildOf>(blocker).unwrap().parent(), root);
        let node = app.world().get::<Node>(blocker).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!([node.left, node.right, node.top, node.bottom], [px(0); 4]);
        let picking = app.world().get::<Pickable>(blocker).unwrap();
        assert!(picking.should_block_lower && !picking.is_hoverable);
        assert!(app.world().get::<GlobalZIndex>(blocker).unwrap().0 < i32::MAX);
        app.world_mut().entity_mut(children[0]).despawn();
        app.world_mut().flush();
        assert!(app.world().get_entity(blocker).is_ok());
        app.world_mut()
            .entity_mut(children[1])
            .remove::<ModalWindow>();
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

    /// 未绑定 Widgetry 根的原生父窗口不能静默退化为 modeless，子资源一并释放。
    #[test]
    fn invalid_parent_cleans_owned_child() {
        let mut app = scene_app();
        app.add_plugins(WindowPlugin);
        let parent = app.world_mut().spawn(Window::default()).id();
        let child = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            template(move |_| Ok(ModalWindow { parent }))
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
