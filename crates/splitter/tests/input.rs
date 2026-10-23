#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_splitter::{
    WidgetrySplitter, WidgetrySplitterOrientation as Orientation, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f32, bool)>);
fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}
fn fixture(
    id: PointerId,
    vertical: bool,
    dpi: f32,
    ui_scale: f32,
) -> (App, Entity, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin)
        .insert_resource(UiScale(ui_scale))
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f32>>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetrySplitter::split_ratio(&world, event.source).unwrap(),
                event.value
            );
            world
                .resource_mut::<Changes>()
                .0
                .push((event.source, event.value, event.is_final));
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1600, 1600)
                    .with_scale_factor_override(dpi),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(1600), dpi);
    let root=spawn_scene(app.world_mut(),bsn! {
        @WidgetrySplitter { @orientation:{if vertical {Orientation::Vertical} else {Orientation::Horizontal}},
            @pane_a:bsn! {Node {min_width:px(200),max_width:px(600),min_height:px(200),max_height:px(600)}},
            @pane_b:bsn! {Node {min_width:px(100),max_width:px(600),min_height:px(100),max_height:px(600)}},
        } Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:{if vertical {px(200)} else {px(808)}},height:{if vertical {px(808)} else {px(200)}}}
        template(move|_|Ok(UiTargetCamera(camera)))
    }).unwrap();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    settle(&mut app);
    let handle = app.world().get::<Children>(root).unwrap()[1];
    let position = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(handle)
        .unwrap()
        .translation
        / dpi;
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position,
    };
    (app, root, handle, window, at)
}
fn send(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    app.update();
}
fn move_by(app: &mut App, id: PointerId, at: &mut Location, delta: Vec2) {
    at.position += delta;
    send(app, id, at, PointerAction::Move { delta });
}
fn ratio(app: &App, root: Entity) -> f32 {
    WidgetrySplitter::split_ratio(app.world(), root).unwrap()
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn handle_drag_captures_clamps_and_reverses_immediately() {
    for id in pointer_ids() {
        for vertical in [false, true] {
            let (mut app, root, handle, _, mut at) = fixture(id, vertical, 1.25, 1.0);
            send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
            send(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert_eq!(
                app.world()
                    .resource::<PointerCaptureMap>()
                    .get(&id)
                    .unwrap()
                    .0,
                handle
            );
            let axis = if vertical { Vec2::Y } else { Vec2::X };
            move_by(&mut app, id, &mut at, axis * 100.0);
            close(ratio(&app, root), 0.625);
            move_by(&mut app, id, &mut at, axis * 1500.0);
            close(ratio(&app, root), 0.75);
            let count = app.world().resource::<Changes>().0.len();
            move_by(&mut app, id, &mut at, axis * 100.0);
            assert_eq!(app.world().resource::<Changes>().0.len(), count);
            move_by(&mut app, id, &mut at, axis * -80.0);
            close(ratio(&app, root), 0.65);
            send(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            assert!(app.world().get::<Pressed>(handle).is_none());
            assert!(
                app.world()
                    .resource::<Changes>()
                    .0
                    .iter()
                    .all(|(e, _, final_)| *e == root && !final_)
            );
        }
    }
}

#[test]
fn no_move_and_cancel_are_silent_and_do_not_roll_back() {
    for id in pointer_ids() {
        let (mut app, root, handle, _, mut at) = fixture(id, false, 1.0, 1.0);
        send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        send(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        move_by(&mut app, id, &mut at, Vec2::new(80.0, 0.0));
        close(ratio(&app, root), 0.6);
        send(&mut app, id, &at, PointerAction::Cancel);
        close(ratio(&app, root), 0.6);
        assert!(app.world().get::<Pressed>(handle).is_none());
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn interruption_cleans_gesture_and_retains_committed_ratio() {
    for cause in 0..7 {
        let id = pointer_ids()[1];
        let (mut app, root, handle, window, mut at) = fixture(id, false, 1.0, 1.0);
        send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        move_by(&mut app, id, &mut at, Vec2::new(80.0, 0.0));
        close(ratio(&app, root), 0.6);
        match cause {
            0 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            1 => {
                app.world_mut()
                    .resource_mut::<PointerCaptureMap>()
                    .release(id);
            }
            2 => {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
            }
            3 => {
                app.world_mut().get_mut::<Node>(root).unwrap().display = Display::None;
            }
            4 => {
                app.world_mut().entity_mut(root).insert(Visibility::Hidden);
            }
            5 => {
                WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, false).unwrap();
            }
            _ => {
                let e = app
                    .world_mut()
                    .query::<(Entity, &PointerId)>()
                    .iter(app.world())
                    .find(|(_, p)| **p == id)
                    .unwrap()
                    .0;
                app.world_mut()
                    .get_mut::<PointerLocation>(e)
                    .unwrap()
                    .location = None;
            }
        }
        settle(&mut app);
        close(ratio(&app, root), 0.6);
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause {cause}"
        );
        assert!(app.world().get::<Pressed>(handle).is_none());
    }
}

#[test]
fn resizable_and_disabled_block_drag_but_allow_programmatic_ratio() {
    let id = pointer_ids()[1];
    for disabled in [false, true] {
        let (mut app, root, _, _, at) = fixture(id, false, 1.0, 1.0);
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, false).unwrap();
        }
        settle(&mut app);
        send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.7).unwrap());
        close(ratio(&app, root), 0.7);
        let a = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(
            app.world().get::<InteractionDisabled>(a).is_some(),
            disabled
        );
    }
}

#[test]
fn dpi_and_ui_scale_map_pointer_delta_to_layout_units() {
    let id = pointer_ids()[1];
    for (dpi, scale) in [(1.5_f32, 1.0_f32), (1.0, 2.0)] {
        let (mut app, root, _, _, mut at) = fixture(id, false, dpi, scale);
        send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        move_by(&mut app, id, &mut at, Vec2::new(80.0, 0.0));
        close(ratio(&app, root), 0.5 + 80.0 / scale / 800.0);
    }
}

#[test]
fn capture_loss_blocks_even_a_matching_queued_move() {
    use bevy::picking::events::{Pointer, PointerDrag};
    let id = pointer_ids()[1];
    let (mut app, root, handle, _, at) = fixture(id, false, 1.0, 1.0);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<PointerCaptureMap>()
        .release(id);
    app.world_mut().trigger(PointerDrag {
        entity: handle,
        pointer: Pointer::new(id, at),
        delta: Vec2::new(80.0, 0.0),
        button: PointerButton::Primary,
        distance: Vec2::new(80.0, 0.0),
    });
    app.world_mut().flush();
    assert_eq!(ratio(&app, root), 0.5);
    assert!(app.world().get::<Pressed>(handle).is_none());
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn queued_final_move_is_committed_before_release_inside_and_outside_handle() {
    for id in pointer_ids() {
        for pixels in [1.0, 80.0, 1500.0] {
            let (mut app, root, handle, _, mut at) = fixture(id, false, 1.0, 1.0);
            send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
            send(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            at.position.x += pixels;
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Move {
                    delta: Vec2::new(pixels, 0.0),
                },
            );
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Release(PointerButton::Primary),
            );
            app.update();
            close(ratio(&app, root), (0.5 + pixels / 800.0).min(0.75));
            assert_eq!(app.world().resource::<Changes>().0.len(), 1);
            assert!(app.world().get::<Pressed>(handle).is_none());
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn programmatic_changes_during_drag_remain_the_only_ratio_authority() {
    let id = pointer_ids()[1];
    let (mut app, root, handle, _, mut at) = fixture(id, false, 1.0, 1.0);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.3).unwrap();
    move_by(&mut app, id, &mut at, Vec2::new(80.0, 0.0));
    close(ratio(&app, root), 0.4);
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        handle
    );
    let changes = &app.world().resource::<Changes>().0;
    assert_eq!(changes.len(), 2);
    assert!(changes[0].2);
    assert!(!changes[1].2);
}

#[test]
fn foreign_terminal_events_cannot_end_the_owned_gesture() {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerCancel, PointerRelease},
    };
    let id = pointer_ids()[1];
    let (mut app, _, handle, _, at) = fixture(id, false, 1.0, 1.0);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut().trigger(PointerCancel {
        entity: handle,
        pointer: Pointer::new(PointerId::Mouse, at.clone()),
        hit: hit.clone(),
    });
    app.world_mut().trigger(PointerRelease {
        entity: handle,
        pointer: Pointer::new(id, at.clone()),
        button: PointerButton::Secondary,
        hit: hit.clone(),
    });
    let mut foreign = at.clone();
    foreign.target = bevy::camera::NormalizedRenderTarget::None {
        width: 1,
        height: 1,
    };
    app.world_mut().trigger(PointerCancel {
        entity: handle,
        pointer: Pointer::new(id, foreign),
        hit,
    });
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        handle
    );
    assert!(app.world().get::<Pressed>(handle).is_some());
    send(&mut app, id, &at, PointerAction::Cancel);
    assert!(app.world().get::<Pressed>(handle).is_none());
}

#[test]
fn escape_and_arrow_keys_leave_a_drag_active_without_value_changes() {
    use bevy::input_focus::{FocusCause, InputFocus};
    let id = pointer_ids()[1];
    let (mut app, root, handle, window, at) = fixture(id, false, 1.0, 1.0);
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(handle, FocusCause::Navigated);
    for key in [KeyCode::Escape, KeyCode::ArrowRight, KeyCode::ArrowDown] {
        bevy_widgetry_test_utils::press_key(&mut app, window, key);
    }
    assert_eq!(ratio(&app, root), 0.5);
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        handle
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(handle)
            .is_none()
    );
}

#[derive(Resource, Default)]
struct Clicks(usize);

#[test]
fn resizable_false_keeps_real_pane_buttons_active_but_ancestor_disabled_does_not() {
    use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
    let mut app = picking_app();
    app.add_plugins((WidgetrySplitterPlugin, WidgetryButtonPlugin))
        .init_resource::<Clicks>();
    app.add_observer(
        |event: On<bevy::ui_widgets::Activate>,
         buttons: Query<(), With<WidgetryButton>>,
         mut clicks: ResMut<Clicks>| {
            if buttons.contains(event.entity) {
                clicks.0 += 1;
            }
        },
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(1200), 1.0);
    let parent=spawn_scene(app.world_mut(),bsn! {
        Node {width:px(808),height:px(200)} template(move|_|Ok(UiTargetCamera(camera)))
        Children [@WidgetrySplitter { @resizable:false,
            @pane_a:bsn! {Children [Name::new("PaneAction") @WidgetryButton {@children:bsn_list! {Text("Pane action")}}]},
        }]
    }).unwrap();
    let id = pointer_ids()[1];
    app.world_mut().spawn(id);
    settle(&mut app);
    let root = app.world().get::<Children>(parent).unwrap()[0];
    let button = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, n)| n.as_str() == "PaneAction")
        .unwrap()
        .0;
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: app
            .world()
            .get::<bevy::ui::UiGlobalTransform>(button)
            .unwrap()
            .translation,
    };
    for disabled in [false, true, false] {
        if disabled {
            app.world_mut()
                .entity_mut(parent)
                .insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(parent)
                .remove::<InteractionDisabled>();
        }
        settle(&mut app);
        let before = app.world().resource::<Clicks>().0;
        send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        send(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        send(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            app.world().resource::<Clicks>().0,
            before + usize::from(!disabled)
        );
        assert_eq!(
            app.world().get::<InteractionDisabled>(button).is_some(),
            disabled
        );
        assert!(!WidgetrySplitter::resizable(app.world(), root).unwrap());
    }
}

#[test]
fn resizable_api_checks_structure_and_commands_at_execution() {
    use bevy::ecs::{error::Severity, world::CommandQueue};
    use bevy_widgetry_test_utils::ErrorCapture;
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetrySplitterPlugin)
        .set_error_handler(ErrorCapture::handler());
    let root = spawn_scene(app.world_mut(), bsn! {@WidgetrySplitter {@resizable:false}}).unwrap();
    assert!(!WidgetrySplitter::resizable(app.world(), root).unwrap());
    assert!(WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, true).unwrap());
    assert!(!WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, true).unwrap());
    let stale = spawn_scene(app.world_mut(), bsn! {@WidgetrySplitter}).unwrap();
    let mut queue = CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetrySplitter::set_resizable(&mut commands, root, false);
        WidgetrySplitter::set_resizable(&mut commands, stale, false);
    }
    assert!(WidgetrySplitter::resizable(app.world(), root).unwrap());
    app.world_mut().despawn(stale);
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), Severity::Error);
    assert!(!WidgetrySplitter::resizable(app.world(), root).unwrap());
    let handle = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().despawn(handle);
    assert!(WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, true).is_err());
    assert!(!WidgetrySplitter::resizable(app.world(), root).unwrap());
}

#[test]
fn updated_constraints_limit_the_next_move_without_a_second_authority() {
    let id = pointer_ids()[1];
    let (mut app, root, handle, _, mut at) = fixture(id, false, 1.0, 1.0);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let a = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().get_mut::<Node>(a).unwrap().max_width = px(500);
    move_by(&mut app, id, &mut at, Vec2::new(300.0, 0.0));
    close(ratio(&app, root), 0.625);
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        handle
    );
}

#[test]
fn nested_handle_drag_changes_only_the_nested_splitter() {
    use bevy_widgetry_core::scene::WidgetrySceneEntityCommandsExt;
    let id = pointer_ids()[1];
    let (mut app, outer, _, _, mut at) = fixture(id, false, 1.0, 1.0);
    let pane = app.world().get::<Children>(outer).unwrap()[0];
    app.world_mut()
        .commands()
        .entity(pane)
        .apply_scene_with_error_handler(bsn! {
            Children [Name::new("Nested") @WidgetrySplitter {@orientation:Orientation::Vertical}]
        });
    app.world_mut().flush();
    settle(&mut app);
    let nested = app.world().get::<Children>(pane).unwrap()[0];
    let handle = app.world().get::<Children>(nested).unwrap()[1];
    at.position = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(handle)
        .unwrap()
        .translation;
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    move_by(&mut app, id, &mut at, Vec2::new(0.0, 48.0));
    close(ratio(&app, outer), 0.5);
    close(ratio(&app, nested), 0.75);
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        handle
    );
}

#[test]
fn despawning_the_splitter_releases_its_pointer_capture() {
    let id = pointer_ids()[1];
    let (mut app, root, _, _, at) = fixture(id, false, 1.0, 1.0);
    send(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
    app.world_mut().despawn(root);
    app.world_mut().flush();
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
}
