use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{
    error::Error,
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub type LoggingFailure = WidgetryFailure<LoggingError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoggingStage {
    LocalOffset,
    Timestamp,
    Directory,
    File,
    Write,
    Flush,
    Queue,
    FlushTimeout,
}

#[derive(Debug)]
pub enum LoggingSource {
    Io(Arc<io::Error>),
    Offset(time::error::IndeterminateOffset),
    Format(time::error::Format),
    PendingBytes(usize),
}

#[derive(Debug)]
pub struct LoggingError {
    pub stage: LoggingStage,
    pub path: PathBuf,
    pub source: LoggingSource,
}
impl fmt::Display for LoggingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "application logging failed at {:?}, path={}",
            self.stage,
            self.path.display()
        )?;
        if let LoggingSource::PendingBytes(bytes) = self.source {
            write!(f, ", pending_bytes={bytes}")?;
        }
        Ok(())
    }
}
impl Error for LoggingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            LoggingSource::Io(source) => Some(source.as_ref()),
            LoggingSource::Offset(source) => Some(source),
            LoggingSource::Format(source) => Some(source),
            LoggingSource::PendingBytes(_) => None,
        }
    }
}
pub(crate) fn frame(stage: LoggingStage, path: &Path) -> ErrorFrame {
    ErrorFrame::new("app_logging")
        .field("stage", format_args!("{stage:?}"))
        .field("path", path.display())
}
#[track_caller]
pub(crate) fn failure(stage: LoggingStage, path: &Path, source: LoggingSource) -> LoggingFailure {
    WidgetryFailure::new(
        LoggingError {
            stage,
            path: path.to_owned(),
            source,
        },
        frame(stage, path),
    )
}
