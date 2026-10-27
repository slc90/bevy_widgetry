use crate::{CanvasDocument, CanvasGeometry, CanvasPathCommand};
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

pub(crate) fn overlay(world: &World, shape: Entity) -> Option<Overlay> {
    let CanvasGeometry::Path(path) = world.get::<CanvasGeometry>(shape)? else {
        return None;
    };
    let transform = world.get::<Transform>(shape)?;
    let project = |p: Vec2| transform.transform_point(p.extend(0.0)).truncate();
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
    Some(output)
}

pub(crate) fn hit(world: &World, shape: Entity, point: Vec2, zoom: f32) -> Result<Option<Point>> {
    if world
        .get::<Transform>(shape)
        .is_none_or(|t| t.scale.x == 0.0 || t.scale.y == 0.0)
    {
        return Ok(None);
    }
    Ok(overlay(world, shape).and_then(|data| {
        data.handles
            .into_iter()
            .filter_map(|(h, p)| {
                let d = point.distance_squared(p) * zoom * zoom;
                (d <= 36.0).then_some((h, d))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(h, _)| h)
    }))
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
        polygon(&directions.map(|d| *center + d * (4.0 / zoom)));
    }
    crate::geometry::mesh(positions, indices)
}

#[derive(Clone, Copy)]
pub(crate) struct Edit {
    pub(crate) document: Entity,
    pub(crate) shape: Entity,
    point: Point,
    commands: usize,
    transform: Transform,
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
        let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) else {
            return Err(BevyError::error("Canvas node Path missing"));
        };
        let origin = path
            .commands
            .get(point.command)
            .and_then(|c| point.read(*c))
            .ok_or_else(|| BevyError::error("Canvas node point missing"))?;
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas node transform missing"))?;
        Ok(Self {
            document,
            shape,
            point,
            commands: path.commands.len(),
            transform,
            start: crate::scale::local_point(transform, at),
            origin,
        })
    }
    pub(crate) fn valid(self, world: &World) -> bool {
        world.get::<Transform>(self.shape) == Some(&self.transform)
            && matches!(world.get::<CanvasGeometry>(self.shape),Some(CanvasGeometry::Path(p)) if p.commands.len()==self.commands && p.commands.get(self.point.command).is_some_and(|c|self.point.read(*c).is_some()))
    }
    pub(crate) fn apply(self, world: &mut World, at: Vec2) -> Result {
        let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(self.shape) else {
            return Ok(());
        };
        let mut path = path.clone();
        let point = (self.origin.as_dvec2() + crate::scale::local_point(self.transform, at)
            - self.start)
            .as_vec2();
        if !point.is_finite() {
            return Ok(());
        }
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
            .ok_or_else(|| BevyError::error("Canvas node paint missing"))?;
        CanvasDocument::update_path(
            world,
            self.document,
            self.shape,
            path,
            self.transform,
            paint,
        )
    }
}
