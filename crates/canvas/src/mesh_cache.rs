use crate::error::CanvasResult as Result;
use crate::{CanvasFillRule, CanvasGeometry, CanvasLineCap, CanvasLineJoin, CanvasPaint};
use bevy::camera::primitives::Aabb;
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
    fill_region: Option<Rect>,
    stroke_region: Option<Rect>,
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
    pub(crate) fill_region: Option<Rect>,
    pub(crate) stroke_region: Option<Rect>,
}

#[derive(Resource, Default)]
pub(crate) struct MeshCache(HashMap<Entity, Geometry>);

pub(crate) fn geometry(
    world: &mut World,
    shape: Entity,
    source: &CanvasGeometry,
    paint: CanvasPaint,
) -> Result<ShapeMeshes> {
    (|| -> Result<ShapeMeshes> {
        let style = GeometryStyle::from(paint);
        let previous = world.resource::<MeshCache>().0.get(&shape).cloned();
        let geometry =
            if let Some(mut geometry) = previous {
                if geometry.source.as_ref() != source || geometry.style != style {
                    let (fill, stroke) = crate::geometry::meshes(source, paint)?;
                    geometry.fill_region = region(&fill)?;
                    geometry.stroke_region = region(&stroke)?;
                    let mut meshes = world.resource_mut::<Assets<Mesh>>();
                    if !meshes.contains(&geometry.fill) {
                        return Err(crate::error::missing(crate::CanvasPart::SharedFillMesh));
                    }
                    if !meshes.contains(&geometry.stroke) {
                        return Err(crate::error::missing(crate::CanvasPart::SharedStrokeMesh));
                    }
                    *meshes.get_mut(&geometry.fill).ok_or_else(|| {
                        crate::error::missing(crate::CanvasPart::SharedFillMesh)
                    })? = fill;
                    *meshes.get_mut(&geometry.stroke).ok_or_else(|| {
                        crate::error::missing(crate::CanvasPart::SharedStrokeMesh)
                    })? = stroke;
                    geometry.source = Arc::new(source.clone());
                    geometry.style = style;
                }
                geometry
            } else {
                let (fill, stroke) = crate::geometry::meshes(source, paint)?;
                Geometry {
                    source: Arc::new(source.clone()),
                    style,
                    fill_region: region(&fill)?,
                    stroke_region: region(&stroke)?,
                    fill: world.resource_mut::<Assets<Mesh>>().add(fill),
                    stroke: world.resource_mut::<Assets<Mesh>>().add(stroke),
                }
            };
        let handles = ShapeMeshes {
            fill: geometry.fill.clone(),
            stroke: geometry.stroke.clone(),
            fill_bounds: aabb(geometry.fill_region),
            stroke_bounds: aabb(geometry.stroke_region),
            fill_region: geometry.fill_region,
            stroke_region: geometry.stroke_region,
        };
        world.resource_mut::<MeshCache>().0.insert(shape, geometry);
        Ok(handles)
    })()
    .map_err(|error| error.at("shape", shape))
}

fn region(mesh: &Mesh) -> Result<Option<Rect>> {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(crate::error::missing(
            crate::CanvasPart::SharedMeshPositions,
        ));
    };
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for p in positions {
        let p = Vec2::new(p[0], p[1]);
        if !p.is_finite() {
            return Err(crate::error::invalid(crate::CanvasIssue::MeshBounds));
        }
        min = min.min(p);
        max = max.max(p);
    }
    Ok((!positions.is_empty()).then_some(Rect { min, max }))
}

fn aabb(region: Option<Rect>) -> Aabb {
    let region = region.unwrap_or_default();
    Aabb::from_min_max(region.min.extend(0.0), region.max.extend(0.0))
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

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejecting_a_missing_stroke_does_not_replace_the_existing_fill() {
        let mut world = World::new();
        world.init_resource::<MeshCache>();
        world.init_resource::<Assets<Mesh>>();
        let shape = world.spawn_empty().id();
        let source = |size| {
            CanvasGeometry::Rectangle(crate::CanvasRectangle {
                size: Vec2::splat(size),
                corner_radius: 0.0,
            })
        };
        let paint = CanvasPaint {
            stroke: Some(crate::CanvasStroke::default()),
            ..default()
        };
        let first = geometry(&mut world, shape, &source(5.0), paint).unwrap();
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let before = meshes
            .get(&first.fill)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .clone();
        meshes.remove(first.stroke.id());
        assert!(geometry(&mut world, shape, &source(10.0), paint).is_err());
        let meshes = world.resource::<Assets<Mesh>>();
        assert_eq!(
            meshes
                .get(&first.fill)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            Some(&before)
        );
    }
}
