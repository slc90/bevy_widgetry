use crate::error::{CanvasResult, invalid};
use crate::{CanvasGeometry, CanvasIssue};
use bevy::{math::DVec2, prelude::*};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Reflect)]
pub enum CanvasAnchor {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Reflect)]
pub enum CanvasConnectorEnd {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum CanvasConnectorEndpoint {
    Free(Vec2),
    Attached {
        target: Entity,
        anchor: CanvasAnchor,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Reflect)]
pub(crate) struct Attachment {
    pub(crate) target: Entity,
    pub(crate) anchor: CanvasAnchor,
}

#[derive(Clone, Copy)]
pub(crate) struct AnchorSnap {
    pub(crate) attachment: Attachment,
    pub(crate) point: Vec2,
}

impl CanvasConnectorEnd {
    pub(crate) const ALL: [Self; 2] = [Self::Start, Self::End];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Start => 0,
            Self::End => 1,
        }
    }

    pub(crate) fn for_point(index: usize, waypoints: usize) -> Option<Self> {
        if index == 0 {
            Some(Self::Start)
        } else {
            (index == waypoints + 1).then_some(Self::End)
        }
    }
}

impl CanvasAnchor {
    pub(crate) const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    const fn direction(self) -> DVec2 {
        match self {
            Self::North => DVec2::NEG_Y,
            Self::East => DVec2::X,
            Self::South => DVec2::Y,
            Self::West => DVec2::NEG_X,
        }
    }
}

pub(crate) fn local_points(geometry: &CanvasGeometry) -> CanvasResult<[Option<DVec2>; 4]> {
    let points =
        |radii: Vec2| CanvasAnchor::ALL.map(|side| Some(side.direction() * radii.as_dvec2()));
    match geometry {
        CanvasGeometry::Rectangle(rectangle) => Ok(points(rectangle.size * 0.5)),
        CanvasGeometry::Ellipse(ellipse) => Ok(points(ellipse.radii)),
        CanvasGeometry::Path(path) => {
            let Some((points, center)) = crate::path::closed_outline(path)? else {
                return Ok([None; 4]);
            };
            Ok(CanvasAnchor::ALL.map(|side| ray_hit(&points, center, side.direction())))
        }
    }
}

fn ray_hit(points: &[DVec2], center: DVec2, direction: DVec2) -> Option<DVec2> {
    let cross = direction.perp();
    let distance = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter_map(|(start, end)| {
            let a = *start - center;
            let b = *end - center;
            let low = a.dot(cross);
            let high = b.dot(cross);
            if low == 0.0 && high == 0.0 {
                return Some(a.dot(direction).max(b.dot(direction)));
            }
            if (low > 0.0 && high > 0.0) || (low < 0.0 && high < 0.0) {
                return None;
            }
            Some(a.lerp(b, low / (low - high)).dot(direction))
        })
        .filter(|distance| *distance > 0.0)
        .max_by(f64::total_cmp)?;
    Some(center + direction * distance)
}

pub(crate) fn document_point(
    world: &World,
    shape: Entity,
    anchor: CanvasAnchor,
) -> CanvasResult<Option<Vec2>> {
    let Some(geometry) = world.get::<CanvasGeometry>(shape) else {
        return Ok(None);
    };
    let projection = crate::groups::document_transform(world, shape)?;
    projected_point(geometry, projection, anchor).map_err(|error| error.at("shape", shape))
}

pub(crate) fn projected_point(
    geometry: &CanvasGeometry,
    projection: bevy::math::Affine3A,
    anchor: CanvasAnchor,
) -> CanvasResult<Option<Vec2>> {
    let Some(point) = local_points(geometry)?[anchor as usize] else {
        return Ok(None);
    };
    let point = crate::snapping::project(projection, point).as_vec2();
    if !point.is_finite() {
        return Err(invalid(CanvasIssue::ConnectorAnchor));
    }
    Ok(Some(point))
}
