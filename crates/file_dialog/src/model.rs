use crate::{
    WidgetryFileDialogFilter, WidgetryFileDialogFilterId, WidgetryFileDialogStorageSnapshot,
    WidgetryFileDialogStorageState,
};
use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetryFileDialogSessionId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetryFileDialogEntryId {
    pub(crate) session: WidgetryFileDialogSessionId,
    pub(crate) serial: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryFileDialogToken {
    pub session: WidgetryFileDialogSessionId,
    pub generation: u64,
    pub query_revision: u64,
    pub edit_revision: u64,
    pub selection_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetryFileDialogEntry {
    pub(crate) id: WidgetryFileDialogEntryId,
    pub(crate) path: PathBuf,
    pub(crate) name: OsString,
    pub(crate) kind: WidgetryFileDialogEntryKind,
    pub(crate) size: Option<u64>,
    pub(crate) modified: Option<std::time::SystemTime>,
    pub(crate) hidden: Option<bool>,
    pub(crate) system: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogProps {
    pub mode: WidgetryFileDialogMode,
    pub initial_directory: Option<PathBuf>,
    pub fallback_directory: Option<PathBuf>,
    pub storage_scope: Option<String>,
    pub filters: Vec<WidgetryFileDialogFilter>,
    pub allow_different_extension: bool,
    pub window: Option<crate::WidgetryFileDialogWindow>,
}

#[derive(Component, Clone)]
pub struct WidgetryFileDialogState {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) mode: WidgetryFileDialogMode,
    pub(crate) session_state: WidgetryFileDialogSessionState,
    pub(crate) result: Option<WidgetryFileDialogResult>,
    pub(crate) directory_state: WidgetryFileDialogDirectoryState,
    pub(crate) requested_path: Option<PathBuf>,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) fallback: Option<PathBuf>,
    pub(crate) history: Arc<Vec<PathBuf>>,
    pub(crate) history_cursor: Option<usize>,
    pub(crate) navigation: WidgetryFileDialogNavigation,
    pub(crate) entries: Arc<im::Vector<WidgetryFileDialogEntry>>,
    pub(crate) visible: Arc<im::Vector<WidgetryFileDialogEntryId>>,
    pub(crate) selected: Arc<BTreeSet<WidgetryFileDialogEntryId>>,
    pub(crate) active: Option<WidgetryFileDialogEntryId>,
    pub(crate) anchor: Option<WidgetryFileDialogEntryId>,
    pub(crate) confirmation: WidgetryFileDialogConfirmation,
    pub(crate) filename: OsString,
    pub(crate) filters: Vec<WidgetryFileDialogFilter>,
    pub(crate) search: String,
    pub(crate) preferences: WidgetryFileDialogStorageSnapshot,
    pub(crate) storage_scope: Option<String>,
    pub(crate) storage_state: WidgetryFileDialogStorageState,
    pub(crate) allow_different_extension: bool,
    pub(crate) error: Option<String>,
    pub(crate) snapshot: Option<Arc<crate::snapshot::WidgetryFileDialogSnapshot>>,
    pub(crate) projection_pending: bool,
    pub(crate) candidate: Option<WidgetryFileDialogResult>,
    pub(crate) selection_job: Option<crate::selection::WidgetryFileDialogSelectionJob>,
    pub(crate) validation_job: Option<crate::confirmation::WidgetryFileDialogValidationJob>,
    pub(crate) activate_after_selection: bool,
    pub(crate) folder_request: Option<WidgetryFileDialogFolderRequest>,
    pub(crate) reveal_path: Option<PathBuf>,
    pub(crate) started_generation: Option<u64>,
    pub(crate) locations: Arc<[crate::WidgetryFileDialogLocation]>,
    pub(crate) location_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogFolderRequest {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryFileDialogMode {
    #[default]
    PickFile,
    PickFiles,
    PickDirectory,
    PickDirectories,
    SaveFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogEntryKind {
    File,
    Directory,
    SymlinkFile,
    SymlinkDirectory,
    DanglingSymlink,
    Unknown,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogSessionState {
    Open,
    Resolved,
    Closing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogDirectoryState {
    Loading,
    Ready,
    Partial {
        errors: usize,
        summary: Arc<[String]>,
    },
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogConfirmation {
    Idle,
    Validating,
    AwaitingOverwrite {
        path: PathBuf,
        token: WidgetryFileDialogToken,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogResult {
    File(PathBuf),
    Files(Arc<[PathBuf]>),
    Directory(PathBuf),
    Directories(Arc<[PathBuf]>),
    SavePath(PathBuf),
    Cancelled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryFileDialogSort {
    #[default]
    NameAscending,
    NameDescending,
    SizeAscending,
    ModifiedDescending,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogNavigation {
    Path(PathBuf),
    Refresh,
    Back,
    Forward,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogSelection {
    Replace,
    Toggle,
    Range,
    ExtendRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogAction {
    Reopen,
    Navigate(WidgetryFileDialogNavigation),
    Filter(WidgetryFileDialogFilterId),
    Search(String),
    Sort(WidgetryFileDialogSort),
    ShowHidden(bool),
    ShowSystem(bool),
    Select {
        id: WidgetryFileDialogEntryId,
        operation: WidgetryFileDialogSelection,
        token: WidgetryFileDialogToken,
    },
    ClearSelection,
    SelectAll,
    Active(Option<WidgetryFileDialogEntryId>),
    Activate {
        id: WidgetryFileDialogEntryId,
        token: WidgetryFileDialogToken,
    },
    Filename(OsString),
    Confirm,
    Overwrite {
        token: WidgetryFileDialogToken,
        accept: bool,
    },
    Cancel,
    Pin(PathBuf),
    Unpin(PathBuf),
    NewFolder(OsString),
    MoveActive(WidgetryFileDialogMove),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryFileDialogMove {
    Previous,
    Next,
    First,
    Last,
    PageUp(usize),
    PageDown(usize),
}

#[derive(Clone, Debug)]
pub enum WidgetryFileDialogReply {
    Started {
        token: WidgetryFileDialogToken,
        path: PathBuf,
    },
    Snapshot {
        snapshot: Arc<crate::WidgetryFileDialogSnapshot>,
        state: WidgetryFileDialogDirectoryState,
        selection: crate::WidgetryFileDialogPreparedSelection,
    },
    Failed {
        token: WidgetryFileDialogToken,
        error: String,
    },
    Validated {
        candidate: crate::WidgetryFileDialogCandidate,
        exists: bool,
    },
    ValidationFailed {
        token: WidgetryFileDialogToken,
        error: String,
    },
    Selection(crate::selection::WidgetryFileDialogPreparedSelection),
    FolderCreated {
        request: WidgetryFileDialogFolderRequest,
        outcome: Result<(), String>,
    },
    Locations {
        token: WidgetryFileDialogToken,
        outcome: Result<Vec<crate::WidgetryFileDialogLocation>, String>,
    },
}

pub(crate) fn contract_error(message: &str) -> BevyError {
    widgetry_error!(error = message, "FileDialog 拒绝无效操作");
    BevyError::error(message.to_owned())
}

impl Default for WidgetryFileDialogProps {
    fn default() -> Self {
        Self {
            mode: WidgetryFileDialogMode::PickFile,
            initial_directory: None,
            fallback_directory: None,
            storage_scope: None,
            filters: vec![WidgetryFileDialogFilter::default()],
            allow_different_extension: true,
            window: None,
        }
    }
}

impl WidgetryFileDialogState {
    pub fn new(mut props: WidgetryFileDialogProps) -> Result<Self, BevyError> {
        if props
            .storage_scope
            .as_ref()
            .is_some_and(|scope| scope.is_empty() || scope.len() > 128)
        {
            return Err(contract_error("storage scope must contain 1..128 bytes"));
        }
        if props.filters.len() > 128 {
            return Err(contract_error("filter capacity exceeded"));
        }
        let mut ids = BTreeSet::new();
        for filter in &props.filters {
            if !ids.insert(filter.id.clone()) {
                return Err(contract_error("duplicate FilterId"));
            }
            filter.validate()?;
        }
        if !props
            .filters
            .iter()
            .any(|filter| filter.suffixes.is_empty())
        {
            props.filters.push(WidgetryFileDialogFilter::default());
        }
        let preferences = WidgetryFileDialogStorageSnapshot {
            filter: props
                .filters
                .first()
                .map(|filter| filter.id.clone())
                .unwrap_or_default(),
            ..Default::default()
        };
        let serial = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| contract_error("session ID exhausted"))?;
        Ok(Self {
            token: WidgetryFileDialogToken {
                session: WidgetryFileDialogSessionId(serial),
                generation: 1,
                query_revision: 1,
                edit_revision: 1,
                selection_revision: 1,
            },
            mode: props.mode,
            session_state: WidgetryFileDialogSessionState::Open,
            result: None,
            directory_state: WidgetryFileDialogDirectoryState::Loading,
            requested_path: props.initial_directory,
            current_path: None,
            fallback: props.fallback_directory,
            history: Arc::new(Vec::new()),
            history_cursor: None,
            navigation: WidgetryFileDialogNavigation::Refresh,
            entries: Arc::new(im::Vector::new()),
            visible: Arc::new(im::Vector::new()),
            selected: Arc::new(BTreeSet::new()),
            active: None,
            anchor: None,
            confirmation: WidgetryFileDialogConfirmation::Idle,
            filename: OsString::new(),
            filters: props.filters,
            search: String::new(),
            preferences,
            storage_scope: props.storage_scope,
            storage_state: WidgetryFileDialogStorageState::Memory,
            allow_different_extension: props.allow_different_extension,
            error: None,
            snapshot: None,
            projection_pending: false,
            candidate: None,
            selection_job: None,
            validation_job: None,
            activate_after_selection: false,
            folder_request: None,
            reveal_path: None,
            started_generation: None,
            locations: Arc::from([]),
            location_error: None,
        })
    }

    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn mode(&self) -> WidgetryFileDialogMode {
        self.mode
    }

    pub fn session_state(&self) -> WidgetryFileDialogSessionState {
        self.session_state
    }

    pub fn result(&self) -> Option<&WidgetryFileDialogResult> {
        self.result.as_ref()
    }

    pub fn directory_state(&self) -> &WidgetryFileDialogDirectoryState {
        &self.directory_state
    }

    pub fn requested_path(&self) -> Option<&Path> {
        self.requested_path.as_deref()
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    pub fn entries(&self) -> &im::Vector<WidgetryFileDialogEntry> {
        &self.entries
    }

    pub fn visible(&self) -> &im::Vector<WidgetryFileDialogEntryId> {
        &self.visible
    }

    pub fn selected(&self) -> &BTreeSet<WidgetryFileDialogEntryId> {
        &self.selected
    }

    pub fn active(&self) -> Option<WidgetryFileDialogEntryId> {
        self.active
    }

    pub fn anchor(&self) -> Option<WidgetryFileDialogEntryId> {
        self.anchor
    }

    pub fn confirmation(&self) -> &WidgetryFileDialogConfirmation {
        &self.confirmation
    }

    pub fn filename(&self) -> &std::ffi::OsStr {
        &self.filename
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn filters(&self) -> &[WidgetryFileDialogFilter] {
        &self.filters
    }

    pub fn preferences(&self) -> &WidgetryFileDialogStorageSnapshot {
        &self.preferences
    }

    pub fn storage_state(&self) -> &WidgetryFileDialogStorageState {
        &self.storage_state
    }

    pub fn locations(&self) -> &[crate::WidgetryFileDialogLocation] {
        &self.locations
    }

    pub fn location_error(&self) -> Option<&str> {
        self.location_error.as_deref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn query(&self) -> crate::WidgetryFileDialogQuery {
        crate::WidgetryFileDialogQuery {
            filter: self
                .filters
                .iter()
                .find(|filter| filter.id == self.preferences.filter)
                .cloned()
                .unwrap_or_default(),
            search: self.search.clone(),
            sort: self.preferences.sort,
            show_hidden: self.preferences.show_hidden,
            show_system: self.preferences.show_system,
        }
    }

    pub fn snapshot(&self) -> Option<&Arc<crate::WidgetryFileDialogSnapshot>> {
        self.snapshot.as_ref()
    }

    pub fn validation_candidate(&self) -> Option<&WidgetryFileDialogResult> {
        self.candidate.as_ref()
    }

    pub fn storage_scope(&self) -> Option<&str> {
        self.storage_scope.as_deref()
    }

    pub fn fallback_directory(&self) -> Option<&Path> {
        self.fallback.as_deref()
    }

    pub fn history(&self) -> &[PathBuf] {
        &self.history
    }

    pub fn history_cursor(&self) -> Option<usize> {
        self.history_cursor
    }

    pub fn projection_pending(&self) -> bool {
        self.projection_pending
    }

    pub fn selection_pending(&self) -> bool {
        self.selection_job.is_some()
    }

    pub fn selection_job(&self) -> Option<crate::selection::WidgetryFileDialogSelectionJob> {
        self.selection_job.clone()
    }

    pub fn validation_job(&self) -> Option<crate::confirmation::WidgetryFileDialogValidationJob> {
        self.validation_job.clone()
    }

    pub fn folder_request(&self) -> Option<WidgetryFileDialogFolderRequest> {
        self.folder_request.clone()
    }

    pub fn reveal_path(&self) -> Option<&Path> {
        self.reveal_path.as_deref()
    }

    pub(crate) fn initialize_storage(&mut self, snapshot: WidgetryFileDialogStorageSnapshot) {
        if self.requested_path.is_none() {
            self.requested_path = snapshot.last_visited_dir.clone();
        }
        let available = self
            .filters
            .iter()
            .any(|filter| filter.id == snapshot.filter);
        let default_filter = self.preferences.filter.clone();
        self.preferences = snapshot;
        if self.preferences.unavailable_paths > 0 {
            self.storage_state = WidgetryFileDialogStorageState::Failed(
                "stored paths from another platform are unavailable".into(),
            );
        }
        if !available {
            self.preferences.filter = default_filter;
            self.storage_state = WidgetryFileDialogStorageState::Failed(
                "stored filter is unavailable; using configured default".into(),
            );
        }
    }

    pub(crate) fn resolve(&mut self, result: WidgetryFileDialogResult) -> bool {
        if self.session_state != WidgetryFileDialogSessionState::Open {
            return false;
        }
        self.result = Some(result);
        self.session_state = WidgetryFileDialogSessionState::Resolved;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.selection_job = None;
        self.validation_job = None;
        self.folder_request = None;
        true
    }
}

impl WidgetryFileDialogFolderRequest {
    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl WidgetryFileDialogEntry {
    pub fn id(&self) -> WidgetryFileDialogEntryId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    pub fn kind(&self) -> WidgetryFileDialogEntryKind {
        self.kind
    }

    pub fn size(&self) -> Option<u64> {
        self.size
    }

    pub fn modified(&self) -> Option<std::time::SystemTime> {
        self.modified
    }

    pub fn hidden(&self) -> Option<bool> {
        self.hidden
    }

    pub fn system(&self) -> Option<bool> {
        self.system
    }
}

impl WidgetryFileDialogEntryKind {
    pub fn is_directory(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkDirectory)
    }

    pub fn is_file(self) -> bool {
        matches!(self, Self::File | Self::SymlinkFile)
    }
}

impl WidgetryFileDialogMode {
    pub fn is_multiple(self) -> bool {
        matches!(self, Self::PickFiles | Self::PickDirectories)
    }

    pub fn accepts(self, kind: WidgetryFileDialogEntryKind) -> bool {
        match self {
            Self::PickFile | Self::PickFiles => kind.is_file(),
            Self::PickDirectory | Self::PickDirectories => kind.is_directory(),
            Self::SaveFile => false,
        }
    }
}

#[cfg(test)]
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! session 的 Open/Resolved 与 result once-only 是本轮局部 contract。
    //! stimulus 为业务 resolve，重复决议必须保留首次 result。
    use super::*;

    #[test]
    fn session_ids_are_unique_and_first_result_wins() {
        let mut first = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let second = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        assert_ne!(first.token().session, second.token().session);
        assert!(first.resolve(WidgetryFileDialogResult::Cancelled));
        assert!(!first.resolve(WidgetryFileDialogResult::File("late".into())));
        assert_eq!(first.result(), Some(&WidgetryFileDialogResult::Cancelled));
        assert_eq!(
            first.session_state(),
            WidgetryFileDialogSessionState::Resolved
        );
    }
}
