#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::pointer::{Location, PointerAction, PointerButton, PointerId},
    prelude::*,
    time::TimeUpdateStrategy,
    ui::InteractionDisabled,
    ui_widgets::{ActivateOnPress, Button, ValueChange},
    window::RequestRedraw,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_number_input::{
    WidgetryNumberInputEditor, WidgetryNumberInputHeadless as Number,
    WidgetryNumberInputHeadlessPlugin, WidgetryNumberInputRepeat, WidgetryNumberInputStep,
};
use bevy_widgetry_text_field::{WidgetryTextFieldEditor, WidgetryTextFieldHeadless};
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<i32>);

fn fixture(read_only: bool) -> (App, Entity, Entity, Entity, PointerId, Location) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.add_plugins(WidgetryNumberInputHeadlessPlugin)
        .add_message::<RequestRedraw>()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<i32>>, mut changes: ResMut<Changes>| changes.0.push(event.value),
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    let root=spawn_scene(app.world_mut(),bsn! {
        @Number<i32> {@value:{Some(0)},@min:{Some(0)},@max:{Some(100)},@read_only:read_only}
        WidgetryNumberInputEditor {field:#Field}
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(40)}
        template(move |_|Ok(UiTargetCamera(camera)))
        Children [#Field @WidgetryTextFieldHeadless {editor:#Editor,@read_only:read_only}
            Children [#Editor @WidgetryTextFieldEditor {root:#Field} Node {width:px(180),height:px(36)}]]
    }).unwrap();
    let trigger=spawn_scene(app.world_mut(),bsn! {
        WidgetryNumberInputStep {root:root,increase:true} WidgetryNumberInputRepeat
        Button ActivateOnPress
        Node {position_type:PositionType::Absolute,left:px(300),top:px(40),width:px(40),height:px(40)}
        template(move |_|Ok(UiTargetCamera(camera)))
    }).unwrap();
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    app.world_mut().spawn(pointer);
    for _ in 0..4 {
        app.update();
    }
    let editor = Number::<i32>::editor_entity(app.world(), root).unwrap();
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(320.0, 60.0),
    };
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    (app, root, editor, trigger, pointer, at)
}

fn input(app: &mut App, pointer: PointerId, at: &Location, action: PointerAction) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    bevy_widgetry_test_utils::queue_pointer(app, pointer, at.clone(), action);
    app.update();
}
fn advance(app: &mut App, ms: u64) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        ms,
    )));
    app.update();
}
fn value(app: &App, root: Entity) -> i32 {
    Number::<i32>::value(app.world(), root).unwrap()
}
fn clear_redraw(app: &mut App) {
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
}
fn redraw(app: &App) -> bool {
    !app.world().resource::<Messages<RequestRedraw>>().is_empty()
}

#[test]
fn public_repeat_runs_headless_after_idle_without_catchup_or_trailing_wakes() {
    let (mut app, root, _, _, pointer, at) = fixture(false);
    assert!(!app.is_plugin_added::<bevy_widgetry_number_input::WidgetryNumberInputPlugin>());
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    advance(&mut app, 5000);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert_eq!(value(&app, root), 1);
    assert!(redraw(&app));
    advance(&mut app, 399);
    assert_eq!(value(&app, root), 1);
    advance(&mut app, 1);
    assert_eq!(value(&app, root), 2);
    advance(&mut app, 100);
    assert_eq!(value(&app, root), 3);
    advance(&mut app, 1000);
    assert_eq!(value(&app, root), 4);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    clear_redraw(&mut app);
    advance(&mut app, 1000);
    assert_eq!(value(&app, root), 4);
    assert!(!redraw(&app));
    assert_eq!(app.world().resource::<Changes>().0, vec![1, 2, 3, 4]);
}

#[test]
fn root_or_editor_disable_cancels_an_external_hold_until_a_fresh_press() {
    for disable_editor in [false, true] {
        let (mut app, root, editor, _, pointer, at) = fixture(false);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        let target = if disable_editor { editor } else { root };
        app.world_mut()
            .entity_mut(target)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        clear_redraw(&mut app);
        advance(&mut app, 500);
        assert_eq!(value(&app, root), 1);
        assert!(!redraw(&app));
        app.world_mut()
            .entity_mut(target)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        clear_redraw(&mut app);
        advance(&mut app, 1000);
        advance(&mut app, 500);
        assert_eq!(value(&app, root), 1);
        assert!(!redraw(&app));
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
        assert_eq!(value(&app, root), 2);
        advance(&mut app, 400);
        assert_eq!(value(&app, root), 3);
    }
}

#[test]
fn readonly_or_lost_step_relation_stops_repeat_without_waking() {
    let (mut app, root, _, _, pointer, at) = fixture(true);
    clear_redraw(&mut app);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    advance(&mut app, 1000);
    assert_eq!(value(&app, root), 0);
    assert!(!redraw(&app));
    let (mut app, root, _, trigger, pointer, at) = fixture(false);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .entity_mut(trigger)
        .remove::<WidgetryNumberInputStep>();
    clear_redraw(&mut app);
    advance(&mut app, 500);
    assert!(!redraw(&app));
    app.world_mut()
        .entity_mut(trigger)
        .insert(WidgetryNumberInputStep {
            root,
            increase: true,
        });
    advance(&mut app, 1000);
    advance(&mut app, 500);
    assert_eq!(value(&app, root), 1);
    assert!(!redraw(&app));
}

#[test]
fn adding_repeat_to_an_existing_press_does_not_resume_that_press() {
    let (mut app, root, _, trigger, pointer, at) = fixture(false);
    app.world_mut()
        .entity_mut(trigger)
        .remove::<WidgetryNumberInputRepeat>();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .entity_mut(trigger)
        .insert(WidgetryNumberInputRepeat);
    advance(&mut app, 500);
    advance(&mut app, 500);
    assert_eq!(value(&app, root), 1);
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
    advance(&mut app, 400);
    assert_eq!(value(&app, root), 3);
}

#[test]
fn disable_and_reenable_before_the_next_tick_still_cancels_the_old_hold() {
    for editor_disabled in [false, true] {
        let (mut app, root, editor, _, pointer, at) = fixture(false);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        let target = if editor_disabled { editor } else { root };
        app.world_mut()
            .entity_mut(target)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(target)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        clear_redraw(&mut app);
        advance(&mut app, 500);
        assert_eq!(value(&app, root), 1);
        assert!(!redraw(&app));
    }
}

#[test]
fn disable_from_the_first_step_notification_cancels_that_same_press() {
    for editor_disabled in [false, true] {
        let (mut app, root, editor, _, pointer, at) = fixture(false);
        let target = if editor_disabled { editor } else { root };
        app.add_observer(move |event: On<ValueChange<i32>>, mut commands: Commands| {
            if event.source == root && event.value == 1 {
                commands.queue(move |world: &mut World| {
                    world.entity_mut(target).insert(InteractionDisabled);
                    world.flush();
                    world.entity_mut(target).remove::<InteractionDisabled>();
                    world.flush();
                });
            }
        });
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(value(&app, root), 1);
        clear_redraw(&mut app);
        advance(&mut app, 500);
        assert_eq!(value(&app, root), 1);
        assert!(!redraw(&app));
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
        advance(&mut app, 400);
        assert_eq!(value(&app, root), 3);
    }
}

#[test]
fn cancellation_or_root_despawn_stops_headless_repeat_and_redraws() {
    let (mut app, root, _, _, pointer, at) = fixture(false);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(&mut app, pointer, &at, PointerAction::Cancel);
    clear_redraw(&mut app);
    advance(&mut app, 1000);
    assert_eq!(value(&app, root), 1);
    assert!(!redraw(&app));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert_eq!(value(&app, root), 2);
    app.world_mut().despawn(root);
    clear_redraw(&mut app);
    advance(&mut app, 1000);
    assert_eq!(app.world().resource::<Changes>().0, vec![1, 2]);
    assert!(!redraw(&app));
}
