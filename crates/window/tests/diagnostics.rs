#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{ecs::schedule::SingleThreadedExecutor, prelude::*};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use bevy_widgetry_window::*;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fixture() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}
fn window(app: &mut App) -> Entity {
    app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list! { WidgetryWindowTitle }, bsn_list! {})
    }).unwrap().id()
}

#[test]
fn all_invalid_window_bindings_reach_the_host_without_destroying_borrowed_resources() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    let mut resources = Vec::new();
    let mut roots = Vec::new();
    for _ in 0..2 {
        let native = app.world_mut().spawn(Window::default()).id();
        let camera = app.world_mut().spawn(Camera2d).id();
        roots.push(app.world_mut().spawn_scene(bsn! {
            @widgetry_window(native, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list! {}, bsn_list! {})
        }).unwrap().id());
        resources.extend([native, camera]);
    }
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let records = errors.take_records();
    let diagnostics = records
        .iter()
        .flat_map(|record| record.managed().unwrap().diagnostics())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2);
    assert!(
        roots
            .iter()
            .all(|root| !app.world().entities().contains(*root))
    );
    assert!(
        resources
            .iter()
            .all(|entity| app.world().entities().contains(*entity))
    );
}

#[test]
fn simultaneous_title_failures_keep_each_origin_until_real_repair() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    let a = window(&mut app);
    let b = window(&mut app);
    app.update();
    let titles = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryWindowTitle>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    for &title in &titles {
        app.world_mut().entity_mut(title).remove::<Text>();
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
            let root = diagnostic
                .contexts()
                .iter()
                .flat_map(|frame| frame.fields())
                .find(|(key, _)| key == "root")
                .unwrap()
                .1
                .clone();
            let origin = origins
                .entry(root)
                .or_insert_with(|| diagnostic.origin().clone());
            assert!(std::sync::Arc::ptr_eq(origin, diagnostic.origin()));
        }
    }
    assert_eq!(origins.len(), 2);
    assert!(origins.contains_key(&a.to_string()) && origins.contains_key(&b.to_string()));
    for &title in &titles {
        app.world_mut().entity_mut(title).insert(Text::default());
    }
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    for origin in origins.values() {
        assert_eq!(
            logs.diagnostic_records(
                origin.id(),
                bevy_widgetry_test_utils::DiagnosticPhase::Source
            )
            .len(),
            1
        );
        assert_eq!(
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );
    }
    app.world_mut().entity_mut(titles[0]).remove::<Text>();
    errors.run(|| app.update());
    let records = errors.take_records();
    let origin = records[0]
        .managed()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap()
        .origin();
    assert!(
        origins
            .values()
            .all(|old| !std::sync::Arc::ptr_eq(old, origin))
    );
}

#[test]
fn ambiguous_title_identity_is_independent_of_archetype_iteration_order() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    window(&mut app);
    app.update();
    let first = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryWindowTitle>>()
        .single(app.world())
        .unwrap();
    let parent = app.world().get::<ChildOf>(first).unwrap().parent();
    let mut titles = vec![first];
    for _ in 0..2 {
        titles.push(
            app.world_mut()
                .spawn_scene(bsn! { WidgetryWindowTitle ChildOf(parent) })
                .unwrap()
                .id(),
        );
    }
    let errors = ErrorCapture::default();
    let mut origin = None;
    for title in titles {
        app.world_mut()
            .entity_mut(title)
            .insert(Name::new("same ambiguous set"));
        errors.run(|| app.update());
        let records = errors.take_records();
        let current = records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
            .clone();
        if let Some(origin) = &origin {
            assert!(std::sync::Arc::ptr_eq(origin, &current));
        } else {
            origin = Some(current);
        }
    }
}

#[test]
fn failed_owned_binding_reclaims_allocations_when_the_camera_binding_is_missing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    let root = window(&mut app);
    let native = widgetry_window_target(app.world(), root).unwrap();
    let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
    app.world_mut().entity_mut(root).remove::<UiTargetCamera>();
    let external = app.world_mut().spawn(Window::default()).id();
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(!app.world().entities().contains(root));
    assert!(!app.world().entities().contains(native));
    assert!(!app.world().entities().contains(camera));
    assert!(app.world().entities().contains(external));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert!(
        diagnostic
            .contexts()
            .iter()
            .flat_map(|frame| frame.fields())
            .any(|(key, value)| key == "owned"
                && value.contains(&format!("{native:?}"))
                && value.contains(&format!("{camera:?}")))
    );
}

#[test]
fn simultaneous_style_failures_keep_all_sources_and_do_not_recover_on_destruction() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    let a = window(&mut app);
    let b = window(&mut app);
    app.update();
    for root in [a, b] {
        app.world_mut().entity_mut(root).remove::<BorderColor>();
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
            let root = diagnostic
                .contexts()
                .iter()
                .flat_map(|frame| frame.fields())
                .find(|(key, _)| key == "root")
                .unwrap()
                .1
                .clone();
            let origin = origins
                .entry(root)
                .or_insert_with(|| diagnostic.origin().clone());
            assert!(std::sync::Arc::ptr_eq(origin, diagnostic.origin()));
        }
    }
    app.world_mut().despawn(a);
    app.world_mut().entity_mut(b).insert(BorderColor::default());
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    for (root, origin) in origins {
        assert_eq!(
            logs.diagnostic_records(
                origin.id(),
                bevy_widgetry_test_utils::DiagnosticPhase::Source
            )
            .len(),
            1
        );
        assert_eq!(
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            usize::from(root == b.to_string())
        );
    }
}

#[test]
fn invalid_colors_keep_the_core_cause_origin_and_do_not_commit() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = fixture();
    let root = window(&mut app);
    app.update();
    let before = WidgetryWindowColorOverrides::get(app.world(), root)
        .unwrap()
        .clone();
    let mut colors = before.clone();
    colors.close.pressed.foreground = Some(Color::srgba(0., 0., 0., f32::NAN));
    let logs = LogCapture::default();
    let error = logs
        .run(|| WidgetryWindowColorOverrides::set_in_world(app.world_mut(), root, colors))
        .unwrap_err();
    assert!(
        matches!(error.kind(), WindowError::Core { source } if matches!(source.as_ref(), bevy_widgetry_core::diagnostics::CoreError::InvalidColor { .. }))
    );
    assert!(
        error
            .origin()
            .location()
            .file()
            .replace('\\', "/")
            .ends_with("core/src/color.rs")
    );
    assert_eq!(
        logs.diagnostic_records(
            error.origin().id(),
            bevy_widgetry_test_utils::DiagnosticPhase::Source
        )
        .len(),
        1
    );
    assert_eq!(
        WidgetryWindowColorOverrides::get(app.world(), root).unwrap(),
        &before
    );
    let fields = error
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
}

#[test]
fn partial_window_scene_failures_keep_lower_origins_native_strategy_and_resource_ownership() {
    use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
    use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for owned in [false, true] {
        for panic in [false, true] {
            let mut app = fixture();
            let origin = std::sync::Arc::new(std::sync::Mutex::new(None));
            let captured = origin.clone();
            let content = bsn_list! { template(move |_| -> Result<Name, BevyError> {
                let failure = WidgetryFailure::new(std::io::Error::other("business window scene"), ErrorFrame::new("business.window.content"));
                *captured.lock().unwrap() = Some(failure.origin().clone());
                Err(failure.into_bevy_error().with_severity(if panic { bevy::ecs::error::Severity::Panic } else { bevy::ecs::error::Severity::Error }).with_payload(Box::new(47u32)))
            }) };
            let native = app
                .world_mut()
                .spawn(prepare_native_window(Window::default()))
                .id();
            let camera = app.world_mut().spawn(Camera2d).id();
            let scene: Box<dyn Scene> = if owned {
                Box::new(owned_widgetry_window(
                    Window::default(),
                    WidgetryWindowControlsConfig::default(),
                    WidgetryWindowBackground::Theme,
                    Default::default(),
                    bsn_list! {},
                    content,
                ))
            } else {
                Box::new(widgetry_window(
                    native,
                    camera,
                    WidgetryWindowControlsConfig::default(),
                    WidgetryWindowBackground::Theme,
                    Default::default(),
                    bsn_list! {},
                    content,
                ))
            };
            let errors = ErrorCapture::default();
            let logs = LogCapture::default();
            logs.run(|| {
                errors.run(|| {
                    app.world_mut()
                        .commands()
                        .spawn_scene_with_error_handler(scene);
                    app.world_mut().flush();
                })
            });
            let mut records = errors.take_records();
            assert_eq!(records.len(), 1);
            let mut record = records.pop().unwrap();
            assert_eq!(
                record.error.severity(),
                if panic {
                    bevy::ecs::error::Severity::Panic
                } else {
                    bevy::ecs::error::Severity::Error
                }
            );
            assert_eq!(
                *record
                    .error
                    .take_payload()
                    .unwrap()
                    .downcast::<u32>()
                    .unwrap(),
                47
            );
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            assert!(std::sync::Arc::ptr_eq(
                origin.lock().unwrap().as_ref().unwrap(),
                diagnostic.origin()
            ));
            assert_eq!(
                logs.diagnostic_records(
                    diagnostic.origin().id(),
                    bevy_widgetry_test_utils::DiagnosticPhase::Source
                )
                .len(),
                1
            );
            assert_eq!(
                app.world_mut().query::<&Window>().iter(app.world()).count(),
                1
            );
            assert_eq!(
                app.world_mut().query::<&Camera>().iter(app.world()).count(),
                1
            );
            assert!(
                app.world().entities().contains(native) && app.world().entities().contains(camera)
            );
        }
    }
}
