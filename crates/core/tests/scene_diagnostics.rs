#![allow(clippy::disallowed_macros)]

use bevy::{
    ecs::error::{ContextExt, Severity},
    prelude::*,
    scene::{ApplySceneError, SpawnSceneError},
};
use bevy_widgetry_core::scene::{SceneFailure, original_scene_error, spawn_scene};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{WidgetryEcsError, into_bevy_error},
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::{io, sync::Arc};

#[test]
fn scene_batch_retains_every_origin_and_typed_source() {
    use bevy_widgetry_diagnostics::bevy::DiagnosticBatch;
    let logs = LogCapture::default();
    let (batch, origins) = logs.run(|| {
        let mut batch = DiagnosticBatch::default();
        let mut origins = Vec::new();
        for index in 0..3 {
            let failure = WidgetryFailure::new(
                io::Error::other(format!("source {index}")),
                ErrorFrame::new("batch template"),
            );
            origins.push(Arc::clone(failure.origin()));
            batch.push(failure);
        }
        (batch, origins)
    });
    let failure = logs.run(|| {
        SceneFailure::from_native(
            SpawnSceneError::ApplySceneError(ApplySceneError::TemplateBuildError(
                batch.into_bevy_error().unwrap(),
            )),
            Entity::PLACEHOLDER,
            "batch",
        )
    });
    let boundary = failure.into_bevy_error();
    let diagnostics: Vec<_> = boundary
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .collect();
    assert_eq!(diagnostics.len(), 3);
    for (index, (diagnostic, origin)) in diagnostics.iter().zip(&origins).enumerate() {
        assert!(Arc::ptr_eq(diagnostic.origin(), origin));
        assert!(
            diagnostic
                .source()
                .unwrap()
                .to_string()
                .contains(&format!("source {index}"))
        );
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
    assert!(std::ptr::eq(
        original_scene_error(diagnostics[0]).unwrap(),
        original_scene_error(diagnostics[2]).unwrap()
    ));
}

#[test]
fn synchronous_failure_explicitly_crosses_a_real_system_boundary() {
    fn fail(world: &mut World) -> Result<(), BevyError> {
        spawn_scene(world, bsn! {
            template(|_| Err::<Node, _>(bevy_widgetry_core::scene_source_error!(io::Error::other("system source"); "同步 Scene 失败")))
        }).map_err(SceneFailure::into_bevy_error)?;
        Ok(())
    }
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_systems(Update, fail);
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.update()));
    let captured = errors.take_records();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].context.kind(), "system");
    assert_eq!(captured[0].error.severity(), Severity::Error);
    let diagnostic = captured[0].managed().unwrap().diagnostics().next().unwrap();
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}

#[inline(never)]
fn native_error_with_trace() -> BevyError {
    BevyError::new_with_backtrace(
        Severity::Panic,
        io::Error::other("native cause\n0: field zero invalid"),
        std::backtrace::Backtrace::force_capture(),
    )
    .with_payload(Box::new(47_u32))
}

#[test]
fn native_panic_payload_and_original_object_survive_without_logging_native_stack() {
    let native = native_error_with_trace();
    assert!(native.to_string().contains("native_error_with_trace"));
    let logs = LogCapture::default();
    let failure = logs.run(|| {
        SceneFailure::from_native(
            SpawnSceneError::ApplySceneError(ApplySceneError::TemplateBuildError(native)),
            Entity::PLACEHOLDER,
            "foreign scene",
        )
    });
    assert!(failure.to_string().contains("field zero invalid"));
    assert!(!failure.to_string().contains("native_error_with_trace"));
    let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
    let original = original_scene_error(diagnostic).unwrap();
    assert!(original.to_string().contains("native_error_with_trace"));
    assert_eq!(logs.records().len(), 1);
    assert!(!logs.records()[0].fields["error"].contains("native_error_with_trace"));
    let mut boundary = failure.into_bevy_error();
    assert_eq!(boundary.severity(), Severity::Panic);
    assert_eq!(
        *boundary.take_payload().unwrap().downcast::<u32>().unwrap(),
        47
    );
}

#[test]
fn cached_related_and_native_context_preserve_original_managed_diagnostic() {
    let logs = LogCapture::default();
    let source = logs.run(|| {
        WidgetryFailure::new(
            io::Error::other("typed source"),
            ErrorFrame::new("business source"),
        )
    });
    let origin = Arc::clone(source.origin());
    let contextual: Result<(), BevyError> = Err(into_bevy_error(source));
    let native = contextual.context("native context").unwrap_err();
    let original = SpawnSceneError::ApplySceneError(ApplySceneError::RelatedSceneError {
        relationship_type_name: "ChildOf",
        index: 3,
        error: Box::new(ApplySceneError::CachedSceneApplyError {
            cached: Some("fixture.scene".into()),
            error: Box::new(ApplySceneError::TemplateBuildError(native)),
        }),
    });
    let failure = logs.run(|| SceneFailure::from_native(original, Entity::PLACEHOLDER, "apply"));
    let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
    assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
    assert!(
        diagnostic
            .contexts()
            .iter()
            .any(|frame| frame.operation() == "RelatedScene")
    );
    assert!(
        diagnostic
            .contexts()
            .iter()
            .any(|frame| frame.operation() == "CachedSceneApply")
    );
    assert!(
        original_scene_error(diagnostic)
            .unwrap()
            .to_string()
            .contains("native context")
    );
    let mut source = diagnostic.source();
    let mut found = false;
    while let Some(error) = source {
        found |= error.downcast_ref::<io::Error>().is_some();
        source = error.source();
    }
    assert!(found);
    let boundary = failure.into_bevy_error();
    assert_eq!(boundary.severity(), Severity::Error);
    let diagnostic = boundary
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
    assert!(
        original_scene_error(diagnostic)
            .unwrap()
            .to_string()
            .contains("fixture.scene")
    );
    assert_eq!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}

#[test]
fn synchronous_scene_failure_returns_diagnostics_without_invoking_host_handler() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let capture = ErrorCapture::default();
    let logs = LogCapture::default();
    let failure = capture.run(|| logs.run(|| spawn_scene(app.world_mut(), bsn! {
        Children [template(|_| Err::<Node, _>(BevyError::error(io::Error::other("foreign cause")))) ]
    }))).unwrap_err();
    assert!(capture.take_records().is_empty());
    let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
    let id = diagnostic.origin().id().clone();
    assert_eq!(
        logs.diagnostic_records(&id, DiagnosticPhase::Source).len(),
        1
    );
    let error = failure.into_bevy_error();
    assert_eq!(error.severity(), Severity::Error);
}
