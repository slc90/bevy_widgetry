use crate::scene::{MessageBox, MessageBoxAction, MessageBoxResultEvent};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};

/// 立即写入的一次性决议状态，防止同帧或重入点击重复发布结果。
#[derive(Component, Default)]
pub(crate) struct MessageBoxState {
    /// 第一次有效结果在触发 observer 之前置为 true。
    resolved: bool,
}

/// 结果 observer 命令应用后才进入关闭阶段。
#[derive(Component)]
pub(crate) struct MessageBoxClosing;

/// Activate 不支持冒泡，使用私有桥接事件将原始结果按钮交给根处理。
#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub(crate) struct MessageBoxClick {
    /// 初始为结果按钮，沿 ChildOf 传播到 MessageBox 根。
    entity: Entity,
}

/// 只在结果按钮上安装；禁用按钮不能通过程序激活绕过限制。
pub(crate) fn forward_activation(
    event: On<Activate>,
    buttons: Query<(), (With<MessageBoxAction>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if buttons.contains(event.entity) {
        commands.trigger(MessageBoxClick {
            entity: event.entity,
        });
    }
}

/// 状态立即写入后才排队触发结果，阻止同帧及结果回调中的重入决议。
pub(crate) fn handle_message_box_click(
    mut event: On<MessageBoxClick>,
    actions: Query<&MessageBoxAction>,
    mut roots: Query<&mut MessageBoxState, With<MessageBox>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.original_event_target()) else {
        return;
    };
    let Ok(mut state) = roots.get_mut(event.entity) else {
        return;
    };
    event.propagate(false);
    if state.resolved {
        return;
    }
    state.resolved = true;
    commands.trigger(MessageBoxResultEvent {
        entity: event.entity,
        result: action.0,
    });
}

/// 结果的所有同步 observer 完成后才应用 Closing，不在结果分发中直接销毁根。
pub(crate) fn begin_closing(
    event: On<MessageBoxResultEvent>,
    roots: Query<(), With<MessageBox>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        commands.entity(event.entity).try_insert(MessageBoxClosing);
    }
}

/// Closing 是独立生命周期边界，其 observer 完成后回收整个 owned 根。
pub(crate) fn finish_closing(event: On<Add, MessageBoxClosing>, mut commands: Commands) {
    commands.entity(event.entity).try_despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::MessageBoxAction;
    use crate::{
        MessageBox, MessageBoxButtons, MessageBoxPlugin, MessageBoxResult, MessageBoxResultEvent,
        message_box,
    };
    use bevy_widgetry_button::StyledButton;
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{WindowControlsConfig, owned_window};

    /// 记录结果和关闭阶段，证明 observer 读取根早于 Closing 的组件添加。
    #[derive(Resource, Default)]
    struct Observed {
        results: Vec<MessageBoxResult>,
        closing: usize,
    }

    /// 正文按钮忽略，同帧重复点击只发一次结果；结果回调期间根和未关闭状态可读。
    #[test]
    fn result_precedes_closing_and_cleans_owned_resources() {
        let mut app = scene_app();
        app.add_plugins(MessageBoxPlugin)
            .init_resource::<Observed>();
        app.add_observer(
            |event: On<MessageBoxResultEvent>,
             roots: Query<(&MessageBoxState, Has<MessageBoxClosing>), With<MessageBox>>,
             mut observed: ResMut<Observed>| {
                let (state, closing) = roots.get(event.entity).unwrap();
                assert!(state.resolved);
                assert!(!closing);
                observed.results.push(event.result);
            },
        );
        app.add_observer(
            |_event: On<Add, MessageBoxClosing>, mut observed: ResMut<Observed>| {
                assert_eq!(observed.results.len(), 1);
                observed.closing += 1;
            },
        );
        app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
        });
        app.update();
        let parent = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let root = app.world_mut().commands().spawn_scene(bsn! {
            message_box(parent, "Resolve", MessageBoxButtons::YesNoCancel, bsn_list![(template(|_| Ok(StyledButton)) Name("ordinary"))])
        }).id();
        app.update();
        let ordinary = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "ordinary")
            .unwrap()
            .0;
        app.world_mut().trigger(Activate { entity: ordinary });
        app.world_mut().flush();
        assert!(app.world().resource::<Observed>().results.is_empty());
        assert!(app.world().get_entity(root).is_ok());
        let action = app
            .world_mut()
            .query::<(Entity, &MessageBoxAction)>()
            .iter(app.world())
            .find(|(_, action)| action.0 == MessageBoxResult::Yes)
            .unwrap()
            .0;
        app.world_mut()
            .commands()
            .trigger(Activate { entity: action });
        app.world_mut()
            .commands()
            .trigger(Activate { entity: action });
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<Observed>().results,
            [MessageBoxResult::Yes]
        );
        assert_eq!(app.world().resource::<Observed>().closing, 1);
        assert!(app.world().get_entity(root).is_err());
        assert_eq!(
            app.world_mut().query::<&Window>().iter(app.world()).count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Camera>().iter(app.world()).count(),
            1
        );
    }
}
