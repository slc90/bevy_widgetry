#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::ecs::error::Severity;
use bevy::prelude::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
    WidgetryWindowTitle, owned_widgetry_window, widgetry_window_target,
};

fn window(app: &mut App, title: impl SceneList) -> (Entity, Entity) {
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @owned_widgetry_window(Window { title: "Original".into(), ..default() },
                WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme,
                Default::default(), title,
                bsn_list!{Name("Body") Text("Original")})
        })
        .unwrap()
        .id();
    (root, widgetry_window_target(app.world(), root).unwrap())
}

fn named(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, current)| current.as_str() == name)
        .unwrap()
        .0
}

fn fixture() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    app
}

#[test]
fn explicit_title_updates_only_the_bound_text_and_native_window() {
    let mut app = fixture();
    let (a, native_a) = window(
        &mut app,
        bsn_list! {
            Node Children [
                Text("Original") Name("Extra")--
                WidgetryWindowTitle Name("TitleA")--
                Text("★") Name("Icon")
            ]
        },
    );
    let (_, native_b) = window(&mut app, bsn_list! { WidgetryWindowTitle Name("TitleB") });
    app.update();
    let title_a = named(&mut app, "TitleA");
    let title_b = named(&mut app, "TitleB");
    let extra = named(&mut app, "Extra");
    let icon = named(&mut app, "Icon");
    assert_eq!(app.world().get::<Text>(title_a).unwrap().0, "Original");
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), a, "新标题").unwrap());
    assert_eq!(app.world().get::<Window>(native_a).unwrap().title, "新标题");
    assert_eq!(app.world().get::<Text>(title_a).unwrap().0, "新标题");
    assert!(!WidgetryWindowTitle::set_in_world(app.world_mut(), a, "新标题").unwrap());
    for entity in [title_b, extra] {
        assert_eq!(app.world().get::<Text>(entity).unwrap().0, "Original");
    }
    assert_eq!(app.world().get::<Text>(icon).unwrap().0, "★");
    assert_eq!(
        app.world().get::<Window>(native_b).unwrap().title,
        "Original"
    );
    for (name, text) in app.world_mut().query::<(&Name, &Text)>().iter(app.world()) {
        if name.as_str() == "Body" {
            assert_eq!(text.0, "Original");
        }
    }
}

#[test]
fn unbound_custom_title_and_late_bound_text_use_latest_native_title() {
    let mut app = fixture();
    let (root, native) = window(&mut app, bsn_list! { Text("Custom") Name("Custom") });
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Before build").unwrap());
    app.update();
    let custom = named(&mut app, "Custom");
    assert_eq!(app.world().get::<Text>(custom).unwrap().0, "Custom");
    app.world_mut()
        .entity_mut(custom)
        .insert(WidgetryWindowTitle);
    app.update();
    assert_eq!(app.world().get::<Text>(custom).unwrap().0, "Before build");
    app.world_mut().get_mut::<Window>(native).unwrap().title = "Native update".into();
    app.update();
    assert_eq!(app.world().get::<Text>(custom).unwrap().0, "Native update");
}

#[test]
fn invalid_or_ambiguous_title_binding_rejects_before_any_mutation() {
    let mut app = fixture();
    let (root, native) = window(&mut app, bsn_list! { WidgetryWindowTitle Name("Title") });
    app.update();
    let title = named(&mut app, "Title");
    let parent = app.world().get::<ChildOf>(title).unwrap().parent();
    let duplicate = app
        .world_mut()
        .spawn_scene(bsn! {
            WidgetryWindowTitle Text("Duplicate") ChildOf(parent)
        })
        .unwrap()
        .id();
    let logs = LogCapture::default();
    let error = logs
        .run(|| WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Rejected"))
        .unwrap_err();
    assert_eq!(error.severity(), Severity::Error);
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::tracing::Level::ERROR)
    );
    assert_eq!(app.world().get::<Window>(native).unwrap().title, "Original");
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "Original");
    assert_eq!(app.world().get::<Text>(duplicate).unwrap().0, "Duplicate");
    app.world_mut().despawn(duplicate);
    app.world_mut().entity_mut(title).remove::<Text>();
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Rejected").is_err());
    assert_eq!(app.world().get::<Window>(native).unwrap().title, "Original");
    app.world_mut()
        .entity_mut(title)
        .insert(Text("Original".into()));
    let body = named(&mut app, "Body");
    app.world_mut().entity_mut(title).insert(ChildOf(body));
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Rejected").is_err());
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "Original");
    assert_eq!(app.world().get::<Window>(native).unwrap().title, "Original");
}

#[test]
fn queued_title_update_checks_the_target_when_executed() {
    let mut app = fixture();
    let (root, native) = window(&mut app, bsn_list! { WidgetryWindowTitle Name("Title") });
    WidgetryWindowTitle::set(&mut app.world_mut().commands(), root, "Queued");
    assert_eq!(app.world().get::<Window>(native).unwrap().title, "Original");
    app.world_mut().flush();
    assert_eq!(app.world().get::<Window>(native).unwrap().title, "Queued");
    let title = named(&mut app, "Title");
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "Queued");
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    errors.run(|| {
        let mut commands = app.world_mut().commands();
        commands.entity(native).despawn();
        WidgetryWindowTitle::set(&mut commands, root, "Stale");
        app.world_mut().flush();
    });
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), Severity::Error);
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "Queued");
}

#[test]
fn invalid_root_is_an_error_and_cannot_change_an_unrelated_window() {
    let mut app = fixture();
    let (root, native) = window(&mut app, bsn_list! {});
    let unrelated = app.world_mut().spawn_empty().id();
    for target in [Entity::PLACEHOLDER, native, unrelated] {
        let error =
            WidgetryWindowTitle::set_in_world(app.world_mut(), target, "Rejected").unwrap_err();
        assert_eq!(error.severity(), Severity::Error);
        assert_eq!(app.world().get::<Window>(native).unwrap().title, "Original");
    }
    app.world_mut().despawn(root);
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Rejected").is_err());
}

#[test]
fn delayed_node_and_nested_window_bind_to_their_own_native_title() {
    let mut app = fixture();
    let (outer, _) = window(&mut app, bsn_list! {Node Name("LateHost")});
    WidgetryWindowTitle::set_in_world(app.world_mut(), outer, "Latest").unwrap();
    let host = named(&mut app, "LateHost");
    let (inner, inner_native) =
        window(&mut app, bsn_list! {WidgetryWindowTitle Name("InnerTitle")});
    app.world_mut().entity_mut(inner).insert(ChildOf(host));
    let late = app
        .world_mut()
        .spawn_scene(bsn! {
            WidgetryWindowTitle ChildOf(host)
        })
        .unwrap()
        .id();
    app.update();
    let inner_title = named(&mut app, "InnerTitle");
    assert_eq!(app.world().get::<Text>(late).unwrap().0, "Latest");
    assert_eq!(app.world().get::<Text>(inner_title).unwrap().0, "Original");
    WidgetryWindowTitle::set_in_world(app.world_mut(), outer, "Outer only").unwrap();
    assert_eq!(app.world().get::<Text>(inner_title).unwrap().0, "Original");
    assert_eq!(
        app.world().get::<Window>(inner_native).unwrap().title,
        "Original"
    );
}

#[test]
fn runtime_binding_errors_retry_with_deduplicated_diagnostics_and_recover() {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    let logs = LogCapture::default();
    let (mut app, root, title) = logs.run(|| {
        let mut app = fixture();
        let (root, _) = window(&mut app, bsn_list! {WidgetryWindowTitle Name("Title")});
        app.update();
        let title = named(&mut app, "Title");
        (app, root, title)
    });
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app.world_mut().entity_mut(title).remove::<Text>();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        })
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == Severity::Error)
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        1
    );
    app.world_mut()
        .entity_mut(title)
        .insert(Text("Stale".into()));
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "Original");
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::INFO
                && record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("标题绑定同步恢复")))
            .count(),
        1
    );
    assert!(!WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Original").unwrap());
}

#[test]
fn closing_native_waits_for_window_closed_without_a_title_failure() {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::window::{WindowClosed, WindowClosing};
    let mut app = fixture();
    let (root, native) = window(&mut app, bsn_list! {WidgetryWindowTitle});
    let (_, other_native) = window(&mut app, bsn_list! {WidgetryWindowTitle Name("OtherTitle")});
    app.update();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app.add_message::<WindowClosing>();
    app.world_mut()
        .write_message(WindowClosing { window: native });
    app.world_mut().despawn(native);
    app.world_mut()
        .get_mut::<Window>(other_native)
        .unwrap()
        .title = "Still open".into();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        })
    });
    assert!(errors.take().is_empty());
    assert!(
        logs.records()
            .iter()
            .all(|record| record.level != bevy::log::tracing::Level::ERROR)
    );
    assert!(app.world().entities().contains(root));
    let other = named(&mut app, "OtherTitle");
    assert_eq!(app.world().get::<Text>(other).unwrap().0, "Still open");
    assert!(WidgetryWindowTitle::set_in_world(app.world_mut(), root, "Invalid").is_err());
    app.world_mut()
        .write_message(WindowClosed { window: native });
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    assert!(!app.world().entities().contains(root));
}

#[test]
fn missing_native_without_closing_signal_remains_a_title_error() {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    let mut app = fixture();
    let (_, native) = window(&mut app, bsn_list! {WidgetryWindowTitle});
    app.update();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app.world_mut().despawn(native);
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), Severity::Error);
}
