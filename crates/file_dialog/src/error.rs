use crate::*;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt, path::PathBuf};

pub type FileDialogFailure = WidgetryFailure<FileDialogError>;

#[derive(Debug)]
pub enum FileDialogError {
    ScopeLength {
        scope: String,
        bytes: usize,
    },
    Capacity {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    DuplicateFilter {
        id: WidgetryFileDialogFilterId,
    },
    Filter {
        filter: Box<WidgetryFileDialogFilter>,
        violation: FileDialogFilterViolation,
        suffix: Option<String>,
    },
    SessionExhausted {
        next: u64,
    },
    RevisionExhausted {
        counter: &'static str,
        value: u64,
        token: Option<Box<WidgetryFileDialogToken>>,
        scope: Option<String>,
    },
    Api {
        root: Option<Entity>,
        token: Option<WidgetryFileDialogToken>,
        action: Option<Box<WidgetryFileDialogAction>>,
        fault: FileDialogApiFault,
    },
    State {
        token: WidgetryFileDialogToken,
        mode: WidgetryFileDialogMode,
        fault: Box<FileDialogStateFault>,
    },
    StorageScope {
        scope: String,
    },
    StorageAcknowledgement {
        scope: String,
        requested: u64,
        current: u64,
    },
    PinnedPath {
        path: PathBuf,
        duplicate: bool,
    },
    Wake {
        source: Box<dyn Error + Send + Sync>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogFilterViolation {
    ReservedId,
    InvalidSuffix,
    DefaultExtension,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogApiFault {
    InvalidRoot,
    IndependentReopen,
    MissingState,
    MissingRuntime,
    MissingStorage,
    MissingPersistence,
}

#[derive(Debug)]
pub enum FileDialogStateFault {
    SessionResolved,
    SessionAlreadyOpen,
    StaleActivationView {
        requested: WidgetryFileDialogToken,
    },
    StaleActivationEntry {
        id: WidgetryFileDialogEntryId,
    },
    ActivationKind,
    PinnedPath {
        path: PathBuf,
    },
    PinnedCapacity {
        actual: usize,
        limit: usize,
    },
    FolderPending,
    FolderUnvalidated,
    FolderMissingDirectory,
    InvalidFolderName {
        name: std::ffi::OsString,
        rejection: String,
    },
    SnapshotMismatch,
    SnapshotBeforeStarted,
    EmptyNavigationPath,
    RefreshMissingDirectory,
    HistoryCursor {
        cursor: Option<usize>,
        len: usize,
    },
    UpMissingDirectory,
    UnknownFilter {
        id: WidgetryFileDialogFilterId,
    },
    InvalidQuery,
    SelectionEntryMissing {
        id: WidgetryFileDialogEntryId,
    },
    InvalidSelectionIntent,
    SelectionMissingSnapshot,
    ProjectionPending,
    StaleSelectionView {
        requested: WidgetryFileDialogToken,
    },
    StaleEntry {
        id: WidgetryFileDialogEntryId,
    },
    SelectionKind,
    MultipleSelectionMode,
    SelectAllMode,
    StaleActiveEntry {
        id: WidgetryFileDialogEntryId,
    },
    SelectionCapacity {
        actual: usize,
        limit: usize,
    },
    StaleProjection {
        requested: WidgetryFileDialogToken,
    },
    FilenameMode,
    ConfirmationPending,
    DirectoryUnavailable,
    CurrentDirectoryMissing,
    ConfirmationSelectionMissing,
    StaleOverwrite {
        requested: WidgetryFileDialogToken,
    },
    MissingOverwriteCandidate,
}

impl fmt::Display for FileDialogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wake { .. } => f.write_str("FileDialog 原生事件循环唤醒失败"),
            other => write!(f, "FileDialog 契约无效：{other:?}"),
        }
    }
}

impl Error for FileDialogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Wake { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum FileDialogOperationFailure {
    Domain(FileDialogFailure),
    Job(bevy_widgetry_job::JobFailure),
}

impl From<FileDialogFailure> for FileDialogOperationFailure {
    fn from(failure: FileDialogFailure) -> Self {
        Self::Domain(failure)
    }
}

// Bevy's blanket conversion would assign Panic severity and hide the original diagnostics.
// Keep this composite outside std::error::Error and provide the actual boundary conversion.
impl From<FileDialogOperationFailure> for BevyError {
    fn from(failure: FileDialogOperationFailure) -> Self {
        failure.into_bevy_error()
    }
}

impl From<FileDialogOperationFailure> for bevy_widgetry_job::JobFailure {
    fn from(failure: FileDialogOperationFailure) -> Self {
        match failure {
            FileDialogOperationFailure::Domain(failure) => failure.into(),
            FileDialogOperationFailure::Job(failure) => failure,
        }
    }
}

impl FileDialogOperationFailure {
    pub fn into_bevy_error(self) -> BevyError {
        match self {
            Self::Domain(error) => error.into_bevy_error(),
            Self::Job(error) => error.into_bevy_error(),
        }
    }
    pub fn with_context(self, frame: ErrorFrame) -> Self {
        match self {
            Self::Domain(error) => Self::Domain(error.with_context(frame)),
            Self::Job(error) => Self::Job(error.with_context(frame)),
        }
    }
}

impl fmt::Display for FileDialogOperationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => fmt::Display::fmt(error, f),
            Self::Job(error) => fmt::Display::fmt(error, f),
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: FileDialogError, operation: &'static str) -> FileDialogFailure {
    let frame = ErrorFrame::new(operation).field("failure", format_args!("{kind:?}"));
    FileDialogFailure::new(kind, frame)
}

#[track_caller]
pub(crate) fn state_failure(
    token: WidgetryFileDialogToken,
    mode: WidgetryFileDialogMode,
    fault: FileDialogStateFault,
) -> FileDialogFailure {
    failure(
        FileDialogError::State {
            token,
            mode,
            fault: Box::new(fault),
        },
        "file_dialog.state",
    )
}

impl WidgetryFileDialogState {
    #[track_caller]
    pub(crate) fn contract(&self, fault: FileDialogStateFault) -> FileDialogFailure {
        state_failure(self.token(), self.mode(), fault)
    }
}
