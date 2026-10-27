use crate::CanvasDocument;
use bevy::{math::DVec2, prelude::*};

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
    Ok(controls(crate::selection::frame(world, shape)?)
        .into_iter()
        .filter_map(|(handle, center)| {
            let distance = (center - point).length_squared() * zoom * zoom;
            (distance <= (HANDLE_SIZE * 0.5 + 2.0).powi(2)).then_some((handle, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b)))
}

#[derive(Clone, Copy)]
pub(crate) struct Scale {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    transform: Transform,
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
        let bounds = crate::selection::local_bounds(world, shape)?;
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas scaling transform missing"))?;
        Ok(Self {
            document,
            shape,
            transform,
            start: local_point(transform, start),
            bounds,
            handle,
        })
    }

    pub(crate) fn apply(self, world: &mut World, point: Vec2) -> Result {
        let delta = local_point(self.transform, point) - self.start;
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
        let old = self.transform.scale.truncate().as_dvec2();
        let mut scaled = old * factors;
        for i in 0..2 {
            scaled[i] = scaled[i]
                .abs()
                .max(f64::from(f32::from_bits(1)))
                .copysign(scaled[i]);
        }
        let mut transform = self.transform;
        transform.scale.x = scaled.x as f32;
        transform.scale.y = scaled.y as f32;
        let project = |t: Transform| {
            let x = (t.rotation * Vec3::X * t.scale.x).truncate().as_dvec2();
            let y = (t.rotation * Vec3::Y * t.scale.y).truncate().as_dvec2();
            x * anchor.x + y * anchor.y
        };
        let translation = self.transform.translation.truncate().as_dvec2()
            + project(self.transform)
            - project(transform);
        transform.translation.x = translation.x as f32;
        transform.translation.y = translation.y as f32;
        CanvasDocument::update_transform(world, self.document, self.shape, transform)
    }
}

pub(crate) fn local_point(transform: Transform, point: Vec2) -> DVec2 {
    let relative = point.as_dvec2() - transform.translation.truncate().as_dvec2();
    let x = (transform.rotation * Vec3::X * transform.scale.x)
        .truncate()
        .as_dvec2();
    let y = (transform.rotation * Vec3::Y * transform.scale.y)
        .truncate()
        .as_dvec2();
    let determinant = x.perp_dot(y);
    DVec2::new(
        relative.perp_dot(y) / determinant,
        x.perp_dot(relative) / determinant,
    )
}
