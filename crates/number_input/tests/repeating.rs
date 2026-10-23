#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::pointer::{Location, PointerAction, PointerButton, PointerId},
    prelude::*,
    time::TimeUpdateStrategy,
    ui::InteractionDisabled,
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, i32)>);

fn fixture(pointer: PointerId, max: i32) -> (App, Entity, Location) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.insert_resource(bevy::picking::input::PointerInputSettings {
        is_mouse_enabled: true,
        is_touch_enabled: false,
    });
    app.add_plugins(WidgetryNumberInputPlugin)
        .add_message::<bevy::window::RequestRedraw>()
        .add_message::<bevy::window::WindowFocused>()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<i32>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value));
        },
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    let root = spawn_scene(app.world_mut(), bsn! {
        @WidgetryNumberInput<i32> { @value: {Some(0)}, @min: {Some(0)}, @max: {Some(max)}, @step: 3 }
        Node { position_type: PositionType::Absolute, left: px(40), top: px(40) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap();
    if pointer != PointerId::Mouse {
        app.world_mut().spawn(pointer);
    }
    for _ in 0..6 {
        app.update();
    }
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(226.0, 56.0),
    };
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    (app, root, at)
}

fn input(app: &mut App, pointer: PointerId, at: &Location, action: PointerAction) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    queue(app, pointer, at, action);
    app.update();
}

fn queue(app: &mut App, pointer: PointerId, at: &Location, action: PointerAction) {
    if pointer == PointerId::Mouse
        && let bevy::camera::NormalizedRenderTarget::Window(window) = &at.target
    {
        let input = match &action {
            PointerAction::Press(button) => Some((button, bevy::input::ButtonState::Pressed)),
            PointerAction::Release(button) => Some((button, bevy::input::ButtonState::Released)),
            _ => None,
        };
        if let PointerAction::Move { delta } = &action {
            app.world_mut()
                .write_message(bevy::window::WindowEvent::CursorMoved(
                    bevy::window::CursorMoved {
                        window: window.entity(),
                        position: at.position,
                        delta: Some(*delta),
                    },
                ));
            return;
        }
        if let Some((button, state)) = input {
            let button = match button {
                PointerButton::Primary => MouseButton::Left,
                PointerButton::Secondary => MouseButton::Right,
                PointerButton::Middle => MouseButton::Middle,
            };
            app.world_mut()
                .write_message(bevy::window::WindowEvent::MouseButtonInput(
                    bevy::input::mouse::MouseButtonInput {
                        button,
                        state,
                        window: window.entity(),
                    },
                ));
            return;
        }
    }
    bevy_widgetry_test_utils::queue_pointer(app, pointer, at.clone(), action);
}

fn focus_event(app: &mut App, window: Entity, focused: bool) {
    app.world_mut().get_mut::<Window>(window).unwrap().focused = focused;
    app.world_mut()
        .write_message(bevy::window::WindowEvent::WindowFocused(
            bevy::window::WindowFocused { window, focused },
        ));
    app.world_mut()
        .write_message(bevy::window::WindowFocused { window, focused });
}

fn advance(app: &mut App, milliseconds: u64) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        milliseconds,
    )));
    app.update();
}

fn value(app: &App, root: Entity) -> i32 {
    WidgetryNumberInput::<i32>::value(app.world(), root).unwrap()
}

#[test]
fn short_press_and_delayed_repeats_use_real_time_without_frame_catchup() {
    for pointer in bevy_widgetry_test_utils::pointer_ids() {
        let (mut app, root, at) = fixture(pointer, 100);
        advance(&mut app, 10_000);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(value(&app, root), 3);
        advance(&mut app, 399);
        assert_eq!(value(&app, root), 3);
        advance(&mut app, 1);
        assert_eq!(value(&app, root), 6);
        advance(&mut app, 99);
        assert_eq!(value(&app, root), 6);
        advance(&mut app, 1);
        assert_eq!(value(&app, root), 9);
        advance(&mut app, 2_000);
        assert_eq!(value(&app, root), 12);
        advance(&mut app, 99);
        assert_eq!(value(&app, root), 12);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 12);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 100);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 15);
        assert_eq!(
            app.world().resource::<Changes>().0,
            [3, 6, 9, 12, 15].map(|v| (root, v))
        );
    }
}

#[test]
fn endpoints_remain_enabled_and_only_real_changes_notify() {
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    let (mut app, root, at) = fixture(pointer, 10);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    advance(&mut app, 400);
    for _ in 0..8 {
        advance(&mut app, 100);
    }
    assert_eq!(value(&app, root), 10);
    assert_eq!(
        app.world().resource::<Changes>().0,
        [3, 6, 9, 10].map(|v| (root, v))
    );
    for button in app.world().get::<Children>(root).unwrap().iter() {
        assert!(app.world().get::<InteractionDisabled>(button).is_none());
    }
}

#[test]
fn cancel_and_inherited_disable_stop_repeats_and_repress_starts_a_new_delay() {
    for disable in [false, true] {
        let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
        let (mut app, root, at) = fixture(pointer, 100);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 400);
        assert_eq!(value(&app, root), 6);
        let parent = spawn_scene(app.world_mut(), bsn! { Node }).unwrap();
        if disable {
            app.world_mut().entity_mut(root).insert(ChildOf(parent));
            app.world_mut()
                .entity_mut(parent)
                .insert(InteractionDisabled);
            app.world_mut().flush();
        } else {
            input(&mut app, pointer, &at, PointerAction::Cancel);
        }
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 6);
        app.world_mut()
            .entity_mut(parent)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 6);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(value(&app, root), 9);
        advance(&mut app, 399);
        assert_eq!(value(&app, root), 9);
        advance(&mut app, 1);
        assert_eq!(value(&app, root), 12);
    }
}

#[test]
fn active_hold_requests_frames_and_release_stops_requests() {
    use bevy::window::RequestRedraw;
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    let (mut app, _, at) = fixture(pointer, 100);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    for delay in [0, 100, 300, 100] {
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        advance(&mut app, delay);
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    advance(&mut app, 100);
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}

#[test]
fn release_and_repress_in_one_frame_restart_the_delay() {
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    let (mut app, root, at) = fixture(pointer, 100);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    advance(&mut app, 400);
    assert_eq!(value(&app, root), 6);
    for action in [
        PointerAction::Release(PointerButton::Primary),
        PointerAction::Press(PointerButton::Primary),
    ] {
        queue(&mut app, pointer, &at, action);
    }
    advance(&mut app, 500);
    assert_eq!(value(&app, root), 9);
    advance(&mut app, 399);
    assert_eq!(value(&app, root), 9);
    advance(&mut app, 1);
    assert_eq!(value(&app, root), 12);
}

#[test]
fn unrelated_secondary_release_does_not_restart_the_primary_hold_delay() {
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    let (mut app, root, at) = fixture(pointer, 100);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    advance(&mut app, 400);
    assert_eq!(value(&app, root), 6);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Secondary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Secondary),
    );
    advance(&mut app, 100);
    assert_eq!(value(&app, root), 9);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    advance(&mut app, 500);
    assert_eq!(value(&app, root), 9);
}

#[test]
fn another_pointer_cannot_restart_or_cancel_the_owners_repeat_clock() {
    for pointer in bevy_widgetry_test_utils::pointer_ids() {
        let (mut app, root, at) = fixture(pointer, 100);
        let other = bevy_widgetry_test_utils::pointer_ids()
            .into_iter()
            .find(|p| *p != pointer)
            .unwrap();
        if other != PointerId::Mouse {
            app.world_mut().spawn(other);
        }
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 400);
        assert_eq!(value(&app, root), 6);
        input(
            &mut app,
            other,
            &at,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        input(
            &mut app,
            other,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 100);
        assert_eq!(value(&app, root), 9);
        input(
            &mut app,
            other,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        advance(&mut app, 100);
        assert_eq!(value(&app, root), 12);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        advance(&mut app, 500);
        assert_eq!(value(&app, root), 12);
    }
}

#[test]
fn native_mouse_window_focus_loss_cancels_without_cancelling_custom_pointer() {
    for pointer in bevy_widgetry_test_utils::pointer_ids() {
        let (mut app, root, at) = fixture(pointer, 100);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        input(
            &mut app,
            PointerId::Mouse,
            &at,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        focus_event(&mut app, window, false);
        advance(&mut app, 1_000);
        let expected = if pointer == PointerId::Mouse { 3 } else { 6 };
        assert_eq!(value(&app, root), expected);
        let mouse = app
            .world_mut()
            .query::<(&PointerId, &bevy::picking::pointer::PointerPressState)>()
            .iter(app.world())
            .find(|(id, _)| **id == PointerId::Mouse)
            .unwrap()
            .1;
        assert!(!mouse.is_any_pressed());
        focus_event(&mut app, window, true);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        advance(&mut app, 1_000);
        let expected = if pointer == PointerId::Mouse { 3 } else { 9 };
        assert_eq!(value(&app, root), expected);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(value(&app, root), expected + 3);
        advance(&mut app, 399);
        assert_eq!(value(&app, root), expected + 3);
        advance(&mut app, 1);
        assert_eq!(value(&app, root), expected + 6);
    }
}

#[test]
fn focus_loss_and_regain_in_one_frame_do_not_resume_the_mouse_hold() {
    let pointer = PointerId::Mouse;
    let (mut app, root, at) = fixture(pointer, 100);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    for focused in [false, true] {
        focus_event(&mut app, window, focused);
    }
    advance(&mut app, 400);
    assert_eq!(value(&app, root), 3);
    advance(&mut app, 1_000);
    assert_eq!(value(&app, root), 3);
}

#[test]
fn a_fresh_press_after_regaining_focus_in_one_frame_gets_its_own_hold() {
    for previous_hold in [false, true] {
        let pointer = PointerId::Mouse;
        let (mut app, root, at) = fixture(pointer, 100);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        if previous_hold {
            input(
                &mut app,
                pointer,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
        }
        for focused in [false, true] {
            focus_event(&mut app, window, focused);
        }
        queue(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 0);
        let start = if previous_hold { 6 } else { 3 };
        assert_eq!(value(&app, root), start);
        advance(&mut app, 399);
        assert_eq!(value(&app, root), start);
        advance(&mut app, 1);
        assert_eq!(value(&app, root), start + 3);
    }
}

#[test]
fn a_press_before_focus_loss_cannot_survive_a_same_frame_refocus() {
    let pointer = PointerId::Mouse;
    let (mut app, root, at) = fixture(pointer, 100);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    queue(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    focus_event(&mut app, window, false);
    focus_event(&mut app, window, true);
    advance(&mut app, 0);
    assert_eq!(value(&app, root), 3);
    advance(&mut app, 400);
    assert_eq!(value(&app, root), 3);
}

#[test]
fn the_first_accepted_button_owns_the_hold_until_its_release() {
    for pointer in bevy_widgetry_test_utils::pointer_ids() {
        let (mut app, root, at) = fixture(pointer, 100);
        queue(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Secondary),
        );
        queue(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        advance(&mut app, 0);
        assert_eq!(value(&app, root), 3);
        advance(&mut app, 400);
        assert_eq!(value(&app, root), 6);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 9);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Secondary),
        );
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 9);
    }
}

#[test]
fn secondary_and_middle_mouse_holds_cancel_on_window_focus_loss() {
    for button in [PointerButton::Secondary, PointerButton::Middle] {
        let (mut app, root, at) = fixture(PointerId::Mouse, 100);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        input(
            &mut app,
            PointerId::Mouse,
            &at,
            PointerAction::Press(button),
        );
        assert_eq!(value(&app, root), 3);
        focus_event(&mut app, window, false);
        advance(&mut app, 1_000);
        assert_eq!(value(&app, root), 3);
    }
}

#[test]
fn every_press_separated_by_native_focus_loss_starts_an_independent_hold() {
    let (mut app, root, at) = fixture(PointerId::Mouse, 100);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    queue(
        &mut app,
        PointerId::Mouse,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    focus_event(&mut app, window, false);
    focus_event(&mut app, window, true);
    queue(
        &mut app,
        PointerId::Mouse,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    advance(&mut app, 0);
    assert_eq!(value(&app, root), 6);
    advance(&mut app, 399);
    assert_eq!(value(&app, root), 6);
    advance(&mut app, 1);
    assert_eq!(value(&app, root), 9);
}
