use bevy::{
    camera::RenderTarget,
    ecs::{
        message::MessageCursor,
        schedule::{ScheduleCleanupPolicy, ScheduleError},
    },
    picking::{
        PickingSystems,
        hover::{generate_hovermap, update_interactions},
        input::PointerInputSettings,
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    window::{WindowEvent, WindowRef},
};
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Resource, Default)]
struct MouseInputState {
    events: MessageCursor<WindowEvent>,
    cursor: Vec2,
    cancelled: bool,
}

pub(super) fn install(app: &mut App) {
    super::routing::install(app);
    app.add_systems(Startup, replace);
}

fn replace(world: &mut World) -> Result {
    world.schedule_scope(First, |world, schedule| -> Result {
        let removed = match schedule.remove_systems_in_set(
            bevy::picking::input::mouse_pick_events,
            world,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        ) {
            Ok(count) => count,
            Err(ScheduleError::SetNotFound) => 0,
            Err(error) => {
                widgetry_error!(%error, "Native Mouse writer 替换失败");
                return Err(crate::scene::logged_error(format!(
                    "Native Mouse writer replacement failed: {error}"
                )));
            }
        };
        if removed > 0 {
            world.init_resource::<MouseInputState>();
            schedule.add_systems(mouse_inputs.in_set(PickingSystems::Input).run_if(
                |settings: Option<Res<PointerInputSettings>>| {
                    settings.is_some_and(|settings| settings.is_mouse_enabled)
                },
            ));
            world.schedule_scope(PreUpdate, |_, schedule| {
                schedule.add_systems(
                    restore_cancelled_mouse_hits
                        .in_set(PickingSystems::Hover)
                        .after(generate_hovermap)
                        .before(update_interactions),
                );
            });
        }
        Ok(())
    })
}

fn mouse_inputs(
    events: Res<Messages<WindowEvent>>,
    mut state: ResMut<MouseInputState>,
    mut inputs: MessageWriter<PointerInput>,
) {
    let state = &mut *state;
    for event in state.events.read(&events) {
        let (window, action) = match event {
            WindowEvent::CursorMoved(event) => {
                let delta = event.position - state.cursor;
                state.cursor = event.position;
                (event.window, PointerAction::Move { delta })
            }
            WindowEvent::MouseButtonInput(event) => {
                let button = match event.button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Right => PointerButton::Secondary,
                    MouseButton::Middle => PointerButton::Middle,
                    _ => continue,
                };
                (
                    event.window,
                    if event.state.is_pressed() {
                        PointerAction::Press(button)
                    } else {
                        PointerAction::Release(button)
                    },
                )
            }
            WindowEvent::MouseWheel(event) => (
                event.window,
                PointerAction::Scroll {
                    x: event.x,
                    y: event.y,
                    unit: event.unit,
                    phase: event.phase,
                },
            ),
            // A cancel must stay between the old press and a fresh press later in this same event batch.
            WindowEvent::WindowFocused(event) if !event.focused => {
                state.cancelled = true;
                (event.window, PointerAction::Cancel)
            }
            _ => continue,
        };
        let Some(target) = RenderTarget::Window(WindowRef::Entity(window)).normalize(None) else {
            continue;
        };
        let location = Location {
            target,
            position: state.cursor,
        };
        inputs.write(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        if matches!(action, PointerAction::Cancel) {
            // Bevy's receive only updates button flags for Release, while Cancel clears click/drag state.
            for button in PointerButton::iter() {
                inputs.write(PointerInput::new(
                    PointerId::Mouse,
                    location.clone(),
                    PointerAction::Release(button),
                ));
            }
        }
    }
}

fn restore_cancelled_mouse_hits(world: &mut World) {
    if std::mem::take(&mut world.resource_mut::<MouseInputState>().cancelled) {
        // Bevy filters cancelled pointers for the whole frame, including presses after cancellation.
        super::refresh_hover(world, PointerId::Mouse);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::{
        input::{
            ButtonState,
            mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
            touch::TouchPhase,
        },
        window::{CursorMoved, PrimaryWindow, WindowFocused},
    };

    fn app(replace_writer: bool) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WindowEvent>()
            .add_message::<PointerInput>()
            .init_resource::<PointerInputSettings>()
            .add_systems(
                First,
                bevy::picking::input::mouse_pick_events.in_set(PickingSystems::Input),
            );
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        if replace_writer {
            install(&mut app);
        }
        (app, window)
    }

    fn inputs(app: &App, cursor: &mut MessageCursor<PointerInput>) -> Vec<PointerInput> {
        cursor
            .read(app.world().resource::<Messages<PointerInput>>())
            .cloned()
            .collect()
    }

    #[test]
    fn native_translation_matches_bevy_and_inserts_cancel_in_event_order() {
        let mut outputs = Vec::new();
        for replacement in [false, true] {
            let (mut app, window) = app(replacement);
            app.update();
            let mut cursor = app
                .world()
                .resource::<Messages<PointerInput>>()
                .get_cursor_current();
            let events = [
                WindowEvent::CursorMoved(CursorMoved {
                    window,
                    position: Vec2::new(20.0, 30.0),
                    delta: None,
                }),
                WindowEvent::MouseButtonInput(MouseButtonInput {
                    window,
                    button: MouseButton::Left,
                    state: ButtonState::Pressed,
                }),
                WindowEvent::WindowFocused(WindowFocused {
                    window,
                    focused: false,
                }),
                WindowEvent::WindowFocused(WindowFocused {
                    window,
                    focused: true,
                }),
                WindowEvent::MouseButtonInput(MouseButtonInput {
                    window,
                    button: MouseButton::Right,
                    state: ButtonState::Pressed,
                }),
                WindowEvent::MouseWheel(MouseWheel {
                    window,
                    unit: MouseScrollUnit::Pixel,
                    x: 2.0,
                    y: -3.0,
                    phase: TouchPhase::Moved,
                }),
                WindowEvent::CursorMoved(CursorMoved {
                    window,
                    position: Vec2::new(25.0, 40.0),
                    delta: Some(Vec2::new(5.0, 10.0)),
                }),
                WindowEvent::MouseButtonInput(MouseButtonInput {
                    window,
                    button: MouseButton::Middle,
                    state: ButtonState::Released,
                }),
                WindowEvent::MouseButtonInput(MouseButtonInput {
                    window,
                    button: MouseButton::Back,
                    state: ButtonState::Pressed,
                }),
            ];
            app.world_mut().write_message_batch(events);
            app.update();
            outputs.push(inputs(&app, &mut cursor));
        }
        let baseline = &outputs[0];
        let replacement = &outputs[1];
        assert_eq!(replacement.len(), baseline.len() + 4);
        assert!(matches!(replacement[2].action, PointerAction::Cancel));
        for (input, button) in replacement[3..6].iter().zip(PointerButton::iter()) {
            assert!(matches!(input.action, PointerAction::Release(released) if released == button));
        }
        for (original, replaced) in baseline.iter().zip(
            replacement
                .iter()
                .enumerate()
                .filter_map(|(index, input)| (!(2..6).contains(&index)).then_some(input)),
        ) {
            assert_eq!(original.pointer_id, replaced.pointer_id);
            assert_eq!(original.location, replaced.location);
            assert_eq!(
                format!("{:?}", original.action),
                format!("{:?}", replaced.action)
            );
        }
    }

    #[test]
    fn startup_handoff_keeps_cursor_and_never_replays_the_first_batch() {
        let (mut app, window) = app(true);
        app.world_mut().write_message_batch([
            WindowEvent::CursorMoved(CursorMoved {
                window,
                position: Vec2::new(41.0, 52.0),
                delta: None,
            }),
            WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button: MouseButton::Left,
                state: ButtonState::Pressed,
            }),
        ]);
        let mut cursor = app
            .world()
            .resource::<Messages<PointerInput>>()
            .get_cursor();
        app.update();
        assert_eq!(inputs(&app, &mut cursor).len(), 2);
        app.world_mut()
            .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button: MouseButton::Left,
                state: ButtonState::Released,
            }));
        app.update();
        let next = inputs(&app, &mut cursor);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].location.position, Vec2::new(41.0, 52.0));
        assert!(matches!(
            next[0].action,
            PointerAction::Release(PointerButton::Primary)
        ));
    }
}
