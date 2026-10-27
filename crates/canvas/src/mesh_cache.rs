use crate::{CanvasFillRule, CanvasGeometry, CanvasLineCap, CanvasLineJoin, CanvasPaint};
use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone)]
struct Geometry {
    source: Arc<CanvasGeometry>,
    style: GeometryStyle,
    fill: Handle<Mesh>,
    stroke: Handle<Mesh>,
    fill_bounds: Aabb,
    stroke_bounds: Aabb,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct GeometryStyle {
    fill_rule: CanvasFillRule,
    stroke: Option<(f32, CanvasLineCap, CanvasLineJoin)>,
}

impl From<CanvasPaint> for GeometryStyle {
    fn from(paint: CanvasPaint) -> Self {
        Self {
            fill_rule: paint.fill_rule,
            stroke: paint.stroke.map(|s| (s.width, s.cap, s.join)),
        }
    }
}

pub(crate) struct ShapeMeshes {
    pub(crate) fill: Handle<Mesh>,
    pub(crate) stroke: Handle<Mesh>,
    pub(crate) fill_bounds: Aabb,
    pub(crate) stroke_bounds: Aabb,
}

#[derive(Resource, Default)]
pub(crate) struct MeshCache(HashMap<Entity, Geometry>);

pub(crate) fn geometry(
    world: &mut World,
    shape: Entity,
    source: &CanvasGeometry,
    paint: CanvasPaint,
) -> Result<ShapeMeshes> {
    let style = GeometryStyle::from(paint);
    let previous = world.resource::<MeshCache>().0.get(&shape).cloned();
    let geometry = if let Some(mut geometry) = previous {
        if geometry.source.as_ref() != source || geometry.style != style {
            let (fill, stroke) = crate::geometry::meshes(source, paint)?;
            geometry.fill_bounds = fill
                .get_aabb()
                .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO));
            geometry.stroke_bounds = stroke
                .get_aabb()
                .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO));
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            *meshes
                .get_mut(&geometry.fill)
                .ok_or_else(|| BevyError::error("Canvas shared fill mesh missing"))? = fill;
            *meshes
                .get_mut(&geometry.stroke)
                .ok_or_else(|| BevyError::error("Canvas shared stroke mesh missing"))? = stroke;
            geometry.source = Arc::new(source.clone());
            geometry.style = style;
        }
        geometry
    } else {
        let (fill, stroke) = crate::geometry::meshes(source, paint)?;
        Geometry {
            source: Arc::new(source.clone()),
            style,
            fill_bounds: fill
                .get_aabb()
                .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO)),
            stroke_bounds: stroke
                .get_aabb()
                .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO)),
            fill: world.resource_mut::<Assets<Mesh>>().add(fill),
            stroke: world.resource_mut::<Assets<Mesh>>().add(stroke),
        }
    };
    let handles = ShapeMeshes {
        fill: geometry.fill.clone(),
        stroke: geometry.stroke.clone(),
        fill_bounds: geometry.fill_bounds,
        stroke_bounds: geometry.stroke_bounds,
    };
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
