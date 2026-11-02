use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type ProgressBarFailure = WidgetryFailure<ProgressBarError>;

#[derive(Debug)]
pub enum ProgressBarError {
    InvalidValue {
        value: f32,
    },
    InvalidHeight {
        height: f32,
    },
    InvalidRoot {
        root: Entity,
    },
    MissingState {
        root: Entity,
        component: &'static str,
    },
    VisualStructure {
        root: Entity,
        track: Entity,
        fill: Entity,
        clip: Entity,
        source: QueryEntityError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for ProgressBarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue { .. } => {
                f.write_str("ProgressBar value must be finite and within [0, 1]")
            }
            Self::InvalidHeight { .. } => {
                f.write_str("ProgressBar height must be finite and positive")
            }
            Self::InvalidRoot { .. } => f.write_str("ProgressBar 根不存在或类型不匹配"),
            Self::MissingState { component, .. } => {
                write!(f, "ProgressBar 缺失必需状态: {component}")
            }
            Self::VisualStructure { .. } => f.write_str("ProgressBar 样式内部结构失效"),
            Self::Core { .. } => f.write_str("ProgressBar 共享操作失败"),
        }
    }
}

impl Error for ProgressBarError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::VisualStructure { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: ProgressBarError, frame: ErrorFrame) -> ProgressBarFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    ProgressBarFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn missing(root: Entity, component: &'static str) -> ProgressBarFailure {
    failure(
        ProgressBarError::MissingState { root, component },
        ErrorFrame::new("progress_bar.access_state"),
    )
}

pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> ProgressBarFailure {
    error
        .map_kind(|source| ProgressBarError::Core { source })
        .with_context(frame)
}
