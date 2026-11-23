const HANDLE_HIT_MARGIN: f32 = 2.0;
const MIN_POSITIVE_SUBNORMAL: f32 = f32::from_bits(1);

use crate::error::CanvasResult as Result;
use bevy::{
    math::{Affine3A, DVec2},
    prelude::*,
};

pub(crate) const HANDLE_SIZE: f32 = 8.0;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ScaleHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl ScaleHandle {
    pub(crate) fn axes(self) -> DVec2 {
        match self {
            Self::TopLeft => DVec2::NEG_ONE,
            Self::Top => DVec2::NEG_Y,
            Self::TopRight => DVec2::new(1.0, -1.0),
            Self::Right => DVec2::X,
            Self::BottomRight => DVec2::ONE,
            Self::Bottom => DVec2::Y,
            Self::BottomLeft => DVec2::new(-1.0, 1.0),
            Self::Left => DVec2::NEG_X,
        }
    }
}

pub(crate) fn controls(points: [Vec2; 4]) -> [(ScaleHandle, Vec2); 8] {
    let mid = |a: Vec2, b: Vec2| ((a.as_dvec2() + b.as_dvec2()) * 0.5).as_vec2();
    [
        (ScaleHandle::TopLeft, points[0]),
        (ScaleHandle::Top, mid(points[0], points[1])),
        (ScaleHandle::TopRight, points[1]),
        (ScaleHandle::Right, mid(points[1], points[2])),
        (ScaleHandle::BottomRight, points[2]),
        (ScaleHandle::Bottom, mid(points[2], points[3])),
        (ScaleHandle::BottomLeft, points[3]),
        (ScaleHandle::Left, mid(points[3], points[0])),
    ]
}

pub(crate) fn hit(
    world: &mut World,
    shape: Entity,
    point: Vec2,
    zoom: f32,
) -> Result<Option<(ScaleHandle, f32)>> {
    (|| -> Result<Option<(ScaleHandle, f32)>> {
        Ok(controls(crate::selection::frame(world, shape)?)
            .into_iter()
            .filter_map(|(handle, center)| {
                let distance = (center - point).length_squared() * zoom * zoom;
                (distance <= (HANDLE_SIZE * 0.5 + HANDLE_HIT_MARGIN).powi(2))
                    .then_some((handle, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b)))
    })()
    .map_err(|error| error.at("shape", shape))
}

#[derive(Clone, Copy)]
pub(crate) struct Scale {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    pub(crate) parent: Affine3A,
    transform: Transform,
    projection: Affine3A,
    uniform: bool,
    start: DVec2,
    bounds: Rect,
    handle: ScaleHandle,
}

impl Scale {
    pub(crate) fn begin(
        world: &mut World,
        document: Entity,
        shape: Entity,
        start: Vec2,
        handle: ScaleHandle,
    ) -> Result<Self> {
        (|| -> Result<Self> {
            let bounds = crate::selection::local_bounds(world, shape)?;
            let transform = *world
                .get::<Transform>(shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::ScalingTransform))?;
            let parent = crate::groups::parent_transform(world, shape)?;
            let projection = parent * transform.compute_affine();
            Ok(Self {
                document,
                shape,
                parent,
                transform,
                projection,
                uniform: world.get::<crate::CanvasGroup>(shape).is_some(),
                start: local_point(projection, start),
                bounds,
                handle,
            })
        })()
        .map_err(|error| error.at("document", document).at("shape", shape))
    }

    pub(crate) fn apply(self, world: &mut World, point: Vec2) -> Result {
        let delta = local_point(self.projection, point) - self.start;
        let axes = self.handle.axes();
        let size = self.bounds.max.as_dvec2() - self.bounds.min.as_dvec2();
        let mut factors = DVec2::ONE;
        let mut anchor = (self.bounds.min.as_dvec2() + self.bounds.max.as_dvec2()) * 0.5;
        for i in 0..2 {
            if axes[i] != 0.0 && size[i] > 0.0 {
                factors[i] += axes[i] * delta[i] / size[i];
                anchor[i] = if axes[i] > 0.0 {
                    f64::from(self.bounds.min[i])
                } else {
                    f64::from(self.bounds.max[i])
                };
            }
        }
        if self.uniform {
            let lever = size * axes;
            let factor = if lever.length_squared() > 0.0 {
                1.0 + delta.dot(lever) / lever.length_squared()
            } else {
                1.0
            };
            if !factor.is_finite() || factor <= 0.0 {
                return Ok(());
            }
            factors = DVec2::splat(factor);
        }
        let old = self.transform.scale.truncate().as_dvec2();
        let mut scaled = old * factors;
        for i in 0..2 {
            scaled[i] = scaled[i]
                .abs()
                .max(f64::from(MIN_POSITIVE_SUBNORMAL))
                .copysign(scaled[i]);
        }
        let mut transform = self.transform;
        transform.scale.x = scaled.x as f32;
        transform.scale.y = scaled.y as f32;
        let project = |t: Transform| {
            let [x, y] = crate::groups::planar_axes(&t.compute_affine());
            x * anchor.x + y * anchor.y
        };
        let translation = self.transform.translation.truncate().as_dvec2()
            + project(self.transform)
            - project(transform);
        transform.translation.x = translation.x as f32;
        transform.translation.y = translation.y as f32;
        crate::groups::update_item_transform(world, self.document, self.shape, transform)
    }
}

pub(crate) fn local_point(transform: Affine3A, point: Vec2) -> DVec2 {
    let relative = point.as_dvec2() - transform.translation.truncate().as_dvec2();
    local_delta(transform, relative)
}

pub(crate) fn local_delta(transform: Affine3A, relative: DVec2) -> DVec2 {
    let [x, y] = crate::groups::planar_axes(&transform);
    let determinant = x.perp_dot(y);
    DVec2::new(
        relative.perp_dot(y) / determinant,
        x.perp_dot(relative) / determinant,
    )
}
