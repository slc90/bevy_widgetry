use crate::scene::{
    MessageBoxAction, WidgetryMessageBox, WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent,
};
use crate::{MessageBoxError, MessageBoxFailure};
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
    mut roots: Query<
        (
            Option<&mut MessageBoxState>,
            Option<&bevy_widgetry_window::internal::WindowRoot>,
        ),
        With<WidgetryMessageBox>,
    >,
    mut commands: Commands,
) -> Result {
    let Ok(action) = actions.get(event.original_event_target()) else {
        return Ok(());
    };
    let Ok((state, binding)) = roots.get_mut(event.entity) else {
        return Ok(());
    };
    event.propagate(false);
    let mut state = state.ok_or_else(|| {
        MessageBoxFailure::new(
            MessageBoxError::Resolve {
                root: event.entity,
                result: action.0,
                missing: "MessageBoxState",
            },
            bevy_widgetry_diagnostics::ErrorFrame::new("message_box.click.resolve")
                .field("message_box", event.entity)
                .field("result", format_args!("{:?}", action.0))
                .field(
                    "native",
                    format_args!("{:?}", binding.map(|binding| binding.target_window)),
                ),
        )
        .into_bevy_error()
    })?;
    if state.resolved {
        return Ok(());
    }
    state.resolved = true;
    let root = event.entity;
    let result = action.0;
    commands.queue(move |world: &mut World| finish_resolution(world, root, result));
    Ok(())
}

fn finish_resolution(world: &mut World, root: Entity, result: WidgetryMessageBoxResult) {
    world.trigger(WidgetryMessageBoxResultEvent {
        entity: root,
        result,
    });
    world.flush();
    // Deferred commands from result observers still need the dialog root.
    // Flush before marking Closing so cleanup cannot destroy callback context first.
    if let Ok(mut entity) = world.get_entity_mut(root) {
        entity.insert(MessageBoxClosing);
    }
}

impl WidgetryMessageBox {
    pub fn resolve(
        world: &mut World,
        root: Entity,
        result: WidgetryMessageBoxResult,
    ) -> Result<bool, MessageBoxFailure> {
        if world.get::<Self>(root).is_none() {
            return Err(MessageBoxFailure::new(
                MessageBoxError::Resolve {
                    root,
                    result,
                    missing: "WidgetryMessageBox",
                },
                crate::error::frame(world, root, root, "message_box.resolve")
                    .field("result", format_args!("{result:?}")),
            ));
        }
        let Some(mut state) = world.get_mut::<MessageBoxState>(root) else {
            return Err(MessageBoxFailure::new(
                MessageBoxError::Resolve {
                    root,
                    result,
                    missing: "MessageBoxState",
                },
                crate::error::frame(world, root, root, "message_box.resolve")
                    .field("result", format_args!("{result:?}")),
            ));
        };
        if state.resolved {
            return Ok(false);
        }
        state.resolved = true;
        finish_resolution(world, root, result);
        Ok(true)
    }
}

pub(crate) fn escape(
    mut event: On<bevy::input_focus::FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    roots: Query<(), With<WidgetryMessageBox>>,
    editors: Query<&bevy::text::EditableText>,
    mut commands: Commands,
) {
    if !roots.contains(event.event_target())
        || event.input.key_code != KeyCode::Escape
        || !event.input.state.is_pressed()
        || editors.get(event.focused_entity).is_ok_and(|editor| {
            editor.is_composing()
                || editor.pending_edits.iter().any(|edit| {
                    matches!(
                        edit,
                        bevy::text::TextEdit::ImeSetCompose { .. }
                            | bevy::text::TextEdit::ImeCommit { .. }
                    )
                })
        })
    {
        return;
    }
    event.propagate(false);
    let root = event.event_target();
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryMessageBox>(root).is_none() {
            return Ok(());
        }
        WidgetryMessageBox::resolve(world, root, WidgetryMessageBoxResult::Cancel)
            .map(|_| ())
            .map_err(MessageBoxFailure::into_bevy_error)
    });
}

pub(crate) fn close_requests(
    mut events: MessageReader<bevy::window::WindowCloseRequested>,
    roots: Query<Entity, With<WidgetryMessageBox>>,
    mut commands: Commands,
) {
    let windows: Vec<_> = events.read().map(|event| event.window).collect();
    if windows.is_empty() {
        return;
    }
    for root in &roots {
        let windows = windows.clone();
        commands.queue(move |world: &mut World| -> Result {
            if bevy_widgetry_window::widgetry_window_target(world, root)
                .is_some_and(|native| windows.contains(&native))
            {
                WidgetryMessageBox::resolve(world, root, WidgetryMessageBoxResult::Cancel)
                    .map_err(MessageBoxFailure::into_bevy_error)?;
            }
            Ok(())
        });
    }
}

pub(crate) fn finish_closing(event: On<Add<MessageBoxClosing>>, mut commands: Commands) {
    commands.entity(event.entity).try_despawn();
}

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
    use bevy_widgetry_window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
    };

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
            |_event: On<Add<MessageBoxClosing>>, mut observed: ResMut<Observed>| {
                assert_eq!(observed.results.len(), 1);
                observed.closing += 1;
            },
        );
        app.world_mut().commands().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
        });
        app.update();
        let parent = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let root = app.world_mut().commands().spawn_scene(bsn! {
            @widgetry_message_box(parent, "Resolve", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),  bsn_list!{@WidgetryButton Name("ordinary")})
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

    #[test]
    fn recognized_result_action_reports_missing_resolution_state_to_the_host() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let mut app = scene_app();
        app.add_plugins(WidgetryMessageBoxPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let parent = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list! {}, bsn_list! {})
        }).unwrap().id();
        let native = bevy_widgetry_window::widgetry_window_target(app.world(), parent).unwrap();
        let root = app.world_mut().spawn_scene(bsn! { @widgetry_message_box(native, "Question", WidgetryMessageBoxButtons::Ok, Default::default(), bsn_list! {}) }).unwrap().id();
        app.update();
        let action = app
            .world_mut()
            .query_filtered::<Entity, With<MessageBoxAction>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(root).remove::<MessageBoxState>();
        let errors = ErrorCapture::default();
        errors.run(|| {
            app.world_mut().trigger(Activate { entity: action });
            app.world_mut().flush();
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(error) = cause {
            if let Some(MessageBoxError::Resolve {
                root: actual,
                result,
                missing,
            }) = error.downcast_ref::<MessageBoxError>()
            {
                assert_eq!(*actual, root);
                assert_eq!(*result, WidgetryMessageBoxResult::Ok);
                assert_eq!(*missing, "MessageBoxState");
                found = true;
            }
            cause = error.source();
        }
        assert!(found);
        assert!(app.world().get::<MessageBoxClosing>(root).is_none());
    }
}
