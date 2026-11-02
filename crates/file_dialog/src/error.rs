use crate::*;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt, path::PathBuf};

pub type FileDialogFailure = WidgetryFailure<FileDialogError>;

#[derive(Debug)]
pub enum FileDialogError {
    DirectoryIo {
        context: Box<FileDialogDirectoryContext>,
        source: std::io::Error,
    },
    Directory {
        context: Box<FileDialogDirectoryContext>,
        fault: Box<FileDialogDirectoryFault>,
    },
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
pub enum FileDialogDirectoryOperation {
    ResolveDirectory,
    CurrentDirectory,
    Canonicalize,
    DirectoryMetadata,
    ReadDirectory,
    ReadEntry,
    EntryMetadata,
    LinkMetadata,
    ModifiedTime,
    CpuSend,
    CpuReply,
    Snapshot,
}

#[derive(Clone, Debug)]
pub struct FileDialogDirectoryContext {
    pub operation: FileDialogDirectoryOperation,
    pub root: Option<Entity>,
    pub token: Option<WidgetryFileDialogToken>,
    pub directory: Option<PathBuf>,
    pub entry: Option<PathBuf>,
    pub ordinal: Option<usize>,
}

impl FileDialogDirectoryContext {
    pub(crate) fn snapshot(token: WidgetryFileDialogToken, directory: PathBuf) -> Self {
        Self {
            operation: FileDialogDirectoryOperation::Snapshot,
            root: None,
            token: Some(token),
            directory: Some(directory),
            entry: None,
            ordinal: None,
        }
    }
    pub(crate) fn frame(&self) -> ErrorFrame {
        ErrorFrame::new("file_dialog.directory").field("directory", format_args!("{self:?}"))
    }
}

#[derive(Debug)]
pub enum FileDialogDirectoryFault {
    CpuDisconnected,
    InvalidCpuEvent,
    Capacity {
        actual: usize,
        limit: usize,
    },
    ByteCapacity {
        actual: usize,
        limit: usize,
    },
    Generation {
        requested: WidgetryFileDialogToken,
        current: WidgetryFileDialogToken,
    },
    EntryPath {
        path: PathBuf,
        name: std::ffi::OsString,
    },
    DuplicatePath {
        path: PathBuf,
    },
    EntryIdExhausted {
        next: u64,
    },
}

impl fmt::Display for FileDialogDirectoryFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuDisconnected => f.write_str("FileDialog CPU channel disconnected"),
            Self::InvalidCpuEvent => f.write_str("FileDialog CPU batch returned an invalid event"),
            Self::Capacity { actual, limit } => write!(
                f,
                "directory snapshot capacity exceeded ({actual} > {limit})"
            ),
            Self::ByteCapacity { actual, limit } => write!(
                f,
                "directory snapshot byte capacity exceeded ({actual} > {limit})"
            ),
            Self::Generation { requested, current } => write!(
                f,
                "cached snapshot belongs to another directory generation ({requested:?}, current {current:?})"
            ),
            Self::EntryPath { path, name } => write!(
                f,
                "entry path/name does not belong to snapshot directory ({path:?}, {name:?})"
            ),
            Self::DuplicatePath { path } => write!(f, "duplicate entry path ({path:?})"),
            Self::EntryIdExhausted { next } => write!(f, "entry ID exhausted ({next})"),
        }
    }
}

#[track_caller]
pub(crate) fn directory_failure(
    context: FileDialogDirectoryContext,
    fault: FileDialogDirectoryFault,
) -> FileDialogFailure {
    failure(
        FileDialogError::Directory {
            context: Box::new(context),
            fault: Box::new(fault),
        },
        "file_dialog.directory",
    )
}

#[track_caller]
pub(crate) fn directory_io(
    context: FileDialogDirectoryContext,
    source: std::io::Error,
) -> bevy_widgetry_job::JobFailure {
    if is_managed_directory_io(&source) {
        return bevy_widgetry_job::JobFailure::from(source).with_context(context.frame());
    }
    let mut context = context;
    if let Some(native) = source
        .get_ref()
        .and_then(|error| error.downcast_ref::<NativeDirectoryIo>())
    {
        context.operation = native.context.operation;
        context.directory = native.context.directory.clone();
        context.entry = native.context.entry.clone();
        context.ordinal = native.context.ordinal;
    }
    failure(
        FileDialogError::DirectoryIo {
            context: Box::new(context),
            source,
        },
        "file_dialog.directory.io",
    )
    .into()
}

pub(crate) fn is_managed_directory_io(error: &std::io::Error) -> bool {
    error.get_ref().is_some_and(|source| {
        source.is::<bevy_widgetry_job::JobFailure>()
            || source.is::<bevy_widgetry_job::SendFailure>()
    })
}

// Native I/O keeps its phase and raw cause until the directory owner can classify
// Interrupted using the actual cancellation state, before creating a diagnostic.
#[derive(Debug)]
struct NativeDirectoryIo {
    context: FileDialogDirectoryContext,
    source: std::io::Error,
}
impl fmt::Display for NativeDirectoryIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}
impl Error for NativeDirectoryIo {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
pub(crate) fn native_directory_io(
    context: FileDialogDirectoryContext,
    source: std::io::Error,
) -> std::io::Error {
    std::io::Error::new(source.kind(), NativeDirectoryIo { context, source })
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
            Self::DirectoryIo { .. } => f.write_str("FileDialog 目录 I/O 失败"),
            Self::Directory { fault, .. } => fmt::Display::fmt(fault, f),
            Self::Wake { .. } => f.write_str("FileDialog 原生事件循环唤醒失败"),
            other => write!(f, "FileDialog 契约无效：{other:?}"),
        }
    }
}

impl Error for FileDialogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DirectoryIo { source, .. } => Some(source),
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
