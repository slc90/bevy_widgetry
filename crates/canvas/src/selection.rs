use crate::error::CanvasResult as Result;
use crate::{
    CanvasConnector, CanvasDocument, CanvasGeometry, CanvasGroup, CanvasPaint, CanvasView,
};
use bevy::math::Affine3A;
use bevy::prelude::*;

mod hit;

pub(crate) fn belongs_to(world: &World, document: Option<Entity>, shape: Entity) -> bool {
    document.is_some_and(|document| {
        world.get::<CanvasDocument>(document).is_some()
            && world.get_entity(shape).as_ref().is_ok_and(|item| {
                crate::model::has_shape_components(item)
                    || item.contains::<crate::CanvasText>()
                    || item
                        .get::<CanvasConnector>()
                        .is_some_and(CanvasConnector::visible)
                    || crate::groups::has_group_components(item)
            })
            && crate::order::item_placement(world, document, shape)
                .is_some_and(|(_, layer)| layer.editable())
    })
}

pub(crate) fn selectable(
    world: &World,
    document: Option<Entity>,
    scope: Option<Entity>,
    item: Entity,
) -> bool {
    let parent = scope.or(document);
    world.get::<ChildOf>(item).map(ChildOf::parent) == parent && belongs_to(world, document, item)
}

pub(crate) fn select_committed_shape(
    world: &mut World,
    view: Entity,
    document: Entity,
    mode: crate::CanvasMode,
    shape: Entity,
) -> Result {
    if world
        .get::<crate::view::ViewState>(view)
        .is_some_and(|state| {
            state.document == Some(document)
                && state.mode == mode
                && selectable(
                    world,
                    Some(document),
                    crate::view::edited_group(world, state),
                    shape,
                )
        })
    {
        CanvasView::_set_selection_in_world(world, view, Some(shape))?;
    }
    Ok(())
}

pub(crate) fn repair(world: &mut World, view: Entity) -> Result<bool> {
    (|| -> Result<bool> {
        let repaired_path = crate::view::repair_group_path(world, view)?;
        if world.get::<crate::view::ViewState>(view).is_none() {
            return Ok(repaired_path);
        }
        let selection = CanvasView::_selection(world, view)?;
        if world
            .get::<crate::view::ViewState>(view)
            .is_some_and(|state| state.selection != selection)
        {
            return CanvasView::_set_selection_in_world(world, view, selection);
        }
        Ok(repaired_path)
    })()
    .map_err(|error| error.at("view", view))
}

pub(crate) fn hit_test(world: &mut World, view: Entity, point: Vec2) -> Result<Option<Entity>> {
    hit_shapes(world, view, point).map_err(|error| error.at("view", view))
}

fn hit_shapes(world: &mut World, view: Entity, point: Vec2) -> Result<Option<Entity>> {
    (|| -> Result<Option<Entity>> {
        let document = CanvasView::_document(world, view)?;
        let Some(document) = document else {
            return Ok(None);
        };
        let scope = world
            .get::<crate::view::ViewState>(view)
            .and_then(|state| crate::view::edited_group(world, state))
            .unwrap_or(document);
        let screen = hit::ScreenSpace::new(world, view, CanvasView::_zoom(world, view)?)?;
        let mut items = Vec::new();
        crate::order::visit_valid_order(
            world,
            document,
            crate::CanvasLayer::editable,
            |entry, _, group| {
                if !group {
                    items.push(entry.id());
                }
                Ok(())
            },
        )?;
        for shape in items.into_iter().rev() {
            if world
                .get::<CanvasConnector>(shape)
                .is_some_and(|connector| !connector.visible())
            {
                continue;
            }
            let Some(target) = target_in_scope(world, document, scope, shape) else {
                continue;
            };
            let transform = crate::groups::document_transform(world, shape)?;
            if let Some(text) = world.get::<crate::CanvasText>(shape) {
                if hit_rect(text.local_bounds(), transform, point) {
                    return Ok(Some(target));
                }
                continue;
            }
            if let Some((label, at)) = world
                .get::<CanvasConnector>(shape)
                .filter(|connector| connector.drawn())
                .and_then(CanvasConnector::label_at)
                && hit_rect(
                    label.local_bounds(),
                    CanvasConnector::label_projection(transform, at),
                    point,
                )
            {
                return Ok(Some(target));
            }
            let (geometry, paint, arrows) = item_geometry(world, shape)?;
            let local = world
                .get::<Transform>(shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::SelectionTransform))?;
            geometry
                .bounds_in_projection(local, &transform, &paint)
                .map_err(|error| error.at("shape", shape).at("document", document))?;
            if paint.fill.is_none() && paint.stroke.is_none() {
                continue;
            }
            let handles =
                crate::mesh_cache::geometry(world, shape, &geometry, paint, arrows.as_ref())?;
            let meshes = world.resource::<Assets<Mesh>>();
            let fill = meshes
                .get(&handles.fill)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::HitFillMesh))?;
            let stroke = meshes
                .get(&handles.stroke)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::HitStrokeMesh))?;
            if (paint.fill.is_some()
                && handles
                    .fill_region
                    .is_some_and(|bounds| screen.near_bounds(bounds, &transform, point))
                && screen.mesh(fill, &transform, point, false)?)
                || (paint.stroke.is_some()
                    && handles
                        .stroke_region
                        .is_some_and(|bounds| screen.near_bounds(bounds, &transform, point))
                    && screen.mesh(stroke, &transform, point, true)?)
            {
                return Ok(Some(target));
            }
        }
        Ok(None)
    })()
    .map_err(|error| error.at("view", view))
}

fn hit_rect(bounds: Rect, projection: Affine3A, point: Vec2) -> bool {
    let local = crate::scale::local_point(projection, point);
    bounds.size().min_element() > 0.0
        && local.cmpge(bounds.min.as_dvec2()).all()
        && local.cmple(bounds.max.as_dvec2()).all()
}

fn item_geometry(
    world: &World,
    item: Entity,
) -> Result<(CanvasGeometry, CanvasPaint, Option<crate::CanvasPath>)> {
    if let Some(connector) = world.get::<CanvasConnector>(item) {
        return Ok(connector.geometry());
    }
    let source = world
        .get::<CanvasGeometry>(item)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry))?;
    let paint = world
        .get::<CanvasPaint>(item)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::Paint))?;
    Ok((source.clone(), *paint, None))
}

fn target_in_scope(
    world: &World,
    document: Entity,
    scope: Entity,
    mut item: Entity,
) -> Option<Entity> {
    // The draw-order traversal already checked ownership at each ancestor.
    while item != document {
        let parent = world.get::<ChildOf>(item)?.parent();
        if parent == scope {
            return Some(item);
        }
        item = parent;
    }
    None
}

pub(crate) fn local_bounds(world: &mut World, shape: Entity) -> Result<Rect> {
    if let Some(text) = world.get::<crate::CanvasText>(shape) {
        return Ok(text.local_bounds());
    }
    if world.get::<CanvasGroup>(shape).is_some() {
        return crate::groups::local_bounds(world, shape);
    }
    (|| -> Result<Rect> {
        let (geometry, paint, arrows) = item_geometry(world, shape)?;
        let transform = world
            .get::<Transform>(shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::SelectionTransform))?;
        geometry._validate(transform, &paint)?;
        let handles = crate::mesh_cache::geometry(world, shape, &geometry, paint, arrows.as_ref())?;
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for (region, include) in [
            (handles.fill_region, paint.fill.is_some()),
            (handles.stroke_region, paint.stroke.is_some()),
        ] {
            if !include {
                continue;
            }
            if let Some(region) = region {
                min = min.min(region.min);
                max = max.max(region.max);
            }
        }
        if !min.is_finite() || !max.is_finite() {
            let bounds = geometry._bounds(
                &Transform::default(),
                &CanvasPaint {
                    stroke: None,
                    ..paint
                },
            )?;
            min = bounds.min;
            max = bounds.max;
        }
        let mut bounds = Rect { min, max };
        if let Some((label, at)) = world
            .get::<CanvasConnector>(shape)
            .filter(|connector| connector.drawn())
            .and_then(CanvasConnector::label_at)
        {
            let rect = label.local_bounds();
            bounds = bounds.union(Rect {
                min: rect.min + at,
                max: rect.max + at,
            });
        }
        Ok(bounds)
    })()
    .map_err(|error| error.at("shape", shape))
}

pub(crate) fn frame(world: &mut World, shape: Entity) -> Result<[Vec2; 4]> {
    (|| -> Result<[Vec2; 4]> {
        let bounds = local_bounds(world, shape)?;
        let (min, max) = (bounds.min, bounds.max);
        let transform = crate::groups::document_transform(world, shape)?;
        let [x, y] = crate::groups::planar_axes(&transform);
        Ok(
            [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)].map(|p| {
                (transform.translation.truncate().as_dvec2()
                    + x * f64::from(p.x)
                    + y * f64::from(p.y))
                .as_vec2()
            }),
        )
    })()
    .map_err(|error| error.at("shape", shape))
}
