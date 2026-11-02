#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_message_box::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn invalid_resolution_has_one_managed_source_with_requested_result() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = scene_app();
    app.add_plugins(WidgetryMessageBoxPlugin);
    let root = app.world_mut().spawn_empty().id();
    let logs = LogCapture::default();
    let error = logs
        .run(|| WidgetryMessageBox::resolve(app.world_mut(), root, WidgetryMessageBoxResult::Yes))
        .unwrap_err();
    let records = logs.records();
    assert_eq!(
        records
            .iter()
            .filter(|record| record.matches_field("phase", "source"))
            .count(),
        1
    );
    assert!(
        records
            .iter()
            .filter_map(|record| record.fields.get("context"))
            .any(|context| context.contains("result") && context.contains("Yes"))
    );
    assert!(
        matches!(error.kind(), MessageBoxError::Resolve { root: actual, result: WidgetryMessageBoxResult::Yes, missing: "WidgetryMessageBox" } if *actual == root)
    );
    let origin = error.origin().clone();
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    errors.run(|| {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error.into_bevy_error()) });
        app.world_mut().flush();
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert!(std::sync::Arc::ptr_eq(
        &origin,
        records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
    ));
    let missing_state = app.world_mut().spawn(WidgetryMessageBox).id();
    let failure =
        WidgetryMessageBox::resolve(app.world_mut(), missing_state, WidgetryMessageBoxResult::No)
            .unwrap_err();
    assert!(matches!(
        failure.kind(),
        MessageBoxError::Resolve {
            result: WidgetryMessageBoxResult::No,
            missing: "MessageBoxState",
            ..
        }
    ));
}

fn fixture() -> (App, Vec<Entity>) {
    use bevy_widgetry_window::*;
    let mut app = scene_app();
    app.add_plugins(WidgetryMessageBoxPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let parent = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list! {}, bsn_list! {})
    }).unwrap().id();
    let native = widgetry_window_target(app.world(), parent).unwrap();
    let mut roots = Vec::new();
    for _ in 0..2 {
        roots.push(app.world_mut().spawn_scene(bsn! { @widgetry_message_box(native, "Question", WidgetryMessageBoxButtons::Ok, Default::default(), bsn_list! {}) }).unwrap().id());
    }
    app.update();
    (app, roots)
}

#[test]
fn all_result_button_structure_failures_keep_origins_until_real_repair() {
    use bevy_widgetry_button::WidgetryButton;
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, roots) = fixture();
    let mut buttons = Vec::new();
    for &root in &roots {
        let button = app
            .world_mut()
            .query::<(Entity, &WidgetryStyleOwner<WidgetryButton>)>()
            .iter(app.world())
            .find(|(_, owner)| owner.entity == root)
            .unwrap()
            .0;
        buttons.push(button);
        app.world_mut().entity_mut(button).remove::<BorderColor>();
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut origins = std::collections::HashMap::new();
    for _ in 0..2 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        let diagnostics = records
            .iter()
            .flat_map(|record| record.managed().unwrap().diagnostics())
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        for diagnostic in diagnostics {
            let entity = diagnostic
                .contexts()
                .iter()
                .flat_map(|frame| frame.fields())
                .find(|(key, _)| key == "entity")
                .unwrap()
                .1
                .clone();
            let origin = origins
                .entry(entity)
                .or_insert_with(|| diagnostic.origin().clone());
            assert!(std::sync::Arc::ptr_eq(origin, diagnostic.origin()));
        }
    }
    assert_eq!(origins.len(), 2);
    app.world_mut().despawn(roots[0]);
    app.world_mut()
        .entity_mut(buttons[1])
        .insert(BorderColor::default());
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    for (entity, origin) in origins {
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            usize::from(entity == buttons[1].to_string())
        );
    }
}

#[test]
fn cancel_and_reentrant_resolution_are_silent_and_observer_commands_run_before_cleanup() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, roots) = fixture();
    app.add_observer(
        |event: On<WidgetryMessageBoxResultEvent>, mut commands: Commands| {
            let root = event.entity;
            commands.queue(move |world: &mut World| {
                assert!(world.get::<WidgetryMessageBox>(root).is_some());
                assert!(
                    !WidgetryMessageBox::resolve(world, root, WidgetryMessageBoxResult::No)
                        .unwrap()
                );
            });
        },
    );
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(
            WidgetryMessageBox::resolve(
                app.world_mut(),
                roots[0],
                WidgetryMessageBoxResult::Cancel
            )
            .unwrap()
        );
        app.world_mut().flush();
    });
    assert!(logs.records().is_empty());
    assert!(!app.world().entities().contains(roots[0]));
    assert!(app.world().entities().contains(roots[1]));
}

#[test]
fn window_color_failures_keep_the_original_lower_origin_through_message_box_ownership() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, roots) = fixture();
    let root = roots[0];
    app.world_mut().entity_mut(root).remove::<BorderColor>();
    let colors = app
        .world()
        .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
        .colors()
        .message_box;
    let logs = LogCapture::default();
    let origin = logs.run(|| {
        let error = bevy_widgetry_window::internal::apply_owned_window_colors(
            app.world_mut(),
            root,
            &colors.window,
        )
        .unwrap_err();
        error.diagnostics().next().unwrap().origin().clone()
    });
    let error = logs
        .run(|| {
            bevy_widgetry_message_box::internal::apply_owned_message_box_colors(
                app.world_mut(),
                root,
                &colors,
            )
        })
        .unwrap_err();
    let diagnostic = error.diagnostics().next().unwrap();
    assert!(std::sync::Arc::ptr_eq(&origin, diagnostic.origin()));
    assert!(
        diagnostic
            .contexts()
            .iter()
            .any(|frame| frame.operation() == "message_box.window.colors")
    );
    assert_eq!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
            .len(),
        1
    );
}

#[test]
fn invalid_nested_colors_preserve_core_cause_and_do_not_commit() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, roots) = fixture();
    let root = roots[0];
    let before = WidgetryMessageBoxColorOverrides::get(app.world(), root)
        .unwrap()
        .clone();
    let mut colors = before.clone();
    colors.window.close.pressed.foreground = Some(Color::srgba(0., 0., 0., f32::NAN));
    let logs = LogCapture::default();
    let failure = logs
        .run(|| WidgetryMessageBoxColorOverrides::set_in_world(app.world_mut(), root, colors))
        .unwrap_err();
    assert!(
        matches!(failure.kind(), MessageBoxError::Core { source } if matches!(source.as_ref(), bevy_widgetry_core::diagnostics::CoreError::InvalidColor { .. }))
    );
    assert!(
        failure
            .origin()
            .location()
            .file()
            .replace('\\', "/")
            .ends_with("core/src/color.rs")
    );
    assert_eq!(
        logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    let fields = failure
        .contexts()
        .iter()
        .flat_map(|frame| frame.fields())
        .collect::<Vec<_>>();
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "part" && value == "close")
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "state" && value == "pressed")
    );
    assert_eq!(
        WidgetryMessageBoxColorOverrides::get(app.world(), root).unwrap(),
        &before
    );
}
