use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasPath, CanvasPathCommand};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasPathSegment {
    Line,
    Quadratic { control: Vec2 },
    Cubic { control1: Vec2, control2: Vec2 },
}

impl CanvasDocument {
    pub fn insert_path_anchor(
        world: &mut World,
        document: Entity,
        shape: Entity,
        before: usize,
        point: Vec2,
    ) -> Result<bool> {
        edit(world, document, shape, before, |commands, index| {
            if !point.is_finite() || !matches!(commands[index], CanvasPathCommand::LineTo(_)) {
                return Err(BevyError::error(
                    "Canvas anchor insertion requires a LineTo and finite point",
                ));
            }
            commands.insert(index, CanvasPathCommand::LineTo(point));
            Ok(true)
        })
    }
    pub fn remove_path_anchor(
        world: &mut World,
        document: Entity,
        shape: Entity,
        command: usize,
    ) -> Result<bool> {
        edit(world, document, shape, command, |commands, index| {
            use CanvasPathCommand::*;
            if matches!(commands[index], Close) {
                return Err(BevyError::error("Canvas Close has no independent anchor"));
            }
            if matches!(commands[index], MoveTo(_)) {
                let end = commands
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .find_map(|(i, c)| matches!(c, MoveTo(_)).then_some(i))
                    .unwrap_or(commands.len());
                if let Some(point) = commands.get(index + 1).copied().and_then(endpoint) {
                    commands[index + 1] = MoveTo(point);
                    commands.remove(index);
                } else {
                    commands.drain(index..end);
                }
            } else {
                commands.remove(index);
            }
            Ok(true)
        })
    }
    pub fn set_path_segment(
        world: &mut World,
        document: Entity,
        shape: Entity,
        command: usize,
        segment: CanvasPathSegment,
    ) -> Result<bool> {
        edit(world, document, shape, command, |commands, index| {
            use CanvasPathCommand::*;
            if matches!(commands[index], MoveTo(_) | Close) {
                return Err(BevyError::error(
                    "Canvas segment conversion requires a drawing command",
                ));
            }
            let to = endpoint(commands[index])
                .ok_or_else(|| BevyError::error("Canvas segment endpoint missing"))?;
            let replacement = match segment {
                CanvasPathSegment::Line => LineTo(to),
                CanvasPathSegment::Quadratic { control } => QuadTo { control, to },
                CanvasPathSegment::Cubic { control1, control2 } => CubicTo {
                    control1,
                    control2,
                    to,
                },
            };
            if replacement == commands[index] {
                return Ok(false);
            }
            commands[index] = replacement;
            Ok(true)
        })
    }
    pub fn set_path_subpath_closed(
        world: &mut World,
        document: Entity,
        shape: Entity,
        command: usize,
        closed: bool,
    ) -> Result<bool> {
        edit(world, document, shape, command, |commands, index| {
            use CanvasPathCommand::*;
            let start = commands[..=index]
                .iter()
                .rposition(|c| matches!(c, MoveTo(_)))
                .ok_or_else(|| BevyError::error("Canvas subpath MoveTo missing"))?;
            let end = commands
                .iter()
                .enumerate()
                .skip(start + 1)
                .find_map(|(i, c)| matches!(c, MoveTo(_)).then_some(i))
                .unwrap_or(commands.len());
            let existing = commands[start..end].iter().any(|c| matches!(c, Close));
            if existing == closed {
                return Ok(false);
            }
            if closed {
                commands.insert(end, Close);
            } else {
                let opened = commands[start..end]
                    .iter()
                    .copied()
                    .filter(|c| !matches!(c, Close))
                    .collect::<Vec<_>>();
                commands.splice(start..end, opened);
            }
            Ok(true)
        })
    }
}

fn endpoint(command: CanvasPathCommand) -> Option<Vec2> {
    use CanvasPathCommand::*;
    match command {
        MoveTo(p)
        | LineTo(p)
        | QuadTo { to: p, .. }
        | CubicTo { to: p, .. }
        | ArcTo { to: p, .. } => Some(p),
        Close => None,
    }
}

fn explicit_subpaths(path: &CanvasPath) -> (Vec<CanvasPathCommand>, Vec<usize>) {
    use CanvasPathCommand::*;
    let mut commands = Vec::new();
    let mut mapping = Vec::with_capacity(path.commands.len());
    let mut start = Vec2::ZERO;
    let mut closed = false;
    for c in &path.commands {
        match *c {
            MoveTo(p) => {
                start = p;
                closed = false;
            }
            Close => closed = true,
            _ => {
                if closed {
                    commands.push(MoveTo(start));
                    closed = false;
                }
            }
        }
        mapping.push(commands.len());
        commands.push(*c);
    }
    (commands, mapping)
}

fn edit(
    world: &mut World,
    document: Entity,
    shape: Entity,
    command: usize,
    operation: impl FnOnce(&mut Vec<CanvasPathCommand>, usize) -> Result<bool>,
) -> Result<bool> {
    let candidate = (|| -> Result<Option<(CanvasPath, Transform, CanvasPaint)>> {
        crate::model::validate_shape(world, document, shape)?;
        let Some(CanvasGeometry::Path(original)) = world.get::<CanvasGeometry>(shape) else {
            return Err(BevyError::error("Canvas topology target must be a Path"));
        };
        let transform = *world
            .get::<Transform>(shape)
            .ok_or_else(|| BevyError::error("Canvas topology transform missing"))?;
        let paint = *world
            .get::<CanvasPaint>(shape)
            .ok_or_else(|| BevyError::error("Canvas topology paint missing"))?;
        original.validate(&transform, &paint)?;
        let (mut commands, mapping) = explicit_subpaths(original);
        let index = *mapping
            .get(command)
            .ok_or_else(|| BevyError::error("Canvas path command index out of range"))?;
        if !operation(&mut commands, index)? {
            return Ok(None);
        }
        let path = CanvasPath { commands };
        path.validate(&transform, &paint)?;
        Ok(Some((path, transform, paint)))
    })()
    .inspect_err(
        |error| widgetry_error!(?document,?shape,command,%error,"Canvas Path 拓扑更新被拒绝"),
    )?;
    let Some((path, transform, paint)) = candidate else {
        return Ok(false);
    };
    CanvasDocument::update_path(world, document, shape, path, transform, paint)?;
    crate::input::cancel_node_edits(world, shape);
    Ok(true)
}
