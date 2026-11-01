use crate::{ScrollAxis, ScrollbarPolicy, ScrollbarVisibility};
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use std::{error::Error, fmt, sync::Arc};

pub type ScrollAreaFailure = WidgetryFailure<ScrollAreaError>;

#[derive(Debug)]
pub struct ScrollAreaContext {
    pub root: Entity,
    pub viewport: Option<Entity>,
    pub content: Option<Entity>,
    pub axis: Option<ScrollAxis>,
    pub policy: Option<ScrollbarPolicy>,
    pub root_size: Option<Vec2>,
    pub viewport_size: Option<Vec2>,
    pub content_size: Option<Vec2>,
}

#[derive(Debug)]
pub enum ScrollAreaError {
    InvalidTarget {
        root: Entity,
    },
    ManagedColors {
        root: Entity,
        owner: Entity,
    },
    InvalidThickness {
        root: Entity,
        axis: ScrollAxis,
        visibility: ScrollbarVisibility,
        thickness: f32,
    },
    MissingPart {
        context: Box<ScrollAreaContext>,
        entity: Entity,
        part: &'static str,
    },
    InvalidScrollbar {
        context: Box<ScrollAreaContext>,
        entity: Entity,
        target: Entity,
        axis: ScrollAxis,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for ScrollAreaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("ScrollArea 目标不存在或类型不匹配"),
            Self::ManagedColors { .. } => f.write_str("ScrollArea 颜色属于组合控件托管"),
            Self::InvalidThickness { .. } => f.write_str("ScrollArea 滚动条宽度必须有限且非负"),
            Self::MissingPart { part, .. } => write!(f, "ScrollArea 缺失必需部件 {part}"),
            Self::InvalidScrollbar { .. } => f.write_str("ScrollArea 滚动条目标或方向与归属不匹配"),
            Self::Core { .. } => f.write_str("ScrollArea 共享状态操作失败"),
        }
    }
}
impl Error for ScrollAreaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: ScrollAreaError, frame: ErrorFrame) -> ScrollAreaFailure {
    let frame = match &kind {
        ScrollAreaError::InvalidTarget { root } => frame.field("root", root),
        ScrollAreaError::ManagedColors { root, owner } => {
            frame.field("root", root).field("owner", owner)
        }
        ScrollAreaError::InvalidThickness {
            root,
            axis,
            visibility,
            thickness,
        } => frame
            .field("root", root)
            .field("axis", format_args!("{axis:?}"))
            .field("visibility", format_args!("{visibility:?}"))
            .field("thickness", thickness),
        ScrollAreaError::MissingPart {
            context,
            entity,
            part,
        } => frame
            .field("context", format_args!("{context:?}"))
            .field("entity", entity)
            .field("part", part),
        ScrollAreaError::InvalidScrollbar {
            context,
            entity,
            target,
            axis,
        } => frame
            .field("context", format_args!("{context:?}"))
            .field("entity", entity)
            .field("target", target)
            .field("actual_axis", format_args!("{axis:?}")),
        ScrollAreaError::Core { .. } => frame,
    };
    ScrollAreaFailure::new(kind, frame)
}
pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> ScrollAreaFailure {
    error
        .map_kind(|source| ScrollAreaError::Core { source })
        .with_context(frame)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartIssue {
    entity: Entity,
    part: &'static str,
    axis: Option<ScrollAxis>,
    actual: Option<(Entity, ScrollAxis)>,
}
impl PartIssue {
    pub(crate) fn new(entity: Entity, part: &'static str, axis: Option<ScrollAxis>) -> Self {
        Self {
            entity,
            part,
            axis,
            actual: None,
        }
    }
    pub(crate) fn invalid_scrollbar(
        entity: Entity,
        expected_axis: ScrollAxis,
        target: Entity,
        actual_axis: ScrollAxis,
    ) -> Self {
        Self {
            entity,
            part: "Scrollbar",
            axis: Some(expected_axis),
            actual: Some((target, actual_axis)),
        }
    }
    fn kind(self, world: &World, root: Entity) -> ScrollAreaError {
        let context = Box::new(crate::layout::error_context(world, root, self.axis));
        match self.actual {
            Some((target, axis)) => ScrollAreaError::InvalidScrollbar {
                context,
                entity: self.entity,
                target,
                axis,
            },
            None => ScrollAreaError::MissingPart {
                context,
                entity: self.entity,
                part: self.part,
            },
        }
    }
    pub(crate) fn into_failure(
        self,
        world: &World,
        root: Entity,
        operation: &'static str,
    ) -> ScrollAreaFailure {
        failure(self.kind(world, root), ErrorFrame::new(operation))
    }
}

#[derive(Component, Default)]
pub(crate) struct DiagnosticStates {
    geometry: FailureState<PartIssue>,
    style: FailureState<PartIssue>,
}
pub(crate) enum Channel {
    Geometry,
    Style,
}
pub(crate) fn observe(
    world: &mut World,
    root: Entity,
    channel: Channel,
    result: Result<(), PartIssue>,
) -> Result<(), WidgetryEcsError> {
    if result.is_ok()
        && world.get::<DiagnosticStates>(root).is_none_or(|states| {
            match channel {
                Channel::Geometry => &states.geometry,
                Channel::Style => &states.style,
            }
            .active()
            .is_none()
        })
    {
        return Ok(());
    }
    let operation = match channel {
        Channel::Geometry => "scroll_area.geometry",
        Channel::Style => "scroll_area.style",
    };
    let kind = result.err().map(|issue| (issue, issue.kind(world, root)));
    let mut states = world.get_mut::<DiagnosticStates>(root);
    let state = states.as_mut().map(|states| match channel {
        Channel::Geometry => &mut states.geometry,
        Channel::Style => &mut states.style,
    });
    match kind {
        Some((key, kind)) => {
            let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                Arc::new(failure(kind, ErrorFrame::new(operation)))
            };
            Err(match state {
                Some(state) => state.failure(key, create),
                None => WidgetryEcsError::from_shared(create()),
            })
        }
        None => {
            let recovered = state.is_some_and(|state| {
                state.recover(|| ErrorFrame::new(operation).field("root", root))
            });
            if recovered && matches!(channel, Channel::Geometry) {
                crate::style::geometry_recovered(world, root);
            }
            Ok(())
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

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn core_wrapper_and_boundary_keep_the_original_origin_and_source() {
        let error =
            bevy_widgetry_core::color::validate_color(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0))
                .unwrap_err();
        let origin = error.origin().clone();
        let error = core(error, ErrorFrame::new("scroll_area.validate_colors"));
        assert!(Arc::ptr_eq(error.origin(), &origin));
        assert!(matches!(
            error
                .source()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<CoreError>(),
            Some(CoreError::InvalidColor { .. })
        ));
        let error = error.into_bevy_error();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(Arc::ptr_eq(
            error
                .downcast_ref::<WidgetryEcsError>()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap()
                .origin(),
            &origin
        ));
    }
}
