use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasShape, CanvasView};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
mod hit;

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

pub(crate) fn selectable(world: &World, document: Option<Entity>, shape: Entity) -> bool {
    belongs_to(world, document, shape)
}

pub(crate) fn repair(world: &mut World, view: Entity) -> Result<bool> {
    let selection = CanvasView::selection(world, view)?;
    if world
        .get::<crate::view::ViewState>(view)
        .is_some_and(|state| state.selection != selection)
    {
        return CanvasView::set_selection_in_world(world, view, selection);
    }
    Ok(false)
}

pub(crate) fn hit_test(world: &mut World, view: Entity, point: Vec2) -> Result<Option<Entity>> {
    hit_shapes(world, view, point)
        .inspect_err(|error| widgetry_error!(?view, %error, "Canvas 几何命中失败"))
}

fn hit_shapes(world: &mut World, view: Entity, point: Vec2) -> Result<Option<Entity>> {
    let document = CanvasView::document(world, view)?;
    let Some(document) = document else {
        return Ok(None);
    };
    let screen = hit::ScreenSpace::new(world, view, CanvasView::zoom(world, view)?)?;
    for shape in CanvasDocument::shape_order(world, document)?
        .into_iter()
        .rev()
    {
        let geometry = world
            .get::<CanvasGeometry>(shape)
            .ok_or_else(|| BevyError::error("Canvas geometry missing"))?
            .clone();
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas transform missing"))?;
        let paint = *world
            .get::<CanvasPaint>(shape)
            .ok_or_else(|| BevyError::error("Canvas paint missing"))?;
        geometry.validate(&transform, &paint)?;
        if paint.fill.is_none() && paint.stroke.is_none() {
            continue;
        }
        let handles = crate::mesh_cache::geometry(world, shape, &geometry, paint)?;
        let meshes = world.resource::<Assets<Mesh>>();
        let fill = meshes
            .get(&handles.fill)
            .ok_or_else(|| BevyError::error("Canvas hit fill mesh missing"))?;
        let stroke = meshes
            .get(&handles.stroke)
            .ok_or_else(|| BevyError::error("Canvas hit stroke mesh missing"))?;
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
            return Ok(Some(shape));
        }
    }
    Ok(None)
}

pub(crate) fn local_bounds(world: &mut World, shape: Entity) -> Result<Rect> {
    let geometry = world
        .get::<CanvasGeometry>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection geometry missing"))?
        .clone();
    let paint = *world
        .get::<CanvasPaint>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection paint missing"))?;
    let handles = crate::mesh_cache::geometry(world, shape, &geometry, paint)?;
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
        let bounds = geometry.bounds(
            &Transform::default(),
            &CanvasPaint {
                stroke: None,
                ..paint
            },
        )?;
        min = bounds.min;
        max = bounds.max;
    }
    Ok(Rect { min, max })
}

pub(crate) fn frame(world: &mut World, shape: Entity) -> Result<[Vec2; 4]> {
    let bounds = local_bounds(world, shape)?;
    let (min, max) = (bounds.min, bounds.max);
    let transform = world
        .get::<Transform>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection transform missing"))?;
    let x = (transform.rotation * Vec3::X * transform.scale.x)
        .truncate()
        .as_dvec2();
    let y = (transform.rotation * Vec3::Y * transform.scale.y)
        .truncate()
        .as_dvec2();
    Ok(
        [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)].map(|p| {
            (transform.translation.truncate().as_dvec2() + x * f64::from(p.x) + y * f64::from(p.y))
                .as_vec2()
        }),
    )
}
