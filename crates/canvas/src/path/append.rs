use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasPathCommand};
use bevy::prelude::*;

impl CanvasDocument {
    pub fn append_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rect: Rect,
    ) -> crate::error::PublicResult {
        Self::_append_rectangle(world, document, shape, rect).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_rectangle",
            ))
        })
    }

    pub(crate) fn _append_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rect: Rect,
    ) -> Result {
        append(world, document, shape, || {
            if !rect.min.is_finite() || !rect.max.is_finite() || rect.size().min_element() <= 0.0 {
                return Err(crate::error::invalid(crate::CanvasIssue::AppendRectangle));
            }
            use CanvasPathCommand::*;
            Ok(vec![
                MoveTo(rect.min),
                LineTo(Vec2::new(rect.max.x, rect.min.y)),
                LineTo(rect.max),
                LineTo(Vec2::new(rect.min.x, rect.max.y)),
                Close,
            ])
        })
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Point {
                    field: "rect_min",
                    value: rect.min,
                })
                .input(crate::CanvasInput::Point {
                    field: "rect_max",
                    value: rect.max,
                })
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
        })
    }

    pub fn append_circle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radius: f32,
    ) -> crate::error::PublicResult {
        Self::_append_circle(world, document, shape, center, radius).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_circle",
            ))
        })
    }

    pub(crate) fn _append_circle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radius: f32,
    ) -> Result {
        Self::_append_ellipse(world, document, shape, center, Vec2::splat(radius)).map_err(
            |error| {
                error
                    .input(crate::CanvasInput::Entity {
                        field: "document",
                        value: document,
                    })
                    .input(crate::CanvasInput::Entity {
                        field: "shape",
                        value: shape,
                    })
                    .input(crate::CanvasInput::Point {
                        field: "center",
                        value: center,
                    })
                    .input(crate::CanvasInput::Float {
                        field: "radius",
                        value: radius,
                    })
            },
        )
    }

    pub fn append_ellipse(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radii: Vec2,
    ) -> crate::error::PublicResult {
        Self::_append_ellipse(world, document, shape, center, radii).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_ellipse",
            ))
        })
    }

    pub(crate) fn _append_ellipse(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radii: Vec2,
    ) -> Result {
        append(world, document, shape, || {
            if !center.is_finite() || !radii.is_finite() || radii.min_element() <= 0.0 {
                return Err(crate::error::invalid(crate::CanvasIssue::AppendEllipse));
            }
            let start = center + Vec2::X * radii.x;
            let opposite = center - Vec2::X * radii.x;
            // SVG corrects undersized arc radii and bends shorter chords away from the intended center.
            // Require exact horizontal endpoints so either rounding direction cannot change the ellipse.
            if !start.is_finite()
                || !opposite.is_finite()
                || f64::from(start.x) - f64::from(center.x) != f64::from(radii.x)
                || f64::from(center.x) - f64::from(opposite.x) != f64::from(radii.x)
            {
                return Err(crate::error::invalid(crate::CanvasIssue::EllipseEndpoints));
            }
            use CanvasPathCommand::*;
            Ok(vec![
                MoveTo(start),
                ArcTo {
                    radii,
                    rotation: 0.0,
                    large_arc: false,
                    sweep: true,
                    to: opposite,
                },
                ArcTo {
                    radii,
                    rotation: 0.0,
                    large_arc: false,
                    sweep: true,
                    to: start,
                },
                Close,
            ])
        })
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Point {
                    field: "center",
                    value: center,
                })
                .input(crate::CanvasInput::Point {
                    field: "radii",
                    value: radii,
                })
        })
    }

    pub fn append_line(
        world: &mut World,
        document: Entity,
        shape: Entity,
        from: Vec2,
        to: Vec2,
    ) -> crate::error::PublicResult {
        Self::_append_line(world, document, shape, from, to).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_line",
            ))
        })
    }

    pub(crate) fn _append_line(
        world: &mut World,
        document: Entity,
        shape: Entity,
        from: Vec2,
        to: Vec2,
    ) -> Result {
        Self::_append_polyline(world, document, shape, &[from, to]).map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Point {
                    field: "from",
                    value: from,
                })
                .input(crate::CanvasInput::Point {
                    field: "to",
                    value: to,
                })
        })
    }

    pub fn append_polyline(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> crate::error::PublicResult {
        Self::_append_polyline(world, document, shape, points).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_polyline",
            ))
        })
    }

    pub(crate) fn _append_polyline(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> Result {
        append(world, document, shape, || polygon(points, false)).map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Points {
                    field: "points",
                    values: points.to_vec(),
                })
        })
    }

    pub fn append_polygon(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> crate::error::PublicResult {
        Self::_append_polygon(world, document, shape, points).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.append_polygon",
            ))
        })
    }

    pub(crate) fn _append_polygon(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> Result {
        append(world, document, shape, || polygon(points, true)).map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Points {
                    field: "points",
                    values: points.to_vec(),
                })
        })
    }
}

fn polygon(points: &[Vec2], closed: bool) -> Result<Vec<CanvasPathCommand>> {
    let min_polygon_points = 3;
    let min_polyline_points = 2;
    if points.len()
        < if closed {
            min_polygon_points
        } else {
            min_polyline_points
        }
        || points.iter().any(|point| !point.is_finite())
    {
        return Err(crate::error::invalid(crate::CanvasIssue::ContourPoints));
    }
    let mut commands = Vec::with_capacity(points.len() + usize::from(closed));
    commands.push(CanvasPathCommand::MoveTo(points[0]));
    commands.extend(points[1..].iter().copied().map(CanvasPathCommand::LineTo));
    if closed {
        commands.push(CanvasPathCommand::Close);
    }
    Ok(commands)
}

fn append(
    world: &mut World,
    document: Entity,
    shape: Entity,
    generate: impl FnOnce() -> Result<Vec<CanvasPathCommand>>,
) -> Result {
    (|| -> Result {
        let result = (|| {
            crate::model::validate_shape(world, document, shape)?;
            let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) else {
                return Err(crate::error::invalid(crate::CanvasIssue::AppendKind));
            };
            let mut path = path.clone();
            path.commands.extend(generate()?);
            let transform = world
                .get::<Transform>(shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::ShapeTransform))?;
            let paint = world
                .get::<CanvasPaint>(shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::ShapePaint))?;
            path._validate(transform, paint)?;
            Ok(path)
        })();
        let path = result?;
        world.entity_mut(shape).insert(CanvasGeometry::Path(path));
        Ok(())
    })()
    .map_err(|error| error.at("document", document).at("shape", shape))
}
