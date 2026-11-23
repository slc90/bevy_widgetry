use crate::error::CanvasResult;
use crate::{CanvasFill, CanvasMode, CanvasPaint};
use bevy::{
    ecs::change_detection::Tick,
    math::{Affine3A, DVec2},
    prelude::*,
};

const HANDLE_RADIUS_LOGICAL: f32 = 8.0;

fn handles(fill: CanvasFill) -> Option<[Vec2; 2]> {
    match fill {
        CanvasFill::Solid(_) => None,
        CanvasFill::LinearGradient { start, end, .. } => Some([start, end]),
        CanvasFill::RadialGradient { center, radius, .. } => {
            Some([center, center + Vec2::X * radius])
        }
    }
}

pub(crate) fn overlay(
    world: &World,
    view: Entity,
    shape: Entity,
) -> CanvasResult<Option<[Vec2; 2]>> {
    let paint = crate::input::gradient_preview(world, view)
        .filter(|(entity, _)| *entity == shape)
        .map(|(_, paint)| paint)
        .or_else(|| world.get::<CanvasPaint>(shape).copied());
    let Some(points) = paint.and_then(|paint| paint.fill).and_then(handles) else {
        return Ok(None);
    };
    let projection = crate::groups::document_transform(world, shape)?;
    let points =
        points.map(|point| crate::snapping::project(projection, point.as_dvec2()).as_vec2());
    Ok(points
        .iter()
        .all(|point| point.is_finite())
        .then_some(points))
}

#[derive(Clone)]
pub(crate) struct Edit {
    pub document: Entity,
    pub shape: Entity,
    pub preview: CanvasPaint,
    initial_tick: Tick,
    projection: Affine3A,
    handle: usize,
    offset: DVec2,
}

impl Edit {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shape: Entity,
        point: Vec2,
        zoom: f32,
    ) -> CanvasResult<Option<Self>> {
        let Some(initial) = world
            .get_entity(shape)
            .ok()
            .and_then(|shape| shape.get_ref::<CanvasPaint>())
        else {
            return Ok(None);
        };
        let Some(handles) = initial.fill.and_then(handles) else {
            return Ok(None);
        };
        let projection = crate::groups::document_transform(world, shape)?;
        let best = handles
            .iter()
            .enumerate()
            .filter_map(|(index, handle)| {
                let projected = crate::snapping::project(projection, handle.as_dvec2()).as_vec2();
                let distance = point.distance(projected) * zoom;
                (distance <= HANDLE_RADIUS_LOGICAL).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1));
        let Some((handle, _)) = best else {
            return Ok(None);
        };
        Ok(Some(Self {
            document,
            shape,
            preview: *initial,
            initial_tick: initial.last_changed(),
            projection,
            handle,
            offset: crate::scale::local_point(projection, point) - handles[handle].as_dvec2(),
        }))
    }

    pub(crate) fn valid(&self, world: &World, view: Entity) -> bool {
        world
            .get::<crate::view::ViewState>(view)
            .is_some_and(|state| {
                state.document == Some(self.document)
                    && state.selection == [self.shape]
                    && state.mode == CanvasMode::GradientEdit
                    && crate::selection::selectable(
                        world,
                        Some(self.document),
                        crate::view::edited_group(world, state),
                        self.shape,
                    )
            })
            && world
                .get_entity(self.shape)
                .ok()
                .and_then(|shape| shape.get_ref::<CanvasPaint>())
                .is_some_and(|paint| paint.last_changed() == self.initial_tick)
            && crate::groups::document_transform(world, self.shape)
                .is_ok_and(|projection| projection == self.projection)
    }

    pub(crate) fn sample(&mut self, point: Vec2) {
        let local = (crate::scale::local_point(self.projection, point) - self.offset).as_vec2();
        let Some(mut fill) = self.preview.fill else {
            return;
        };
        match &mut fill {
            CanvasFill::Solid(_) => return,
            CanvasFill::LinearGradient { start, end, .. } => {
                if self.handle == 0 {
                    *start = local;
                } else {
                    *end = local;
                }
            }
            CanvasFill::RadialGradient { center, radius, .. } => {
                if self.handle == 0 {
                    *center = local;
                } else {
                    *radius = (local.as_dvec2() - center.as_dvec2()).length() as f32;
                }
            }
        }
        if fill.validate().is_ok() {
            self.preview.fill = Some(fill);
        }
    }
}
