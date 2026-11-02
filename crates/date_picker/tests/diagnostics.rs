#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_date_picker::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;
use time::macros::date;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn picker(app: &mut App) -> Entity {
    bevy_widgetry_core::scene::spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryDatePicker {
                @selected_date: {Some(date!(2026-10-15))},
                @min_date: {Some(date!(2026-10-01))},
                @max_date: {Some(date!(2026-10-31))},
            }
        },
    )
    .unwrap()
}

#[test]
fn invalid_dates_keep_exact_request_bounds_source_and_origin() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin);
    let root = picker(&mut app);
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, Some(date!(2026-11-01))).unwrap_err();
        assert!(matches!(error.kind(), DatePickerError::SelectionOutOfRange { selected, min, max }
            if *selected == date!(2026-11-01) && *min == Some(date!(2026-10-01)) && *max == Some(date!(2026-10-31))));
        let origin = error.origin().clone();
        let boundary = error.into_bevy_error();
        assert_eq!(boundary.severity(), bevy::ecs::error::Severity::Error);
        let managed = boundary.downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>().unwrap();
        assert!(std::sync::Arc::ptr_eq(&origin, managed.diagnostics().next().unwrap().origin()));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());

        for displayed in [DatePickerMonth {year:i32::MAX, month:Month::January}, Date::MAX.into()] {
            let error = WidgetryDatePicker::set_displayed_month_in_world(app.world_mut(), root, displayed).unwrap_err();
            let DatePickerError::Calendar { displayed: actual, source, grid_index, .. } = error.kind() else { panic!("{error}"); };
            assert_eq!(*actual, displayed);
            assert_eq!(error.kind().source().unwrap().downcast_ref::<time::error::ComponentRange>().unwrap(), source);
            assert_eq!(grid_index.is_some(), displayed == DatePickerMonth::from(Date::MAX));
        }
    });
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 15))
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        date!(2026 - 10 - 01).into()
    );
}

#[test]
fn commands_preserve_domain_diagnostics_and_colors_keep_core_source() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin)
        .set_error_handler(ErrorCapture::handler());
    let root = picker(&mut app);
    let logs = LogCapture::default();
    let capture = ErrorCapture::default();
    logs.run(|| {
        WidgetryDatePicker::set_selected_date(
            &mut app.world_mut().commands(),
            root,
            Some(date!(2026 - 12 - 01)),
        );
        capture.run(|| app.world_mut().flush());
        let errors = capture.take();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = errors[0]
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "date_picker.set_selected_date")
        );

        let error = WidgetryDatePickerColorOverrides::set_in_world(
            app.world_mut(),
            root,
            WidgetryDatePickerColorOverrides {
                foreground: Some(Color::linear_rgba(f32::NAN, 0., 0., 1.)),
                ..default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error.kind(),
            DatePickerError::Core {
                source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor { .. }
            }
        ));
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
        assert!(
            error
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "date_picker.set_colors")
        );
    });
}

#[test]
fn invalid_initial_bounds_preserve_exact_dates_through_scene_expansion() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin);
    let logs = LogCapture::default();
    logs.run(|| {
        let error = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetryDatePicker {
                    @min_date: {Some(date!(2026-10-31))},
                    @max_date: {Some(date!(2026-10-01))},
                }
            },
        )
        .unwrap_err()
        .into_bevy_error();
        let diagnostic = error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let mut source = diagnostic.source();
        let mut found = false;
        while let Some(error) = source {
            if let Some(DatePickerError::ReversedBounds { min, max }) =
                error.downcast_ref::<DatePickerError>()
            {
                assert_eq!(*min, date!(2026 - 10 - 31));
                assert_eq!(*max, date!(2026 - 10 - 01));
                found = true;
                break;
            }
            source = error.source();
        }
        assert!(found);
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "date_picker.initialize")
        );
    });
}
