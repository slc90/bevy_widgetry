use crate::DatePickerMonth;
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt, num::TryFromIntError};
use time::{
    Date, Weekday,
    error::{ComponentRange, IndeterminateOffset},
};

pub type DatePickerFailure = WidgetryFailure<DatePickerError>;

#[derive(Debug)]
pub enum DatePickerError {
    ReversedBounds {
        min: Date,
        max: Date,
    },
    SelectionOutOfRange {
        selected: Date,
        min: Option<Date>,
        max: Option<Date>,
    },
    Calendar {
        displayed: DatePickerMonth,
        week_start: Weekday,
        grid_index: Option<usize>,
        julian_day: Option<i32>,
        source: ComponentRange,
    },
    MonthShift {
        displayed: DatePickerMonth,
        delta: i32,
        source: TryFromIntError,
    },
    LocalTime {
        source: IndeterminateOffset,
    },
    InvalidRoot {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        component: &'static str,
    },
    DayCount {
        root: Entity,
        actual: usize,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for DatePickerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReversedBounds { .. } => f.write_str("DatePicker min_date exceeds max_date"),
            Self::SelectionOutOfRange { .. } => {
                f.write_str("DatePicker selected_date is outside date bounds")
            }
            Self::Calendar { .. } => f.write_str("DatePicker 日期计算失败"),
            Self::MonthShift { .. } => f.write_str("DatePicker 月份偏移无法表示"),
            Self::LocalTime { .. } => f.write_str("DatePicker 本地日期读取失败"),
            Self::InvalidRoot { .. } => f.write_str("DatePicker 根不存在"),
            Self::MissingPart { component, .. } => {
                write!(f, "DatePicker 缺失必需部件: {component}")
            }
            Self::DayCount { actual, .. } => {
                write!(f, "DatePicker 需要 42 个日期格，实际 {actual}")
            }
            Self::Core { .. } => f.write_str("DatePicker 内部样式操作失败"),
        }
    }
}

impl Error for DatePickerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Calendar { source, .. } => Some(source),
            Self::MonthShift { source, .. } => Some(source),
            Self::LocalTime { source } => Some(source),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: DatePickerError, frame: ErrorFrame) -> DatePickerFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    DatePickerFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn missing(
    root: Entity,
    entity: Entity,
    component: &'static str,
    operation: &'static str,
) -> DatePickerFailure {
    failure(
        DatePickerError::MissingPart {
            root,
            entity,
            component,
        },
        ErrorFrame::new(operation),
    )
}

pub(crate) fn core(error: CoreFailure, root: Entity, operation: &'static str) -> DatePickerFailure {
    error
        .map_kind(|source| DatePickerError::Core { source })
        .with_context(ErrorFrame::new(operation).field("root", root))
}
