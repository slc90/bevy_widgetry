use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type TooltipFailure = WidgetryFailure<TooltipError>;

#[derive(Debug)]
pub enum TooltipError {
    MissingContent {
        anchor: Entity,
    },
    InvalidTarget {
        entity: Entity,
    },
    MissingColorState {
        entity: Entity,
    },
    ManagedColor {
        entity: Entity,
        owner: Entity,
    },
    AnchorColors {
        anchor: Entity,
        popup: Option<Entity>,
        source: Option<QueryEntityError>,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for TooltipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContent { .. } => f.write_str("Tooltip 构造必须提供 content"),
            Self::InvalidTarget { .. } => f.write_str("Tooltip 颜色目标不存在或类型不匹配"),
            Self::MissingColorState { .. } => f.write_str("Tooltip 缺失颜色配置"),
            Self::ManagedColor { .. } => f.write_str("Tooltip 托管部件的颜色不可直接修改"),
            Self::AnchorColors { .. } => f.write_str("Tooltip popup 缺失 anchor 颜色配置"),
            Self::Core { .. } => f.write_str("Tooltip 颜色配置无效"),
        }
    }
}

impl Error for TooltipError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::AnchorColors {
                source: Some(source),
                ..
            } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: TooltipError, frame: ErrorFrame) -> TooltipFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    TooltipFailure::new(kind, frame)
}

pub(crate) fn core(error: CoreFailure, entity: Entity, field: &'static str) -> TooltipFailure {
    error
        .map_kind(|source| TooltipError::Core { source })
        .with_context(
            ErrorFrame::new("tooltip.validate_colors")
                .field("anchor", entity)
                .field("field", field),
        )
}
