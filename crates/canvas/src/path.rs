use bevy::math::DVec2;
use bevy::prelude::*;
mod append;
mod edit;
pub use edit::CanvasPathSegment;
mod geometry;
pub(crate) use geometry::meshes;

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CanvasPath {
    pub commands: Vec<CanvasPathCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum CanvasPathCommand {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo {
        control: Vec2,
        to: Vec2,
    },
    CubicTo {
        control1: Vec2,
        control2: Vec2,
        to: Vec2,
    },
    /// SVG endpoint arc; rotation is in radians. Zero radii produce a line.
    ArcTo {
        radii: Vec2,
        rotation: f32,
        large_arc: bool,
        sweep: bool,
        to: Vec2,
    },
    Close,
}

impl CanvasPath {
    pub fn validate(&self, transform: &Transform, paint: &crate::CanvasPaint) -> Result {
        self.bounds(transform, paint).map(|_| ())
    }

    pub fn bounds(&self, transform: &Transform, paint: &crate::CanvasPaint) -> Result<Rect> {
        crate::model::validate_transform_and_paint(transform, paint)?;
        let (min, max) = self.local_bounds()?;
        let margin = paint.stroke.map_or(0.0, |stroke| {
            // Miter limit four; a rotated square cap can extend sqrt(2) radii along either axis.
            let factor = if stroke.join == crate::CanvasLineJoin::Miter {
                4.0
            } else if stroke.cap == crate::CanvasLineCap::Square {
                std::f64::consts::SQRT_2
            } else {
                1.0
            };
            f64::from(stroke.width) * 0.5 * factor
        });
        let min = min - DVec2::splat(margin);
        let max = max + DVec2::splat(margin);
        let local = [min, DVec2::new(max.x, min.y), max, DVec2::new(min.x, max.y)];
        let x = (transform.rotation * Vec3::X * transform.scale.x)
            .truncate()
            .as_dvec2();
        let y = (transform.rotation * Vec3::Y * transform.scale.y)
            .truncate()
            .as_dvec2();
        let center = transform.translation.truncate().as_dvec2();
        let points = local.map(|p| (center + x * p.x + y * p.y).as_vec2());
        let bounds = Rect {
            min: points
                .into_iter()
                .fold(Vec2::splat(f32::INFINITY), Vec2::min),
            max: points
                .into_iter()
                .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max),
        };
        if !min.as_vec2().is_finite()
            || !max.as_vec2().is_finite()
            || !(max - min).as_vec2().is_finite()
            || points.iter().any(|p| !p.is_finite())
            || !bounds.size().is_finite()
        {
            return Err(BevyError::error("Canvas Path bounds overflow"));
        }
        Ok(bounds)
    }

    pub(crate) fn local_bounds(&self) -> Result<(DVec2, DVec2)> {
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        for segment in self.segments()? {
            let points: &[DVec2] = match &segment {
                Segment::Move(p) | Segment::Line(p) => std::slice::from_ref(p),
                Segment::Quad(points) => points,
                Segment::Cubic(points) => points,
                Segment::Close => &[],
            };
            for point in points {
                if !point.is_finite() {
                    return Err(BevyError::error("Canvas Path derived geometry overflow"));
                }
                min = min.min(*point);
                max = max.max(*point);
            }
        }
        Ok((min, max))
    }

    pub(crate) fn segments(&self) -> Result<Vec<Segment>> {
        use CanvasPathCommand::*;
        if self.commands.is_empty() {
            return Err(BevyError::error("Canvas Path requires MoveTo"));
        }
        let mut segments = Vec::new();
        let mut current = None;
        let mut start = DVec2::ZERO;
        let mut open = false;
        let point = |p: Vec2| -> Result<DVec2> {
            if p.is_finite() {
                Ok(p.as_dvec2())
            } else {
                Err(BevyError::error("Canvas Path points must be finite"))
            }
        };
        for command in &self.commands {
            if let MoveTo(p) = *command {
                let p = point(p)?;
                segments.push(Segment::Move(p));
                current = Some(p);
                start = p;
                open = true;
                continue;
            }
            let from =
                current.ok_or_else(|| BevyError::error("Canvas Path must begin with MoveTo"))?;
            if !open && !matches!(command, Close) {
                // SVG drawing after Close starts a fresh subpath at the closed subpath's start.
                segments.push(Segment::Move(from));
                start = from;
                open = true;
            }
            current = Some(match *command {
                MoveTo(_) => from,
                LineTo(to) => {
                    let to = point(to)?;
                    segments.push(Segment::Line(to));
                    to
                }
                QuadTo { control, to } => {
                    let to = point(to)?;
                    segments.push(Segment::Quad([point(control)?, to]));
                    to
                }
                CubicTo {
                    control1,
                    control2,
                    to,
                } => {
                    let to = point(to)?;
                    segments.push(Segment::Cubic([point(control1)?, point(control2)?, to]));
                    to
                }
                ArcTo {
                    radii,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                } => {
                    if !radii.is_finite() || radii.min_element() < 0.0 || !rotation.is_finite() {
                        return Err(BevyError::error(
                            "Canvas SVG arc requires finite nonnegative radii and finite rotation",
                        ));
                    }
                    let to = point(to)?;
                    arc(
                        &mut segments,
                        from,
                        to,
                        radii.as_dvec2(),
                        f64::from(rotation),
                        large_arc,
                        sweep,
                    )?;
                    to
                }
                Close => {
                    if !open {
                        return Err(BevyError::error("Canvas Path has no open subpath to close"));
                    }
                    segments.push(Segment::Close);
                    open = false;
                    start
                }
            });
        }
        Ok(segments)
    }
}

pub(crate) enum Segment {
    Move(DVec2),
    Line(DVec2),
    Quad([DVec2; 2]),
    Cubic([DVec2; 3]),
    Close,
}

fn arc(
    output: &mut Vec<Segment>,
    from: DVec2,
    to: DVec2,
    mut radii: DVec2,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> Result {
    if from == to {
        return Ok(());
    }
    if radii.min_element() == 0.0 {
        output.push(Segment::Line(to));
        return Ok(());
    }
    let (sin, cos) = rotation.sin_cos();
    let rotate = |p: DVec2| DVec2::new(cos * p.x - sin * p.y, sin * p.x + cos * p.y);
    let half = (from - to) * 0.5;
    let p = DVec2::new(cos * half.x + sin * half.y, -sin * half.x + cos * half.y);
    let factor = (p / radii).length();
    if factor > 1.0 {
        radii *= factor;
    }
    let unit = p / radii;
    let length = unit.length();
    // Avoid the fourth powers in the usual SVG formula, which overflow for finite f32 inputs.
    let center_unit = DVec2::new(unit.y, -unit.x) / length
        * (1.0 - length * length).max(0.0).sqrt()
        * if large == sweep { -1.0 } else { 1.0 };
    let start = unit - center_unit;
    let short = 2.0 * length.min(1.0).asin();
    let angle = if large {
        std::f64::consts::TAU - short
    } else {
        short
    } * if sweep { 1.0 } else { -1.0 };
    let count = (angle.abs() / std::f64::consts::FRAC_PI_4).ceil().max(1.0) as usize;
    let step = angle / count as f64;
    let k = (step * 0.25).tan() * (4.0 / 3.0);
    for index in 0..count {
        // Anchor at the original endpoint: subtracting huge centers or nearly equal angles
        // loses short arcs when their radii are much larger than the endpoint separation.
        let sample = |t: f64| {
            from + rotate(
                (start * (-2.0 * (t * 0.5).sin().powi(2)) + start.perp() * t.sin()) * radii,
            )
        };
        let tangent = |t: f64| rotate((start * t.cos() + start.perp() * t.sin()).perp() * radii);
        let a = step * index as f64;
        let b = step * (index + 1) as f64;
        let points = [
            sample(a) + tangent(a) * k,
            sample(b) - tangent(b) * k,
            if index + 1 == count { to } else { sample(b) },
        ];
        if points.iter().any(|p| !p.is_finite()) {
            return Err(BevyError::error("Canvas SVG arc overflow"));
        }
        output.push(Segment::Cubic(points));
    }
    Ok(())
}
