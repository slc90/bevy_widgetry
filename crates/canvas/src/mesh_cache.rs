use crate::error::CanvasResult as Result;
use crate::{CanvasFillRule, CanvasGeometry, CanvasLineCap, CanvasLineJoin, CanvasPaint};
use bevy::camera::primitives::Aabb;
use bevy::math::{Affine3A, DMat2};
use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone)]
struct Geometry {
    source: Arc<CanvasGeometry>,
    arrows: Option<crate::CanvasPath>,
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
    arrows: Option<&crate::CanvasPath>,
) -> Result<ShapeMeshes> {
    (|| -> Result<ShapeMeshes> {
        let style = GeometryStyle::from(paint);
        let previous = world.resource::<MeshCache>().0.get(&shape).cloned();
        let geometry = if let Some(mut geometry) = previous {
            if geometry.source.as_ref() != source
                || geometry.style != style
                || geometry.arrows.as_ref() != arrows
            {
                let (fill, stroke) = crate::geometry::meshes_with_arrows(source, paint, arrows)?;
                geometry.fill_region = region(&fill)?;
                geometry.stroke_region = region(&stroke)?;
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                if !meshes.contains(&geometry.fill) {
                    return Err(crate::error::missing(crate::CanvasPart::SharedFillMesh));
                }
                if !meshes.contains(&geometry.stroke) {
                    return Err(crate::error::missing(crate::CanvasPart::SharedStrokeMesh));
                }
                *meshes
                    .get_mut(&geometry.fill)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::SharedFillMesh))? =
                    fill;
                *meshes
                    .get_mut(&geometry.stroke)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::SharedStrokeMesh))? =
                    stroke;
                geometry.source = Arc::new(source.clone());
                geometry.arrows = arrows.cloned();
                geometry.style = style;
            }
            geometry
        } else {
            let (fill, stroke) = crate::geometry::meshes_with_arrows(source, paint, arrows)?;
            Geometry {
                source: Arc::new(source.clone()),
                arrows: arrows.cloned(),
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
    projected_region(mesh, |point| point)
}

pub(crate) fn painted_bounds(
    world: &World,
    shape: Entity,
    source: &CanvasGeometry,
    paint: CanvasPaint,
    projection: &Affine3A,
    arrows: Option<&crate::CanvasPath>,
) -> Result<Option<Rect>> {
    if paint.fill.is_none() && paint.stroke.is_none() {
        return Ok(None);
    }
    let [x, y] = crate::groups::planar_axes(projection);
    let matrix = DMat2::from_cols(x, y);
    let offset = projection.translation.truncate().as_dvec2();
    let project = |point: Vec2| (offset + matrix * point.as_dvec2()).as_vec2();
    let measure = |fill: &Mesh, stroke: &Mesh| -> Result<Option<Rect>> {
        let mut bounds: Option<Rect> = None;
        for mesh in [paint.fill.map(|_| fill), paint.stroke.map(|_| stroke)]
            .into_iter()
            .flatten()
        {
            if let Some(region) = projected_region(mesh, project)? {
                bounds = Some(bounds.map_or(region, |current| current.union(region)));
            }
        }
        if bounds.is_some_and(|bounds| !bounds.size().is_finite()) {
            return Err(crate::error::invalid(crate::CanvasIssue::MeshBounds));
        }
        Ok(bounds)
    };
    if let Some(cache) = world.get_resource::<MeshCache>()
        && let Some(current) = cache.0.get(&shape)
        && current.source.as_ref() == source
        && current.arrows.as_ref() == arrows
        && current.style == GeometryStyle::from(paint)
        && let Some(meshes) = world.get_resource::<Assets<Mesh>>()
        && let (Some(fill), Some(stroke)) = (meshes.get(&current.fill), meshes.get(&current.stroke))
    {
        return measure(fill, stroke);
    }

    let (fill, stroke) = crate::geometry::meshes_with_arrows(source, paint, arrows)?;
    measure(&fill, &stroke)
}

fn projected_region(mesh: &Mesh, project: impl Fn(Vec2) -> Vec2) -> Result<Option<Rect>> {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(crate::error::missing(
            crate::CanvasPart::SharedMeshPositions,
        ));
    };
    let indices = mesh
        .indices()
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::SharedMeshIndices))?;
    let invalid_bounds = || crate::error::invalid(crate::CanvasIssue::MeshBounds);
    let position = |index| {
        let &[x, y, _]: &[f32; 3] = positions.get(index).ok_or_else(invalid_bounds)?;
        let point = Vec2::new(x, y);
        if !point.is_finite() {
            return Err(invalid_bounds());
        }
        Ok(point)
    };
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let mut triangles = indices.iter();
    while let Some(first) = triangles.next() {
        let second = triangles.next().ok_or_else(invalid_bounds)?;
        let third = triangles.next().ok_or_else(invalid_bounds)?;
        let [a, b, c] = [position(first)?, position(second)?, position(third)?];
        if !triangle_has_area([a, b, c]) {
            continue;
        }
        for point in [a, b, c] {
            let projected = project(point);
            if !projected.is_finite() {
                return Err(invalid_bounds());
            }
            min = min.min(projected);
            max = max.max(projected);
        }
    }
    Ok(min.is_finite().then_some(Rect { min, max }))
}

fn triangle_has_area(points: [Vec2; 3]) -> bool {
    let [a, b, c] = points.map(Vec2::as_dvec2);
    let u = b - a;
    let v = c - a;
    let positive = u.x * v.y;
    let negative = u.y * v.x;
    let error_bound = (positive.abs() + negative.abs()) * (8.0 * f64::EPSILON);
    if (positive - negative).abs() > error_bound {
        return true;
    }

    // Products of f32 coordinates are exact in f64. Preserve their low parts
    // when summing so a short edge survives cancellation between distant points.
    let terms = [
        a.x * b.y,
        b.x * c.y,
        c.x * a.y,
        -(a.y * b.x),
        -(b.y * c.x),
        -(c.y * a.x),
    ];
    let mut partials: [f64; 6] = [0.0; 6];
    let mut count = 0;
    for mut term in terms {
        let mut kept = 0;
        for mut partial in partials.into_iter().take(count) {
            if term.abs() < partial.abs() {
                std::mem::swap(&mut term, &mut partial);
            }
            let sum = term + partial;
            let remainder = partial - (sum - term);
            if remainder != 0.0 {
                partials[kept] = remainder;
                kept += 1;
            }
            term = sum;
        }
        if term != 0.0 {
            partials[kept] = term;
            kept += 1;
        }
        count = kept;
    }
    count != 0
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
        let first = geometry(&mut world, shape, &source(5.0), paint, None).unwrap();
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let before = meshes
            .get(&first.fill)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .clone();
        meshes.remove(first.stroke.id());
        assert!(geometry(&mut world, shape, &source(10.0), paint, None).is_err());
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
