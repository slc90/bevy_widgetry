use crate::CanvasView;
use crate::error::CanvasResult as Result;
use bevy::prelude::*;

#[derive(Clone)]
pub(crate) struct MoveSelection {
    pub(crate) document: Entity,
    pub(crate) shapes: Vec<(Entity, Vec2)>,
    pub(crate) hit: Entity,
    pub(crate) moved: bool,
    start: bevy::math::DVec2,
    anchor: bevy::math::DVec2,
    parent: bevy::math::Affine3A,
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
            let parent = crate::groups::parent_transform(world, hit)?;
            let transform = crate::groups::local_transform(world, hit)?;
            let anchor = transform.translation.truncate().as_dvec2()
                + crate::grid::move_anchor_offset(world, hit, transform);
            Ok(Self {
                document,
                shapes,
                hit,
                moved: false,
                start: crate::scale::local_point(parent, start),
                anchor,
                parent,
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
                && self.shapes.iter().all(|(item, _)| {
                    crate::selection::selectable(
                        world,
                        Some(self.document),
                        crate::view::edited_group(world, s),
                        *item,
                    )
                })
        }) && crate::groups::parent_transform(world, self.hit)
            .is_ok_and(|parent| parent == self.parent)
    }
    pub(crate) fn apply(&self, world: &mut World, view: Entity, point: Vec2) -> Result<bool> {
        let delta = crate::scale::local_point(self.parent, point) - self.start;
        let moved = delta != bevy::math::DVec2::ZERO;
        if !moved && !self.moved {
            return Ok(false);
        }
        let correction = crate::grid::local_delta(
            world,
            view,
            crate::grid::SnapTarget::Movement,
            self.parent,
            self.anchor + delta,
        );
        let delta = delta + correction;
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
        crate::batch::update_selected_transforms(world, self.document, &updates)?;
        Ok(moved)
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
