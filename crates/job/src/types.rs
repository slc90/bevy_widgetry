use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub(crate) u64);

impl JobId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobKind {
    Io,
    Cpu,
    Thread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Stopping,
    Completed,
    Failed,
    Canceled,
}

impl JobState {
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobError {
    Business(String),
    Panic(String),
    Infrastructure(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Business(message) => write!(f, "{message}"),
            Self::Panic(message) => write!(f, "job panicked: {message}"),
            Self::Infrastructure(message) => write!(f, "job infrastructure failed: {message}"),
        }
    }
}

impl std::error::Error for JobError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobOutcome {
    Completed,
    Failed(JobError),
    Canceled,
}

#[derive(Clone, Debug, Default)]
pub struct JobOptions {
    pub label: Option<String>,
    pub buffer_capacity: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobStatus {
    pub id: JobId,
    pub kind: JobKind,
    pub state: JobState,
    pub outcome: Option<JobOutcome>,
    pub label: Option<String>,
}

/// `Empty` is returned only by nonblocking reads. `Closed` follows the single `Finished`.
#[derive(Debug)]
pub enum JobRead<T> {
    Item(std::sync::Arc<T>),
    Finished(JobOutcome),
    Empty,
    Closed,
}

/// `Canceled` suppresses delivery without acknowledging cancellation or failing the job.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendStatus {
    Sent,
    Canceled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelResult {
    CancelRequested,
    AlreadyStopping,
    AlreadyFinished,
}
