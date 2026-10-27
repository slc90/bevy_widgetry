#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_canvas::*;

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
            Err(CanvasExportError::EmptyDocument)
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
        Err(CanvasExportError::InvalidScale)
    ));
    assert!(matches!(
        CanvasDocument::export_png(
            &mut world,
            document,
            file.path(),
            CanvasPngOptions::default()
        ),
        Err(CanvasExportError::RendererUnavailable)
    ));
    assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    assert_eq!(world.entities().len(), entities);
}
