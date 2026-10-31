use bevy::{
    ecs::{schedule::ScheduleError, world::error::EntityMutableFetchError},
    prelude::*,
};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type CoreFailure = WidgetryFailure<CoreError>;

#[derive(Debug)]
pub enum CoreError {
    InvalidColor {
        color: Color,
    },
    InvalidTarget {
        entity: Entity,
        missing: &'static str,
    },
    ManagedColor {
        entity: Entity,
        owner: Entity,
    },
    EntityAccess {
        entity: Entity,
        source: EntityMutableFetchError,
    },
    MissingResource {
        entity: Entity,
        resource: &'static str,
    },
    EmptySelection {
        root: Entity,
    },
    InvalidSelection {
        target: Entity,
    },
    MultipleChecked {
        root: Entity,
        options: Vec<Entity>,
    },
    MouseSchedule {
        source: Box<ScheduleError>,
    },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidColor { .. } => f.write_str("颜色分量必须有限且 alpha 位于 [0,1]"),
            Self::InvalidTarget { missing, .. } => write!(f, "目标不存在或缺失 {missing}"),
            Self::ManagedColor { .. } => f.write_str("颜色属于托管部件"),
            Self::EntityAccess { .. } => f.write_str("共享状态目标不存在或不可访问"),
            Self::MissingResource { resource, .. } => write!(f, "共享状态操作缺失 {resource}"),
            Self::EmptySelection { .. } => {
                f.write_str("Exclusive selection requires at least one option")
            }
            Self::MultipleChecked { .. } => {
                f.write_str("Exclusive selection has multiple initial Checked options")
            }
            Self::InvalidSelection { .. } => {
                f.write_str("Exclusive selection target is not an option")
            }
            Self::MouseSchedule { .. } => f.write_str("Native Mouse writer 替换失败"),
        }
    }
}

impl Error for CoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EntityAccess { source, .. } => Some(source),
            Self::MouseSchedule { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: CoreError, operation: &'static str) -> CoreFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        CoreError::InvalidColor { color } => {
            context = context.field("color", format_args!("{color:?}"))
        }
        CoreError::InvalidTarget { entity, missing } => {
            context = context.field("entity", entity).field("missing", missing)
        }
        CoreError::ManagedColor { entity, owner } => {
            context = context.field("entity", entity).field("owner", owner)
        }
        CoreError::EntityAccess { entity, .. } => context = context.field("entity", entity),
        CoreError::MissingResource { entity, resource } => {
            context = context.field("entity", entity).field("resource", resource)
        }
        CoreError::EmptySelection { root } => context = context.field("root", root),
        CoreError::InvalidSelection { target } => context = context.field("target", target),
        CoreError::MultipleChecked { root, options } => {
            context = context
                .field("root", root)
                .field("options", format_args!("{options:?}"))
        }
        CoreError::MouseSchedule { .. } => context = context.field("schedule", "First"),
    }
    CoreFailure::new(kind, context)
}
