use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type TextFieldFailure = WidgetryFailure<TextFieldError>;

#[derive(Debug)]
pub enum TextFieldError {
    EarlyEditFilter {
        root: Entity,
        editor: Entity,
    },
    InvalidRoot {
        root: Entity,
        expected: &'static str,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        component: &'static str,
    },
    MissingResource {
        root: Entity,
        resource: &'static str,
    },
    InvalidAutoSize {
        multiline: bool,
        config: crate::AutoSize,
    },
    HeightConflict {
        root: Entity,
        height: Val,
    },
    EditorOutsideRoot {
        root: Entity,
        editor: Entity,
    },
    InvalidEditorBinding {
        root: Entity,
        editor: Entity,
        bound: Entity,
    },
    InvalidEditorOwner {
        root: Entity,
        editor: Entity,
        actual: Entity,
    },
    InvalidSlots {
        root: Entity,
        frame: Entity,
        child: Entity,
        prefix: bool,
        suffix: bool,
    },
    InvalidEditorMapping {
        root: Entity,
        frame: Entity,
        editor: Entity,
    },
    OwnedStyle {
        root: Entity,
    },
    FilterConflict {
        root: Entity,
        editor: Entity,
        max: usize,
    },
    Query {
        root: Entity,
        entity: Entity,
        component: &'static str,
        source: QueryEntityError,
    },
    Clipboard {
        root: Entity,
        editor: Entity,
        source: bevy::clipboard::ClipboardError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for TextFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EarlyEditFilter { .. } => {
                f.write_str("提前编辑不支持不透明的 EditableTextFilter")
            }
            Self::InvalidRoot { .. } => f.write_str("TextField 根不存在或类型不匹配"),
            Self::MissingPart { component, .. } => write!(f, "TextField 缺失必需部件: {component}"),
            Self::MissingResource { resource, .. } => {
                write!(f, "TextField 缺失必需资源: {resource}")
            }
            Self::InvalidAutoSize { .. } => {
                f.write_str("TextField AutoSize 要求 multiline 且 1 <= min_rows <= max_rows")
            }
            Self::HeightConflict { .. } => {
                f.write_str("TextField AutoSize 要求根 Node.height 为 Auto")
            }
            Self::InvalidEditorOwner { .. } => f.write_str("TextField 编辑器身份不匹配"),
            Self::InvalidEditorBinding { .. } => f.write_str("TextField 编辑器未绑定到声明的根"),
            Self::EditorOutsideRoot { .. } => f.write_str("TextField 编辑器不属于根的实体树"),
            Self::InvalidSlots { .. } => {
                f.write_str("TextField 内容必须是唯一的 Prefix 或 Suffix 容器")
            }
            Self::InvalidEditorMapping { .. } => f.write_str("TextField 内部编辑实体映射失效"),
            Self::OwnedStyle { .. } => f.write_str("TextField 颜色属于托管部件"),
            Self::FilterConflict { .. } => f.write_str("max_length 不能与 EditableTextFilter 组合"),
            Self::Query { component, .. } => write!(f, "TextField 部件查询失败: {component}"),
            Self::Clipboard { .. } => f.write_str("TextField 剪贴板读取失败"),
            Self::Core { .. } => f.write_str("TextField 共享操作失败"),
        }
    }
}
impl Error for TextFieldError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::Query { source, .. } => Some(source),
            Self::Clipboard { source, .. } => Some(source),
            _ => None,
        }
    }
}
#[track_caller]
pub(crate) fn failure(kind: TextFieldError, frame: ErrorFrame) -> TextFieldFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    TextFieldFailure::new(kind, frame)
}
#[track_caller]
pub(crate) fn missing(
    root: Entity,
    entity: Entity,
    component: &'static str,
    operation: &'static str,
) -> TextFieldFailure {
    failure(
        TextFieldError::MissingPart {
            root,
            entity,
            component,
        },
        ErrorFrame::new(operation),
    )
}
pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> TextFieldFailure {
    error
        .map_kind(|source| TextFieldError::Core { source })
        .with_context(frame)
}
#[track_caller]
pub(crate) fn query(
    root: Entity,
    entity: Entity,
    component: &'static str,
    source: QueryEntityError,
    operation: &'static str,
) -> TextFieldFailure {
    failure(
        TextFieldError::Query {
            root,
            entity,
            component,
            source,
        },
        ErrorFrame::new(operation),
    )
}
