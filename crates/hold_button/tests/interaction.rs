#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{hover::Hovered, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry_hold_button::{WidgetryHoldButton, WidgetryHoldButtonPlugin};
use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};

#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

fn fixture(id: PointerId) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    app.init_asset::<bevy::shader::Shader>();
    app.add_plugins(WidgetryHoldButtonPlugin)
        .init_resource::<Activations>();
    app.add_observer(|event: On<Activate>, mut seen: ResMut<Activations>| {
        seen.0.push(event.entity)
    });
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
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryHoldButton { @diameter: 112.0, @children: {Some(Box::new(bsn_list! {Text("Confirm") bevy_widgetry_core::text::WidgetryText}) as Box<dyn SceneList>)}, }
        Node { position_type: PositionType::Absolute, left: px(40), top: px(40) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    app.update();
    app.update();
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::splat(96.0),
    };
    (app, root, camera, location)
}
fn input(app: &mut App, id: PointerId, location: &Location, action: PointerAction) {
    queue_pointer(app, id, location.clone(), action);
    app.update();
}
fn frames(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

#[test]
fn real_picking_corner_passes_through_and_center_charges_once() {
    for id in pointer_ids() {
        let (mut app, root, camera, mut at) = fixture(id);
        let under = app.world_mut().spawn_scene(bsn! {
            Node { position_type: PositionType::Absolute, left: px(40), top: px(40), width: px(112), height: px(112) }
            template(move |_| Ok(UiTargetCamera(camera)))
            on(|event: On<bevy::picking::events::PointerClick>, mut seen: ResMut<Activations>| { seen.0.push(event.entity); })
        }).unwrap().id();
        app.world_mut().entity_mut(root).insert(ZIndex(1));
        app.update();
        app.update();
        at.position = Vec2::splat(43.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        assert!(!app.world().get::<Hovered>(root).unwrap().0);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Activations>().0, [under]);
        app.world_mut().resource_mut::<Activations>().0.clear();
        at.position = Vec2::splat(96.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        assert!(app.world().get::<Hovered>(root).unwrap().0);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        frames(&mut app, 60);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().resource::<Activations>().0.is_empty());
        frames(&mut app, 80);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        frames(&mut app, 189);
        assert_eq!(app.world().resource::<Activations>().0, [root]);
        frames(&mut app, 190);
        assert_eq!(app.world().resource::<Activations>().0, [root]);
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
        frames(&mut app, 189);
        assert_eq!(app.world().resource::<Activations>().0, [root, root]);
        assert!(app.world().get::<bevy::ui_widgets::Button>(root).is_none());
        assert_eq!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(root)
                .unwrap()
                .0,
            -1
        );
    }
}

#[test]
fn drag_out_cancels_and_reenter_requires_a_fresh_press() {
    for id in pointer_ids() {
        let (mut app, root, _, mut at) = fixture(id);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        frames(&mut app, 50);
        at.position = Vec2::splat(43.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        assert!(app.world().get::<Pressed>(root).is_none());
        at.position = Vec2::splat(96.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        frames(&mut app, 200);
        assert!(app.world().resource::<Activations>().0.is_empty());
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
        frames(&mut app, 189);
        assert_eq!(app.world().resource::<Activations>().0, [root]);
    }
}

#[test]
fn parent_disabled_and_hidden_reject_held_pointer_after_recovery() {
    for hidden in [false, true] {
        let id = PointerId::Mouse;
        let (mut app, root, camera, at) = fixture(id);
        let parent = app.world_mut().spawn_scene(bsn! { Node { width: px(400), height: px(400) } template(move |_| Ok(UiTargetCamera(camera))) }).unwrap().id();
        app.world_mut().entity_mut(parent).add_child(root);
        app.update();
        app.update();
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        frames(&mut app, 70);
        if hidden {
            app.world_mut()
                .entity_mut(parent)
                .insert(Visibility::Hidden);
        } else {
            app.world_mut()
                .entity_mut(parent)
                .insert(InteractionDisabled);
        }
        app.update();
        assert!(app.world().get::<Pressed>(root).is_none());
        if hidden {
            app.world_mut()
                .entity_mut(parent)
                .insert(Visibility::Inherited);
        } else {
            app.world_mut()
                .entity_mut(parent)
                .remove::<InteractionDisabled>();
        }
        frames(&mut app, 200);
        assert!(app.world().resource::<Activations>().0.is_empty());
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
        frames(&mut app, 189);
        assert_eq!(app.world().resource::<Activations>().0, [root]);
    }
}

#[test]
fn keyboard_and_pointer_cannot_inherit_each_others_held_input() {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, queue_key};
    let id = PointerId::Mouse;
    let (mut app, root, _, at) = fixture(id);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    let key = |app: &mut App, state, repeat| {
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::Space,
                logical_key: Key::Space,
                state,
                repeat,
                text: None,
                window,
            },
        );
        app.update();
    };
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    key(&mut app, ButtonState::Pressed, false);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    key(&mut app, ButtonState::Pressed, true);
    frames(&mut app, 200);
    assert!(app.world().get::<Pressed>(root).is_none());
    assert!(app.world().resource::<Activations>().0.is_empty());
    key(&mut app, ButtonState::Released, false);
    key(&mut app, ButtonState::Pressed, false);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    key(&mut app, ButtonState::Released, false);
    frames(&mut app, 200);
    assert!(app.world().get::<Pressed>(root).is_none());
    assert!(app.world().resource::<Activations>().0.is_empty());
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
    frames(&mut app, 189);
    assert_eq!(app.world().resource::<Activations>().0, [root]);
}

#[test]
fn picking_tracks_layout_diameter_and_ui_transform() {
    let id = PointerId::Mouse;
    let (mut app, root, _, mut at) = fixture(id);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(160);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(160);
    app.world_mut().entity_mut(root).insert(UiTransform {
        scale: Vec2::splat(1.2),
        rotation: Rot2::degrees(30.0),
        ..default()
    });
    app.update();
    app.update();
    let transform = *app.world().get::<UiGlobalTransform>(root).unwrap();
    at.position = transform.transform_point2(Vec2::new(70.0, 0.0));
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    at.position = transform.transform_point2(Vec2::splat(70.0));
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    assert!(!app.world().get::<Hovered>(root).unwrap().0);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(app.world().get::<Pressed>(root).is_none());
}

#[test]
fn existing_nested_content_reparented_into_button_remains_non_pickable() {
    let id = PointerId::Mouse;
    let (mut app, root, camera, mut at) = fixture(id);
    let branch = app
        .world_mut()
        .spawn_scene(bsn! {
            Node { width: px(70), height: px(50) }
            template(move |_| Ok(UiTargetCamera(camera)))
            Children [Node { width: percent(100), height: percent(100) } Children [Text("Moved")]]
        })
        .unwrap()
        .id();
    app.update();
    app.update();
    let content = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().entity_mut(content).add_child(branch);
    app.update();
    app.update();
    let nested = app.world().get::<Children>(branch).unwrap()[0];
    at.position = app
        .world()
        .get::<UiGlobalTransform>(nested)
        .unwrap()
        .transform_point2(Vec2::ZERO);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(app.world().get::<Pressed>(root).is_some());
    frames(&mut app, 189);
    assert_eq!(app.world().resource::<Activations>().0, [root]);
}
