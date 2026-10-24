use crate::{CanvasDocument, CanvasPaint, CanvasRectangle};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

pub(crate) const HANDLE_SIZE: f32 = 8.0;

#[derive(Clone, Copy)]
pub(crate) enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl ResizeHandle {
    fn axes(self) -> Vec2 {
        match self {
            Self::TopLeft => Vec2::new(-1.0, -1.0),
            Self::Top => Vec2::new(0.0, -1.0),
            Self::TopRight => Vec2::new(1.0, -1.0),
            Self::Right => Vec2::new(1.0, 0.0),
            Self::BottomRight => Vec2::ONE,
            Self::Bottom => Vec2::new(0.0, 1.0),
            Self::BottomLeft => Vec2::new(-1.0, 1.0),
            Self::Left => Vec2::new(-1.0, 0.0),
        }
    }
}

pub(crate) fn controls(
    rectangle: &CanvasRectangle,
    transform: &Transform,
    paint: &CanvasPaint,
) -> [(ResizeHandle, Vec2); 8] {
    let outer = rectangle.size + Vec2::splat(paint.stroke.map_or(0.0, |stroke| stroke.width));
    [
        ResizeHandle::TopLeft,
        ResizeHandle::Top,
        ResizeHandle::TopRight,
        ResizeHandle::Right,
        ResizeHandle::BottomRight,
        ResizeHandle::Bottom,
        ResizeHandle::BottomLeft,
        ResizeHandle::Left,
    ]
    .map(|handle| {
        (
            handle,
            transform
                .transform_point((handle.axes() * outer * 0.5).extend(0.0))
                .truncate(),
        )
    })
}

pub(crate) fn hit(
    world: &World,
    shape: Entity,
    point: Vec2,
    zoom: f32,
) -> Result<Option<ResizeHandle>> {
    let rectangle = world
        .get::<CanvasRectangle>(shape)
        .ok_or_else(|| BevyError::error("Canvas resize rectangle missing"))?;
    let transform = world
        .get::<Transform>(shape)
        .ok_or_else(|| BevyError::error("Canvas resize transform missing"))?;
    let paint = world
        .get::<CanvasPaint>(shape)
        .ok_or_else(|| BevyError::error("Canvas resize paint missing"))?;
    rectangle
        .validate(transform, paint)
        .inspect_err(|error| widgetry_error!(?shape,%error,"Canvas 尺寸控制点几何无效"))?;
    Ok(controls(rectangle, transform, paint)
        .into_iter()
        .filter_map(|(handle, center)| {
            let distance = (center - point).length_squared() * zoom * zoom;
            (distance <= (HANDLE_SIZE * 0.5 + 2.0).powi(2)).then_some((handle, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(handle, _)| handle))
}

#[derive(Clone, Copy)]
pub(crate) struct Resize {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    rectangle: CanvasRectangle,
    transform: Transform,
    start: Vec2,
    handle: ResizeHandle,
}

impl Resize {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shape: Entity,
        start: Vec2,
        handle: ResizeHandle,
    ) -> Result<Self> {
        Ok(Self {
            document,
            shape,
            start,
            handle,
            rectangle: *world
                .get::<CanvasRectangle>(shape)
                .ok_or_else(|| BevyError::error("Canvas resize rectangle missing"))?,
            transform: *world
                .get::<Transform>(shape)
                .ok_or_else(|| BevyError::error("Canvas resize transform missing"))?,
        })
    }

    pub(crate) fn apply(self, world: &mut World, point: Vec2) -> Result {
        let delta = (self.transform.rotation.conjugate() * (point - self.start).extend(0.0))
            .truncate()
            / self.transform.scale.truncate();
        if !delta.is_finite() {
            widgetry_error!(shape=?self.shape, "Canvas 尺寸调整局部位移溢出");
            return Err(BevyError::error("Canvas resize local delta overflow"));
        }
        let axes = self.handle.axes();
        let size = (self.rectangle.size + axes * delta).max(Vec2::ONE);
        let shift = (size - self.rectangle.size) * axes * 0.5;
        let offset = (self.transform.rotation
            * (shift * self.transform.scale.truncate()).extend(0.0))
        .truncate();
        let mut transform = self.transform;
        transform.translation.x += offset.x;
        transform.translation.y += offset.y;
        let paint = *world
            .get::<CanvasPaint>(self.shape)
            .ok_or_else(|| BevyError::error("Canvas resize paint missing"))?;
        CanvasDocument::update_rectangle(
            world,
            self.document,
            self.shape,
            CanvasRectangle { size },
            transform,
            paint,
        )
    }
}
