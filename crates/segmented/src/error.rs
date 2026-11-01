use bevy::prelude::Entity;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{DiagnosticBatch, WidgetryEcsError},
};
use std::{error::Error, fmt};

pub type SegmentedFailure = WidgetryFailure<SegmentedError>;

#[derive(Debug)]
pub enum SegmentedError {
    InitialCheckedConflict {
        group: Entity,
        options: Vec<Entity>,
    },
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
    Core {
        source: CoreError,
    },
}

impl fmt::Display for SegmentedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialCheckedConflict { .. } => f.write_str("Segmented 初始 Checked 配置冲突"),
            Self::InvalidTarget { widget, .. } => write!(f, "{widget} 目标不存在或类型不匹配"),
            Self::EmptyGroup { .. } => f.write_str("Segmented 至少需要一个 Option"),
            Self::InvalidChild { .. } => f.write_str("Segmented 直接子实体不是 SegmentedOption"),
            Self::InvalidParent { .. } => {
                f.write_str("SegmentedOption 必须属于直接 Segmented 父级")
            }
            Self::IndexOutOfBounds { index, len, .. } => {
                write!(f, "Segmented 选择索引越界: {index}, 选项数 {len}")
            }
            Self::MissingPart { part, .. } => write!(f, "Segmented/Option 缺失必需部件 {part}"),
            Self::ManagedColors { .. } => f.write_str("Segmented/Option 颜色属于托管部件"),
            Self::Core { .. } => f.write_str("Segmented/Option 共享状态操作失败"),
        }
    }
}
impl Error for SegmentedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: SegmentedError, frame: ErrorFrame) -> SegmentedFailure {
    let mut frame = frame;
    match &kind {
        SegmentedError::InitialCheckedConflict { group, options } => {
            frame = frame
                .field("group", group)
                .field("options", format_args!("{options:?}"));
        }
        SegmentedError::InvalidTarget { entity, widget } => {
            frame = frame.field("entity", entity).field("widget", widget)
        }
        SegmentedError::EmptyGroup { group } => frame = frame.field("group", group),
        SegmentedError::InvalidChild { group, option } => {
            frame = frame.field("group", group).field("option", option)
        }
        SegmentedError::InvalidParent { option, parent } => {
            frame = frame
                .field("option", option)
                .field("parent", format_args!("{parent:?}"))
        }
        SegmentedError::IndexOutOfBounds { group, index, len } => {
            frame = frame
                .field("group", group)
                .field("index", index)
                .field("len", len)
        }
        SegmentedError::ManagedColors { entity, owner } => {
            frame = frame.field("entity", entity).field("owner", owner)
        }
        SegmentedError::MissingPart {
            group,
            option,
            entity,
            part,
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
    SegmentedFailure::new(kind, frame)
}

pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> SegmentedFailure {
    error
        .map_kind(|source| SegmentedError::Core { source })
        .with_context(frame)
}

pub(crate) fn collect(batch: &mut DiagnosticBatch, result: Result<(), WidgetryEcsError>) {
    if let Err(error) = result {
        for diagnostic in error.shared_diagnostics() {
            batch.push_shared(diagnostic);
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    use std::sync::Arc;
    #[test]
    fn core_wrapping_keeps_original_origin_source_and_boundary_severity() {
        let logs = LogCapture::default();
        logs.run(|| {
            let original = bevy_widgetry_core::color::validate_color(
                bevy::prelude::Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0),
            )
            .unwrap_err();
            let origin = original.origin().clone();
            let wrapped = core(original, ErrorFrame::new("segmented.validate_colors"));
            assert!(Arc::ptr_eq(&origin, wrapped.origin()));
            assert!(matches!(
                wrapped
                    .source()
                    .unwrap()
                    .source()
                    .unwrap()
                    .downcast_ref::<CoreError>(),
                Some(CoreError::InvalidColor { .. })
            ));
            let boundary = wrapped.into_bevy_error();
            assert_eq!(boundary.severity(), bevy::ecs::error::Severity::Error);
            let diagnostic = boundary
                .downcast_ref::<WidgetryEcsError>()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap();
            assert!(Arc::ptr_eq(&origin, diagnostic.origin()));
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        });
    }
}
