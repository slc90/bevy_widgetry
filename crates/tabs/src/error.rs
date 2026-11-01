use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type TabsFailure = WidgetryFailure<TabsError>;

#[derive(Debug)]
pub enum TabsError {
    InvalidTarget {
        entity: Entity,
        expected: &'static str,
    },
    Structure {
        source: TabsStructureError,
    },
    InvalidIndex {
        root: Entity,
        index: usize,
        count: usize,
        initial: bool,
    },
    IncompleteTab {
        header: Entity,
        has_header: bool,
        has_content: bool,
    },
    FixedPairs {
        root: Entity,
        expected: Vec<[Entity; 2]>,
        actual: Vec<[Entity; 2]>,
    },
    Core {
        source: CoreError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabsStructureError {
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
    Empty {
        root: Entity,
        strip: Entity,
    },
    UnexpectedChild {
        root: Entity,
        parent: Entity,
        entity: Entity,
    },
    PageCount {
        root: Entity,
        container: Entity,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for TabsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { expected, .. } => {
                write!(f, "Tabs 目标不存在、类型不匹配或属于托管部件: {expected}")
            }
            Self::Structure { .. } => f.write_str("Tabs 配对结构失效"),
            Self::InvalidIndex { .. } => f.write_str("Tabs 选择索引越界"),
            Self::IncompleteTab { .. } => f.write_str("Tab 必须同时提供 Header 与 Content Scene"),
            Self::FixedPairs { .. } => f.write_str("Tabs 构造后的固定配对发生变化"),
            Self::Core { .. } => f.write_str("Tabs 共享状态操作失败"),
        }
    }
}
impl Error for TabsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Structure { source } => Some(source),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}
impl fmt::Display for TabsStructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPart { part, .. } => write!(f, "Tabs 缺失必需部件: {part}"),
            Self::InvalidParent { part, .. } => write!(f, "Tabs 部件父级无效: {part}"),
            Self::Empty { .. } => f.write_str("Tabs 至少需要一个 Tab"),
            Self::UnexpectedChild { .. } => f.write_str("Tabs 标签栏包含非配对实体"),
            Self::PageCount { .. } => f.write_str("Tabs 页面容器数量与固定配对不符"),
        }
    }
}
impl Error for TabsStructureError {}

#[track_caller]
pub(crate) fn failure(kind: TabsError, frame: ErrorFrame) -> TabsFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    TabsFailure::new(kind, frame)
}
#[track_caller]
pub(crate) fn structural(source: TabsStructureError, operation: &'static str) -> TabsFailure {
    failure(TabsError::Structure { source }, ErrorFrame::new(operation))
}
#[track_caller]
pub(crate) fn missing(root: Entity, entity: Entity, part: &'static str) -> TabsFailure {
    structural(
        TabsStructureError::MissingPart { root, entity, part },
        "tabs.access_part",
    )
}
pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> TabsFailure {
    error
        .map_kind(|source| TabsError::Core { source })
        .with_context(frame)
}
