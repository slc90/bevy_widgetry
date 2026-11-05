#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(feature = "png_export")]
use bevy::prelude::*;
use bevy_widgetry_canvas::*;

#[test]
fn request_failure_is_managed_with_owner_path_and_scale() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(10.0),
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let failure = CanvasDocument::export_png(
        &mut world,
        document,
        "untouched.png",
        CanvasPngOptions {
            scale: f32::NAN,
            ..default()
        },
    )
    .unwrap_err();
    let diagnostic = failure.diagnostics().next().unwrap();
    assert!(diagnostic.contexts().iter().any(|frame| {
        frame.operation() == "canvas.png.request"
            && frame
                .fields()
                .iter()
                .any(|(key, value)| key == "path" && value == "untouched.png")
    }));
}

#[test]
fn empty_and_invalid_png_requests_never_touch_existing_files_or_allocate_render_resources() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"keep me").unwrap();
    for region in [
        None,
        Some(Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0))),
    ] {
        assert!(matches!(
            CanvasDocument::export_png(
                &mut world,
                document,
                file.path(),
                CanvasPngOptions {
                    region,
                    ..default()
                }
            ),
            Err(ref error) if matches!(cause::<CanvasExportError>(error), Some(CanvasExportError::EmptyDocument))
        ));
        assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    }
    CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::new(120.0, 80.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let entities = world.entities().len();
    assert!(matches!(
        CanvasDocument::export_png(
            &mut world,
            document,
            file.path(),
            CanvasPngOptions {
                scale: 0.0,
                ..default()
            }
        ),
        Err(ref error) if matches!(cause::<CanvasExportError>(error), Some(CanvasExportError::InvalidScale))
    ));
    assert!(matches!(
        CanvasDocument::export_png(
            &mut world,
            document,
            file.path(),
            CanvasPngOptions::default()
        ),
        Err(ref error) if matches!(cause::<CanvasExportError>(error), Some(CanvasExportError::RendererUnavailable))
    ));
    assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    assert_eq!(world.entities().len(), entities);
}

fn cause<'a, T: std::error::Error + 'static>(
    error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a T> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(found) = error.downcast_ref::<T>() {
            return Some(found);
        }
        current = error.source();
    }
    None
}
