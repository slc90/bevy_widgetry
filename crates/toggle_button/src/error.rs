use bevy::{ecs::query::QueryEntityError, prelude::Entity};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type ToggleButtonFailure = WidgetryFailure<ToggleButtonError>;

#[derive(Debug)]
pub enum ToggleButtonError {
    InvalidTarget {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        part: &'static str,
    },
    ManagedColors {
        root: Entity,
        owner: Entity,
    },
    MissingStyle {
        root: Entity,
        missing: Vec<&'static str>,
        source: QueryEntityError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for ToggleButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("ToggleButton 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "ToggleButton 缺失内部部件 {part}"),
            Self::ManagedColors { .. } => f.write_str("ToggleButton 颜色属于托管部件"),
            Self::MissingStyle { missing, .. } => {
                write!(f, "ToggleButton 缺失内部样式组件 {missing:?}")
            }
            Self::Core { .. } => f.write_str("ToggleButton 共享颜色操作失败"),
        }
    }
}

impl Error for ToggleButtonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::MissingStyle { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: ToggleButtonError, operation: &'static str) -> ToggleButtonFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        ToggleButtonError::InvalidTarget { root } => context = context.field("root", root),
        ToggleButtonError::MissingPart { root, part } => {
            context = context.field("root", root).field("part", part)
        }
        ToggleButtonError::ManagedColors { root, owner } => {
            context = context.field("root", root).field("owner", owner)
        }
        ToggleButtonError::MissingStyle { root, missing, .. } => {
            context = context
                .field("root", root)
                .field("missing", format_args!("{missing:?}"))
        }
        _ => {}
    }
    ToggleButtonFailure::new(kind, context)
}

pub(crate) fn core(
    error: CoreFailure,
    root: Entity,
    state: &'static str,
    field: &'static str,
) -> ToggleButtonFailure {
    error
        .map_kind(|source| ToggleButtonError::Core { source })
        .with_context(
            ErrorFrame::new("toggle_button.validate_colors")
                .field("root", root)
                .field("state", state)
                .field("field", field),
        )
}
