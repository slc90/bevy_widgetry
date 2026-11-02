use crate::{TableFailure, WidgetryTableCell, WidgetryTableEditRequestId};
use bevy::{ecs::error::Severity, platform::cell::SyncCell, prelude::*};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{Diagnostic, WidgetryEcsError},
};
use std::{
    any::{Any, TypeId},
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEditStage {
    Begin,
    Submit,
    Read,
    Feedback,
    Resolve,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEditReceiptReason {
    NoSession,
    WrongView,
    NotPending,
    WrongRequest,
    SourceChanged,
    ModelInstanceChanged,
}
#[derive(Debug)]
pub enum TableEditFault {
    MissingPart {
        entity: Entity,
        component: &'static str,
    },
    InvalidCell {
        cell: WidgetryTableCell,
    },
    NotVisible {
        cell: WidgetryTableCell,
    },
    ModelInstanceChanged {
        previous_source: Entity,
        current_source: Entity,
    },
    TypeMismatch {
        expected: TypeId,
        actual: TypeId,
    },
    RequestExhausted {
        serial: u64,
    },
    Receipt {
        reason: TableEditReceiptReason,
        received: WidgetryTableEditRequestId,
        pending: Option<WidgetryTableEditRequestId>,
    },
    Read {
        source: Box<dyn Error + Send + Sync>,
    },
    Runtime {
        fault: crate::TableRuntimeFault,
    },
}
impl fmt::Display for TableEditFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingPart { .. } => "Table 编辑所需部件失效",
            Self::InvalidCell { .. } => "Table 编辑单元格身份失效",
            Self::NotVisible { .. } => "Table 编辑单元格尚未物化",
            Self::ModelInstanceChanged { .. } => "Table 编辑 Model 实例已更换",
            Self::TypeMismatch { .. } => "Table 编辑器初值类型不匹配",
            Self::RequestExhausted { .. } => "Table 编辑请求计数器已耗尽",
            Self::Receipt { .. } => "Table 编辑回执与当前请求不匹配",
            Self::Read { .. } => "Table 编辑器读取失败",
            Self::Runtime { .. } => "Table 编辑器运行结构失效",
        })
    }
}
impl Error for TableEditFault {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

pub struct TableEditFailure {
    diagnostics: WidgetryEcsError,
    severity: Severity,
    payload: SyncCell<Option<Box<dyn Any + Send>>>,
}
impl TableEditFailure {
    pub(crate) fn scene(
        failure: bevy_widgetry_core::scene::SceneFailure,
        frame: ErrorFrame,
    ) -> Self {
        let failure = failure.with_context(frame);
        let diagnostics = failure.diagnostics().clone();
        let mut native = failure.into_bevy_error();
        Self {
            diagnostics,
            severity: native.severity(),
            payload: SyncCell::new(native.take_payload()),
        }
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &dyn Diagnostic> {
        self.diagnostics.diagnostics()
    }
    pub fn with_context(mut self, frame: ErrorFrame) -> Self {
        self.diagnostics = self.diagnostics.with_context(frame);
        self
    }
    pub fn into_bevy_error(mut self) -> BevyError {
        let mut error = self
            .diagnostics
            .into_bevy_error()
            .with_severity(self.severity);
        if let Some(payload) = self.payload.get().take() {
            error = error.with_payload(payload);
        }
        error
    }
}
impl From<TableFailure> for TableEditFailure {
    fn from(failure: TableFailure) -> Self {
        Self::from(WidgetryEcsError::new(failure))
    }
}
impl From<WidgetryEcsError> for TableEditFailure {
    fn from(diagnostics: WidgetryEcsError) -> Self {
        Self {
            diagnostics,
            severity: Severity::Error,
            payload: SyncCell::new(None),
        }
    }
}
impl fmt::Display for TableEditFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.diagnostics, f)
    }
}
impl fmt::Debug for TableEditFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TableEditFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.diagnostics)
    }
}
