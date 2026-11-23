pub(crate) const MITER_LIMIT: f64 = 4.0;
pub(crate) const RELATIVE_TESSELLATION_TOLERANCE: f64 = 1.0 / 4096.0;

use crate::error::CanvasResult as Result;
use bevy::math::{Affine3A, DMat2, DVec2};
use bevy::prelude::*;
mod append;
pub(crate) use append::contour;
mod edit;
pub use edit::CanvasPathSegment;
mod geometry;
mod stroke_projection;
pub(crate) use geometry::{connector_meshes, meshes};

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
    pub fn validate(
        &self,
        transform: &Transform,
        paint: &crate::CanvasPaint,
    ) -> crate::error::PublicResult {
        self._validate(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasPath.validate",
            ))
        })
    }

    pub(crate) fn _validate(&self, transform: &Transform, paint: &crate::CanvasPaint) -> Result {
        self._bounds(transform, paint).map(|_| ()).map_err(|error| {
            error
                .input(crate::CanvasInput::Geometry(crate::CanvasGeometry::Path(
                    self.clone(),
                )))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }

    pub fn bounds(
        &self,
        transform: &Transform,
        paint: &crate::CanvasPaint,
    ) -> crate::error::PublicResult<Rect> {
        self._bounds(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasPath.bounds",
            ))
        })
    }

    pub(crate) fn _bounds(
        &self,
        transform: &Transform,
        paint: &crate::CanvasPaint,
    ) -> Result<Rect> {
        crate::model::validate_transform_and_paint(transform, paint)
            .and_then(|()| self.projected_bounds(&transform.compute_affine(), paint))
            .map_err(|error| {
                error
                    .input(crate::CanvasInput::Geometry(crate::CanvasGeometry::Path(
                        self.clone(),
                    )))
                    .input(crate::CanvasInput::Transform(*transform))
                    .input(crate::CanvasInput::Paint(*paint))
            })
    }

    pub(crate) fn projected_bounds(
        &self,
        transform: &Affine3A,
        paint: &crate::CanvasPaint,
    ) -> Result<Rect> {
        let (min, max) = self.local_bounds()?;
        let margin = stroke_margin(paint);
        let min = min - DVec2::splat(margin);
        let max = max + DVec2::splat(margin);
        let local = [min, DVec2::new(max.x, min.y), max, DVec2::new(min.x, max.y)];
        let [x, y] = crate::groups::planar_axes(transform);
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
            return Err(crate::error::invalid(crate::CanvasIssue::PathBounds));
        }
        Ok(bounds)
    }

    pub(crate) fn projected_curve_bounds(
        &self,
        axes: [DVec2; 2],
        offset: DVec2,
        paint: &crate::CanvasPaint,
    ) -> Result<Option<(DVec2, DVec2)>> {
        use CanvasPathCommand::{Close, MoveTo};

        let mut bounds: Option<(DVec2, DVec2)> = None;
        let mut start = Vec2::ZERO;
        for commands in self
            .commands
            .chunk_by(|previous, next| !matches!(previous, Close) && !matches!(next, MoveTo(_)))
        {
            let continuation;
            let commands = if let Some(MoveTo(point)) = commands.first() {
                start = *point;
                commands
            } else {
                // Drawing after Close resumes at the closed contour's first point.
                continuation = [MoveTo(start)]
                    .into_iter()
                    .chain(commands.iter().copied())
                    .collect::<Vec<_>>();
                &continuation
            };
            if let Some((min, max)) = Self::projected_contour_bounds(commands, axes, offset, paint)?
            {
                bounds =
                    Some(bounds.map_or((min, max), |(low, high)| (low.min(min), high.max(max))));
            }
        }
        Ok(bounds)
    }

    fn projected_contour_bounds(
        commands: &[CanvasPathCommand],
        [x, y]: [DVec2; 2],
        offset: DVec2,
        paint: &crate::CanvasPaint,
    ) -> Result<Option<(DVec2, DVec2)>> {
        use CanvasPathCommand::*;
        use lyon::geom::{CubicBezierSegment, QuadraticBezierSegment, point};

        let matrix = DMat2::from_cols(x, y);
        if let Some(stroke) = paint.stroke {
            let segments = Self::segments_from_commands(commands)?;
            if segments.iter().all(|segment| {
                matches!(
                    segment,
                    Segment::Move(_) | Segment::Line(_) | Segment::Close
                )
            }) {
                return Ok(stroke_projection::linear_bounds(
                    &segments, matrix, offset, stroke,
                ));
            }
        }
        let project = |local: DVec2| {
            let point_on_curve = offset + matrix * local;
            point(point_on_curve.x, point_on_curve.y)
        };
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        let mut include = |p: lyon::geom::Point<f64>| {
            let point = DVec2::new(p.x, p.y);
            min = min.min(point);
            max = max.max(point);
        };
        let mut start = DVec2::ZERO;
        let mut current = DVec2::ZERO;
        for &command in commands {
            current = match command {
                MoveTo(to) => {
                    start = to.as_dvec2();
                    current = start;
                    continue;
                }
                LineTo(to) => {
                    include(project(current));
                    to.as_dvec2()
                }
                QuadTo { control, to } => {
                    let to = to.as_dvec2();
                    let bounds = QuadraticBezierSegment {
                        from: project(current),
                        ctrl: project(control.as_dvec2()),
                        to: project(to),
                    }
                    .bounding_box();
                    include(bounds.min);
                    include(bounds.max);
                    to
                }
                CubicTo {
                    control1,
                    control2,
                    to,
                } => {
                    let to = to.as_dvec2();
                    let bounds = CubicBezierSegment {
                        from: project(current),
                        ctrl1: project(control1.as_dvec2()),
                        ctrl2: project(control2.as_dvec2()),
                        to: project(to),
                    }
                    .bounding_box();
                    include(bounds.min);
                    include(bounds.max);
                    to
                }
                ArcTo {
                    radii,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                } => {
                    let to = to.as_dvec2();
                    if current == to {
                        continue;
                    }
                    include(project(current));
                    if let Some(arc) = ArcGeometry::from_endpoints(
                        current,
                        to,
                        radii.as_dvec2(),
                        f64::from(rotation),
                        large_arc,
                        sweep,
                    ) {
                        arc.projected_extrema(matrix, |local| include(project(local)));
                    }
                    to
                }
                Close => {
                    current = start;
                    continue;
                }
            };
            include(project(current));
        }
        if min == DVec2::splat(f64::INFINITY) {
            return Ok(None);
        }
        let padding = DVec2::new(x.x.hypot(y.x), x.y.hypot(y.y)) * stroke_margin(paint);
        let (min, max) = (min - padding, max + padding);
        if !min.is_finite() || !max.is_finite() {
            return Err(crate::error::invalid(crate::CanvasIssue::PathBounds));
        }
        Ok(Some((min, max)))
    }

    pub(crate) fn local_bounds(&self) -> Result<(DVec2, DVec2)> {
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        for (segment_index, segment) in self.segments()?.into_iter().enumerate() {
            let points: &[DVec2] = match &segment {
                Segment::Move(p) | Segment::Line(p) => std::slice::from_ref(p),
                Segment::Quad(points) => points,
                Segment::Cubic(points) => points,
                Segment::Close => &[],
            };
            for point in points {
                if !point.is_finite() {
                    return Err(
                        crate::error::invalid(crate::CanvasIssue::PathDerivedGeometry).input(
                            crate::CanvasInput::Index {
                                field: "derived_segment",
                                value: segment_index,
                            },
                        ),
                    );
                }
                min = min.min(*point);
                max = max.max(*point);
            }
        }
        Ok((min, max))
    }

    pub(crate) fn segments(&self) -> Result<Vec<Segment>> {
        Self::segments_from_commands(&self.commands)
    }

    fn segments_from_commands(commands: &[CanvasPathCommand]) -> Result<Vec<Segment>> {
        use CanvasPathCommand::*;
        if commands.is_empty() {
            return Err(crate::error::invalid(crate::CanvasIssue::EmptyPath));
        }
        let mut segments = Vec::new();
        let mut current = None;
        let mut start = DVec2::ZERO;
        let mut open = false;
        for (index, command) in commands.iter().enumerate() {
            let failure = |issue| {
                crate::error::invalid(issue).input(crate::CanvasInput::Command {
                    index,
                    value: *command,
                })
            };
            let point = |p: Vec2| -> Result<DVec2> {
                if p.is_finite() {
                    Ok(p.as_dvec2())
                } else {
                    Err(failure(crate::CanvasIssue::PathPoint))
                }
            };
            if let MoveTo(p) = *command {
                let p = point(p)?;
                segments.push(Segment::Move(p));
                current = Some(p);
                start = p;
                open = true;
                continue;
            }
            let from = current.ok_or_else(|| failure(crate::CanvasIssue::PathStart))?;
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
                        return Err(failure(crate::CanvasIssue::ArcParameters));
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
                    )
                    .map_err(|error| {
                        error.input(crate::CanvasInput::Command {
                            index,
                            value: *command,
                        })
                    })?;
                    to
                }
                Close => {
                    if !open {
                        return Err(failure(crate::CanvasIssue::PathClose));
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

fn stroke_margin(paint: &crate::CanvasPaint) -> f64 {
    paint.stroke.map_or(0.0, |stroke| {
        // Miter limit four; a square cap corner lies sqrt(2) radii from its endpoint.
        let factor = if stroke.join == crate::CanvasLineJoin::Miter {
            MITER_LIMIT
        } else if stroke.cap == crate::CanvasLineCap::Square {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        f64::from(stroke.width) * 0.5 * factor
    })
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
    radii: DVec2,
    rotation: f64,
    large: bool,
    sweep: bool,
) -> Result {
    let Some(arc) = ArcGeometry::from_endpoints(from, to, radii, rotation, large, sweep) else {
        if from != to {
            output.push(Segment::Line(to));
        }
        return Ok(());
    };
    let count = (arc.angle.abs() / std::f64::consts::FRAC_PI_4)
        .ceil()
        .max(1.0) as usize;
    let step = arc.angle / count as f64;
    let k = (step * 0.25).tan() * (4.0 / 3.0);
    for index in 0..count {
        let a = step * index as f64;
        let b = step * (index + 1) as f64;
        let points = [
            arc.sample(a) + arc.tangent(a) * k,
            arc.sample(b) - arc.tangent(b) * k,
            if index + 1 == count {
                to
            } else {
                arc.sample(b)
            },
        ];
        if points.iter().any(|p| !p.is_finite()) {
            return Err(crate::error::invalid(crate::CanvasIssue::ArcOverflow));
        }
        output.push(Segment::Cubic(points));
    }
    Ok(())
}

struct ArcGeometry {
    from: DVec2,
    radii: DVec2,
    start: DVec2,
    sin: f64,
    cos: f64,
    angle: f64,
}

impl ArcGeometry {
    fn from_endpoints(
        from: DVec2,
        to: DVec2,
        mut radii: DVec2,
        rotation: f64,
        large: bool,
        sweep: bool,
    ) -> Option<Self> {
        if from == to || radii.min_element() == 0.0 {
            return None;
        }
        let (sin, cos) = rotation.sin_cos();
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
        Some(Self {
            from,
            radii,
            start,
            sin,
            cos,
            angle,
        })
    }

    fn rotate(&self, point: DVec2) -> DVec2 {
        DVec2::new(
            self.cos * point.x - self.sin * point.y,
            self.sin * point.x + self.cos * point.y,
        )
    }

    fn sample(&self, angle: f64) -> DVec2 {
        // Anchor at the source endpoint to retain short arcs even when their radii are huge.
        self.from
            + self.rotate(
                (self.start * (-2.0 * (angle * 0.5).sin().powi(2))
                    + self.start.perp() * angle.sin())
                    * self.radii,
            )
    }

    fn tangent(&self, angle: f64) -> DVec2 {
        self.rotate(
            (self.start * angle.cos() + self.start.perp() * angle.sin()).perp() * self.radii,
        )
    }

    fn projected_extrema(&self, matrix: DMat2, mut include: impl FnMut(DVec2)) {
        let radial = matrix * self.rotate(self.start * self.radii);
        let tangent = matrix * self.tangent(0.0);
        let sweep = self.angle.min(0.0)..=self.angle.max(0.0);
        for axis in 0..2 {
            // The projected derivative vanishes at these half-turn intervals.
            let extremum = tangent[axis].atan2(radial[axis]);
            for half_turn in -2..=2 {
                let offset = extremum + f64::from(half_turn) * std::f64::consts::PI;
                if sweep.contains(&offset) {
                    include(self.sample(offset));
                }
            }
        }
    }
}
