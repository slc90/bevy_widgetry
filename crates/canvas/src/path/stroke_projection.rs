use super::{MITER_LIMIT, Segment};
use crate::{CanvasLineCap, CanvasLineJoin, CanvasStroke};
use bevy::math::{DMat2, DVec2};

pub(super) fn linear_bounds(
    segments: &[Segment],
    matrix: DMat2,
    offset: DVec2,
    stroke: CanvasStroke,
) -> Option<(DVec2, DVec2)> {
    let mut projection = StrokeProjection {
        matrix,
        offset,
        minimum: DVec2::splat(f64::INFINITY),
        maximum: DVec2::splat(f64::NEG_INFINITY),
        radius: f64::from(stroke.width) * 0.5,
    };
    let mut points = Vec::new();
    for segment in segments {
        match *segment {
            Segment::Move(point) => {
                projection.contour(&mut points, false, stroke);
                points.clear();
                points.push(point);
            }
            Segment::Line(point) => points.push(point),
            Segment::Close => {
                let start = *points.first()?;
                projection.contour(&mut points, true, stroke);
                points.clear();
                points.push(start);
            }
            _ => return None,
        }
    }
    projection.contour(&mut points, false, stroke);
    projection
        .minimum
        .is_finite()
        .then_some((projection.minimum, projection.maximum))
}

struct StrokeProjection {
    matrix: DMat2,
    offset: DVec2,
    minimum: DVec2,
    maximum: DVec2,
    radius: f64,
}

impl StrokeProjection {
    fn include(&mut self, point: DVec2) {
        let value = self.offset + self.matrix * point;
        self.minimum = self.minimum.min(value);
        self.maximum = self.maximum.max(value);
    }

    fn contour(&mut self, points: &mut Vec<DVec2>, closed: bool, stroke: CanvasStroke) {
        if points.is_empty() || (points.len() == 1 && !closed) {
            return;
        }
        points.dedup();
        if closed && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        let Some(&start) = points.first() else {
            return;
        };
        if points.len() == 1 {
            match stroke.cap {
                CanvasLineCap::Butt => {}
                CanvasLineCap::Round => self.include_round(start, |_| true),
                CanvasLineCap::Square => {
                    for corner in [
                        DVec2::ONE,
                        DVec2::NEG_ONE,
                        DVec2::new(1.0, -1.0),
                        DVec2::new(-1.0, 1.0),
                    ] {
                        self.include(start + corner * self.radius);
                    }
                }
            }
            return;
        }
        let segment_count = points.len() - usize::from(!closed);
        let mut preceding = None;
        let mut first_direction = DVec2::ZERO;
        for index in 0..segment_count {
            let from = points[index];
            let to = points[(index + 1) % points.len()];
            let direction = (to - from).normalize();
            let normal = direction.perp() * self.radius;
            for corner in [from - normal, from + normal, to - normal, to + normal] {
                self.include(corner);
            }
            if let Some(preceding) = preceding {
                self.join(from, preceding, direction, stroke.join);
            } else {
                first_direction = direction;
            }
            preceding = Some(direction);
        }
        if let Some(last_direction) = preceding {
            if closed {
                self.join(start, last_direction, first_direction, stroke.join);
            } else {
                self.cap(start, -first_direction, stroke.cap);
                self.cap(points[points.len() - 1], last_direction, stroke.cap);
            }
        }
    }

    fn cap(&mut self, point: DVec2, outward: DVec2, cap: CanvasLineCap) {
        match cap {
            CanvasLineCap::Butt => {}
            CanvasLineCap::Round => self.include_round(point, |axis| outward.dot(axis) >= 0.0),
            CanvasLineCap::Square => {
                let tip = point + outward * self.radius;
                let normal = outward.perp() * self.radius;
                self.include(tip + normal);
                self.include(tip - normal);
            }
        }
    }

    fn join(&mut self, point: DVec2, before: DVec2, after: DVec2, join: CanvasLineJoin) {
        let turn = before.perp_dot(after);
        let angle_cosine = before.dot(after);
        if turn == 0.0 && angle_cosine >= 0.0 {
            return;
        }
        let sweep = if turn > 0.0 { 1.0 } else { -1.0 };
        let start = before.perp() * -sweep;
        let end = after.perp() * -sweep;
        match join {
            CanvasLineJoin::Bevel => {}
            CanvasLineJoin::Round => self.include_round(point, |axis| {
                sweep * start.perp_dot(axis) >= 0.0 && sweep * axis.perp_dot(end) >= 0.0
            }),
            CanvasLineJoin::Miter => {
                let denominator = 1.0 + angle_cosine;
                if denominator >= 2.0 / MITER_LIMIT.powi(2) {
                    self.include(point + (start + end) * (self.radius / denominator));
                }
            }
        }
    }

    fn include_round(&mut self, point: DVec2, contains: impl Fn(DVec2) -> bool) {
        for axis in [self.matrix.row(0), self.matrix.row(1)] {
            let axis = axis.normalize_or_zero();
            for candidate in [axis, -axis] {
                if contains(candidate) {
                    self.include(point + candidate * self.radius);
                }
            }
        }
    }
}
