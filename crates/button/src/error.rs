use bevy::{
    ecs::{query::QueryEntityError, system::SystemParamValidationError},
    prelude::Entity,
};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type ButtonFailure = WidgetryFailure<ButtonError>;

#[derive(Debug)]
pub enum ButtonError {
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
    MissingColorOutputs {
        root: Entity,
        missing: Vec<&'static str>,
        source: QueryEntityError,
    },
    ThemeQuery {
        source: SystemParamValidationError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for ButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("Button 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "Button 缺失内部部件 {part}"),
            Self::ManagedColors { .. } => f.write_str("Button 颜色属于托管部件"),
            Self::MissingColorOutputs { missing, .. } => {
                write!(f, "Button 缺失托管颜色输出 {missing:?}")
            }
            Self::ThemeQuery { .. } => f.write_str("Button Theme 颜色查询失败"),
            Self::Core { .. } => f.write_str("Button 共享状态操作失败"),
        }
    }
}

impl Error for ButtonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::ThemeQuery { source } => Some(source),
            Self::MissingColorOutputs { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: ButtonError, operation: &'static str) -> ButtonFailure {
    let mut frame = ErrorFrame::new(operation);
    match &kind {
        ButtonError::InvalidTarget { root } => frame = frame.field("root", root),
        ButtonError::MissingPart { root, part } => {
            frame = frame.field("root", root).field("part", part)
        }
        ButtonError::ManagedColors { root, owner } => {
            frame = frame.field("root", root).field("owner", owner)
        }
        ButtonError::MissingColorOutputs { root, missing, .. } => {
            frame = frame
                .field("root", root)
                .field("missing", format_args!("{missing:?}"))
        }
        _ => {}
    }
    ButtonFailure::new(kind, frame)
}

pub(crate) fn core(error: CoreFailure, root: Entity, operation: &'static str) -> ButtonFailure {
    error
        .map_kind(|source| ButtonError::Core { source })
        .with_context(ErrorFrame::new(operation).field("root", root))
}
