use crate::scene::{MessageBoxAction, WidgetryMessageBox, WidgetryMessageBoxResultEvent};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};

#[derive(Component, Default)]
pub(crate) struct MessageBoxState {
    resolved: bool,
}

#[derive(Component)]
pub(crate) struct MessageBoxClosing;

#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub(crate) struct MessageBoxClick {
    entity: Entity,
}

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
    let root = event.entity;
    let result = action.0;
    commands.queue(move |world: &mut World| {
        world.trigger(WidgetryMessageBoxResultEvent {
            entity: root,
            result,
        });
        world.flush();
        // result observer 的 deferred command 仍需访问 dialog root。
        // 先 flush 再 Closing，避免关闭清理先销毁 callback 所需的上下文。
        if let Ok(mut entity) = world.get_entity_mut(root) {
            entity.insert(MessageBoxClosing);
        }
    });
}

pub(crate) fn finish_closing(event: On<Add, MessageBoxClosing>, mut commands: Commands) {
    commands.entity(event.entity).try_despawn();
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
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

    #[derive(Resource, Default)]
    struct Observed {
        results: Vec<WidgetryMessageBoxResult>,
        closing: usize,
    }

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
