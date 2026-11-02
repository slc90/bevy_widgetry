use bevy_widgetry_diagnostics::{WidgetryFailure, bevy::WidgetryEcsError};
use std::{error::Error, fmt, io, sync::Arc};

pub type PowerFailure = WidgetryFailure<PowerError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestKind {
    Display,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerOperation {
    Create,
    Set,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerPhase {
    Enable,
    Disable,
    Rollback,
    Drop,
}

#[derive(Debug)]
pub struct PowerError {
    pub operation: PowerOperation,
    pub phase: PowerPhase,
    pub request: Option<RequestKind>,
    pub display: bool,
    pub system: bool,
    pub source: io::Error,
}

impl fmt::Display for PowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "电源请求 {:?} 失败（{:?}，{:?}）",
            self.operation, self.phase, self.request
        )
    }
}

impl Error for PowerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
pub struct PowerFailures {
    first: PowerFailure,
    remaining: Vec<PowerFailure>,
}

impl PowerFailures {
    pub(crate) fn new(first: PowerFailure) -> Self {
        Self {
            first,
            remaining: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, failure: PowerFailure) {
        self.remaining.push(failure);
    }

    pub(crate) fn extend(&mut self, failures: Self) {
        self.remaining.push(failures.first);
        self.remaining.extend(failures.remaining);
    }

    pub fn failures(&self) -> impl Iterator<Item = &PowerFailure> {
        std::iter::once(&self.first).chain(&self.remaining)
    }

    pub fn into_bevy_error(self) -> bevy::prelude::BevyError {
        let mut error = WidgetryEcsError::new(self.first);
        for failure in self.remaining {
            error.push_shared(Arc::new(failure));
        }
        error.into_bevy_error()
    }
}

impl fmt::Display for PowerFailures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.failures().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{failure}")?;
        }
        Ok(())
    }
}

impl Error for PowerFailures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.first)
    }
}
