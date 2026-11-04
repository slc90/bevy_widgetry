use crate::{WaveformError, WaveformFailure, WaveformPart};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{BoundaryFailure, DiagnosticBatch, FailureState, WidgetryEcsError},
};

pub(crate) type ViewResult<T = ()> = std::result::Result<T, ViewFailure>;

#[derive(Debug)]
pub(crate) enum ViewFailure {
    Structure {
        entity: Option<Entity>,
        part: WaveformPart,
    },
    Style(f32),
    Query {
        entity: Entity,
        part: WaveformPart,
        source: bevy::ecs::query::QueryEntityError,
    },
    Managed(bevy_widgetry_job::JobFailure),
    Boundary(BevyError),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FaultKey {
    Structure(Option<Entity>, WaveformPart),
    Style(u32),
}

impl From<BevyError> for ViewFailure {
    fn from(error: BevyError) -> Self {
        Self::Boundary(error)
    }
}

impl ViewFailure {
    pub fn at(self, target: Entity) -> Self {
        match self {
            Self::Structure { entity, part } => Self::Structure {
                entity: entity.or(Some(target)),
                part,
            },
            other => other,
        }
    }

    fn key(&self) -> Option<FaultKey> {
        match self {
            Self::Structure { entity, part } => Some(FaultKey::Structure(*entity, *part)),
            Self::Query { entity, part, .. } => Some(FaultKey::Structure(Some(*entity), *part)),
            Self::Style(width) => Some(FaultKey::Style(width.to_bits())),
            Self::Boundary(_) | Self::Managed(_) => None,
        }
    }

    fn report(self, frame: ErrorFrame) -> WidgetryEcsError {
        let kind = match self {
            Self::Structure { entity, part } => WaveformError::Structure { entity, part },
            Self::Style(line_width) => WaveformError::Style { line_width },
            Self::Query {
                entity,
                part,
                source,
            } => WaveformError::Query {
                entity,
                part,
                source,
            },
            Self::Managed(error) => return error.into_report().with_context(frame),
            Self::Boundary(error) => {
                return bevy_widgetry_job::JobFailure::from(error)
                    .into_report()
                    .with_context(frame);
            }
        };
        let frame = match &kind {
            WaveformError::Structure { entity, part } => frame
                .field("part", format_args!("{part:?}"))
                .field("target", format_args!("{entity:?}")),
            WaveformError::Style { line_width } => frame.field("line_width", line_width),
            WaveformError::Query { entity, part, .. } => frame
                .field("part", format_args!("{part:?}"))
                .field("target", entity),
            _ => frame,
        };
        WidgetryEcsError::new(WaveformFailure::new(kind, frame))
    }

    pub fn into_failure(
        self,
        root: Option<Entity>,
        operation: &'static str,
    ) -> bevy_widgetry_job::JobFailure {
        let frame = ErrorFrame::new(operation).field("root", format_args!("{root:?}"));
        let failure = match root {
            Some(root) => self.at(root),
            None => self,
        };
        match failure {
            Self::Managed(error) => error.with_context(frame),
            Self::Boundary(error) => bevy_widgetry_job::JobFailure::from(error).with_context(frame),
            fault => fault.report(frame).into(),
        }
    }
    pub fn into_bevy_error(self, root: Option<Entity>, operation: &'static str) -> BevyError {
        let frame = ErrorFrame::new(operation).field("root", format_args!("{root:?}"));
        let failure = match root {
            Some(root) => self.at(root),
            None => self,
        };
        match failure {
            Self::Managed(error) => error.with_context(frame).into_bevy_error(),
            Self::Boundary(error) => context(error, frame),
            fault => fault.report(frame).into_bevy_error(),
        }
    }
}

pub(crate) fn missing(part: WaveformPart) -> ViewFailure {
    ViewFailure::Structure { entity: None, part }
}

pub(crate) fn context(error: BevyError, frame: ErrorFrame) -> BevyError {
    if error.severity() == bevy::ecs::error::Severity::Error
        && let Some(managed) = error.downcast_ref::<WidgetryEcsError>()
    {
        managed.clone().with_context(frame).into_bevy_error()
    } else {
        BoundaryFailure::Native(error)
            .with_context(frame)
            .into_bevy_error()
    }
}

#[derive(Default)]
pub(crate) struct ViewFailureState(FailureState<FaultKey>);

impl ViewFailureState {
    pub fn observe<T>(
        &mut self,
        result: ViewResult<T>,
        root: Entity,
        operation: &'static str,
    ) -> std::result::Result<T, BevyError> {
        let frame = || ErrorFrame::new(operation).field("root", root);
        match result {
            Ok(value) => {
                self.0.recover(frame);
                Ok(value)
            }
            Err(error) => {
                let error = error.at(root);
                match error.key() {
                    Some(key) => Err(self
                        .0
                        .failure_batch(key, || error.report(frame()))
                        .into_bevy_error()),
                    None => Err(error.into_bevy_error(Some(root), operation)),
                }
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct ViewErrors {
    managed: DiagnosticBatch,
    native: Vec<BevyError>,
}

impl ViewErrors {
    pub fn push(&mut self, error: BevyError) {
        if error.severity() == bevy::ecs::error::Severity::Error
            && let Some(report) = error.downcast_ref::<WidgetryEcsError>()
        {
            for diagnostic in report.shared_diagnostics() {
                self.managed.push_shared(diagnostic);
            }
        } else {
            self.native.push(error);
        }
    }
    pub fn finish(mut self, mut commands: Commands) -> Result {
        let error = self.managed.into_bevy_error().or_else(|| self.native.pop());
        for error in self.native {
            commands.queue(move |_: &mut World| -> Result { Err(error) });
        }
        error.map_or(Ok(()), Err)
    }
}

impl From<bevy_widgetry_job::JobFailure> for ViewFailure {
    fn from(error: bevy_widgetry_job::JobFailure) -> Self {
        Self::Managed(error)
    }
}

impl<E: std::error::Error + Send + Sync + 'static>
    From<bevy_widgetry_diagnostics::WidgetryFailure<E>> for ViewFailure
{
    fn from(error: bevy_widgetry_diagnostics::WidgetryFailure<E>) -> Self {
        Self::Managed(error.into())
    }
}
