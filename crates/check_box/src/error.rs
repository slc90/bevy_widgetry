use bevy::{
    ecs::{query::QueryEntityError, system::SystemParamValidationError},
    prelude::Entity,
};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type CheckBoxFailure = WidgetryFailure<CheckBoxError>;

#[derive(Debug)]
pub enum CheckBoxError {
    InvalidTarget {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        part: &'static str,
    },
    ManagedColors {
        root: Entity,
        owner: Entity,
    },
    Query {
        root: Entity,
        entity: Entity,
        part: &'static str,
        source: QueryEntityError,
    },
    SystemQuery {
        root: Entity,
        source: SystemParamValidationError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for CheckBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("CheckBox 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "CheckBox 缺失必需部件 {part}"),
            Self::ManagedColors { .. } => f.write_str("CheckBox 颜色属于托管部件"),
            Self::Query { part, .. } => write!(f, "CheckBox 部件查询失败 {part}"),
            Self::SystemQuery { .. } => f.write_str("CheckBox 颜色查询失败"),
            Self::Core { .. } => f.write_str("CheckBox 共享颜色操作失败"),
        }
    }
}

impl Error for CheckBoxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Query { source, .. } => Some(source),
            Self::SystemQuery { source, .. } => Some(source),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: CheckBoxError, operation: &'static str) -> CheckBoxFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        CheckBoxError::InvalidTarget { root } | CheckBoxError::SystemQuery { root, .. } => {
            context = context.field("root", root)
        }
        CheckBoxError::MissingPart { root, entity, part }
        | CheckBoxError::Query {
            root, entity, part, ..
        } => {
            context = context
                .field("root", root)
                .field("entity", entity)
                .field("part", part)
        }
        CheckBoxError::ManagedColors { root, owner } => {
            context = context.field("root", root).field("owner", owner)
        }
        _ => {}
    }
    CheckBoxFailure::new(kind, context)
}

pub(crate) fn core(
    error: CoreFailure,
    root: Entity,
    state: &'static str,
    interaction: &'static str,
    field: &'static str,
) -> CheckBoxFailure {
    error
        .map_kind(|source| CheckBoxError::Core { source })
        .with_context(
            ErrorFrame::new("check_box.validate_colors")
                .field("root", root)
                .field("state", state)
                .field("interaction", interaction)
                .field("field", field),
        )
}
