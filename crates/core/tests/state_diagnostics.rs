#![allow(clippy::disallowed_macros)]

use bevy::{prelude::*, ui::Checked};
use bevy_widgetry_core::{
    color::validate_color,
    diagnostics::{CoreError, CoreFailure},
    disabled::{queue_intrinsic_disabled, set_intrinsic_disabled},
    exclusive_selection::{InitialSelection, SelectionHistory},
    scene::spawn_scene,
    text::{WidgetryText, WidgetryTextColorOverrides, WidgetryTextStateColorOverrides},
};
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

#[test]
fn invalid_colors_keep_original_values_and_nested_template_identity() {
    let logs = LogCapture::default();
    logs.run(|| {
        let bad = Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0);
        let failure = validate_color(bad).unwrap_err();
        assert!(matches!(failure.kind(), CoreError::InvalidColor { color } if color.to_linear().red.is_nan()));
        assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
        let mut app = scene_app();
        let result = spawn_scene(app.world_mut(), bsn! { Node Children [WidgetryText Node Children [
            @WidgetryText { @colors: {WidgetryTextColorOverrides { normal: WidgetryTextStateColorOverrides { foreground: Some(bad) }, ..default() }} }
        ]] }).unwrap_err();
        let diagnostic = result.diagnostics().diagnostics().next().unwrap();
        let mut cause: &(dyn Error + 'static) = diagnostic;
        while cause.downcast_ref::<CoreFailure>().is_none() { cause = cause.source().unwrap(); }
        let source = cause.downcast_ref::<CoreFailure>().unwrap();
        assert!(matches!(source.kind(), CoreError::InvalidColor { .. }));
        assert_eq!(source.origin().id(), diagnostic.origin().id());
        assert_eq!(logs.diagnostic_records(source.origin().id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(source.origin().id(), DiagnosticPhase::Handled).is_empty());
    });
}

#[test]
fn disabled_requests_reject_stale_targets_and_missing_runtime_without_mutation() {
    let mut world = World::new();
    let target = world.spawn(Node::default()).id();
    let error = set_intrinsic_disabled(&mut world, target, true).unwrap_err();
    assert!(matches!(error.kind(), CoreError::MissingResource { entity, .. } if *entity == target));
    assert!(world.get::<bevy::ui::InteractionDisabled>(target).is_none());
    world.despawn(target);
    let error = set_intrinsic_disabled(&mut world, target, true).unwrap_err();
    assert!(matches!(error.kind(), CoreError::EntityAccess { entity, .. } if *entity == target));
    assert!(error.kind().source().is_some());
    let errors = ErrorCapture::default();
    let mut app = App::new();
    app.set_error_handler(ErrorCapture::handler());
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    errors.run(|| {
        queue_intrinsic_disabled(&mut app.world_mut().commands(), stale, true);
        app.world_mut().flush();
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert!(records[0].error.is::<WidgetryEcsError>());
}

#[test]
fn duplicate_disabled_and_text_setters_are_silent_and_return_no_change() {
    let mut app = scene_app();
    let text = app
        .world_mut()
        .spawn((Node::default(), Text::new("content"), WidgetryText))
        .id();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!set_intrinsic_disabled(app.world_mut(), text, false).unwrap());
        assert!(set_intrinsic_disabled(app.world_mut(), text, true).unwrap());
        assert!(!set_intrinsic_disabled(app.world_mut(), text, true).unwrap());
        assert!(WidgetryText::set_color_in_world(app.world_mut(), text, Color::BLACK).unwrap());
        assert!(!WidgetryText::set_color_in_world(app.world_mut(), text, Color::BLACK).unwrap());
        assert!(logs.records().is_empty());
    });
}

#[test]
fn exclusive_selection_errors_keep_root_and_native_access_source() {
    let mut world = World::new();
    let root = world.spawn_empty().id();
    let failure =
        SelectionHistory::repair(&mut world, root, &[], InitialSelection::First).unwrap_err();
    assert!(
        matches!(failure.kind(), CoreError::EmptySelection { root: actual } if *actual == root)
    );
    let a = world.spawn(Checked).id();
    let b = world.spawn(Checked).id();
    let failure =
        SelectionHistory::repair(&mut world, root, &[a, b], InitialSelection::Checked).unwrap_err();
    assert!(
        matches!(failure.kind(), CoreError::MultipleChecked { root: actual, .. } if *actual == root)
    );
    world.despawn(root);
    let failure = SelectionHistory::reset(&mut world, root).unwrap_err();
    assert!(matches!(failure.kind(), CoreError::EntityAccess { entity, .. } if *entity == root));
    assert!(failure.kind().source().is_some());
}

#[test]
fn stale_selection_structure_is_rejected_before_changing_valid_options() {
    let mut world = World::new();
    let root = world.spawn_empty().id();
    let selected = world.spawn(Checked).id();
    let next = world.spawn_empty().id();
    world.despawn(root);
    assert!(
        SelectionHistory::repair(&mut world, root, &[next, selected], InitialSelection::First)
            .is_err()
    );
    assert!(world.get::<Checked>(selected).is_some());
    assert!(world.get::<Checked>(next).is_none());
    assert!(SelectionHistory::select(&mut world, &[selected], next).is_err());
    assert!(world.get::<Checked>(selected).is_some());
    world.despawn(next);
    assert!(SelectionHistory::select(&mut world, &[selected, next], next).is_err());
    assert!(world.get::<Checked>(selected).is_some());
}
