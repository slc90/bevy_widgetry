use bevy::{ecs::system::SystemParamValidationError, prelude::Entity};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type LinkButtonFailure = WidgetryFailure<LinkButtonError>;

#[derive(Debug)]
pub enum LinkButtonError {
    InvalidTarget { root: Entity },
    MissingPart { root: Entity, part: &'static str },
    ThemeQuery { source: SystemParamValidationError },
    Core { source: CoreError },
}

impl fmt::Display for LinkButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("LinkButton 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "LinkButton 缺失内部部件 {part}"),
            Self::ThemeQuery { .. } => f.write_str("LinkButton Theme 颜色查询失败"),
            Self::Core { .. } => f.write_str("LinkButton 共享状态操作失败"),
        }
    }
}

impl Error for LinkButtonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ThemeQuery { source } => Some(source),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: LinkButtonError, operation: &'static str) -> LinkButtonFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        LinkButtonError::InvalidTarget { root } => context = context.field("root", root),
        LinkButtonError::MissingPart { root, part } => {
            context = context.field("root", root).field("part", part)
        }
        _ => {}
    }
    LinkButtonFailure::new(kind, context)
}

pub(crate) fn core(error: CoreFailure, root: Entity, field: &'static str) -> LinkButtonFailure {
    error
        .map_kind(|source| LinkButtonError::Core { source })
        .with_context(
            ErrorFrame::new("link_button.validate_colors")
                .field("root", root)
                .field("field", field),
        )
}
