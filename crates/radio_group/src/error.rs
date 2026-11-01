use bevy::{ecs::query::QueryEntityError, prelude::Entity};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use std::{error::Error, fmt, sync::Arc};

pub type RadioGroupFailure = WidgetryFailure<RadioGroupError>;

#[derive(Debug)]
pub enum RadioGroupError {
    InvalidTarget {
        entity: Entity,
        widget: &'static str,
    },
    EmptyGroup {
        group: Entity,
    },
    InvalidChild {
        group: Entity,
        option: Entity,
    },
    InvalidParent {
        option: Entity,
        parent: Option<Entity>,
    },
    IndexOutOfBounds {
        group: Entity,
        index: usize,
        len: usize,
    },
    MissingPart {
        group: Option<Entity>,
        option: Option<Entity>,
        entity: Entity,
        part: &'static str,
    },
    ManagedColors {
        entity: Entity,
        owner: Entity,
    },
    Query {
        group: Option<Entity>,
        option: Option<Entity>,
        entity: Entity,
        part: &'static str,
        source: QueryEntityError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for RadioGroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { widget, .. } => write!(f, "{widget} 目标不存在或类型不匹配"),
            Self::EmptyGroup { .. } => f.write_str("RadioGroup 至少需要一个 Option"),
            Self::InvalidChild { .. } => f.write_str("RadioGroup 直接子实体不是 RadioOption"),
            Self::InvalidParent { .. } => f.write_str("RadioOption 必须属于直接 RadioGroup 父级"),
            Self::IndexOutOfBounds { index, len, .. } => {
                write!(f, "RadioGroup 选择索引越界: {index}, 选项数 {len}")
            }
            Self::MissingPart { part, .. } => write!(f, "RadioGroup/Option 缺失必需部件 {part}"),
            Self::ManagedColors { .. } => f.write_str("RadioGroup/Option 颜色属于托管部件"),
            Self::Query { part, .. } => write!(f, "RadioGroup/Option 部件查询失败 {part}"),
            Self::Core { .. } => f.write_str("RadioGroup/Option 共享状态操作失败"),
        }
    }
}
impl Error for RadioGroupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::Query { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: RadioGroupError, frame: ErrorFrame) -> RadioGroupFailure {
    let mut frame = frame;
    match &kind {
        RadioGroupError::InvalidTarget { entity, widget } => {
            frame = frame.field("entity", entity).field("widget", widget)
        }
        RadioGroupError::EmptyGroup { group } => frame = frame.field("group", group),
        RadioGroupError::InvalidChild { group, option } => {
            frame = frame.field("group", group).field("option", option)
        }
        RadioGroupError::InvalidParent { option, parent } => {
            frame = frame
                .field("option", option)
                .field("parent", format_args!("{parent:?}"))
        }
        RadioGroupError::IndexOutOfBounds { group, index, len } => {
            frame = frame
                .field("group", group)
                .field("index", index)
                .field("len", len)
        }
        RadioGroupError::ManagedColors { entity, owner } => {
            frame = frame.field("entity", entity).field("owner", owner)
        }
        RadioGroupError::MissingPart {
            group,
            option,
            entity,
            part,
        }
        | RadioGroupError::Query {
            group,
            option,
            entity,
            part,
            ..
        } => {
            frame = frame.field("entity", entity).field("part", part);
            if let Some(group) = group {
                frame = frame.field("group", group);
            }
            if let Some(option) = option {
                frame = frame.field("option", option);
            }
        }
        _ => {}
    }
    RadioGroupFailure::new(kind, frame)
}

pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> RadioGroupFailure {
    error
        .map_kind(|source| RadioGroupError::Core { source })
        .with_context(frame)
}

#[derive(PartialEq, Eq)]
pub(crate) struct StyleFailureKey {
    group: Option<Entity>,
    entity: Entity,
    part: &'static str,
    query: bool,
}
pub(crate) struct StyleIssue {
    group: Option<Entity>,
    option: Option<Entity>,
    entity: Entity,
    part: &'static str,
    source: Option<QueryEntityError>,
}
impl StyleIssue {
    pub(crate) fn missing(
        group: Option<Entity>,
        option: Option<Entity>,
        entity: Entity,
        part: &'static str,
    ) -> Self {
        Self {
            group,
            option,
            entity,
            part,
            source: None,
        }
    }
    pub(crate) fn query(
        group: Option<Entity>,
        option: Option<Entity>,
        entity: Entity,
        part: &'static str,
        source: QueryEntityError,
    ) -> Self {
        Self {
            group,
            option,
            entity,
            part,
            source: Some(source),
        }
    }
    fn into_kind(self) -> RadioGroupError {
        let Self {
            group,
            option,
            entity,
            part,
            source,
        } = self;
        match source {
            Some(source) => RadioGroupError::Query {
                group,
                option,
                entity,
                part,
                source,
            },
            None => RadioGroupError::MissingPart {
                group,
                option,
                entity,
                part,
            },
        }
    }
}
pub(crate) fn observe_style(
    state: &mut FailureState<StyleFailureKey>,
    result: Result<(), StyleIssue>,
    frame: impl FnOnce() -> ErrorFrame,
) -> Result<(), WidgetryEcsError> {
    match result {
        Ok(()) => {
            state.recover(frame);
            Ok(())
        }
        Err(issue) => {
            let key = StyleFailureKey {
                group: issue.group,
                entity: issue.entity,
                part: issue.part,
                query: issue.source.is_some(),
            };
            Err(state.failure(key, || Arc::new(failure(issue.into_kind(), frame()))))
        }
    }
}
pub(crate) fn collect(batch: &mut DiagnosticBatch, result: Result<(), WidgetryEcsError>) {
    if let Err(error) = result {
        for diagnostic in error.shared_diagnostics() {
            batch.push_shared(diagnostic);
        }
    }
}
