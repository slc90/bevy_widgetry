#![cfg(feature = "bevy")]
#![allow(clippy::disallowed_macros)]

use bevy_app::{App, Update};
use bevy_ecs::{
    error::{BevyError, ErrorContext, Severity},
    prelude::*,
    schedule::SingleThreadedExecutor,
};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{BoundaryFailure, DiagnosticBatch, WidgetryEcsError, dispatch_error, into_bevy_error},
};
use std::{cell::RefCell, io, sync::Arc};

#[test]
fn shared_diagnostic_survives_multiple_managed_and_bevy_context_layers() {
    let failure = Arc::new(WidgetryFailure::new(
        io::Error::other("底层对象"),
        ErrorFrame::new("源头"),
    ));
    let origin = Arc::clone(failure.origin());
    let managed = WidgetryEcsError::from_shared(failure)
        .with_context(ErrorFrame::new("上层一"))
        .with_context(ErrorFrame::new("上层二"));
    let error = BoundaryFailure::Widgetry(managed).into_bevy_error();
    let error = BoundaryFailure::Native(error)
        .with_context(ErrorFrame::new("Bevy 包装"))
        .into_bevy_error();
    let managed = error.downcast_ref::<WidgetryEcsError>().unwrap();
    let diagnostic = managed.diagnostics().next().unwrap();
    assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
    assert_eq!(
        diagnostic
            .contexts()
            .iter()
            .map(|context| context.operation())
            .collect::<Vec<_>>(),
        ["源头", "上层一", "上层二"]
    );
    let mut cause = diagnostic.source();
    let mut found = false;
    while let Some(error) = cause {
        found |= error.downcast_ref::<io::Error>().is_some();
        cause = error.source();
    }
    assert!(found);
    assert!(format!("{error}").contains("Bevy 包装"));
    dispatch_error(
        error,
        ErrorContext::Command {
            name: "wrapped".into(),
        },
        |diagnostic, outer, context| {
            assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
            assert!(format!("{outer}").contains("Bevy 包装"));
            assert_eq!(context.kind(), "command");
        },
        |_, _| panic!("managed diagnostic became native"),
    );
}

thread_local! {
    static CAPTURED: RefCell<Vec<(BevyError, ErrorContext)>> = const { RefCell::new(Vec::new()) };
}

fn capture(error: BevyError, context: ErrorContext) {
    CAPTURED.with(|errors| errors.borrow_mut().push((error, context)));
}

#[derive(Resource)]
struct Pending(Option<WidgetryFailure<io::Error>>);

fn fail_system(mut pending: ResMut<Pending>) -> Result<(), BevyError> {
    match pending.0.take() {
        Some(failure) => Err(into_bevy_error(failure)),
        None => Ok(()),
    }
}

#[derive(Event)]
struct Trigger;

fn fail_observer(_: On<Trigger>, pending: ResMut<Pending>) -> Result<(), BevyError> {
    fail_system(pending)
}

#[test]
fn real_system_observer_and_command_keep_source_origin_and_error_severity() {
    for route in ["system", "observer", "command"] {
        CAPTURED.with(|errors| errors.borrow_mut().clear());
        let failure = WidgetryFailure::new(io::Error::other("真实源头"), ErrorFrame::new(route));
        let origin = Arc::clone(failure.origin());
        let mut app = App::new();
        app.set_error_handler(capture);
        app.insert_resource(Pending(Some(failure)));
        match route {
            "system" => {
                app.add_systems(Update, fail_system);
                app.edit_schedule(Update, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
                app.update();
            }
            "observer" => {
                app.add_observer(fail_observer);
                app.world_mut().trigger(Trigger);
            }
            _ => {
                app.world_mut()
                    .commands()
                    .queue(|world: &mut World| -> Result<(), BevyError> {
                        if let Some(error) = world.resource_mut::<Pending>().0.take() {
                            return Err(into_bevy_error(error));
                        }
                        Ok(())
                    });
                app.world_mut().flush();
            }
        }
        let captured = CAPTURED.with(|errors| std::mem::take(&mut *errors.borrow_mut()));
        assert_eq!(captured.len(), 1, "{route}");
        let (error, context) = &captured[0];
        assert_eq!(error.severity(), Severity::Error);
        assert_eq!(context.kind(), route);
        let diagnostic = error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
        assert!(
            diagnostic
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .is_some()
        );
        assert!(format!("{error}").contains("真实源头"));
    }
}

#[test]
fn batch_transports_every_existing_failure_and_empty_batch_is_success() {
    assert!(DiagnosticBatch::default().into_bevy_error().is_none());
    let mut batch = DiagnosticBatch::default();
    let mut origins = Vec::new();
    for index in 0..3 {
        let failure = WidgetryFailure::new(
            io::Error::other("同文案失败"),
            ErrorFrame::new("批量操作").field("index", index),
        );
        origins.push(Arc::clone(failure.origin()));
        batch.push(failure);
    }
    let error = batch.into_bevy_error().unwrap();
    assert_eq!(error.severity(), Severity::Error);
    let mut reported = Vec::new();
    dispatch_error(
        error,
        ErrorContext::Command {
            name: "batch".into(),
        },
        |diagnostic, _, _| reported.push(Arc::clone(diagnostic.origin())),
        |_, _| panic!("managed batch became native"),
    );
    assert_eq!(reported.len(), 3);
    assert!(
        reported
            .iter()
            .zip(&origins)
            .all(|(left, right)| Arc::ptr_eq(left, right))
    );
}

#[test]
fn foreign_bevy_errors_keep_the_object_context_severity_and_panic_payload() {
    let native = BevyError::panic(io::Error::other("外来失败"), Box::new(73_u32));
    let native = BoundaryFailure::Native(native)
        .with_context(ErrorFrame::new("外部场景"))
        .into_bevy_error();
    assert_eq!(native.severity(), Severity::Panic);
    assert!(native.downcast_ref::<io::Error>().is_some());
    assert!(format!("{native}").contains("外部场景"));
    dispatch_error(
        native,
        ErrorContext::Command {
            name: "foreign".into(),
        },
        |_, _, _| panic!("native error became managed"),
        |mut native, _| {
            assert_eq!(native.severity(), Severity::Panic);
            assert_eq!(
                *native.take_payload().unwrap().downcast::<u32>().unwrap(),
                73
            );
        },
    );
}
