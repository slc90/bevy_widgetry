#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{prelude::*, text::EditableText};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture, scene_app};
use bevy_widgetry_text_field::{
    TextFieldError, WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn both_variants_identify_the_missing_editor_and_preserve_diagnostic_origin() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    for read_only in [false, true] {
        let root = if read_only {
            spawn_scene(app.world_mut(), bsn! { @WidgetryReadOnlyTextField }).unwrap()
        } else {
            spawn_scene(app.world_mut(), bsn! { @WidgetryTextField }).unwrap()
        };
        let frame = bevy_widgetry_text_field::internal::frame(app.world(), root).unwrap();
        let editor = app.world().get::<Children>(frame).unwrap()[0];
        app.world_mut().entity_mut(editor).remove::<EditableText>();
        let logs = LogCapture::default();
        logs.run(|| {
            let error = if read_only { WidgetryReadOnlyTextField::set_text_in_world(app.world_mut(), root, "new") }
            else { WidgetryTextField::set_text_in_world(app.world_mut(), root, "new") }.unwrap_err();
            assert!(matches!(error.kind(), TextFieldError::MissingPart { root: owner, entity, component: "EditableText" } if *owner == root && *entity == editor));
            let origin = error.origin().clone();
            let error = error.into_bevy_error();
            let diagnostic = error.downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
            assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
            assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
        });
    }
}

#[test]
fn count_failure_reports_that_content_was_written_and_commands_keep_the_diagnosis() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryTextField { @value: {"before".to_owned()}, @show_count: true } },
    )
    .unwrap();
    let count = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().entity_mut(count).remove::<Text>();
    let error = WidgetryTextField::set_text_in_world(app.world_mut(), root, "after").unwrap_err();
    assert!(
        matches!(error.kind(), TextFieldError::MissingPart { entity, component: "Text", .. } if *entity == count)
    );
    assert!(error.contexts().iter().any(|frame| {
        frame.operation() == "text_field.set_text.count_projection"
            && frame
                .fields()
                .iter()
                .any(|(field, value)| field == "text_written" && value == "true")
    }));
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "after");
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            WidgetryTextField::set_text(&mut app.world_mut().commands(), root, "queued");
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), root).unwrap(),
        "queued"
    );
}

#[test]
fn missing_resources_fail_before_writing_and_valid_truncation_is_silent() {
    use bevy::text::FontCx;
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryTextField { @value: {"old".to_owned()}, @max_length: {Some(3)} } },
    )
    .unwrap();
    let fonts = app.world_mut().remove_resource::<FontCx>().unwrap();
    let error =
        WidgetryTextField::set_text_in_world(app.world_mut(), root, "new text").unwrap_err();
    assert!(
        matches!(error.kind(), TextFieldError::MissingResource { root: owner, resource: "FontCx" } if *owner == root)
    );
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "old");
    app.insert_resource(fonts);
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(WidgetryTextField::set_text_in_world(app.world_mut(), root, "新文本过长").unwrap());
        assert!(
            !WidgetryTextField::set_text_in_world(app.world_mut(), root, "新文本过长").unwrap()
        );
        assert_eq!(
            WidgetryTextField::text(app.world(), root).unwrap(),
            "新文本"
        );
    });
    assert!(logs.records().is_empty());
}

#[test]
fn auto_size_and_core_color_failures_keep_original_fields_and_causes() {
    use bevy_widgetry_text_field::{AutoSize, TextFieldFailure, WidgetryTextFieldColorOverrides};
    use std::error::Error;
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    for (multiline, config) in [
        (false, AutoSize::default()),
        (
            true,
            AutoSize {
                min_rows: 0,
                max_rows: 4,
            },
        ),
        (
            true,
            AutoSize {
                min_rows: 7,
                max_rows: 3,
            },
        ),
    ] {
        let error = spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryTextField { @multiline: multiline, @auto_size: {Some(config)} } },
        )
        .unwrap_err()
        .into_bevy_error();
        let diagnostic = error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let mut cause: &dyn Error = diagnostic;
        let original = loop {
            if let Some(original) = cause.downcast_ref::<TextFieldFailure>() {
                break original;
            }
            cause = cause.source().unwrap();
        };
        assert!(
            matches!(original.kind(), TextFieldError::InvalidAutoSize { multiline: actual_multiline, config: actual } if *actual_multiline == multiline && *actual == config)
        );
        assert!(std::sync::Arc::ptr_eq(
            original.origin(),
            diagnostic.origin()
        ));
    }
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryTextField }).unwrap();
    let mut colors = WidgetryTextFieldColorOverrides::default();
    colors.editable.focused.caret = Some(Color::srgb(f32::NAN, 0.0, 0.0));
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetryTextFieldColorOverrides::set_in_world(app.world_mut(), root, colors)
            .unwrap_err();
        assert!(matches!(error.kind(), TextFieldError::Core { .. }));
        assert!(
            error
                .kind()
                .source()
                .unwrap()
                .is::<bevy_widgetry_core::diagnostics::CoreError>()
        );
        assert_eq!(
            logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(error.contexts().iter().any(|frame| {
            frame
                .fields()
                .iter()
                .any(|(field, value)| field == "field" && value == "caret")
        }));
    });
}
