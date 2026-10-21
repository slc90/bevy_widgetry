#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
    },
    input_focus::{FocusCause, FocusedInput, InputFocus},
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::ValueChange,
};
use bevy_widgetry_slider::{
    SliderDragEnded, SliderDragOutcome, SliderDragStarted, WidgetrySlider,
    WidgetrySliderOrientation, WidgetrySliderPlugin,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, add_keyboard_dispatch, picking_app, pointer_ids, queue_pointer,
    spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Changes(Vec<(f32, bool)>);

#[derive(Debug, PartialEq)]
enum Notice {
    Start(f32),
    Value(f32, bool),
    End(f32, SliderDragOutcome),
}

#[derive(Resource, Default)]
struct Lifecycle(Vec<Notice>);

fn update(app: &mut App) {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}

fn fixture(id: PointerId, vertical: bool, stepped: bool) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetrySliderPlugin)
        .init_resource::<Changes>()
        .init_resource::<Lifecycle>();
    app.add_observer(
        |event: On<SliderDragStarted>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetrySlider::value(&world, event.entity).unwrap(),
                event.value
            );
            assert!(world.get::<Pressed>(event.entity).is_some());
            world
                .resource_mut::<Lifecycle>()
                .0
                .push(Notice::Start(event.value));
        },
    );
    app.add_observer(
        |event: On<SliderDragEnded>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetrySlider::value(&world, event.entity).unwrap(),
                event.value
            );
            assert!(world.get::<Pressed>(event.entity).is_none());
            assert!(
                !world
                    .resource::<PointerCaptureMap>()
                    .0
                    .iter()
                    .any(|(_, (owner, _))| *owner == event.entity)
            );
            world
                .resource_mut::<Lifecycle>()
                .0
                .push(Notice::End(event.value, event.outcome));
        },
    );
    app.set_error_handler(ErrorCapture::handler());
    app.add_observer(
        |event: On<ValueChange<f32>>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetrySlider::value(&world, event.source).unwrap(),
                event.value
            );
            world
                .resource_mut::<Changes>()
                .0
                .push((event.value, event.is_final));
            world
                .resource_mut::<Lifecycle>()
                .0
                .push(Notice::Value(event.value, event.is_final));
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let orientation = if vertical {
        WidgetrySliderOrientation::Vertical
    } else {
        WidgetrySliderOrientation::Horizontal
    };
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetrySlider { @max: 10.0, @value: {Some(if stepped {0.0} else {5.0})}, @step: {stepped.then_some(3.0)}, @orientation: orientation }
        Node { position_type: PositionType::Absolute, left: px(40), top: px(40) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    update(&mut app);
    update(&mut app);
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(140.0, 54.0),
    };
    (app, root, window, at)
}

fn input(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    update(app);
}
fn moved(app: &mut App, id: PointerId, at: &mut Location, point: Vec2) {
    let delta = point - at.position;
    at.position = point;
    input(app, id, at, PointerAction::Move { delta });
}
fn value(app: &App, root: Entity) -> f32 {
    WidgetrySlider::value(app.world(), root).unwrap()
}

#[test]
fn unmoved_thumb_has_one_start_and_completed_end_without_value_notifications() {
    for id in pointer_ids() {
        let (mut app, root, _, mut at) = fixture(id, false, false);
        moved(&mut app, id, &mut at, axis(false, 5.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Lifecycle>().0, [Notice::Start(5.0)]);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        input(&mut app, id, &at, PointerAction::Cancel);
        assert_eq!(
            app.world().resource::<Lifecycle>().0,
            [
                Notice::Start(5.0),
                Notice::End(5.0, SliderDragOutcome::Completed)
            ]
        );
        assert_eq!(value(&app, root), 5.0);
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn rejected_presses_and_programmatic_or_keyboard_changes_do_not_create_gestures() {
    let id = pointer_ids()[1];
    let (mut app, root, window, mut at) = fixture(id, false, false);
    moved(&mut app, id, &mut at, axis(false, 5.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Secondary),
    );
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Secondary),
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    WidgetrySlider::set_value_in_world(app.world_mut(), root, 6.0).unwrap();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    key(&mut app, window, KeyCode::ArrowRight, false);
    assert_eq!(app.world().resource::<Lifecycle>().0.len(), 2);
    assert!(
        app.world()
            .resource::<Lifecycle>()
            .0
            .iter()
            .all(|notice| matches!(notice, Notice::Value(_, true)))
    );
}

#[test]
fn disabling_in_start_observer_cancels_before_track_value_commit() {
    let id = pointer_ids()[1];
    let (mut app, root, _, mut at) = fixture(id, false, false);
    app.world_mut().add_observer(
        move |event: On<SliderDragStarted>, mut commands: Commands| {
            commands.entity(event.entity).insert(InteractionDisabled);
        },
    );
    moved(&mut app, id, &mut at, axis(false, 8.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert_eq!(value(&app, root), 5.0);
    assert_eq!(
        app.world().resource::<Lifecycle>().0,
        [
            Notice::Start(5.0),
            Notice::End(5.0, SliderDragOutcome::Cancelled)
        ]
    );
}

#[test]
fn despawning_in_start_observer_releases_capture_without_a_stale_commit() {
    let id = pointer_ids()[1];
    let (mut app, root, _, mut at) = fixture(id, false, false);
    app.world_mut()
        .add_observer(|event: On<SliderDragStarted>, mut commands: Commands| {
            commands.entity(event.entity).despawn();
        });
    moved(&mut app, id, &mut at, axis(false, 8.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(app.world().get_entity(root).is_err());
    assert!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .is_none()
    );
    assert_eq!(app.world().resource::<Lifecycle>().0, [Notice::Start(5.0)]);
}

#[test]
fn drag_end_without_release_completes_and_late_terminals_are_ignored() {
    use bevy::picking::events::{Pointer, PointerDragEnd, PointerRelease};
    let id = pointer_ids()[1];
    let (mut app, root, _, mut at) = fixture(id, false, false);
    moved(&mut app, id, &mut at, axis(false, 5.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut().trigger(PointerDragEnd {
        entity: root,
        pointer: Pointer::new(id, at.clone()),
        button: PointerButton::Primary,
        distance: Vec2::ZERO,
    });
    app.world_mut().flush();
    app.world_mut().trigger(PointerRelease {
        entity: root,
        pointer: Pointer::new(id, at),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    });
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Lifecycle>().0,
        [
            Notice::Start(5.0),
            Notice::End(5.0, SliderDragOutcome::Completed)
        ]
    );
}

#[test]
fn nonfinite_position_and_existing_capture_reject_press_without_notifications() {
    use bevy::picking::events::{Pointer, PointerPress};
    let id = pointer_ids()[1];
    let (mut app, root, _, at) = fixture(id, false, false);
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    let mut invalid = at.clone();
    invalid.position.x = f32::NAN;
    app.world_mut().trigger(PointerPress {
        entity: root,
        pointer: Pointer::new(id, invalid),
        button: PointerButton::Primary,
        hit: hit.clone(),
        count: 1,
    });
    app.world_mut().flush();
    let other = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<PointerCaptureMap>()
        .capture(id, other, hit.clone());
    app.world_mut().trigger(PointerPress {
        entity: root,
        pointer: Pointer::new(id, at),
        button: PointerButton::Primary,
        hit,
        count: 1,
    });
    app.world_mut().flush();
    assert!(app.world().resource::<Lifecycle>().0.is_empty());
    assert_eq!(value(&app, root), 5.0);
}
fn axis(vertical: bool, value: f32) -> Vec2 {
    if vertical {
        Vec2::new(54.0, 230.0 - value * 18.0)
    } else {
        Vec2::new(50.0 + value * 18.0, 54.0)
    }
}

#[test]
fn thumb_grab_preserves_offset_and_dragging_outside_clamps_without_release_event() {
    for id in pointer_ids() {
        for vertical in [false, true] {
            let (mut app, root, _, mut at) = fixture(id, vertical, false);
            let offset = if vertical {
                Vec2::new(0.0, 6.0)
            } else {
                Vec2::new(6.0, 0.0)
            };
            moved(&mut app, id, &mut at, axis(vertical, 5.0) + offset);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
            assert_eq!(value(&app, root), 5.0);
            assert!(app.world().resource::<Changes>().0.is_empty());
            moved(&mut app, id, &mut at, axis(vertical, 7.0) + offset);
            assert!((value(&app, root) - 7.0).abs() < 0.0001);
            moved(&mut app, id, &mut at, axis(vertical, 15.0) + offset);
            assert_eq!(value(&app, root), 10.0);
            let count = app.world().resource::<Changes>().0.len();
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(matches!(
                app.world().resource::<Lifecycle>().0.first(),
                Some(Notice::Start(5.0))
            ));
            assert_eq!(
                app.world()
                    .resource::<Lifecycle>()
                    .0
                    .iter()
                    .filter(|notice| matches!(notice, Notice::Start(_)))
                    .count(),
                1
            );
            assert_eq!(
                app.world()
                    .resource::<Lifecycle>()
                    .0
                    .iter()
                    .filter(|notice| matches!(notice, Notice::End(_, _)))
                    .count(),
                1
            );
            assert_eq!(
                app.world().resource::<Lifecycle>().0.last(),
                Some(&Notice::End(10.0, SliderDragOutcome::Completed))
            );
            assert!(app.world().get::<Pressed>(root).is_none());
            assert_eq!(app.world().resource::<Changes>().0.len(), count);
            assert!(
                app.world()
                    .resource::<Changes>()
                    .0
                    .iter()
                    .all(|&(_, final_)| !final_)
            );
            moved(&mut app, id, &mut at, axis(vertical, 3.0));
            assert_eq!(value(&app, root), 10.0);
        }
    }
}

#[test]
fn track_click_moves_immediately_and_can_continue_dragging_in_both_orientations() {
    for vertical in [false, true] {
        let id = pointer_ids()[1];
        let (mut app, root, _, mut at) = fixture(id, vertical, false);
        moved(&mut app, id, &mut at, axis(vertical, 8.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!((value(&app, root) - 8.0).abs() < 0.0001);
        moved(&mut app, id, &mut at, axis(vertical, 2.0));
        assert!((value(&app, root) - 2.0).abs() < 0.0001);
        input(&mut app, id, &at, PointerAction::Cancel);
        assert!(app.world().get::<Pressed>(root).is_none());
        let notices = &app.world().resource::<Lifecycle>().0;
        assert_eq!(notices.len(), 4);
        assert_eq!(notices[0], Notice::Start(5.0));
        assert!(matches!(notices[1], Notice::Value(value, false) if (value - 8.0).abs() < 0.0001));
        assert!(
            matches!(notices[3], Notice::End(value, SliderDragOutcome::Cancelled) if (value - 2.0).abs() < 0.0001)
        );
    }
}

#[test]
fn pointer_step_results_do_not_depend_on_move_event_density() {
    for vertical in [false, true] {
        for subdivisions in [1, 30] {
            let id = pointer_ids()[1];
            let (mut app, root, _, mut at) = fixture(id, vertical, true);
            moved(&mut app, id, &mut at, axis(vertical, 0.0));
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            let mut previous = 0.0;
            for endpoint in [3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0, 0.0] {
                for index in 1..=subdivisions {
                    let p = previous + (endpoint - previous) * index as f32 / subdivisions as f32;
                    moved(&mut app, id, &mut at, axis(vertical, p));
                }
                assert_eq!(value(&app, root), endpoint);
                previous = endpoint;
            }
            assert_eq!(
                app.world()
                    .resource::<Changes>()
                    .0
                    .iter()
                    .map(|&(value, _)| value)
                    .collect::<Vec<_>>(),
                [3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0, 0.0]
            );
        }
    }
}

#[test]
fn disabled_focus_loss_capture_loss_and_pointer_loss_end_gestures() {
    for cause in 0..6 {
        let id = pointer_ids()[1];
        let (mut app, root, window, mut at) = fixture(id, false, false);
        moved(&mut app, id, &mut at, axis(false, 5.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        match cause {
            0 => {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
            }
            1 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            2 => {
                app.world_mut()
                    .resource_mut::<PointerCaptureMap>()
                    .release(id);
            }
            3 => {
                let pointer = app
                    .world_mut()
                    .query::<(Entity, &PointerId)>()
                    .iter(app.world())
                    .find(|(_, p)| **p == id)
                    .unwrap()
                    .0;
                app.world_mut().despawn(pointer);
            }
            4 => {
                app.world_mut().entity_mut(root).insert(Visibility::Hidden);
            }
            _ => app.world_mut().get_mut::<Node>(root).unwrap().display = Display::None,
        }
        update(&mut app);
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_eq!(value(&app, root), 5.0);
        assert!(app.world().resource::<Changes>().0.is_empty());
        assert_eq!(
            app.world().resource::<Lifecycle>().0,
            [
                Notice::Start(5.0),
                Notice::End(5.0, SliderDragOutcome::Cancelled)
            ]
        );
    }
}

fn key(app: &mut App, window: Entity, code: KeyCode, repeat: bool) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state: ButtonState::Pressed,
        text: None,
        repeat,
        window,
    });
    update(app);
}

#[test]
fn keyboard_requires_enabled_focus_and_supports_repeat_steps_and_one_percent() {
    for vertical in [false, true] {
        let (mut app, root, window, _) = fixture(PointerId::Mouse, vertical, true);
        let plus = if vertical {
            KeyCode::ArrowUp
        } else {
            KeyCode::ArrowRight
        };
        let minus = if vertical {
            KeyCode::ArrowDown
        } else {
            KeyCode::ArrowLeft
        };
        key(&mut app, window, plus, false);
        assert_eq!(value(&app, root), 0.0);
        assert!(
            app.world()
                .resource::<Lifecycle>()
                .0
                .iter()
                .all(|notice| matches!(notice, Notice::Value(_, true)))
        );
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(root, FocusCause::Navigated);
        for expected in [3.0, 6.0, 9.0, 10.0, 10.0] {
            key(&mut app, window, plus, true);
            assert_eq!(value(&app, root), expected);
        }
        for expected in [7.0, 4.0, 1.0, 0.0, 0.0] {
            key(&mut app, window, minus, true);
            assert_eq!(value(&app, root), expected);
        }
        assert_eq!(app.world().resource::<Changes>().0.len(), 8);
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        key(&mut app, window, plus, true);
        assert_eq!(value(&app, root), 0.0);
    }
    let (mut app, root, window, _) = fixture(PointerId::Mouse, false, false);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    key(&mut app, window, KeyCode::ArrowRight, false);
    assert!((value(&app, root) - 5.1).abs() < 0.00001);
    key(&mut app, window, KeyCode::ArrowRight, true);
    assert!((value(&app, root) - 5.2).abs() < 0.00001);
    key(&mut app, window, KeyCode::ArrowUp, true);
    key(&mut app, window, KeyCode::Home, false);
    key(&mut app, window, KeyCode::End, false);
    assert_eq!(app.world().resource::<Changes>().0.len(), 2);
}

#[test]
fn repeated_keyboard_steps_at_large_offsets_remain_valid_api_values() {
    let (mut app, _, window, _) = fixture(PointerId::Mouse, false, false);
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetrySlider { @min: 1_000_000.0, @max: 1_000_010.0, @step: {Some(0.3)} } },
        )
        .unwrap()
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for _ in 0..40 {
        key(&mut app, window, KeyCode::ArrowRight, true);
    }
    assert_eq!(value(&app, root), 1_000_010.0);
}

#[test]
fn inherited_disabled_blocks_new_pointer_and_keyboard_input_and_interrupts_drag() {
    let id = pointer_ids()[1];
    let (mut app, root, window, mut at) = fixture(id, false, false);
    let parent = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    update(&mut app);
    moved(&mut app, id, &mut at, axis(false, 5.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(root).is_none());
    moved(&mut app, id, &mut at, axis(false, 9.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    key(&mut app, window, KeyCode::ArrowRight, true);
    assert_eq!(value(&app, root), 5.0);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn camera_viewport_offset_preserves_thumb_grab_and_track_coordinates() {
    let id = pointer_ids()[1];
    let (mut app, root, _, mut at) = fixture(id, false, false);
    let camera = app
        .world()
        .get::<bevy::ui::ComputedUiTargetCamera>(root)
        .unwrap()
        .get()
        .unwrap();
    app.world_mut()
        .get_mut::<Camera>(camera)
        .unwrap()
        .viewport
        .as_mut()
        .unwrap()
        .physical_position = UVec2::new(80, 60);
    update(&mut app);
    let offset = Vec2::new(80.0, 60.0);
    moved(&mut app, id, &mut at, axis(false, 5.0) + offset);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert_eq!(value(&app, root), 5.0);
    moved(&mut app, id, &mut at, axis(false, 7.0) + offset);
    assert!((value(&app, root) - 7.0).abs() < 0.0001);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, axis(false, 2.0) + offset);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!((value(&app, root) - 2.0).abs() < 0.0001);
}

#[derive(Resource, Default)]
struct ParentKeys(Vec<KeyCode>);

#[test]
fn perpendicular_and_disabled_keys_keep_bubbling_to_the_parent() {
    for vertical in [false, true] {
        let (mut app, root, window, _) = fixture(PointerId::Mouse, vertical, false);
        app.init_resource::<ParentKeys>();
        let parent=app.world_mut().spawn_scene(bsn! {
            Node
            on(|event:On<FocusedInput<KeyboardInput>>,mut keys:ResMut<ParentKeys>| {keys.0.push(event.input.key_code);})
        }).unwrap().id();
        app.world_mut().entity_mut(root).insert(ChildOf(parent));
        update(&mut app);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(root, FocusCause::Navigated);
        let handled = if vertical {
            KeyCode::ArrowUp
        } else {
            KeyCode::ArrowRight
        };
        let perpendicular = if vertical {
            KeyCode::ArrowRight
        } else {
            KeyCode::ArrowUp
        };
        key(&mut app, window, perpendicular, false);
        key(&mut app, window, handled, false);
        assert_eq!(app.world().resource::<ParentKeys>().0, [perpendicular]);
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        key(&mut app, window, handled, false);
        assert_eq!(
            app.world().resource::<ParentKeys>().0,
            [perpendicular, handled]
        );
    }
}

#[test]
fn explicit_visible_overrides_hidden_ancestors_without_overriding_disabled_or_display() {
    for visible_on_root in [false, true] {
        let id = pointer_ids()[1];
        let (mut app, root, window, mut at) = fixture(id, false, false);
        let hidden = app
            .world_mut()
            .spawn_scene(bsn! { Node Visibility::Hidden })
            .unwrap()
            .id();
        let middle = app
            .world_mut()
            .spawn_scene(bsn! { Node template(move |_| Ok(ChildOf(hidden))) })
            .unwrap()
            .id();
        app.world_mut().entity_mut(root).insert(ChildOf(middle));
        app.world_mut()
            .entity_mut(if visible_on_root { root } else { middle })
            .insert(Visibility::Visible);
        update(&mut app);
        moved(&mut app, id, &mut at, axis(false, 5.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_some());
        moved(&mut app, id, &mut at, axis(false, 7.0));
        assert!((value(&app, root) - 7.0).abs() < 0.0001);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        key(&mut app, window, KeyCode::ArrowRight, false);
        assert!((value(&app, root) - 7.1).abs() < 0.0001);
        app.world_mut()
            .entity_mut(hidden)
            .insert(InteractionDisabled);
        key(&mut app, window, KeyCode::ArrowRight, false);
        assert!((value(&app, root) - 7.1).abs() < 0.0001);
        app.world_mut()
            .entity_mut(hidden)
            .remove::<InteractionDisabled>();
        app.world_mut().get_mut::<Node>(hidden).unwrap().display = Display::None;
        key(&mut app, window, KeyCode::ArrowRight, false);
        assert!((value(&app, root) - 7.1).abs() < 0.0001);
    }
}
