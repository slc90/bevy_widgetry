const SIZE_HANDLE_INSET: f32 = 14.0;
const SIZE_HANDLE_MAX_INSET_FRACTION: f32 = 0.35;
const CORNER_HANDLE_INSET: f32 = 26.0;
const CORNER_HANDLE_MAX_INSET_FRACTION: f32 = 0.65;
const HANDLE_HIT_RADIUS_SQUARED: f32 = 25.0;
const ROUND_HANDLE_SEGMENTS: u32 = 12;
const SIZE_HANDLE_SEGMENTS: u32 = 4;
const HANDLE_HALF_SIZE: f32 = 4.0;

use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasEllipse, CanvasGeometry, CanvasPaint, CanvasRectangle};
use bevy::{math::DVec2, prelude::*};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ParameterHandle {
    Size(crate::scale::ScaleHandle),
    CornerRadius,
}

#[derive(Clone, Copy)]
enum Geometry {
    Rectangle(CanvasRectangle),
    Ellipse(CanvasEllipse),
}

impl Geometry {
    fn get(world: &World, shape: Entity) -> Option<Self> {
        match world.get::<CanvasGeometry>(shape)? {
            CanvasGeometry::Rectangle(r) => Some(Self::Rectangle(*r)),
            CanvasGeometry::Ellipse(e) => Some(Self::Ellipse(*e)),
            CanvasGeometry::Path(_) => None,
        }
    }
    fn size(self) -> Vec2 {
        match self {
            Self::Rectangle(r) => r.size,
            Self::Ellipse(e) => e.radii * 2.0,
        }
    }
}

pub(crate) fn controls(
    world: &World,
    shape: Entity,
    zoom: f32,
) -> Result<Vec<(ParameterHandle, Vec2)>> {
    (|| -> Result<Vec<(ParameterHandle, Vec2)>> {
        let Some(geometry) = Geometry::get(world, shape) else {
            return Ok(Vec::new());
        };
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::ParameterTransform))?;
        let half = geometry.size() * 0.5;
        let center = transform.translation.truncate();
        let project = |p: Vec2| transform.transform_point(p.extend(0.0)).truncate();
        let corners = [
            -half,
            Vec2::new(half.x, -half.y),
            half,
            Vec2::new(-half.x, half.y),
        ]
        .map(project);
        let inset = |p: Vec2, d: f32, limit: f32| {
            let delta = center - p;
            p + delta.normalize_or_zero() * (d / zoom).min(delta.length() * limit)
        };
        let mut handles = crate::scale::controls(corners)
            .map(|(h, p)| {
                (
                    ParameterHandle::Size(h),
                    inset(p, SIZE_HANDLE_INSET, SIZE_HANDLE_MAX_INSET_FRACTION),
                )
            })
            .to_vec();
        if let Geometry::Rectangle(rectangle) = geometry {
            handles.push((
                ParameterHandle::CornerRadius,
                inset(
                    project(-half + Vec2::splat(rectangle.corner_radius)),
                    CORNER_HANDLE_INSET,
                    CORNER_HANDLE_MAX_INSET_FRACTION,
                ),
            ));
        }
        Ok(handles)
    })()
    .map_err(|error| error.at("shape", shape))
}

pub(crate) fn hit(
    world: &World,
    shape: Entity,
    point: Vec2,
    zoom: f32,
) -> Result<Option<(ParameterHandle, f32)>> {
    (|| -> Result<Option<(ParameterHandle, f32)>> {
        Ok(controls(world, shape, zoom)?
            .into_iter()
            .filter_map(|(h, p)| {
                let distance = (point - p).length_squared() * zoom * zoom;
                (distance <= HANDLE_HIT_RADIUS_SQUARED).then_some((h, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b)))
    })()
    .map_err(|error| error.at("shape", shape))
}

pub(crate) fn decorate(mesh: Mesh, handles: &[(ParameterHandle, Vec2)], zoom: f32) -> Result<Mesh> {
    let mut positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|v| v.as_float3())
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::OutlinePositions))?
        .to_vec();
    let mut indices = mesh
        .indices()
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::OutlineIndices))?
        .iter()
        .map(|i| i as u32)
        .collect::<Vec<_>>();
    for (handle, center) in handles {
        let base = positions.len() as u32;
        let count = if *handle == ParameterHandle::CornerRadius {
            ROUND_HANDLE_SEGMENTS
        } else {
            SIZE_HANDLE_SEGMENTS
        };
        positions.push(center.extend(0.0).to_array());
        for i in 0..count {
            let angle = i as f32 * std::f32::consts::TAU / count as f32;
            positions.push(
                (*center + Vec2::from_angle(angle) * (HANDLE_HALF_SIZE / zoom))
                    .extend(0.0)
                    .to_array(),
            );
            indices.extend([base, base + 1 + i, base + 1 + (i + 1) % count]);
        }
    }
    Ok(crate::geometry::mesh(positions, indices))
}

#[derive(Clone, Copy)]
pub(crate) struct Edit {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    geometry: Geometry,
    transform: Transform,
    start: DVec2,
    handle: ParameterHandle,
}

impl Edit {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shape: Entity,
        point: Vec2,
        handle: ParameterHandle,
    ) -> Result<Self> {
        (|| -> Result<Self> {
            let geometry = Geometry::get(world, shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::ParameterGeometry))?;
            let transform = *world
                .get::<Transform>(shape)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::ParameterTransform))?;
            Ok(Self {
                document,
                shape,
                geometry,
                transform,
                start: crate::scale::local_point(transform, point),
                handle,
            })
        })()
        .map_err(|error| error.at("document", document).at("shape", shape))
    }
    pub(crate) fn apply(self, world: &mut World, point: Vec2) -> Result {
        let delta = crate::scale::local_point(self.transform, point) - self.start;
        let mut geometry = self.geometry;
        let mut transform = self.transform;
        match self.handle {
            ParameterHandle::Size(handle) => {
                let axes = handle.axes();
                let old = geometry.size().as_dvec2();
                let mut size = old;
                for i in 0..2 {
                    if axes[i] != 0.0 {
                        size[i] = (old[i] + axes[i] * delta[i]).max(1.0);
                    }
                }
                if !size.is_finite() || !size.as_vec2().is_finite() {
                    return Ok(());
                }
                let offset = (size - old) * axes * 0.5;
                let x = (transform.rotation * Vec3::X * transform.scale.x)
                    .truncate()
                    .as_dvec2();
                let y = (transform.rotation * Vec3::Y * transform.scale.y)
                    .truncate()
                    .as_dvec2();
                let center =
                    transform.translation.truncate().as_dvec2() + x * offset.x + y * offset.y;
                if !center.as_vec2().is_finite() {
                    return Ok(());
                }
                transform.translation.x = center.x as f32;
                transform.translation.y = center.y as f32;
                match &mut geometry {
                    Geometry::Rectangle(r) => {
                        r.size = size.as_vec2();
                        r.corner_radius = r.corner_radius.min(r.size.min_element() * 0.5);
                    }
                    Geometry::Ellipse(e) => e.radii = size.as_vec2() * 0.5,
                }
            }
            ParameterHandle::CornerRadius => {
                if let Geometry::Rectangle(r) = &mut geometry {
                    r.corner_radius = (f64::from(r.corner_radius) + (delta.x + delta.y) * 0.5)
                        .clamp(0.0, f64::from(r.size.min_element()) * 0.5)
                        as f32;
                }
            }
        }
        let paint = *world
            .get::<CanvasPaint>(self.shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::ParameterPaint))?;
        match geometry {
            Geometry::Rectangle(r) => CanvasDocument::_update_rectangle(
                world,
                self.document,
                self.shape,
                r,
                transform,
                paint,
            ),
            Geometry::Ellipse(e) => CanvasDocument::_update_ellipse(
                world,
                self.document,
                self.shape,
                e,
                transform,
                paint,
            ),
        }
    }
}
