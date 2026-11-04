use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasView};
use bevy::prelude::*;

#[derive(Clone)]
pub(crate) struct MoveSelection {
    pub(crate) document: Entity,
    pub(crate) shapes: Vec<(Entity, Vec2)>,
    pub(crate) hit: Entity,
    pub(crate) moved: bool,
    start: Vec2,
}
impl MoveSelection {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shapes: Vec<Entity>,
        hit: Entity,
        start: Vec2,
    ) -> Result<Self> {
        (|| -> Result<Self> {
            let shapes = shapes
                .into_iter()
                .map(|shape| {
                    world
                        .get::<Transform>(shape)
                        .map(|t| (shape, t.translation.truncate()))
                        .ok_or_else(|| crate::error::missing(crate::CanvasPart::SelectionTransform))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Self {
                document,
                shapes,
                hit,
                moved: false,
                start,
            })
        })()
        .map_err(|error| error.at("document", document).at("hit", hit))
    }
    pub(crate) fn valid(&self, world: &World, view: Entity) -> bool {
        world.get::<crate::view::ViewState>(view).is_some_and(|s| {
            s.document == Some(self.document)
                && s.mode == crate::CanvasMode::Select
                && s.selection.len() == self.shapes.len()
                && s.selection
                    .iter()
                    .zip(&self.shapes)
                    .all(|(a, (b, _))| a == b)
        }) && self
            .shapes
            .iter()
            .all(|(s, _)| crate::selection::belongs_to(world, Some(self.document), *s))
    }
    pub(crate) fn apply(&self, world: &mut World, point: Vec2) -> Result<bool> {
        let delta = point.as_dvec2() - self.start.as_dvec2();
        if delta == bevy::math::DVec2::ZERO && !self.moved {
            return Ok(false);
        }
        let mut updates = Vec::with_capacity(self.shapes.len());
        for (shape, origin) in &self.shapes {
            let position = (origin.as_dvec2() + delta).as_vec2();
            if !position.is_finite() {
                return Ok(false);
            }
            let mut transform = *world
                .get::<Transform>(*shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::SelectionTransform))?;
            transform.translation.x = position.x;
            transform.translation.y = position.y;
            updates.push((*shape, transform));
        }
        CanvasDocument::_update_transforms(world, self.document, &updates)?;
        Ok(delta != bevy::math::DVec2::ZERO)
    }
    pub(crate) fn finish(self, world: &mut World, view: Entity) -> Result {
        (|| -> Result {
            if !self.moved && self.valid(world, view) {
                CanvasView::_set_selection_in_world(world, view, [self.hit])?;
            }
            Ok(())
        })()
        .map_err(|error| error.at("view", view))
    }
}
