#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_core::{
    icon::{IconError, IconFailure, WidgetryIcon, WidgetryIconPlugin},
    scene::{WidgetrySceneCommandsExt, spawn_scene},
};
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

#[allow(clippy::expect_used)]
fn icon_kind<'a>(mut error: &'a (dyn Error + 'static)) -> &'a IconError {
    loop {
        if let Some(failure) = error.downcast_ref::<IconFailure>() {
            return failure.kind();
        }
        error = error.source().expect("original Icon failure must survive");
    }
}

#[test]
fn independent_invalid_requests_keep_target_path_and_distinct_origins() {
    let logs = LogCapture::default();
    logs.run(|| {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let a = WidgetryIcon::set_svg_in_world(&mut world, entity, "next.svg").unwrap_err();
        let b = WidgetryIcon::set_svg_in_world(&mut world, entity, "next.svg").unwrap_err();
        assert!(
            matches!(a.kind(), IconError::InvalidTarget { entity: target, .. } if *target == entity)
        );
        assert_eq!(
            a.contexts()
                .iter()
                .flat_map(|context| context.fields())
                .find(|(k, _)| k == "path")
                .unwrap()
                .1,
            "next.svg"
        );
        assert_ne!(a.origin().id(), b.origin().id());
        for error in [&a, &b] {
            assert_eq!(
                logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    });
}

#[test]
fn nested_template_and_queued_setters_preserve_domain_types_at_real_boundaries() {
    let mut app = scene_app();
    app.add_plugins(WidgetryIconPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let synchronous =
        spawn_scene(app.world_mut(), bsn! { Node Children [@WidgetryIcon] }).unwrap_err();
    let diagnostic = synchronous.diagnostics().diagnostics().next().unwrap();
    assert!(matches!(icon_kind(diagnostic), IconError::EmptyPath { .. }));
    let errors = ErrorCapture::default();
    errors.run(|| {
        let target = app.world_mut().spawn_empty().id();
        WidgetryIcon::set_color(&mut app.world_mut().commands(), target, Color::BLACK);
        app.world_mut()
            .commands()
            .spawn_scene_with_error_handler(bsn! { Node Children [@WidgetryIcon] });
        app.world_mut().flush();
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    for record in &records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record
            .error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(matches!(
            icon_kind(diagnostic),
            IconError::InvalidTarget { .. } | IconError::EmptyPath { .. }
        ));
    }
}
