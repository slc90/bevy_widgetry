use crate::CanvasDocument;
use bevy::{math::DVec2, prelude::*};

pub(crate) const OFFSET: f32 = 24.0;
pub(crate) const HANDLE_SIZE: f32 = 8.0;

pub(crate) fn control(points: [Vec2; 4], zoom: f32) -> Vec2 {
    let center = ((points[0].as_dvec2() + points[2].as_dvec2()) * 0.5).as_vec2();
    let top = ((points[0].as_dvec2() + points[1].as_dvec2()) * 0.5).as_vec2();
    let direction = (top - center).try_normalize().unwrap_or_else(|| {
        let edge = (points[1] - points[0]).normalize_or_zero();
        Vec2::new(edge.y, -edge.x)
            .try_normalize()
            .unwrap_or(Vec2::NEG_Y)
    });
    top + direction * (OFFSET / zoom)
}

pub(crate) fn hit(world: &mut World, shape: Entity, point: Vec2, zoom: f32) -> Result<bool> {
    Ok(
        (control(crate::selection::frame(world, shape)?, zoom) - point).length_squared()
            * zoom
            * zoom
            <= (HANDLE_SIZE * 0.5 + 2.0).powi(2),
    )
}

#[derive(Clone, Copy)]
pub(crate) struct Rotate {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    transform: Transform,
    anchor: DVec2,
    pivot: DVec2,
    direction: DVec2,
}

impl Rotate {
    pub(crate) fn begin(
        world: &mut World,
        document: Entity,
        shape: Entity,
        point: Vec2,
    ) -> Result<Option<Self>> {
        let bounds = crate::selection::local_bounds(world, shape)?;
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas rotation transform missing"))?;
        let anchor = (bounds.min.as_dvec2() + bounds.max.as_dvec2()) * 0.5;
        let pivot = project(transform, anchor);
        let Some(direction) = (point.as_dvec2() - pivot).try_normalize() else {
            return Ok(None);
        };
        Ok(Some(Self {
            document,
            shape,
            transform,
            anchor,
            pivot,
            direction,
        }))
    }

    pub(crate) fn apply(self, world: &mut World, point: Vec2, snap: bool) -> Result {
        let Some(direction) = (point.as_dvec2() - self.pivot).try_normalize() else {
            return Ok(());
        };
        let delta = self
            .direction
            .perp_dot(direction)
            .atan2(self.direction.dot(direction));
        let base =
            2.0 * f64::from(self.transform.rotation.z).atan2(f64::from(self.transform.rotation.w));
        let mut angle = base + delta;
        if snap {
            let step = std::f64::consts::PI / 12.0;
            angle = (angle / step).round() * step;
        }
        let mut transform = self.transform;
        transform.rotation = Quat::from_rotation_z(angle as f32);
        let center = project(
            Transform {
                translation: Vec3::ZERO,
                ..transform
            },
            self.anchor,
        );
        let translation = self.pivot - center;
        transform.translation.x = translation.x as f32;
        transform.translation.y = translation.y as f32;
        CanvasDocument::update_transform(world, self.document, self.shape, transform)
    }
}

fn project(transform: Transform, point: DVec2) -> DVec2 {
    let x = (transform.rotation * Vec3::X * transform.scale.x)
        .truncate()
        .as_dvec2();
    let y = (transform.rotation * Vec3::Y * transform.scale.y)
        .truncate()
        .as_dvec2();
    transform.translation.truncate().as_dvec2() + x * point.x + y * point.y
}
