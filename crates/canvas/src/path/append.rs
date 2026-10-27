use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasPathCommand};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

impl CanvasDocument {
    pub fn append_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rect: Rect,
    ) -> Result {
        append(world, document, shape, || {
            if !rect.min.is_finite() || !rect.max.is_finite() || rect.size().min_element() <= 0.0 {
                return Err(BevyError::error(
                    "Canvas path rectangle requires finite positive extents",
                ));
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
    }

    pub fn append_circle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radius: f32,
    ) -> Result {
        Self::append_ellipse(world, document, shape, center, Vec2::splat(radius))
    }

    pub fn append_ellipse(
        world: &mut World,
        document: Entity,
        shape: Entity,
        center: Vec2,
        radii: Vec2,
    ) -> Result {
        append(world, document, shape, || {
            if !center.is_finite() || !radii.is_finite() || radii.min_element() <= 0.0 {
                return Err(BevyError::error(
                    "Canvas path ellipse requires a finite center and positive finite radii",
                ));
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
                return Err(BevyError::error(
                    "Canvas path ellipse horizontal endpoints must exactly represent center plus or minus radius",
                ));
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
    }

    pub fn append_line(
        world: &mut World,
        document: Entity,
        shape: Entity,
        from: Vec2,
        to: Vec2,
    ) -> Result {
        Self::append_polyline(world, document, shape, &[from, to])
    }

    pub fn append_polyline(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> Result {
        append(world, document, shape, || polygon(points, false))
    }

    pub fn append_polygon(
        world: &mut World,
        document: Entity,
        shape: Entity,
        points: &[Vec2],
    ) -> Result {
        append(world, document, shape, || polygon(points, true))
    }
}

fn polygon(points: &[Vec2], closed: bool) -> Result<Vec<CanvasPathCommand>> {
    if points.len() < if closed { 3 } else { 2 } || points.iter().any(|point| !point.is_finite()) {
        return Err(BevyError::error(
            "Canvas path contour requires finite points and at least two open or three closed vertices",
        ));
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
    let result = (|| {
        crate::model::validate_shape(world, document, shape)?;
        let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) else {
            return Err(BevyError::error("Canvas append target must be a Path"));
        };
        let mut path = path.clone();
        path.commands.extend(generate()?);
        let transform = world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas shape transform missing"))?;
        let paint = world
            .get::<CanvasPaint>(shape)
            .ok_or_else(|| BevyError::error("Canvas shape paint missing"))?;
        path.validate(transform, paint)?;
        Ok(path)
    })();
    let path = result.inspect_err(
        |error| widgetry_error!(?document, ?shape, %error, "Canvas 路径轮廓追加被拒绝"),
    )?;
    world.entity_mut(shape).insert(CanvasGeometry::Path(path));
    Ok(())
}
