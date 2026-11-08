#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputFocus,
    picking::{
        hover::Hovered,
        pointer::{Location, PointerAction, PointerButton, PointerId},
    },
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button, ButtonPlugin},
    window::PrimaryWindow,
};
use bevy_widgetry_button::{WidgetryButtonHeadless, WidgetryButtonPlugin};
use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, picking_app, queue_key, queue_pointer, spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

#[derive(Resource, Default)]
struct AncestorInputs(usize);

#[test]
fn public_headless_uses_real_primary_input_focus_and_disabled_without_visuals() {
    let mut app = picking_app();
    // The host may also install native buttons for other widgets.
    app.add_plugins((ButtonPlugin, WidgetryButtonPlugin))
        .init_resource::<Activations>();
    add_keyboard_dispatch(&mut app);
    app.add_observer(|event: On<Activate>, mut events: ResMut<Activations>| {
        events.0.push(event.entity)
    });
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let parent = app
        .world_mut()
        .spawn_scene(bsn! {
            Node template(move |_| Ok(UiTargetCamera(camera)))
            Children [@WidgetryButtonHeadless Node { width: px(160), height: px(50) }]
        })
        .unwrap()
        .id();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    app.init_resource::<AncestorInputs>();
    app.world_mut()
        .entity_mut(parent)
        .observe(
            |_: On<bevy::picking::events::PointerClick>, mut events: ResMut<AncestorInputs>| {
                events.0 += 1
            },
        )
        .observe(
            |_: On<bevy::picking::events::PointerDragStart>, mut events: ResMut<AncestorInputs>| {
                events.0 += 1
            },
        );
    app.update();
    app.update();
    assert!(app.world().get::<Button>(root).is_none());
    assert!(app.world().get::<Children>(root).is_none());
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::NONE
    );
    assert_eq!(app.world().get::<Node>(root).unwrap().border, UiRect::ZERO);
    assert!(
        !app.world()
            .inspect_entity(root)
            .unwrap()
            .any(|(_, info)| info.name().contains("Loading"))
    );
    assert_eq!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(root)
            .unwrap()
            .0,
        -1
    );
    let location = |position| Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position,
    };
    let inside = Vec2::new(60.0, 25.0);
    let send = |app: &mut App, position, action| {
        queue_pointer(app, PointerId::Mouse, location(position), action);
        app.update();
    };
    send(&mut app, inside, PointerAction::Move { delta: Vec2::ONE });
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    for button in [
        PointerButton::Secondary,
        PointerButton::Middle,
        PointerButton::Primary,
    ] {
        send(&mut app, inside, PointerAction::Press(button));
        assert_eq!(
            app.world().get::<Pressed>(root).is_some(),
            button == PointerButton::Primary
        );
        send(&mut app, inside, PointerAction::Release(button));
        assert!(app.world().get::<Pressed>(root).is_none());
    }
    assert_eq!(app.world().resource::<Activations>().0, [root]);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    send(
        &mut app,
        inside,
        PointerAction::Press(PointerButton::Primary),
    );
    send(
        &mut app,
        inside,
        PointerAction::Press(PointerButton::Secondary),
    );
    send(
        &mut app,
        inside,
        PointerAction::Release(PointerButton::Secondary),
    );
    assert!(app.world().get::<Pressed>(root).is_some());
    assert_eq!(app.world().resource::<Activations>().0.len(), 1);
    send(&mut app, inside, PointerAction::Cancel);
    assert!(app.world().get::<Pressed>(root).is_none());
    for action in [
        PointerAction::Cancel,
        PointerAction::Move {
            delta: Vec2::splat(200.0),
        },
    ] {
        send(
            &mut app,
            inside,
            PointerAction::Press(PointerButton::Primary),
        );
        let outside = if matches!(action, PointerAction::Move { .. }) {
            Vec2::splat(300.0)
        } else {
            inside
        };
        send(&mut app, outside, action);
        if outside != inside {
            send(
                &mut app,
                inside,
                PointerAction::Move {
                    delta: inside - outside,
                },
            );
        }
        send(
            &mut app,
            inside,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_eq!(app.world().resource::<Activations>().0.len(), 1);
    }
    for disabled_target in [root, parent] {
        app.world_mut()
            .entity_mut(disabled_target)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        assert!(
            app.world()
                .get::<WidgetryEffectiveDisabled>(root)
                .unwrap()
                .is_disabled()
        );
        send(&mut app, inside, PointerAction::Move { delta: Vec2::ONE });
        send(
            &mut app,
            inside,
            PointerAction::Press(PointerButton::Primary),
        );
        send(
            &mut app,
            inside,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Activations>().0.len(), 1);
        app.world_mut()
            .entity_mut(disabled_target)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        assert!(
            !app.world()
                .get::<WidgetryEffectiveDisabled>(root)
                .unwrap()
                .is_disabled()
        );
    }
    for key in [KeyCode::Enter, KeyCode::Space] {
        for (state, repeat) in [
            (ButtonState::Pressed, false),
            (ButtonState::Pressed, true),
            (ButtonState::Released, false),
        ] {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code: key,
                    logical_key: Key::Space,
                    state,
                    repeat,
                    window,
                    text: None,
                },
            );
            app.update();
        }
    }
    assert_eq!(app.world().resource::<Activations>().0, vec![root; 3]);
    assert_eq!(app.world().resource::<AncestorInputs>().0, 0);
}
