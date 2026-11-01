use bevy::prelude::*;
use bevy_widgetry_core::{
    diagnostics::{CoreError, CoreFailure},
    icon::{IconError, IconFailure},
};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type CollapsibleFailure = WidgetryFailure<CollapsibleError>;

#[derive(Debug)]
pub enum CollapsibleError {
    Structure {
        source: CollapsibleStructureError,
    },
    InvalidOwner {
        button: Entity,
        root: Option<Entity>,
        expected_button: Option<Entity>,
        parent: Option<Entity>,
    },
    Core {
        source: CoreError,
    },
    Icon {
        source: IconError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollapsibleStructureError {
    InvalidRoot {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        part: &'static str,
    },
    InvalidParent {
        root: Entity,
        entity: Entity,
        part: &'static str,
        expected: Entity,
        actual: Option<Entity>,
    },
    InvalidButton {
        root: Entity,
        button: Entity,
    },
}
impl fmt::Display for CollapsibleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure { .. } => f.write_str("Collapsible 组合结构失效"),
            Self::InvalidOwner { .. } => f.write_str("Collapsible 切换按钮归属无效"),
            Self::Core { .. } => f.write_str("Collapsible 共享状态操作失败"),
            Self::Icon { .. } => f.write_str("Collapsible 箭头更新失败"),
        }
    }
}
impl Error for CollapsibleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Structure { source } => Some(source),
            Self::Core { source } => Some(source),
            Self::Icon { source } => Some(source),
            Self::InvalidOwner { .. } => None,
        }
    }
}
impl fmt::Display for CollapsibleStructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot { .. } => f.write_str("Collapsible 根不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "Collapsible 缺失必需部件: {part}"),
            Self::InvalidParent { part, .. } => write!(f, "Collapsible 部件父级无效: {part}"),
            Self::InvalidButton { .. } => {
                f.write_str("Collapsible 内部切换部件不是 WidgetryButton")
            }
        }
    }
}
impl Error for CollapsibleStructureError {}

#[track_caller]
pub(crate) fn failure(kind: CollapsibleError, frame: ErrorFrame) -> CollapsibleFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    CollapsibleFailure::new(kind, frame)
}
#[track_caller]
pub(crate) fn structural(
    source: CollapsibleStructureError,
    frame: ErrorFrame,
) -> CollapsibleFailure {
    failure(CollapsibleError::Structure { source }, frame)
}
#[track_caller]
pub(crate) fn missing(root: Entity, entity: Entity, part: &'static str) -> CollapsibleFailure {
    structural(
        CollapsibleStructureError::MissingPart { root, entity, part },
        ErrorFrame::new("collapsible.access_part"),
    )
}
pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> CollapsibleFailure {
    error
        .map_kind(|source| CollapsibleError::Core { source })
        .with_context(frame)
}
pub(crate) fn icon(error: IconFailure, frame: ErrorFrame) -> CollapsibleFailure {
    error
        .map_kind(|source| CollapsibleError::Icon { source })
        .with_context(frame)
}
