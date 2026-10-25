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
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Business(message) => write!(f, "{message}"),
            Self::Panic(message) => write!(f, "job panicked: {message}"),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobStatus {
    pub id: JobId,
    pub kind: JobKind,
    pub state: JobState,
    pub outcome: Option<JobOutcome>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobAccessError {
    NotFound,
    NotFinished(JobState),
}

impl fmt::Display for JobAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for JobAccessError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmitError {
    IdExhausted,
    PoolUnavailable(JobKind),
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SubmitError {}

#[derive(Debug)]
pub enum JobMessage<T> {
    Item(std::sync::Arc<T>),
    Finished(JobOutcome),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Closed,
}
