use crate::scene::{MessageBoxAction, WidgetryMessageBox, WidgetryMessageBoxResultEvent};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};

/// 立即写入的一次性决议 state，防止同帧或 reentrant click 重复发布结果。
#[derive(Component, Default)]
pub(crate) struct MessageBoxState {
    /// 第一次有效结果在触发 observer 之前置为 true。
    resolved: bool,
}

/// 结果 observer command 应用后才进入关闭阶段。
#[derive(Component)]
pub(crate) struct MessageBoxClosing;

/// Activate 不支持 bubbling，使用私有桥接 event 将原始结果 button 交给 root 处理。
#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub(crate) struct MessageBoxClick {
    /// 初始为结果 button，沿 ChildOf 传播到 WidgetryMessageBox root。
    entity: Entity,
}

/// 只在结果 button 上安装；disabled button 不能通过程序 Activate 绕过限制。
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

/// state 立即写入后才排队触发结果，阻止同帧及结果 callback 中的 reentrant 决议。
pub(crate) fn handle_message_box_click(
    mut event: On<MessageBoxClick>,
    actions: Query<&MessageBoxAction>,
    mut roots: Query<&mut MessageBoxState, With<WidgetryMessageBox>>,
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
    commands.trigger(WidgetryMessageBoxResultEvent {
        entity: event.entity,
        result: action.0,
    });
}

/// 结果的所有同步 observer 完成后才应用 Closing，不在结果 dispatch 中直接销毁 root。
pub(crate) fn begin_closing(
    event: On<WidgetryMessageBoxResultEvent>,
    roots: Query<(), With<WidgetryMessageBox>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        commands.entity(event.entity).try_insert(MessageBoxClosing);
    }
}

/// Closing 是独立 lifecycle 边界，其 observer 完成后回收整个 owned root。
pub(crate) fn finish_closing(event: On<Add, MessageBoxClosing>, mut commands: Commands) {
    commands.entity(event.entity).try_despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::MessageBoxAction;
    use crate::{
        WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin,
        WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent, widgetry_message_box,
    };
    use bevy_widgetry_button::WidgetryButton;
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{WidgetryWindowControlsConfig, owned_widgetry_window};

    /// 记录结果和关闭阶段，证明 observer 读取 root 早于 Closing component 的添加。
    #[derive(Resource, Default)]
    struct Observed {
        results: Vec<WidgetryMessageBoxResult>,
        closing: usize,
    }

    /// 正文 button 忽略，同帧重复 click 只发一次结果；结果 callback 期间 root 和未关闭的 state 可读。
    #[test]
    fn result_precedes_closing_and_cleans_owned_resources() {
        let mut app = scene_app();
        app.add_plugins(WidgetryMessageBoxPlugin)
            .init_resource::<Observed>();
        app.add_observer(
            |event: On<WidgetryMessageBoxResultEvent>,
             roots: Query<(&MessageBoxState, Has<MessageBoxClosing>), With<WidgetryMessageBox>>,
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
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
        });
        app.update();
        let parent = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let root = app.world_mut().commands().spawn_scene(bsn! {
            widgetry_message_box(parent, "Resolve", WidgetryMessageBoxButtons::YesNoCancel, bsn_list![(@WidgetryButton Name("ordinary"))])
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
            .find(|(_, action)| action.0 == WidgetryMessageBoxResult::Yes)
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
            [WidgetryMessageBoxResult::Yes]
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
