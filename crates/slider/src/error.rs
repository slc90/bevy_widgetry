use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type SliderFailure = WidgetryFailure<SliderError>;

#[derive(Debug, Clone, Copy)]
pub struct SliderNumericConfig {
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderValueRejection {
    NonFiniteRange,
    UnorderedRange,
    NonFiniteSpan,
    InvalidStep,
    OutOfRange,
    Unreachable,
}

#[derive(Debug)]
pub enum SliderError {
    Numeric {
        config: SliderNumericConfig,
        reason: SliderValueRejection,
    },
    InvalidDimensions {
        track_thickness: f32,
        thumb_diameter: f32,
        field: &'static str,
    },
    InvalidPosition {
        root: Entity,
        position: f64,
    },
    Strategy {
        root: Entity,
        source: Box<dyn Error + Send + Sync>,
    },
    InvalidRoot {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        component: &'static str,
    },
    VisualStructure {
        root: Entity,
        parts: [Entity; 3],
        source: QueryEntityError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for SliderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric { reason, .. } => f.write_str(match reason {
                SliderValueRejection::NonFiniteRange
                | SliderValueRejection::UnorderedRange
                | SliderValueRejection::NonFiniteSpan => {
                    "Slider requires finite min < max and finite span"
                }
                SliderValueRejection::InvalidStep => "Slider step must be finite and positive",
                SliderValueRejection::OutOfRange => {
                    "Slider value must be finite and within [min, max]"
                }
                SliderValueRejection::Unreachable => {
                    "Slider value must be reachable from either endpoint by step"
                }
            }),
            Self::InvalidDimensions { .. } => {
                f.write_str("Slider dimensions must be finite and positive")
            }
            Self::InvalidPosition { .. } => f.write_str("Slider 策略返回非有限位置"),
            Self::Strategy { .. } => f.write_str("Slider 交互策略失败"),
            Self::InvalidRoot { .. } => f.write_str("Slider 根不存在或类型不匹配"),
            Self::MissingPart { component, .. } => write!(f, "Slider 缺失必需部件: {component}"),
            Self::VisualStructure { .. } => f.write_str("Slider 样式内部结构失效"),
            Self::Core { .. } => f.write_str("Slider 共享操作失败"),
        }
    }
}

impl Error for SliderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::Strategy { source, .. } => Some(source.as_ref()),
            Self::VisualStructure { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: SliderError, frame: ErrorFrame) -> SliderFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    SliderFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn missing(root: Entity, entity: Entity, component: &'static str) -> SliderFailure {
    failure(
        SliderError::MissingPart {
            root,
            entity,
            component,
        },
        ErrorFrame::new("slider.access_part"),
    )
}

pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> SliderFailure {
    error
        .map_kind(|source| SliderError::Core { source })
        .with_context(frame)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StyleKey {
    Query([Entity; 3], Entity, QueryFailure),
    Outline(Entity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QueryFailure {
    Missing,
    NotSpawned,
    Aliased,
}

pub(crate) enum StyleProblem {
    Query(QueryEntityError),
    Outline(Entity),
}

impl StyleProblem {
    pub(crate) fn key(&self, parts: [Entity; 3]) -> StyleKey {
        match self {
            Self::Outline(entity) => StyleKey::Outline(*entity),
            Self::Query(source) => {
                let (entity, category) = match source {
                    QueryEntityError::QueryDoesNotMatch(entity, _) => {
                        (*entity, QueryFailure::Missing)
                    }
                    QueryEntityError::NotSpawned(source) => {
                        (source.entity(), QueryFailure::NotSpawned)
                    }
                    QueryEntityError::AliasedMutability(entity) => (*entity, QueryFailure::Aliased),
                };
                StyleKey::Query(parts, entity, category)
            }
        }
    }

    pub(crate) fn into_failure(
        self,
        root: Entity,
        parts: [Entity; 3],
        frame: ErrorFrame,
    ) -> SliderFailure {
        match self {
            Self::Query(source) => failure(
                SliderError::VisualStructure {
                    root,
                    parts,
                    source,
                },
                frame,
            ),
            Self::Outline(entity) => missing(root, entity, "Outline").with_context(frame),
        }
    }
}
