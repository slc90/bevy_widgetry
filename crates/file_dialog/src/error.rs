use crate::*;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame, ErrorOrigin, WidgetryFailure,
    bevy::{Diagnostic, WidgetryEcsError},
};
use std::{error::Error, fmt, path::PathBuf, sync::Arc};

pub type FileDialogFailure = WidgetryFailure<FileDialogError>;

#[derive(Debug)]
pub enum FileDialogError {
    PreferenceIo {
        context: Box<FileDialogPreferenceContext>,
        source: std::io::Error,
    },
    PreferenceManaged {
        context: Arc<FileDialogPreferenceContext>,
        source: Arc<dyn Diagnostic>,
    },
    RequestManaged {
        context: Arc<FileDialogRequestContext>,
        source: Arc<dyn Diagnostic>,
    },
    RequestIo {
        context: Box<FileDialogRequestContext>,
        source: std::io::Error,
    },
    Request {
        context: Box<FileDialogRequestContext>,
        fault: Box<FileDialogRequestFault>,
    },
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
pub enum FileDialogPreferenceOperation {
    Load,
    Save,
    Open,
    Read,
    DecodeJson,
    DecodeValues,
    EncodeValues,
    EncodeJson,
    CreateTemporary,
    WriteTemporary,
    SyncTemporary,
    Replace,
}

#[derive(Clone)]
pub struct FileDialogPreferenceContext {
    pub path: PathBuf,
    pub resource: Option<PathBuf>,
    pub operation: FileDialogPreferenceOperation,
    pub scope: Option<String>,
    pub revisions: Arc<std::collections::BTreeMap<String, u64>>,
}
impl fmt::Debug for FileDialogPreferenceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDialogPreferenceContext")
            .field("path", &self.path)
            .field("resource", &self.resource)
            .field("operation", &self.operation)
            .field("scope", &self.scope)
            .field("revision_count", &self.revisions.len())
            .field(
                "revision",
                &self
                    .scope
                    .as_ref()
                    .and_then(|scope| self.revisions.get(scope)),
            )
            .finish()
    }
}
impl FileDialogPreferenceContext {
    pub(crate) fn frame(&self) -> ErrorFrame {
        ErrorFrame::new("file_dialog.preferences")
            .field("path", format_args!("{:?}", self.path))
            .field("resource", format_args!("{:?}", self.resource))
            .field("operation", format_args!("{:?}", self.operation))
            .field("scope", format_args!("{:?}", self.scope))
            .field("revision_count", self.revisions.len())
            .field(
                "revision",
                format_args!(
                    "{:?}",
                    self.scope
                        .as_ref()
                        .and_then(|scope| self.revisions.get(scope))
                ),
            )
    }
}

#[derive(Debug)]
pub enum FileDialogPreferenceFault {
    FileCapacity {
        actual: usize,
        limit: usize,
    },
    Schema {
        version: u32,
        scopes: usize,
    },
    ScopeCapacity {
        actual: usize,
        limit: usize,
    },
    ScopeValues {
        scope: String,
        bytes: usize,
        pinned: usize,
        filter_bytes: usize,
    },
    Sort {
        value: u8,
    },
    Path {
        path: PathBuf,
        issue: &'static str,
    },
    DuplicatePinned {
        path: PathBuf,
    },
}
impl fmt::Display for FileDialogPreferenceFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid FileDialog preferences: {self:?}")
    }
}
impl Error for FileDialogPreferenceFault {}

#[derive(Debug)]
struct NativePreferenceIo {
    operation: FileDialogPreferenceOperation,
    scope: Option<String>,
    resource: Option<PathBuf>,
    source: std::io::Error,
}
impl fmt::Display for NativePreferenceIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}
impl Error for NativePreferenceIo {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
pub(crate) fn native_preference_io(
    operation: FileDialogPreferenceOperation,
    scope: Option<&str>,
    resource: Option<PathBuf>,
    source: std::io::Error,
) -> std::io::Error {
    std::io::Error::new(
        source.kind(),
        NativePreferenceIo {
            operation,
            scope: scope.map(str::to_owned),
            resource,
            source,
        },
    )
}

#[track_caller]
pub(crate) fn preference_io(
    mut context: FileDialogPreferenceContext,
    source: std::io::Error,
) -> bevy_widgetry_job::JobFailure {
    if is_managed_io(&source) {
        let context = Arc::new(context);
        return contextual_io(source, context.frame(), |source| {
            FileDialogError::PreferenceManaged {
                context: context.clone(),
                source,
            }
        });
    }
    if let Some(native) = source
        .get_ref()
        .and_then(|source| source.downcast_ref::<NativePreferenceIo>())
    {
        context.operation = native.operation;
        context.scope = native.scope.clone();
        context.resource = native.resource.clone();
    }
    failure(
        FileDialogError::PreferenceIo {
            context: Box::new(context),
            source,
        },
        "file_dialog.preferences.io",
    )
    .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogRequestOperation {
    PrepareCandidate,
    ValidateCandidate,
    SaveParentMetadata,
    SaveTargetMetadata,
    SaveTargetLinkMetadata,
    CandidateMetadata,
    CreateDirectory,
    Locations,
    UserDirectories,
    LocationMetadata,
    PrepareSelection,
}

#[derive(Clone)]
pub struct FileDialogRequestContext {
    pub root: Option<Entity>,
    pub token: WidgetryFileDialogToken,
    pub mode: Option<WidgetryFileDialogMode>,
    pub operation: FileDialogRequestOperation,
    pub path: Option<PathBuf>,
    pub resource: Option<PathBuf>,
    pub paths: Option<std::sync::Arc<[PathBuf]>>,
    pub index: Option<usize>,
}
impl fmt::Debug for FileDialogRequestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDialogRequestContext")
            .field("root", &self.root)
            .field("token", &self.token)
            .field("mode", &self.mode)
            .field("operation", &self.operation)
            .field("path", &self.path)
            .field("resource", &self.resource)
            .field(
                "candidate_count",
                &self.paths.as_ref().map(|paths| paths.len()),
            )
            .field("index", &self.index)
            .finish()
    }
}
impl FileDialogRequestContext {
    pub(crate) fn frame(&self) -> ErrorFrame {
        ErrorFrame::new("file_dialog.request").field("request", format_args!("{self:?}"))
    }
}

#[derive(Debug)]
pub enum FileDialogRequestFault {
    LocationCapacity {
        actual: usize,
        limit: usize,
    },
    SelectionSnapshotMissing,
    SelectedEntryMissing {
        id: WidgetryFileDialogEntryId,
    },
    SelectionOutsideSnapshot {
        selected: usize,
        visible: usize,
    },
    SelectionKind {
        id: WidgetryFileDialogEntryId,
        kind: WidgetryFileDialogEntryKind,
    },
    SelectionCount {
        actual: usize,
    },
}

impl fmt::Display for FileDialogRequestFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocationCapacity { actual, limit } => {
                write!(f, "location capacity exceeded: {actual} > {limit}")
            }
            other => write!(f, "invalid candidate preparation: {other:?}"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FileDialogRequestError {
    Rejected(String),
    Canceled(&'static str),
    Failed(bevy_widgetry_job::JobFailure),
}
impl FileDialogRequestError {
    pub(crate) fn failure(&self) -> Option<&bevy_widgetry_job::JobFailure> {
        match self {
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}
impl fmt::Display for FileDialogRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(message) => f.write_str(message),
            Self::Canceled(message) => f.write_str(message),
            Self::Failed(error) => fmt::Display::fmt(error, f),
        }
    }
}

#[track_caller]
pub(crate) fn request_failure(
    context: FileDialogRequestContext,
    fault: FileDialogRequestFault,
) -> FileDialogFailure {
    failure(
        FileDialogError::Request {
            context: Box::new(context),
            fault: Box::new(fault),
        },
        "file_dialog.request",
    )
}

#[track_caller]
pub(crate) fn request_io(
    mut context: FileDialogRequestContext,
    source: std::io::Error,
) -> bevy_widgetry_job::JobFailure {
    if is_managed_io(&source) {
        let context = Arc::new(context);
        return contextual_io(source, context.frame(), |source| {
            FileDialogError::RequestManaged {
                context: context.clone(),
                source,
            }
        });
    }
    if let Some(native) = source
        .get_ref()
        .and_then(|error| error.downcast_ref::<NativeRequestIo>())
    {
        context.operation = native.operation;
        context.resource = native.path.clone();
        context.index = native.index;
    }
    failure(
        FileDialogError::RequestIo {
            context: Box::new(context),
            source,
        },
        "file_dialog.request.io",
    )
    .into()
}

fn contextual_io(
    source: std::io::Error,
    frame: ErrorFrame,
    wrap: impl Fn(Arc<dyn Diagnostic>) -> FileDialogError,
) -> bevy_widgetry_job::JobFailure {
    let report = bevy_widgetry_job::JobFailure::from(source).into_report();
    let failures = report
        .shared_diagnostics()
        .map(|source| {
            let mut contexts = source.contexts().to_vec();
            contexts.push(frame.clone());
            let diagnostic = ContextualFileDialogDiagnostic {
                origin: source.origin().clone(),
                contexts,
                kind: wrap(source),
            };
            bevy_widgetry_job::JobFailure::from(WidgetryEcsError::from_shared(Arc::new(diagnostic)))
        })
        .reduce(bevy_widgetry_job::JobFailure::combine);
    failures.unwrap_or_else(|| report.into())
}

// A string frame cannot retain typed request data on an existing failure.
// This propagation wrapper keeps the typed payload with the original lifecycle.
struct ContextualFileDialogDiagnostic {
    kind: FileDialogError,
    origin: Arc<ErrorOrigin>,
    contexts: Vec<ErrorFrame>,
}
impl fmt::Display for ContextualFileDialogDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}
impl fmt::Debug for ContextualFileDialogDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ContextualFileDialogDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}
impl Diagnostic for ContextualFileDialogDiagnostic {
    fn origin(&self) -> &Arc<ErrorOrigin> {
        &self.origin
    }
    fn contexts(&self) -> &[ErrorFrame] {
        &self.contexts
    }
}

#[derive(Debug)]
struct NativeRequestIo {
    operation: FileDialogRequestOperation,
    path: Option<PathBuf>,
    index: Option<usize>,
    source: std::io::Error,
}
impl fmt::Display for NativeRequestIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}
impl Error for NativeRequestIo {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
pub(crate) fn native_request_io(
    operation: FileDialogRequestOperation,
    path: Option<PathBuf>,
    index: Option<usize>,
    source: std::io::Error,
) -> std::io::Error {
    std::io::Error::new(
        source.kind(),
        NativeRequestIo {
            operation,
            path,
            index,
            source,
        },
    )
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
    if is_managed_io(&source) {
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

pub(crate) fn is_managed_io(error: &std::io::Error) -> bool {
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
            Self::PreferenceIo { .. } => f.write_str("FileDialog 偏好 I/O 失败"),
            Self::PreferenceManaged { context, source } => {
                write!(f, "{source} [{:?}]", context.frame())
            }
            Self::RequestManaged { context, source } => {
                write!(f, "{source} [{:?}]", context.frame())
            }
            Self::RequestIo { .. } => f.write_str("FileDialog 请求 I/O 失败"),
            Self::Request { fault, .. } => fmt::Display::fmt(fault, f),
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
            Self::PreferenceIo { source, .. } => Some(source),
            Self::PreferenceManaged { source, .. } => Some(source.as_ref()),
            Self::RequestManaged { source, .. } => Some(source.as_ref()),
            Self::RequestIo { source, .. } => Some(source),
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
