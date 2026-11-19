#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::{
    ecs::error::Severity,
    prelude::*,
    window::{EnabledButtons, WindowCloseRequested},
};
use bevy_widgetry_test_utils::{ErrorCapture, scene_app};
use bevy_widgetry_window::{
    WidgetryWindowAction as Action, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    WidgetryWindowPlugin, WindowError, WindowFailure, WindowFault, owned_widgetry_window,
    prepare_native_window, widgetry_window, widgetry_window_target,
};

fn windows(app: &mut App) -> [(Entity, Entity); 2] {
    app.add_plugins(WidgetryWindowPlugin);
    let controls = WidgetryWindowControlsConfig {
        minimize_visible: false,
        maximize_visible: false,
        close_visible: false,
        resizable: false,
        ..default()
    };
    let world = app.world_mut();
    let borrowed = world.spawn(prepare_native_window(Window::default())).id();
    let camera = world.spawn(Camera2d).id();
    let borrowed_root = world
        .spawn_scene(bsn! {
            @widgetry_window(borrowed, camera, controls, WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list!{})
        })
        .unwrap()
        .id();
    let owned_root = world
        .spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), controls, WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list!{})
        })
        .unwrap()
        .id();
    app.update();
    let owned = widgetry_window_target(app.world(), owned_root).unwrap();
    [(borrowed_root, borrowed), (owned_root, owned)]
}

#[test]
fn actions_submit_requests_to_the_bound_window_without_default_controls() {
    let mut app = scene_app();
    let bindings = windows(&mut app);
    let world = app.world_mut();
    for (index, (root, native)) in bindings.into_iter().enumerate() {
        let (other_root, other) = bindings[1 - index];
        Action::Minimize.queue(&mut world.commands(), root);
        world.flush();
        assert_eq!(
            world
                .get_mut::<Window>(native)
                .unwrap()
                .internal
                .take_minimize_request(),
            Some(true)
        );
        assert_eq!(
            world
                .get_mut::<Window>(other)
                .unwrap()
                .internal
                .take_minimize_request(),
            None
        );

        for (action, expected) in [
            (Action::Maximize, true),
            (Action::RestoreFromMaximized, false),
        ] {
            assert!(action.apply(world, root).unwrap());
            assert_eq!(
                world
                    .get_mut::<Window>(native)
                    .unwrap()
                    .internal
                    .take_maximize_request(),
                Some(expected)
            );
            assert_eq!(
                world
                    .get_mut::<Window>(other)
                    .unwrap()
                    .internal
                    .take_maximize_request(),
                None
            );
        }

        assert!(Action::RequestClose.apply(world, root).unwrap());
        let requests = world
            .resource_mut::<Messages<WindowCloseRequested>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].window, native);
        for entity in [root, native, other_root, other] {
            assert!(world.get_entity(entity).is_ok());
        }
    }
}

#[test]
fn disabled_actions_do_not_submit_requests() {
    let mut app = scene_app();
    let [(root, native), _] = windows(&mut app);
    let world = app.world_mut();
    let no_minimize = EnabledButtons {
        minimize: false,
        ..default()
    };
    let no_maximize = EnabledButtons {
        maximize: false,
        ..default()
    };
    let no_close = EnabledButtons {
        close: false,
        ..default()
    };
    for (action, buttons) in [
        (Action::Minimize, no_minimize),
        (Action::Maximize, no_maximize),
        (Action::RestoreFromMaximized, no_maximize),
        (Action::RequestClose, no_close),
    ] {
        world.get_mut::<Window>(native).unwrap().enabled_buttons = buttons;
        assert!(!action.apply(world, root).unwrap());
    }
    let mut window = world.get_mut::<Window>(native).unwrap();
    assert_eq!(window.internal.take_minimize_request(), None);
    assert_eq!(window.internal.take_maximize_request(), None);
    assert!(
        world
            .resource::<Messages<WindowCloseRequested>>()
            .is_empty()
    );
}

#[test]
fn queued_action_revalidates_binding_and_preserves_the_failure() {
    let mut app = scene_app();
    let [(root, native), (_, other)] = windows(&mut app);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    {
        let mut commands = app.world_mut().commands();
        commands.entity(native).despawn();
        Action::Maximize.queue(&mut commands, root);
    }
    assert!(app.world().get::<Window>(native).is_some());
    errors.run(|| app.world_mut().flush());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].error.severity(), Severity::Error);
    let diagnostic: &dyn std::error::Error =
        records[0].managed().unwrap().diagnostics().next().unwrap();
    let failure = diagnostic.downcast_ref::<WindowFailure>().unwrap();
    assert!(matches!(
        failure.kind(),
        WindowError::Structure {
            fault: WindowFault::MissingPart { entity, component: "Window" },
        } if *entity == native
    ));
    assert_eq!(
        app.world_mut()
            .get_mut::<Window>(other)
            .unwrap()
            .internal
            .take_maximize_request(),
        None
    );
}

#[test]
fn stale_bindings_do_not_redirect_actions_to_other_windows() {
    let mut app = scene_app();
    let [(root, native), (_, other)] = windows(&mut app);
    let world = app.world_mut();
    world.despawn(native);
    for invalid in [root, other] {
        let error = Action::Minimize.apply(world, invalid).unwrap_err();
        assert!(matches!(
            error.kind(),
            WindowError::Structure {
                fault: WindowFault::MissingPart { .. }
            }
        ));
    }
    assert_eq!(
        world
            .get_mut::<Window>(other)
            .unwrap()
            .internal
            .take_minimize_request(),
        None
    );
}
