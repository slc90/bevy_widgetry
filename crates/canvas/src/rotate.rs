use crate::{CanvasDocument, CanvasPaint, CanvasRectangle};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

pub(crate) const OFFSET: f32 = 24.0;
pub(crate) const HANDLE_SIZE: f32 = 8.0;

pub(crate) fn control(
    rectangle: &CanvasRectangle,
    transform: &Transform,
    paint: &CanvasPaint,
    zoom: f32,
) -> Vec2 {
    let half = (rectangle.size.y + paint.stroke.map_or(0.0, |stroke| stroke.width)) * 0.5;
    let top = transform
        .transform_point(Vec3::new(0.0, -half, 0.0))
        .truncate();
    top + (transform.rotation * Vec3::new(0.0, -transform.scale.y.signum(), 0.0)).truncate()
        * (OFFSET / zoom)
}

pub(crate) fn hit(world: &World, shape: Entity, point: Vec2, zoom: f32) -> Result<bool> {
    let rectangle = world
        .get::<crate::CanvasGeometry>(shape)
        .and_then(crate::CanvasGeometry::rectangle)
        .ok_or_else(|| BevyError::error("Canvas rotation rectangle missing"))?;
    let transform = world
        .get::<Transform>(shape)
        .ok_or_else(|| BevyError::error("Canvas rotation transform missing"))?;
    let paint = world
        .get::<CanvasPaint>(shape)
        .ok_or_else(|| BevyError::error("Canvas rotation paint missing"))?;
    rectangle
        .validate(transform, paint)
        .inspect_err(|error| widgetry_error!(?shape,%error,"Canvas 旋转控制点几何无效"))?;
    Ok(
        (control(rectangle, transform, paint, zoom) - point).length_squared() * zoom * zoom
            <= (HANDLE_SIZE * 0.5 + 2.0).powi(2),
    )
}

#[derive(Clone, Copy)]
pub(crate) struct Rotate {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    transform: Transform,
    direction: Vec2,
}

impl Rotate {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shape: Entity,
        point: Vec2,
    ) -> Result<Option<Self>> {
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas rotation transform missing"))?;
        let Some(direction) = (point - transform.translation.truncate()).try_normalize() else {
            return Ok(None);
        };
        Ok(Some(Self {
            document,
            shape,
            transform,
            direction,
        }))
    }

    pub(crate) fn apply(self, world: &mut World, point: Vec2, snap: bool) -> Result {
        let Some(direction) = (point - self.transform.translation.truncate()).try_normalize()
        else {
            return Ok(());
        };
        let delta = self
            .direction
            .perp_dot(direction)
            .atan2(self.direction.dot(direction));
        let base = 2.0 * self.transform.rotation.z.atan2(self.transform.rotation.w);
        let mut angle = base + delta;
        if snap {
            let step = std::f32::consts::PI / 12.0;
            angle = (angle / step).round() * step;
        }
        let mut transform = self.transform;
        transform.rotation = Quat::from_rotation_z(angle);
        let rectangle = *world
            .get::<crate::CanvasGeometry>(self.shape)
            .and_then(crate::CanvasGeometry::rectangle)
            .ok_or_else(|| BevyError::error("Canvas rotation rectangle missing"))?;
        let paint = *world
            .get::<CanvasPaint>(self.shape)
            .ok_or_else(|| BevyError::error("Canvas rotation paint missing"))?;
        CanvasDocument::update_rectangle(
            world,
            self.document,
            self.shape,
            rectangle,
            transform,
            paint,
        )
    }
}
