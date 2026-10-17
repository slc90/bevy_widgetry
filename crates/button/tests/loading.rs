#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_test_utils::scene_app;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let button = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    (app, button)
}

#[test]
fn loading_commits_synchronously_and_queued_calls_wait_for_flush() {
    let (mut app, button) = fixture();
    assert!(!WidgetryButton::is_loading(app.world(), button).unwrap());
    assert!(!WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap());
    assert!(WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap());
    assert!(WidgetryButton::is_loading(app.world(), button).unwrap());
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    assert!(!WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap());
    WidgetryButton::set_loading(&mut app.world_mut().commands(), button, false);
    assert!(WidgetryButton::is_loading(app.world(), button).unwrap());
    app.world_mut().flush();
    assert!(!WidgetryButton::is_loading(app.world(), button).unwrap());
    assert!(app.world().get::<InteractionDisabled>(button).is_none());
}

#[test]
fn loading_preserves_local_and_inherited_disabled_requests() {
    let (mut app, button) = fixture();
    let parent = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(button).insert(ChildOf(parent));
    for local_first in [true, false] {
        for ancestor_first in [true, false] {
            if local_first {
                app.world_mut()
                    .entity_mut(button)
                    .insert(InteractionDisabled);
            }
            if ancestor_first {
                app.world_mut()
                    .entity_mut(parent)
                    .insert(InteractionDisabled);
            }
            WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
            if !local_first {
                app.world_mut()
                    .entity_mut(button)
                    .insert(InteractionDisabled);
            }
            if !ancestor_first {
                app.world_mut()
                    .entity_mut(parent)
                    .insert(InteractionDisabled);
            }
            WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
            assert!(app.world().get::<InteractionDisabled>(button).is_some());
            app.world_mut()
                .entity_mut(parent)
                .remove::<InteractionDisabled>();
            assert!(app.world().get::<InteractionDisabled>(button).is_some());
            app.world_mut()
                .entity_mut(button)
                .remove::<InteractionDisabled>();
            assert!(app.world().get::<InteractionDisabled>(button).is_none());
        }
    }
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    app.world_mut()
        .entity_mut(button)
        .remove::<InteractionDisabled>();
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    assert!(app.world().get::<InteractionDisabled>(button).is_none());
}

#[test]
fn loading_does_not_cancel_another_intrinsic_reason() {
    let (mut app, button) = fixture();
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), button, true);
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), button, false);
    assert!(app.world().get::<InteractionDisabled>(button).is_none());
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), button, true);
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), button, false);
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    assert!(app.world().get::<InteractionDisabled>(button).is_none());
}

#[test]
fn content_with_explicit_visibility_is_hidden_without_changing_layout_or_child_state() {
    use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};
    let (mut app, _) = fixture();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(600), 1.0);
    let button = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton {
                @children: {Some(Box::new(bsn_list! {
                    Node { width: px(92), height: px(22) } Visibility::Visible--
                    Node { width: px(16), height: px(22) } Visibility::Hidden
                }) as Box<dyn bevy::prelude::SceneList>)},
            }
            Node { width: px(180), height: px(48) }
        })
        .unwrap()
        .id();
    app.update();
    let content = app.world().get::<Children>(button).unwrap()[0];
    let children = app.world().get::<Children>(content).unwrap().to_vec();
    let initial_size = app.world().get::<ComputedNode>(button).unwrap().size();
    let initial_transform = *app.world().get::<UiTransform>(content).unwrap();
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(button).unwrap().size(),
        initial_size
    );
    assert_eq!(
        *app.world().get::<Visibility>(children[0]).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        *app.world().get::<Visibility>(children[1]).unwrap(),
        Visibility::Hidden
    );
    // Explicit Visible descendants escape Visibility::Hidden inheritance.
    // The owned wrapper must also suppress their geometry without changing layout.
    assert!(
        app.world()
            .get::<UiGlobalTransform>(children[0])
            .unwrap()
            .try_inverse()
            .is_none()
    );
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    app.update();
    assert_eq!(
        *app.world().get::<UiTransform>(content).unwrap(),
        initial_transform
    );
    assert_eq!(
        app.world().get::<Children>(content).unwrap().to_vec(),
        children
    );
    assert_eq!(
        *app.world().get::<Visibility>(children[1]).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn changes_notify_after_commit_and_invalid_calls_reach_the_host_without_partial_updates() {
    use bevy::ecs::error::Severity;
    use bevy_widgetry_button::WidgetryButtonLoadingChanged;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    #[derive(Resource, Default)]
    struct Changes(Vec<bool>);
    let (mut app, button) = fixture();
    app.init_resource::<Changes>().add_observer(
        |event: On<WidgetryButtonLoadingChanged>, world: &World| {
            assert_eq!(
                WidgetryButton::is_loading(world, event.entity).unwrap(),
                event.loading
            );
        },
    );
    app.add_observer(
        |event: On<WidgetryButtonLoadingChanged>, mut changes: ResMut<Changes>| {
            changes.0.push(event.loading)
        },
    );
    for loading in [false, true, true, false, false] {
        WidgetryButton::set_loading_in_world(app.world_mut(), button, loading).unwrap();
    }
    assert_eq!(app.world().resource::<Changes>().0, [true, false]);
    let wrong = app.world_mut().spawn(Node::default()).id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let logs = LogCapture::default();
    for invalid in [wrong, stale] {
        for loading in [false, true] {
            let error = logs
                .run(|| WidgetryButton::set_loading_in_world(app.world_mut(), invalid, loading))
                .unwrap_err();
            assert_eq!(error.severity(), Severity::Error);
        }
        assert!(
            logs.run(|| WidgetryButton::is_loading(app.world(), invalid))
                .is_err()
        );
    }
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    WidgetryButton::set_loading(&mut app.world_mut().commands(), stale, true);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert!(!logs.records().is_empty());
    assert_eq!(app.world().resource::<Changes>().0, [true, false]);
    assert!(!WidgetryButton::is_loading(app.world(), button).unwrap());
}

#[test]
fn spinner_uses_normal_foreground_and_shared_masks_and_stops_when_loading_ends() {
    use bevy_widgetry_button::WidgetryButtonColorOverrides;
    use bevy_widgetry_theme::WidgetryThemeMode;
    let (mut app, button) = fixture();
    let other = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    app.update();
    let spinner = app.world().get::<Children>(button).unwrap()[1];
    let images = app.world().get::<Children>(spinner).unwrap().to_vec();
    let other_spinner = app.world().get::<Children>(other).unwrap()[1];
    let other_arc = app.world().get::<Children>(other_spinner).unwrap()[1];
    let arc = images[1];
    assert_eq!(
        app.world().get::<ImageNode>(arc).unwrap().image,
        app.world().get::<ImageNode>(other_arc).unwrap().image
    );
    let original = *app.world().get::<UiTransform>(arc).unwrap();
    app.update();
    assert_eq!(*app.world().get::<UiTransform>(arc).unwrap(), original);
    app.world_mut()
        .entity_mut(button)
        .insert(InteractionDisabled);
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        let foreground = app.world().get::<ImageNode>(arc).unwrap().color;
        assert_eq!(foreground, mode.colors().button.normal.foreground);
        assert!(
            app.world()
                .get::<ImageNode>(images[0])
                .unwrap()
                .color
                .alpha()
                < foreground.alpha()
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(button).unwrap().0,
            mode.colors().button.normal.background
        );
    }
    assert_ne!(*app.world().get::<UiTransform>(arc).unwrap(), original);
    assert_eq!(
        *app.world().get::<UiTransform>(other_arc).unwrap(),
        original
    );
    let mut colors = WidgetryButtonColorOverrides::default();
    colors.normal.foreground = Some(Color::srgb_u8(120, 20, 180));
    WidgetryButtonColorOverrides::set_in_world(app.world_mut(), button, colors.clone()).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(arc).unwrap().color,
        colors.normal.foreground.unwrap()
    );
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    let stopped = *app.world().get::<UiTransform>(arc).unwrap();
    app.update();
    app.update();
    assert_eq!(*app.world().get::<UiTransform>(arc).unwrap(), stopped);
    assert_eq!(
        *app.world().get::<Visibility>(spinner).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(button).unwrap().0,
        WidgetryThemeMode::KamuriViolet
            .colors()
            .button
            .disabled
            .background
    );
}

#[test]
fn real_pointer_and_keyboard_inputs_stop_activating_but_explicit_activate_remains_business_owned() {
    use bevy::input_focus::InputFocus;
    use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId};
    use bevy::ui_widgets::Activate;
    use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
    use bevy_widgetry_test_utils::{
        add_keyboard_dispatch, picking_app, press_key, queue_pointer, spawn_picking_camera,
    };
    #[derive(Resource, Default)]
    struct Activations(Vec<Entity>);
    let mut app = picking_app();
    app.add_plugins(WidgetryButtonPlugin)
        .init_resource::<Activations>();
    add_keyboard_dispatch(&mut app);
    app.add_observer(
        |event: On<Activate>, mut activations: ResMut<Activations>| {
            activations.0.push(event.entity)
        },
    );
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
    let button = app.world_mut().spawn_scene(bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { Node { width: px(80), height: px(24) } }) as Box<dyn SceneList>)}, }
        Node { width: px(160), height: px(50) } template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(60.0, 25.0),
    };
    app.update();
    app.update();
    let click = |app: &mut App| {
        for action in [
            PointerAction::Move { delta: Vec2::ONE },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            queue_pointer(app, PointerId::Mouse, location.clone(), action);
            app.update();
        }
    };
    click(&mut app);
    assert_eq!(app.world().resource::<Activations>().0, [button]);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    assert_eq!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(button),
        Some(&bevy::input_focus::tab_navigation::TabIndex(-1))
    );
    WidgetryButton::set_loading_in_world(app.world_mut(), button, true).unwrap();
    click(&mut app);
    for key in [KeyCode::Enter, KeyCode::Space] {
        press_key(&mut app, window, key);
    }
    assert_eq!(app.world().resource::<Activations>().0, [button]);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Wait))
    );
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(app.world().resource::<Activations>().0, [button, button]);
    WidgetryButton::set_loading_in_world(app.world_mut(), button, false).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    for key in [KeyCode::Enter, KeyCode::Space] {
        press_key(&mut app, window, key);
    }
    click(&mut app);
    assert_eq!(app.world().resource::<Activations>().0, vec![button; 5]);
}
