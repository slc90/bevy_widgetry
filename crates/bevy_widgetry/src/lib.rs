#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

#[cfg(feature = "job")]
pub mod job {
    pub use bevy_widgetry_job::*;
}

#[cfg(feature = "tray")]
pub mod tray {
    pub use bevy_widgetry_tray::*;
}

#[cfg(feature = "power_inhibit")]
pub mod power_inhibit {
    pub use bevy_widgetry_power_inhibit::*;
}

#[cfg(feature = "app_logging")]
pub mod app_logging {
    pub use bevy_widgetry_app_logging::{AppLogging, WorkerGuard, file_layer, terminal_layer};
}

pub mod scene {
    pub use bevy_widgetry_core::scene::{
        WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, spawn_scene,
    };
}

#[cfg(feature = "canvas")]
pub mod canvas {
    pub use bevy_widgetry_canvas::*;
}

#[cfg(feature = "button")]
pub mod button {
    pub use bevy_widgetry_button::{
        WidgetryButton, WidgetryButtonColorOverrides, WidgetryButtonLoadingChanged,
        WidgetryButtonPlugin, WidgetryButtonProps, WidgetryButtonStateColorOverrides,
        WidgetryLinkButton, WidgetryLinkButtonColorOverrides, WidgetryLinkButtonProps,
    };
}

#[cfg(feature = "combo_box")]
pub mod combo_box {
    pub use bevy_widgetry_combo_box::{
        WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxColorOverrides,
        WidgetryComboBoxContent, WidgetryComboBoxFieldColorOverrides,
        WidgetryComboBoxFieldStateColorOverrides, WidgetryComboBoxMatcher, WidgetryComboBoxPlugin,
        WidgetryComboBoxPopupColorOverrides, WidgetryComboBoxPopupStateColorOverrides,
        WidgetryComboBoxProps, WidgetryComboBoxSearchState, WidgetryListViewColorOverrides,
        WidgetryListViewContainerColorOverrides, WidgetryListViewContainerStateColorOverrides,
        WidgetryListViewItemColorOverrides, WidgetryListViewItemStateColorOverrides,
    };
}

#[cfg(feature = "radio_group")]
pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}

#[cfg(feature = "segmented")]
pub mod segmented {
    pub use bevy_widgetry_segmented::*;
}

#[cfg(feature = "tabs")]
pub mod tabs {
    pub use bevy_widgetry_tabs::*;
}

#[cfg(feature = "collapsible")]
pub mod collapsible {
    pub use bevy_widgetry_collapsible::*;
}

#[cfg(feature = "progress_bar")]
pub mod progress_bar {
    pub use bevy_widgetry_progress_bar::*;
}

#[cfg(feature = "slider")]
pub mod slider {
    pub use bevy_widgetry_slider::*;
}

#[cfg(feature = "number_input")]
pub mod number_input {
    pub use bevy_widgetry_number_input::*;
}

#[cfg(feature = "splitter")]
pub mod splitter {
    pub use bevy_widgetry_splitter::*;
}

#[cfg(feature = "scroll_area")]
pub mod scroll_area {
    pub use bevy_widgetry_scroll_area::{
        ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
        WidgetryScrollAreaColorOverrides, WidgetryScrollAreaContent, WidgetryScrollAreaPlugin,
        WidgetryScrollAreaProps, WidgetryScrollAreaViewport, WidgetryScrollAxisColorOverrides,
        WidgetryScrollIntoView, WidgetryScrollThumbColorOverrides,
        WidgetryScrollThumbStateColorOverrides, WidgetryScrollTrackColorOverrides,
        WidgetryScrollTrackStateColorOverrides,
    };
}

#[cfg(feature = "list_view")]
pub mod list_view {
    pub use bevy_widgetry_list_view::{
        WidgetryListData, WidgetryListDataSource, WidgetryListItemId, WidgetryListKey,
        WidgetryListModel, WidgetryListModelSource, WidgetryListProjection,
        WidgetryListProjectionPolicy, WidgetryListSelectionChanged, WidgetryListSelectionMode,
        WidgetryListSelectionState, WidgetryListValue, WidgetryListView, WidgetryListViewAppExt,
        WidgetryListViewColorOverrides, WidgetryListViewContainerColorOverrides,
        WidgetryListViewContainerStateColorOverrides, WidgetryListViewItem,
        WidgetryListViewItemColorOverrides, WidgetryListViewItemStateColorOverrides,
        WidgetryListViewPlugin, WidgetryListViewProps, WidgetryListViewRenderer,
        WidgetryListViewState, WidgetryListViewSystems,
    };
}

#[cfg(feature = "tree")]
pub mod tree {
    pub use bevy_widgetry_tree::{
        WidgetryListViewContainerColorOverrides, WidgetryListViewContainerStateColorOverrides,
        WidgetryListViewItemColorOverrides, WidgetryListViewItemStateColorOverrides,
        WidgetryTreeAppExt, WidgetryTreeChildrenState, WidgetryTreeColorOverrides,
        WidgetryTreeDropPlacement, WidgetryTreeEvent, WidgetryTreeEventKind,
        WidgetryTreeExpanderColorOverrides, WidgetryTreeExpanderInteractionColorOverrides,
        WidgetryTreeExpanderStateColorOverrides, WidgetryTreeIcons, WidgetryTreeModel,
        WidgetryTreeMove, WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeRenderer,
        WidgetryTreeState, WidgetryTreeView, WidgetryTreeViewProps, WidgetryTreeVisibleItem,
    };
}

#[cfg(feature = "table")]
pub mod table {
    pub use bevy_widgetry_table::WidgetryTableSubmitEdit;
    pub use bevy_widgetry_table::{
        WidgetryTable, WidgetryTableAppExt, WidgetryTableBody, WidgetryTableCell,
        WidgetryTableCellRenderer, WidgetryTableCellRendererRegistry, WidgetryTableCellValue,
        WidgetryTableColorOverrides, WidgetryTableColumn, WidgetryTableColumnHeader,
        WidgetryTableColumnHeaders, WidgetryTableColumnId, WidgetryTableColumnReordering,
        WidgetryTableColumnWidth, WidgetryTableCorner, WidgetryTableEvent, WidgetryTableEventKind,
        WidgetryTableFrozenBody, WidgetryTableFrozenColumnHeaders, WidgetryTableHeaderAction,
        WidgetryTableHeaderActionKind, WidgetryTableHeaderActions, WidgetryTableHeaderRenderer,
        WidgetryTableHeaderRendererRegistry, WidgetryTableHeaderValue, WidgetryTableLayout,
        WidgetryTableModel, WidgetryTablePlugin, WidgetryTableProps,
        WidgetryTableRegionColorOverrides, WidgetryTableRegionStyle, WidgetryTableRowHeader,
        WidgetryTableRowHeaders, WidgetryTableRowId, WidgetryTableSelection,
        WidgetryTableSelectionRange, WidgetryTableState, WidgetryTableStateColorOverrides,
        WidgetryTableStyle,
    };
    pub use bevy_widgetry_table::{
        WidgetryTableCellEditor, WidgetryTableEditRequestId, WidgetryTableEditRequested,
        WidgetryTableEditResult, WidgetryTableEditState, WidgetryTableEditing,
        WidgetryTableEditorAppExt, WidgetryTableEditorMode, WidgetryTableEditorPopup,
        WidgetryTableEditorRegistry,
    };
    pub use bevy_widgetry_table::{
        WidgetryTableCopyAppExt, WidgetryTableCopyFormatter, WidgetryTableCopyFormatterRegistry,
    };
    pub use bevy_widgetry_table::{
        WidgetryTableSort, WidgetryTableSortDirection, WidgetryTableSystems,
        WidgetryTableViewState, WidgetryTableViewUpdate,
    };
}

#[cfg(feature = "waveform")]
pub mod waveform {
    pub use bevy_widgetry_waveform::{
        ChannelView, LttbReducer, MinMaxReducer, PlanarBuffer, RangeBoundary, ReducedChannel,
        ReductionInput, ReductionStats, Waveform, WaveformAlgorithm, WaveformAlgorithmChanged,
        WaveformAmplitude, WaveformAmplitudeChanged, WaveformChannelCount,
        WaveformChannelCountChanged, WaveformChannelLayout, WaveformConfig, WaveformCursor,
        WaveformDurationChanged, WaveformGridChanged, WaveformMeasurement,
        WaveformMeasurementChanged, WaveformMetadata, WaveformOutputLength, WaveformPlayback,
        WaveformPlaybackAction, WaveformPlaybackChanged, WaveformPlaybackRequested, WaveformPlugin,
        WaveformPoint, WaveformProps, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
        WaveformReducedReadStatus, WaveformReducer, WaveformRenderPlugin, WaveformRunningChanged,
        WaveformRuntime, WaveformSessionChanged, WaveformSessionKind, WaveformSource, WaveformSpan,
        WaveformStyle, WaveformSystems, WaveformUnit, WaveformUpdateKind, WaveformUpdateStats,
        WidgetryWaveformColorOverrides, WidgetryWaveformStateColorOverrides, duration_from_frames,
        sample_boundary,
    };
}

#[cfg(feature = "check_box")]
pub mod check_box {
    pub use bevy_widgetry_check_box::{
        WidgetryCheckBox, WidgetryCheckBoxColorOverrides,
        WidgetryCheckBoxInteractionColorOverrides, WidgetryCheckBoxPlugin, WidgetryCheckBoxProps,
        WidgetryCheckBoxStateColorOverrides, WidgetryCheckState, WidgetryTriStateCheckbox,
    };
}

pub mod style {
    pub use bevy_widgetry_core::WidgetryAppExt;
    pub use bevy_widgetry_core::z_index;
}

pub mod disabled {
    pub use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;
}

pub mod cursor {
    pub use bevy_widgetry_cursor::{
        CursorLease, WidgetryCursorHint, WidgetryCursorPlugin, WidgetryCursorTheme,
    };
}

pub mod theme {
    pub use bevy_widgetry_theme::*;
}

#[cfg(feature = "tooltip")]
pub mod tooltip {
    pub use bevy_widgetry_tooltip::{
        TooltipContentFactory, WidgetryTooltip, WidgetryTooltipColorOverrides,
        WidgetryTooltipPlugin, WidgetryTooltipPopupColorOverrides, WidgetryTooltipProps,
        WidgetryTooltipStateColorOverrides,
    };
}

#[cfg(feature = "window")]
pub mod window {
    pub use bevy_widgetry_window::{
        WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowButtonColorOverrides,
        WidgetryWindowButtonStateColorOverrides, WidgetryWindowColorOverrides,
        WidgetryWindowControlsConfig, WidgetryWindowImageBackground, WidgetryWindowImageMode,
        WidgetryWindowInitialFocus, WidgetryWindowPlugin, WidgetryWindowStateColorOverrides,
        WidgetryWindowSurfaceColorOverrides, WidgetryWindowTitle, is_widgetry_window,
        owned_widgetry_window, prepare_native_window, taskbar_icon, transparent_render_creation,
        widgetry_window, widgetry_window_target,
    };
}

#[cfg(feature = "message_box")]
pub mod message_box {
    pub use bevy_widgetry_message_box::{
        WidgetryButtonColorOverrides, WidgetryButtonStateColorOverrides, WidgetryMessageBox,
        WidgetryMessageBoxBodyColorOverrides, WidgetryMessageBoxBodyStateColorOverrides,
        WidgetryMessageBoxButtons, WidgetryMessageBoxColorOverrides, WidgetryMessageBoxPlugin,
        WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent,
        WidgetryWindowButtonColorOverrides, WidgetryWindowButtonStateColorOverrides,
        WidgetryWindowColorOverrides, WidgetryWindowStateColorOverrides,
        WidgetryWindowSurfaceColorOverrides, widgetry_message_box,
    };
}

#[cfg(feature = "file_dialog")]
pub mod file_dialog {
    pub use bevy_widgetry_file_dialog::{
        WidgetryButtonColorOverrides, WidgetryButtonStateColorOverrides,
        WidgetryCheckBoxColorOverrides, WidgetryCheckBoxInteractionColorOverrides,
        WidgetryCheckBoxStateColorOverrides, WidgetryComboBoxColorOverrides,
        WidgetryComboBoxFieldColorOverrides, WidgetryComboBoxFieldStateColorOverrides,
        WidgetryComboBoxPopupColorOverrides, WidgetryComboBoxPopupStateColorOverrides,
        WidgetryFileDialog, WidgetryFileDialogAction, WidgetryFileDialogBackend,
        WidgetryFileDialogBodyColorOverrides, WidgetryFileDialogBodyStateColorOverrides,
        WidgetryFileDialogCandidate, WidgetryFileDialogChangeEvent,
        WidgetryFileDialogColorOverrides, WidgetryFileDialogConfirmation,
        WidgetryFileDialogDirectoryState, WidgetryFileDialogEntry,
        WidgetryFileDialogEntryColorOverrides, WidgetryFileDialogEntryData,
        WidgetryFileDialogEntryId, WidgetryFileDialogEntryKind,
        WidgetryFileDialogEntryStateColorOverrides, WidgetryFileDialogFileSystem,
        WidgetryFileDialogFilter, WidgetryFileDialogFilterId, WidgetryFileDialogFolderRequest,
        WidgetryFileDialogHeadlessPlugin, WidgetryFileDialogLocation, WidgetryFileDialogModality,
        WidgetryFileDialogMode, WidgetryFileDialogMove, WidgetryFileDialogNativeFileSystem,
        WidgetryFileDialogNavigation, WidgetryFileDialogPersistence, WidgetryFileDialogPlugin,
        WidgetryFileDialogPreparedSelection, WidgetryFileDialogProps, WidgetryFileDialogQuery,
        WidgetryFileDialogReply, WidgetryFileDialogResult, WidgetryFileDialogResultEvent,
        WidgetryFileDialogRuntimeOptions, WidgetryFileDialogRuntimeStatus,
        WidgetryFileDialogSelection, WidgetryFileDialogSelectionJob, WidgetryFileDialogSessionId,
        WidgetryFileDialogSessionState, WidgetryFileDialogSnapshot, WidgetryFileDialogSort,
        WidgetryFileDialogState, WidgetryFileDialogStatusColorOverrides,
        WidgetryFileDialogStatusStateColorOverrides, WidgetryFileDialogStorage,
        WidgetryFileDialogStorageSnapshot, WidgetryFileDialogStorageState, WidgetryFileDialogStyle,
        WidgetryFileDialogToken, WidgetryFileDialogValidationJob, WidgetryFileDialogWindow,
        WidgetryListViewColorOverrides, WidgetryListViewContainerColorOverrides,
        WidgetryListViewContainerStateColorOverrides, WidgetryListViewItemColorOverrides,
        WidgetryListViewItemStateColorOverrides, WidgetryMessageBoxBodyColorOverrides,
        WidgetryMessageBoxBodyStateColorOverrides, WidgetryMessageBoxColorOverrides,
        WidgetryScrollAreaColorOverrides, WidgetryScrollAxisColorOverrides,
        WidgetryScrollThumbColorOverrides, WidgetryScrollThumbStateColorOverrides,
        WidgetryScrollTrackColorOverrides, WidgetryScrollTrackStateColorOverrides,
        WidgetryTextFieldColorOverrides, WidgetryTextFieldInteractionColorOverrides,
        WidgetryTextFieldStateColorOverrides, WidgetryWindowButtonColorOverrides,
        WidgetryWindowButtonStateColorOverrides, WidgetryWindowColorOverrides,
        WidgetryWindowStateColorOverrides, WidgetryWindowSurfaceColorOverrides,
    };
}

#[cfg(feature = "text_field")]
pub mod text_field {
    pub use bevy_widgetry_text_field::{
        AutoSize, WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldColorOverrides,
        WidgetryTextFieldInteractionColorOverrides, WidgetryTextFieldPlugin,
        WidgetryTextFieldPrefix, WidgetryTextFieldProps, WidgetryTextFieldStateColorOverrides,
        WidgetryTextFieldSuffix,
    };
}

pub mod icon {
    pub use bevy_widgetry_core::icon::{
        WidgetryIcon, WidgetryIconColorOverrides, WidgetryIconPlugin, WidgetryIconProps,
        WidgetryIconStateColorOverrides,
    };
}

pub mod text {
    pub use bevy_widgetry_core::text::*;
}

#[cfg(feature = "hold_button")]
pub mod hold_button {
    pub use bevy_widgetry_hold_button::*;
}

#[cfg(feature = "toggle_button")]
pub mod toggle_button {
    pub use bevy_widgetry_toggle_button::*;
}
