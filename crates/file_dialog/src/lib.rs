#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod api;
mod behavior;
mod candidate;
mod confirmation;
mod controls;
mod directory;
mod filesystem;
mod filter;
mod folder;
mod input;
mod locations;
mod model;
mod persistence;
mod preferences_job;
mod projection;
mod retention;
mod runtime;
mod selection;
mod selection_task;
mod snapshot;
mod storage;
mod style;
mod view;
mod window;

pub use api::{
    WidgetryFileDialog, WidgetryFileDialogChangeEvent, WidgetryFileDialogHeadlessPlugin,
    WidgetryFileDialogPlugin, WidgetryFileDialogResultEvent,
};
pub use confirmation::{WidgetryFileDialogCandidate, WidgetryFileDialogValidationJob};
pub use filesystem::{
    WidgetryFileDialogBackend, WidgetryFileDialogFileSystem, WidgetryFileDialogLocation,
    WidgetryFileDialogNativeFileSystem,
};
pub use filter::{WidgetryFileDialogFilter, WidgetryFileDialogFilterId};
pub use model::{
    WidgetryFileDialogAction, WidgetryFileDialogConfirmation, WidgetryFileDialogDirectoryState,
    WidgetryFileDialogEntry, WidgetryFileDialogEntryId, WidgetryFileDialogEntryKind,
    WidgetryFileDialogFolderRequest, WidgetryFileDialogMode, WidgetryFileDialogMove,
    WidgetryFileDialogNavigation, WidgetryFileDialogProps, WidgetryFileDialogReply,
    WidgetryFileDialogResult, WidgetryFileDialogSelection, WidgetryFileDialogSessionId,
    WidgetryFileDialogSessionState, WidgetryFileDialogSort, WidgetryFileDialogState,
    WidgetryFileDialogToken,
};
pub use runtime::{
    WidgetryFileDialogPersistence, WidgetryFileDialogRuntimeOptions,
    WidgetryFileDialogRuntimeStatus,
};
pub use selection::{WidgetryFileDialogPreparedSelection, WidgetryFileDialogSelectionJob};
pub use snapshot::{
    WidgetryFileDialogEntryData, WidgetryFileDialogQuery, WidgetryFileDialogSnapshot,
};
pub use storage::{
    WidgetryFileDialogStorage, WidgetryFileDialogStorageSnapshot, WidgetryFileDialogStorageState,
};
pub use style::WidgetryFileDialogStyle;
pub use window::{WidgetryFileDialogModality, WidgetryFileDialogWindow};

mod colors;
pub use colors::*;
