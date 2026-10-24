use crate::{CanvasPaint, CanvasRectangle};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct Geometry {
    size: Vec2,
    stroke_width: f32,
    fill: Handle<Mesh>,
    stroke: Handle<Mesh>,
}

#[derive(Resource, Default)]
pub(crate) struct MeshCache(HashMap<Entity, Geometry>);

pub(crate) fn geometry(
    world: &mut World,
    shape: Entity,
    rectangle: CanvasRectangle,
    paint: CanvasPaint,
) -> Result<(Handle<Mesh>, Handle<Mesh>)> {
    let width = paint.stroke.map_or(0.0, |stroke| stroke.width);
    let previous = world.resource::<MeshCache>().0.get(&shape).cloned();
    let geometry = if let Some(mut geometry) = previous {
        if geometry.size != rectangle.size || geometry.stroke_width != width {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            *meshes
                .get_mut(&geometry.fill)
                .ok_or_else(|| BevyError::error("Canvas shared fill mesh missing"))? =
                crate::geometry::fill(rectangle.size);
            *meshes
                .get_mut(&geometry.stroke)
                .ok_or_else(|| BevyError::error("Canvas shared stroke mesh missing"))? =
                crate::geometry::stroke(rectangle.size, width);
            geometry.size = rectangle.size;
            geometry.stroke_width = width;
        }
        geometry
    } else {
        Geometry {
            size: rectangle.size,
            stroke_width: width,
            fill: world
                .resource_mut::<Assets<Mesh>>()
                .add(crate::geometry::fill(rectangle.size)),
            stroke: world
                .resource_mut::<Assets<Mesh>>()
                .add(crate::geometry::stroke(rectangle.size, width)),
        }
    };
    let handles = (geometry.fill.clone(), geometry.stroke.clone());
    world.resource_mut::<MeshCache>().0.insert(shape, geometry);
    Ok(handles)
}

pub(crate) fn retain(world: &mut World, used: &HashSet<Entity>) {
    let stale = world
        .resource::<MeshCache>()
        .0
        .iter()
        .filter(|(shape, _)| !used.contains(shape))
        .map(|(shape, geometry)| (*shape, geometry.clone()))
        .collect::<Vec<_>>();
    for (shape, geometry) in stale {
        world
            .resource_mut::<Assets<Mesh>>()
            .remove(geometry.fill.id());
        world
            .resource_mut::<Assets<Mesh>>()
            .remove(geometry.stroke.id());
        world.resource_mut::<MeshCache>().0.remove(&shape);
    }
}
