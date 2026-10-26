use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasShape, CanvasView};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

pub(crate) fn belongs_to(world: &World, document: Option<Entity>, shape: Entity) -> bool {
    document.is_some_and(|document| {
        world.get::<CanvasDocument>(document).is_some()
            && world.get::<CanvasShape>(shape).is_some()
            && world.get::<CanvasGeometry>(shape).is_some()
            && world.get::<Transform>(shape).is_some()
            && world.get::<CanvasPaint>(shape).is_some()
            && world
                .get::<ChildOf>(shape)
                .is_some_and(|parent| parent.parent() == document)
    })
}

pub(crate) fn repair(world: &mut World, view: Entity) -> Result<bool> {
    let stale = world
        .get::<crate::view::ViewState>(view)
        .is_some_and(|state| {
            state
                .selection
                .is_some_and(|shape| !belongs_to(world, state.document, shape))
        });
    if stale {
        return CanvasView::set_selection_in_world(world, view, None);
    }
    Ok(false)
}

pub(crate) fn hit_test(world: &mut World, view: Entity, point: Vec2) -> Result<Option<Entity>> {
    let document = CanvasView::document(world, view)?;
    let mut shapes =
        world.query_filtered::<(Entity, &ChildOf, &CanvasGeometry, &Transform, &CanvasPaint), With<CanvasShape>>();
    for (shape, parent, geometry, transform, paint) in shapes.iter(world) {
        if Some(parent.parent()) != document {
            continue;
        }
        let CanvasGeometry::Rectangle(rectangle) = geometry;
        rectangle
            .validate(transform, paint)
            .inspect_err(|error| widgetry_error!(?view,?shape,%error,"Canvas 命中几何无效"))?;
        // Only XY is inverted: model Z scale may legitimately be zero and has no planar meaning.
        let relative = point - transform.translation.truncate();
        let local = (transform.rotation.conjugate() * relative.extend(0.0)).truncate()
            / transform.scale.truncate();
        let half =
            (rectangle.size + Vec2::splat(paint.stroke.map_or(0.0, |stroke| stroke.width))) * 0.5;
        if local.is_finite() && local.abs().cmple(half).all() {
            return Ok(Some(shape));
        }
    }
    Ok(None)
}
