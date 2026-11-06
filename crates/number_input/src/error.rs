use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use bevy_widgetry_text_field::{TextFieldError, TextFieldFailure};
use std::{error::Error, fmt};

pub type NumberInputFailure = WidgetryFailure<NumberInputError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberInputValue {
    Signed(i128),
    Unsigned(u128),
    Float32(f32),
    Float64(f64),
}

#[derive(Debug, Clone)]
pub struct NumberInputConfigSnapshot {
    pub numeric_type: &'static str,
    pub value: Option<NumberInputValue>,
    pub min: Option<NumberInputValue>,
    pub max: Option<NumberInputValue>,
    pub step: NumberInputValue,
    pub decimals: Option<u8>,
}

impl NumberInputConfigSnapshot {
    pub(crate) fn new<T: crate::WidgetryNumber>(
        value: Option<T>,
        min: Option<T>,
        max: Option<T>,
        step: T,
        decimals: Option<u8>,
    ) -> Self {
        Self {
            numeric_type: std::any::type_name::<T>(),
            value: value.map(T::diagnostic_value),
            min: min.map(T::diagnostic_value),
            max: max.map(T::diagnostic_value),
            step: step.diagnostic_value(),
            decimals,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberInputRejection {
    MissingValue,
    NonFiniteBounds,
    ReversedBounds,
    InvalidStep,
    IntegerDecimals,
    OutOfRange,
}

#[derive(Debug)]
pub enum NumberInputError {
    Numeric {
        config: Box<NumberInputConfigSnapshot>,
        reason: NumberInputRejection,
    },
    InvalidRoot {
        root: Entity,
        numeric_type: &'static str,
    },
    InvalidEditorBinding {
        root: Entity,
        field: Entity,
        read_only: bool,
    },
    IncompatibleEditor {
        root: Entity,
        field: Entity,
        editor: Entity,
        max_length: Option<usize>,
        multiline: bool,
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
    Query {
        root: Entity,
        entity: Entity,
        component: &'static str,
        source: QueryEntityError,
    },
    TextField {
        source: TextFieldError,
    },
}

impl fmt::Display for NumberInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric { reason, .. } => f.write_str(match reason {
                NumberInputRejection::MissingValue => {
                    "NumberInput requires an explicit initial value"
                }
                NumberInputRejection::NonFiniteBounds | NumberInputRejection::ReversedBounds => {
                    "NumberInput requires finite bounds with min <= max"
                }
                NumberInputRejection::InvalidStep => "NumberInput step must be finite and positive",
                NumberInputRejection::IntegerDecimals => {
                    "NumberInput decimals is only supported for floating-point types"
                }
                NumberInputRejection::OutOfRange => {
                    "NumberInput value must be finite and within its bounds"
                }
            }),
            Self::InvalidRoot { .. } => f.write_str("NumberInput 根不存在或数值类型不匹配"),
            Self::InvalidEditorBinding { .. } => {
                f.write_str("NumberInput 编辑区必须属于当前根且只读模式一致")
            }
            Self::IncompatibleEditor { .. } => {
                f.write_str("NumberInput 编辑区必须为不限长度的单行文本")
            }
            Self::MissingPart { component, .. } => {
                write!(f, "NumberInput 缺失必需部件: {component}")
            }
            Self::MissingResource { resource, .. } => {
                write!(f, "NumberInput 缺失必需资源: {resource}")
            }
            Self::Query { component, .. } => write!(f, "NumberInput 部件查询失败: {component}"),
            Self::TextField { .. } => f.write_str("NumberInput 内部文本操作失败"),
        }
    }
}
impl Error for NumberInputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::TextField { source } => Some(source),
            Self::Query { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: NumberInputError, frame: ErrorFrame) -> NumberInputFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    NumberInputFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn missing(
    root: Entity,
    entity: Entity,
    component: &'static str,
    operation: &'static str,
) -> NumberInputFailure {
    failure(
        NumberInputError::MissingPart {
            root,
            entity,
            component,
        },
        ErrorFrame::new(operation),
    )
}

pub(crate) fn text_field(
    error: TextFieldFailure,
    root: Entity,
    field: Entity,
    operation: &'static str,
) -> NumberInputFailure {
    error
        .map_kind(|source| NumberInputError::TextField { source })
        .with_context(
            ErrorFrame::new(operation)
                .field("root", root)
                .field("field", field),
        )
}

#[track_caller]
pub(crate) fn query(
    root: Entity,
    entity: Entity,
    component: &'static str,
    source: QueryEntityError,
    operation: &'static str,
) -> NumberInputFailure {
    failure(
        NumberInputError::Query {
            root,
            entity,
            component,
            source,
        },
        ErrorFrame::new(operation),
    )
}
