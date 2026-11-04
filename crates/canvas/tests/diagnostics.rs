#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
use std::error::Error;

fn cause<'a, T: Error + 'static>(mut error: &'a (dyn Error + 'static)) -> Option<&'a T> {
    loop {
        if let Some(value) = error.downcast_ref() {
            return Some(value);
        }
        error = error.source()?;
    }
}

#[test]
fn invalid_geometry_keeps_typed_values_and_independent_request_origins() {
    let logs = LogCapture::default();
    let rectangle = CanvasRectangle {
        size: Vec2::new(f32::NAN, 2.0),
        corner_radius: 0.0,
    };
    let first = logs.run(|| {
        rectangle
            .validate(&Transform::default(), &CanvasPaint::default())
            .unwrap_err()
    });
    let second = logs.run(|| {
        rectangle
            .validate(&Transform::default(), &CanvasPaint::default())
            .unwrap_err()
    });
    let error = cause::<CanvasError>(&first).unwrap();
    assert_eq!(
        error.kind,
        CanvasErrorKind::Invalid(CanvasIssue::RectangleSize)
    );
    assert!(error.inputs.iter().any(
        |input| matches!(input, CanvasInput::Rectangle(r) if r.size.x.is_nan() && r.size.y == 2.0)
    ));
    let origin = first.diagnostics().next().unwrap().origin();
    assert_ne!(
        origin.id(),
        second.diagnostics().next().unwrap().origin().id()
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
fn invalid_path_keeps_the_actual_command_index_and_value() {
    let path = CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(Vec2::ZERO),
            CanvasPathCommand::LineTo(Vec2::new(f32::INFINITY, 3.0)),
        ],
    };
    let error = path
        .validate(&Transform::default(), &CanvasPaint::default())
        .unwrap_err();
    let error = cause::<CanvasError>(&error).unwrap();
    assert_eq!(error.kind, CanvasErrorKind::Invalid(CanvasIssue::PathPoint));
    assert!(error.inputs.iter().any(|input| matches!(input, CanvasInput::Command { index: 1, value: CanvasPathCommand::LineTo(point) } if point.x.is_infinite() && point.y == 3.0)));
}

#[test]
fn rejected_ownership_preserves_both_documents_and_the_shape() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(5.0),
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let error = CanvasDocument::remove_shape(&mut world, other, shape).unwrap_err();
    let error = cause::<CanvasError>(&error).unwrap();
    assert_eq!(error.kind, CanvasErrorKind::Invalid(CanvasIssue::Ownership));
    for entity in [document, other, shape] {
        assert!(
            error.inputs.iter().any(
                |input| matches!(input, CanvasInput::Entity { value, .. } if *value == entity)
            )
        );
    }
    assert_eq!(world.get::<ChildOf>(shape).unwrap().parent(), document);
}

#[test]
fn rejected_selection_and_append_keep_the_requested_values() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let world = app.world_mut();
    let document = world.spawn(CanvasDocument).id();
    let view = world
        .spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } })
        .unwrap()
        .id();
    let error = CanvasView::set_selection_in_world(world, view, [Entity::PLACEHOLDER]).unwrap_err();
    let error = cause::<CanvasError>(&error).unwrap();
    assert!(error.inputs.iter().any(|input| matches!(input, CanvasInput::Entities { values, .. } if values == &[Entity::PLACEHOLDER])));
    assert!(error.inputs.iter().any(|input| matches!(input, CanvasInput::Entity { field: "document", value } if *value == document)));
    let shape = CanvasDocument::create_path(
        world,
        document,
        CanvasPath {
            commands: vec![CanvasPathCommand::MoveTo(Vec2::ZERO)],
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let rect = Rect {
        min: Vec2::splat(f32::NAN),
        max: Vec2::ONE,
    };
    let failure = CanvasDocument::append_rectangle(world, document, shape, rect).unwrap_err();
    let error = cause::<CanvasError>(&failure).unwrap();
    assert!(error.inputs.iter().any(
        |input| matches!(input, CanvasInput::Point { field: "rect_min", value } if value.x.is_nan())
    ));
    assert!(error.inputs.iter().any(|input| matches!(input, CanvasInput::Point { field: "rect_max", value } if *value == rect.max)));
}

#[test]
fn export_model_rejections_preserve_owner_and_emit_only_the_source_error() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(5.0),
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    world
        .entity_mut(shape)
        .insert(CanvasGeometry::Rectangle(CanvasRectangle {
            size: Vec2::splat(f32::NAN),
            corner_radius: 0.0,
        }));
    let file = tempfile::NamedTempFile::new().unwrap();
    for png in [false, true] {
        let logs = LogCapture::default();
        let error = logs
            .run(|| {
                if png {
                    CanvasDocument::export_png(
                        &mut world,
                        document,
                        file.path(),
                        CanvasPngOptions::default(),
                    )
                    .map(|_| ())
                } else {
                    CanvasDocument::export_svg(&mut world, document, file.path(), None)
                }
            })
            .unwrap_err();
        assert_eq!(logs.records().len(), 1);
        let domain = cause::<CanvasError>(&error).unwrap();
        for entity in [document, shape] {
            assert!(domain.inputs.iter().any(
                |input| matches!(input, CanvasInput::Entity { value, .. } if *value == entity)
            ));
        }
        assert_eq!(logs.records()[0].phase(), Some(DiagnosticPhase::Source));
    }
}

#[test]
fn document_rejection_keeps_the_requested_document_at_sync_and_command_boundaries() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @CanvasView })
        .unwrap()
        .id();
    let missing = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(missing);
    let failure =
        CanvasView::set_document_in_world(app.world_mut(), view, Some(missing)).unwrap_err();
    let domain = cause::<CanvasError>(&failure).unwrap();
    assert!(domain.inputs.iter().any(|input| matches!(input, CanvasInput::Entity { field: "document", value } if *value == missing)));
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    CanvasView::set_document(&mut app.world_mut().commands(), view, Some(missing));
    errors.run(|| app.world_mut().flush());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let domain = records[0]
        .managed()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<CanvasError>()
        .unwrap();
    for expected in [view, missing] {
        assert!(
            domain.inputs.iter().any(
                |input| matches!(input, CanvasInput::Entity { value, .. } if *value == expected)
            )
        );
    }
    assert_eq!(CanvasView::document(app.world(), view).unwrap(), None);
}
