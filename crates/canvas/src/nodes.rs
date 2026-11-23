const NODE_HIT_RADIUS_SQUARED: f32 = 36.0;
const NODE_HALF_SIZE: f32 = 4.0;

use crate::error::CanvasResult as Result;
use crate::{CanvasConnector, CanvasDocument, CanvasGeometry, CanvasPathCommand};
use bevy::{math::DVec2, prelude::*};

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Anchor,
    Control1,
    Control2,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Point {
    command: usize,
    role: Role,
}

impl Point {
    fn read(self, mut command: CanvasPathCommand) -> Option<Vec2> {
        self.coordinate(&mut command).copied()
    }
    fn coordinate(self, command: &mut CanvasPathCommand) -> Option<&mut Vec2> {
        use CanvasPathCommand::*;
        match (self.role, command) {
            (
                Role::Anchor,
                MoveTo(p)
                | LineTo(p)
                | QuadTo { to: p, .. }
                | CubicTo { to: p, .. }
                | ArcTo { to: p, .. },
            ) => Some(p),
            (
                Role::Control1,
                QuadTo { control, .. }
                | CubicTo {
                    control1: control, ..
                },
            ) => Some(control),
            (Role::Control2, CubicTo { control2, .. }) => Some(control2),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Overlay {
    handles: Vec<(Point, Vec2)>,
    links: Vec<[Vec2; 2]>,
}

pub(crate) fn overlay(world: &World, shape: Entity) -> Result<Option<Overlay>> {
    if let Some(connector) = world.get::<CanvasConnector>(shape) {
        let transform = crate::groups::document_transform(world, shape)?;
        return Ok(Some(Overlay {
            handles: connector
                .route()
                .vertices()
                .enumerate()
                .map(|(command, point)| {
                    (
                        Point {
                            command,
                            role: Role::Anchor,
                        },
                        transform.transform_point3(point.extend(0.0)).truncate(),
                    )
                })
                .collect(),
            links: Vec::new(),
        }));
    }
    let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) else {
        return Ok(None);
    };
    let transform = crate::groups::document_transform(world, shape)?;
    let project = |p: Vec2| transform.transform_point3(p.extend(0.0)).truncate();
    let mut output = Overlay {
        handles: Vec::new(),
        links: Vec::new(),
    };
    let mut from = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    for (command, c) in path.commands.iter().enumerate() {
        use CanvasPathCommand::*;
        let end = match *c {
            MoveTo(p) => {
                start = p;
                p
            }
            LineTo(p) | ArcTo { to: p, .. } => p,
            QuadTo { control, to } => {
                output.links.extend([
                    [project(from), project(control)],
                    [project(control), project(to)],
                ]);
                to
            }
            CubicTo {
                control1,
                control2,
                to,
            } => {
                output.links.extend([
                    [project(from), project(control1)],
                    [project(control2), project(to)],
                ]);
                to
            }
            Close => {
                from = start;
                continue;
            }
        };
        for role in [Role::Anchor, Role::Control1, Role::Control2] {
            let point = Point { command, role };
            if let Some(p) = point.read(*c) {
                output.handles.push((point, project(p)));
            }
        }
        from = end;
    }
    Ok(Some(output))
}

pub(crate) fn hit(world: &World, shape: Entity, point: Vec2, zoom: f32) -> Result<Option<Point>> {
    (|| -> Result<Option<Point>> {
        if world
            .get::<Transform>(shape)
            .is_none_or(|t| t.scale.x == 0.0 || t.scale.y == 0.0)
        {
            return Ok(None);
        }
        Ok(overlay(world, shape)?.and_then(|data| {
            data.handles
                .into_iter()
                .filter_map(|(h, p)| {
                    let d = point.distance_squared(p) * zoom * zoom;
                    (d <= NODE_HIT_RADIUS_SQUARED).then_some((h, d))
                })
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(h, _)| h)
        }))
    })()
    .map_err(|error| error.at("shape", shape))
}

pub(crate) fn mesh(data: &Overlay, zoom: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut polygon = |points: &[Vec2]| {
        let base = positions.len() as u32;
        positions.extend(points.iter().map(|p| p.extend(0.0).to_array()));
        for i in 1..points.len() - 1 {
            indices.extend([base, base + i as u32, base + i as u32 + 1]);
        }
    };
    for [a, b] in &data.links {
        let n = (*b - *a).perp().normalize_or_zero() * (0.5 / zoom);
        polygon(&[*a - n, *b - n, *b + n, *a + n]);
    }
    for (p, center) in &data.handles {
        let directions = if p.role == Role::Anchor {
            [
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::ONE,
                Vec2::new(-1.0, 1.0),
            ]
        } else {
            [Vec2::NEG_X, Vec2::NEG_Y, Vec2::X, Vec2::Y]
        };
        polygon(&directions.map(|d| *center + d * (NODE_HALF_SIZE / zoom)));
    }
    crate::geometry::mesh(positions, indices)
}

#[derive(Clone, Copy)]
enum Source {
    Path(usize),
    Connector(usize),
}

#[derive(Clone, Copy)]
pub(crate) struct Edit {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    point: Point,
    source: Source,
    projection: bevy::math::Affine3A,
    start: DVec2,
    origin: Vec2,
}
impl Edit {
    pub(crate) fn begin(
        world: &World,
        document: Entity,
        shape: Entity,
        point: Point,
        at: Vec2,
    ) -> Result<Self> {
        (|| -> Result<Self> {
            let (origin, source) = if let Some(connector) = world.get::<CanvasConnector>(shape) {
                let route = connector.route();
                (
                    route.point(point.command),
                    Source::Connector(route.waypoints.len()),
                )
            } else if let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) {
                (
                    path.commands
                        .get(point.command)
                        .and_then(|c| point.read(*c)),
                    Source::Path(path.commands.len()),
                )
            } else {
                return Err(crate::error::missing(crate::CanvasPart::NodePath));
            };
            let origin =
                origin.ok_or_else(|| crate::error::missing(crate::CanvasPart::NodePoint))?;
            let projection = crate::groups::document_transform(world, shape)?;
            Ok(Self {
                document,
                shape,
                point,
                source,
                projection,
                start: crate::scale::local_point(projection, at),
                origin,
            })
        })()
        .map_err(|error| error.at("document", document).at("shape", shape))
    }
    pub(crate) fn valid(self, world: &World) -> bool {
        crate::groups::document_transform(world, self.shape)
            .is_ok_and(|transform| transform == self.projection)
            && match self.source {
                Source::Path(count) => matches!(
                    world.get::<CanvasGeometry>(self.shape), Some(CanvasGeometry::Path(path))
                        if path.commands.len() == count
                        && path.commands.get(self.point.command).is_some_and(|c| self.point.read(*c).is_some())
                ),
                Source::Connector(waypoint_count) => world
                    .get::<CanvasConnector>(self.shape)
                    .is_some_and(|connector| connector.route().waypoints.len() == waypoint_count),
            }
    }
    pub(crate) fn apply(self, world: &mut World, view: Entity, at: Vec2) -> Result {
        let position =
            self.origin.as_dvec2() + crate::scale::local_point(self.projection, at) - self.start;
        let connector = matches!(self.source, Source::Connector(_));
        let target = if connector {
            crate::grid::SnapTarget::ConnectorPoint
        } else {
            crate::grid::SnapTarget::PathNode
        };
        let correction = crate::grid::local_delta(world, view, target, self.projection, position);
        let point = (position + correction).as_vec2();
        if !point.is_finite() {
            return Ok(());
        }
        if connector {
            return CanvasDocument::move_connector_point(
                world,
                self.document,
                self.shape,
                self.point.command,
                point,
            );
        }
        let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(self.shape) else {
            return Ok(());
        };
        let mut path = path.clone();
        let Some(p) = path
            .commands
            .get_mut(self.point.command)
            .and_then(|c| self.point.coordinate(c))
        else {
            return Ok(());
        };
        *p = point;
        let paint = *world
            .get::<crate::CanvasPaint>(self.shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::NodePaint))?;
        let transform = *world
            .get::<Transform>(self.shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::NodeTransform))?;
        CanvasDocument::_update_path(world, self.document, self.shape, path, transform, paint)
    }
}
