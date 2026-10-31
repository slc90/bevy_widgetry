use crate::{JobId, JobKind, JobStatus};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

#[derive(Clone, Debug)]
pub struct SubmitRequest {
    pub kind: Option<JobKind>,
    pub label: Option<String>,
    pub capacity: usize,
}

#[derive(Debug)]
pub enum SubmitError {
    Preparation {
        label: String,
        job: Option<JobId>,
        source: std::io::Error,
    },
    IdExhausted {
        request: SubmitRequest,
        last_id: u64,
    },
    PoolUnavailable {
        request: SubmitRequest,
        pool: JobKind,
    },
    ZeroCapacity {
        request: SubmitRequest,
    },
    ThreadSpawn {
        request: SubmitRequest,
        job: Option<JobId>,
        source: std::io::Error,
    },
    ShuttingDown {
        request: SubmitRequest,
    },
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job submission rejected: {self:?}")
    }
}
impl Error for SubmitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ThreadSpawn { source, .. } | Self::Preparation { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobAccessError {
    NotFound {
        job: JobId,
    },
    NotFinished {
        status: Box<JobStatus>,
    },
    NotSubscribable {
        status: Box<JobStatus>,
    },
    TypeMismatch {
        status: Box<JobStatus>,
        requested: &'static str,
        actual: &'static str,
    },
}
impl fmt::Display for JobAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job access rejected: {self:?}")
    }
}
impl Error for JobAccessError {}

#[derive(Debug)]
pub enum SendError {
    Closed { job: JobId },
    Overflowed { job: JobId },
}
impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job send failed: {self:?}")
    }
}
impl Error for SendError {}

#[derive(Debug)]
pub enum ReceiveError {
    Overflowed { job: JobId, count: u64 },
}
impl fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job receive failed: {self:?}")
    }
}
impl Error for ReceiveError {}

pub type SubmitFailure = WidgetryFailure<SubmitError>;
pub type JobAccessFailure = WidgetryFailure<JobAccessError>;
pub type SendFailure = WidgetryFailure<SendError>;
pub type ReceiveFailure = WidgetryFailure<ReceiveError>;

#[track_caller]
pub(crate) fn submit(kind: SubmitError, operation: &'static str) -> SubmitFailure {
    let frame = match &kind {
        SubmitError::Preparation { label, job, .. } => ErrorFrame::new(operation)
            .field("label", label)
            .field("job", format_args!("{job:?}")),
        SubmitError::IdExhausted { request, .. }
        | SubmitError::PoolUnavailable { request, .. }
        | SubmitError::ZeroCapacity { request }
        | SubmitError::ThreadSpawn { request, .. }
        | SubmitError::ShuttingDown { request } => ErrorFrame::new(operation)
            .field("kind", format_args!("{:?}", request.kind))
            .field("label", format_args!("{:?}", request.label))
            .field("capacity", request.capacity),
    };
    SubmitFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn access(kind: JobAccessError, operation: &'static str) -> JobAccessFailure {
    let job = match &kind {
        JobAccessError::NotFound { job } => *job,
        JobAccessError::NotFinished { status }
        | JobAccessError::NotSubscribable { status }
        | JobAccessError::TypeMismatch { status, .. } => status.id,
    };
    JobAccessFailure::new(kind, ErrorFrame::new(operation).field("job_id", job.get()))
}
