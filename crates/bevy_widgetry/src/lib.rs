//! 提供 Widgetry 的公共入口，供应用构造基础 Widget、数据视图及窗口组合界面。
//! 调用方可按所需功能使用各公开 module 中的 Widget、Plugin、Model 和 state API。
//!
//! 提供 Button、CheckBox、RadioGroup、TextField、ComboBox 和 Tooltip 的构造与交互入口。
//! 提供 ScrollArea、ListView、Tree 和 Table，用于浏览滚动内容、列表、层级数据和二维数据。
//! 提供 Waveform，用于显示多 channel 的时间序列数据。
//! 提供 Window、MessageBox 和 FileDialog，用于构造自定义窗口、modal 结果与后台文件选择交互。
//! theme module 提供完整 Light/Dark 配色与主题切换入口。
//! text 与 icon module 提供内容颜色覆盖，标记文字和 Icon 可跟随 Widget 状态与 Theme。
//! style module 提供默认字体设置和 z-index 标识。
//! disabled module 提供 hierarchy 中的只读实际禁用结果。
//! icon module 提供 SVG Icon 的 Scene 构造、颜色设置与运行时替换入口。
//! scene module 提供 deferred Scene 构造与应用的扩展方法，将失败交给宿主 error handler。
//! 同步 spawn_scene 返回 Result，并在构造失败时回收 root 与空预约 entity。
//! window::taskbar_icon! 接收应用 package 相对 PNG 路径，在编译期嵌入图标并返回可失败的 Plugin。
//! 运行时在主窗口就绪后设置窗口与任务栏图标一次，不需要分发原 PNG。
//! EXE 的 ICO 由应用 build script 调用独立的 bevy_widgetry_app_icon_build::set_exe_icon 嵌入。
//!
//! 各 Widget 通过自己的 Plugin 或类型注册入口启用对应功能。
//! Props 用于 Scene 的一次性构造，运行时 state 通过对应 Widget API 更新和查询。
//! selection、focus、disabled 与变化通知的具体含义遵循各 Widget 的公开契约。
//! 应用可以组合多个 Widget，并自行组织内容、layout 和业务响应。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

pub mod scene {
    pub use bevy_widgetry_core::scene::{
        WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, spawn_scene,
    };
}

pub mod button {
    pub use bevy_widgetry_button::{
        WidgetryButton, WidgetryButtonColorOverrides, WidgetryButtonPlugin, WidgetryButtonProps,
        WidgetryButtonStateColorOverrides,
    };
}

pub mod combo_box {
    pub use bevy_widgetry_combo_box::{
        WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxColorOverrides,
        WidgetryComboBoxFieldColorOverrides, WidgetryComboBoxFieldStateColorOverrides,
        WidgetryComboBoxPlugin, WidgetryComboBoxPopupColorOverrides,
        WidgetryComboBoxPopupStateColorOverrides, WidgetryComboBoxProps,
        WidgetryListViewColorOverrides, WidgetryListViewContainerColorOverrides,
        WidgetryListViewContainerStateColorOverrides, WidgetryListViewItemColorOverrides,
        WidgetryListViewItemStateColorOverrides,
    };
}

pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}

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

pub mod list_view {
    pub use bevy_widgetry_list_view::{
        WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewAppExt,
        WidgetryListViewColorOverrides, WidgetryListViewContainerColorOverrides,
        WidgetryListViewContainerStateColorOverrides, WidgetryListViewItem,
        WidgetryListViewItemColorOverrides, WidgetryListViewItemStateColorOverrides,
        WidgetryListViewPlugin, WidgetryListViewProps, WidgetryListViewRenderer,
        WidgetryListViewState, WidgetryListViewSystems,
    };
}

pub mod tree {
    pub use bevy_widgetry_tree::{
        WidgetryListViewContainerColorOverrides, WidgetryListViewContainerStateColorOverrides,
        WidgetryListViewItemColorOverrides, WidgetryListViewItemStateColorOverrides,
        WidgetryTreeAppExt, WidgetryTreeChildrenState, WidgetryTreeColorOverrides,
        WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeExpanderColorOverrides,
        WidgetryTreeExpanderInteractionColorOverrides, WidgetryTreeExpanderStateColorOverrides,
        WidgetryTreeIcons, WidgetryTreeModel, WidgetryTreeNode, WidgetryTreePlugin,
        WidgetryTreeRenderer, WidgetryTreeState, WidgetryTreeView, WidgetryTreeViewProps,
        WidgetryTreeVisibleItem,
    };
}

pub mod table {
    pub use bevy_widgetry_table::{
        WidgetryTable, WidgetryTableAppExt, WidgetryTableBody, WidgetryTableCell,
        WidgetryTableCellRenderer, WidgetryTableCellRendererRegistry, WidgetryTableCellValue,
        WidgetryTableColorOverrides, WidgetryTableColumn, WidgetryTableColumnHeader,
        WidgetryTableColumnHeaders, WidgetryTableColumnId, WidgetryTableColumnWidth,
        WidgetryTableCorner, WidgetryTableEvent, WidgetryTableEventKind,
        WidgetryTableHeaderRenderer, WidgetryTableHeaderRendererRegistry, WidgetryTableHeaderValue,
        WidgetryTableLayout, WidgetryTableModel, WidgetryTablePlugin, WidgetryTableProps,
        WidgetryTableRegionColorOverrides, WidgetryTableRegionStyle, WidgetryTableRowHeader,
        WidgetryTableRowHeaders, WidgetryTableRowId, WidgetryTableSelection, WidgetryTableState,
        WidgetryTableStateColorOverrides, WidgetryTableStyle,
    };
}

pub mod waveform {
    pub use bevy_widgetry_waveform::{
        ChannelView, MinMaxReducer, PlanarBuffer, RangeBoundary, ReducedChannel, ReductionInput,
        ReductionStats, Waveform, WaveformConfig, WaveformCursor, WaveformOutputLength,
        WaveformPlugin, WaveformPoint, WaveformProps, WaveformReadError, WaveformReducer,
        WaveformRenderPlugin, WaveformRuntime, WaveformSource, WaveformSpan, WaveformStyle,
        WaveformSystems, WaveformUpdateKind, WaveformUpdateStats, WidgetryWaveformColorOverrides,
        WidgetryWaveformStateColorOverrides, duration_from_frames, sample_boundary,
    };
}

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

pub mod theme {
    pub use bevy_widgetry_theme::*;
}

pub mod tooltip {
    pub use bevy_widgetry_tooltip::{
        TooltipContentFactory, WidgetryTooltip, WidgetryTooltipColorOverrides,
        WidgetryTooltipPlugin, WidgetryTooltipPopupColorOverrides, WidgetryTooltipProps,
        WidgetryTooltipStateColorOverrides,
    };
}

pub mod window {
    pub use bevy_widgetry_window::{
        WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowButtonColorOverrides,
        WidgetryWindowButtonStateColorOverrides, WidgetryWindowColorOverrides,
        WidgetryWindowControlsConfig, WidgetryWindowImageBackground, WidgetryWindowImageMode,
        WidgetryWindowInitialFocus, WidgetryWindowPlugin, WidgetryWindowStateColorOverrides,
        WidgetryWindowSurfaceColorOverrides, is_widgetry_window, owned_widgetry_window,
        prepare_native_window, taskbar_icon, transparent_render_creation, widgetry_window,
        widgetry_window_target,
    };
}

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

pub mod text_field {
    pub use bevy_widgetry_text_field::{
        WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldColorOverrides,
        WidgetryTextFieldInteractionColorOverrides, WidgetryTextFieldPlugin,
        WidgetryTextFieldProps, WidgetryTextFieldStateColorOverrides,
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
