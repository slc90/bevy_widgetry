use super::TextKey;

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::FileFilterLabel => "{label} ({suffixes})",
        TextKey::WidgetGallery => "Widget Gallery",
        TextKey::Button => "Button",
        TextKey::CheckBox => "CheckBox",
        TextKey::ComboBox => "ComboBox",
        TextKey::ScrollArea => "ScrollArea",
        TextKey::ListView => "ListView",
        TextKey::Tree => "Tree",
        TextKey::Table => "Table",
        TextKey::TextField => "TextField",
        TextKey::Tooltip => "Tooltip",
        TextKey::Window => "Window",
        TextKey::Waveform => "Waveform",
        TextKey::Cursor => "Cursor",
        TextKey::RadioGroup => "RadioGroup",
        TextKey::ReadOnly => "ReadOnly",
        TextKey::Normal => "Normal",
        TextKey::Disabled => "Disabled",
        TextKey::RadioButton => "Radio Button",
        TextKey::Horizontal => "Horizontal",
        TextKey::Grid => "Grid",
        TextKey::Text => "Text",
        TextKey::IconText => "Icon + Text",
        TextKey::TextIcon => "Text + Icon",
        TextKey::Icon => "Icon",
        TextKey::ReceiveUpdates => "Receive updates",
        TextKey::Unchecked => "Unchecked",
        TextKey::Checked => "Checked",
        TextKey::Indeterminate => "Indeterminate",
        TextKey::TriState => "Tri-State",
        TextKey::TriStateCheckBox => "Tri-State CheckBox",
        TextKey::Renderers => "Renderers",
        TextKey::DataDriven => "Data-Driven",
        TextKey::InsertMoveOrEditSelectionKeepsIts => {
            "Insert, move or edit: selection keeps its identity. Remove selected: Field becomes empty.\nBanana starts disabled. Disabled items stay visible but cannot be selected. Program selection and clear notify on change."
        }
        TextKey::Height80290 => "Height: 80 / 290",
        TextKey::ToggleContent => "Toggle content",
        TextKey::ShowTarget => "Show target",
        TextKey::Basic => "Basic",
        TextKey::RichContent => "Rich Content",
        TextKey::Placement => "Placement",
        TextKey::BasicTooltip => "Basic tooltip",
        TextKey::HoverMe => "Hover me",
        TextKey::RichTooltip => "Rich tooltip",
        TextKey::IconAndMultipleLines => "Icon and multiple lines",
        TextKey::RichContent2 => "Rich content",
        TextKey::DisabledControlsStillHaveTooltips => "Disabled controls still have tooltips",
        TextKey::EnableDisable => "Enable / Disable",
        TextKey::SelectLastApi => "Select last (API)",
        TextKey::FileDialog => "File Dialog",
        TextKey::ModalBlocksMainWindow => "Modal (blocks main window)",
        TextKey::ChooseFilesOrFoldersTypeAFile => {
            "Choose files or folders. Type a file name to open; use File type to filter extensions. Nonmodal dialogs can stay open together."
        }
        TextKey::Result => "Result: ...",
        TextKey::IndependentWindow => "Independent Window",
        TextKey::Theme => "Theme",
        TextKey::ThisIsAnIndependentWidgetryWindow => "This is an independent Widgetry window.",
        TextKey::ClickMe => "Click me",
        TextKey::Clicked => "Clicked!",
        TextKey::MessageBox => "MessageBox",
        TextKey::ChooseAResultBelow => "Choose a result below.",
        TextKey::ContentButtonKeepsDialogOpen => "Content button (keeps dialog open)",
        TextKey::ManagedText => "Managed text",
        TextKey::NativeFixedText => "Native fixed text",
        TextKey::TextInsideButton => "Text inside Button",
        TextKey::IconInsideButton => "Icon inside Button",
        TextKey::HoverPress => "Hover / Press",
        TextKey::Apple => "Apple",
        TextKey::Banana => "Banana",
        TextKey::Orange => "Orange",
        TextKey::Grape => "Grape",
        TextKey::Pear => "Pear",
        TextKey::Star => "Star",
        TextKey::Logo => "Logo",
        TextKey::Inserted => "Inserted",
        TextKey::CheckMe => "Check me",
        TextKey::ScrollHoverVerticalThumb => "Scroll / hover vertical thumb",
        TextKey::TooltipReadableThemeForeground => "Tooltip: readable Theme foreground",
        TextKey::HoverForTooltip => "Hover for tooltip",
        TextKey::InsertFirst => "Insert first",
        TextKey::RemoveSelected => "Remove selected",
        TextKey::MoveSelectedLast => "Move selected → last",
        TextKey::EditSelected => "Edit selected",
        TextKey::EnableDisableFirst => "Enable/disable first",
        TextKey::SetSelectedFirst => "Set selected first",
        TextKey::ClearSelection => "Clear selection",
        TextKey::ResetModel => "Reset model",
        TextKey::ListSharedText => "Shared Model A: text",
        TextKey::ListSharedIcon => "Shared Model B: icon + text",
        TextKey::ListSharedDescription => {
            "Shared data: edit / insert / remove / move. Selection, navigation, scroll and enable / disable stay independent."
        }
        TextKey::SmallList => "Small List",
        TextKey::ArrowHomeEndActiveSpaceEnterSelect => {
            "Arrow/Home/End: active; Space/Enter: select. Click text or row."
        }
        TextKey::VirtualizedLargeList => "Virtualized Large List",
        TextKey::Text10000ItemsWheelPageupPagedownJump => {
            "10,000 items; wheel/PageUp/PageDown; jump, edit, shrink and resize."
        }
        TextKey::DisabledListview => "Disabled ListView",
        TextKey::UserInputBlockedModelSelectionAndScroll => {
            "User input blocked; model, selection and scroll buttons remain available."
        }
        TextKey::DisabledListviewitem => "Disabled ListViewItem",
        TextKey::InitiallyItems13DisabledArrowClick => {
            "Initially items 1/3 disabled. Arrow/click can activate; Space/Enter cannot select."
        }
        TextKey::SetSelected => "Set selected",
        TextKey::EditVisible => "Edit visible",
        TextKey::RemoveFirst => "Remove first",
        TextKey::MoveFirstLast => "Move first → last",
        TextKey::ClearRepopulate => "Clear + repopulate",
        TextKey::Edit7500 => "Edit #7500",
        TextKey::ShrinkTo3 => "Shrink to 3",
        TextKey::ResizeViewport => "Resize viewport",
        TextKey::Scroll96Px => "Scroll +96px",
        TextKey::EnableDisable1 => "Enable/disable #1",
        TextKey::VerticalAuto => "Vertical Auto",
        TextKey::WheelYPageupPagedownHomeEnd => "wheel Y / PageUp / PageDown / Home / End",
        TextKey::HorizontalAuto => "Horizontal Auto",
        TextKey::WheelXLeftRight => "wheel X / Left / Right",
        TextKey::BothAuto => "Both Auto",
        TextKey::Always => "Always",
        TextKey::Hidden => "Hidden",
        TextKey::AutoTransition => "Auto transition",
        TextKey::Scrollintoview => "ScrollIntoView",
        TextKey::Text300290Content => "300 × 290 content",
        TextKey::FitsViewport => "Fits viewport",
        TextKey::ScrollMeWithoutABar => "Scroll me without a bar",
        TextKey::Before1 => "Before 1",
        TextKey::Before2 => "Before 2",
        TextKey::Before3 => "Before 3",
        TextKey::Before4 => "Before 4",
        TextKey::Target => "Target",
        TextKey::AfterTarget => "After target",
        TextKey::SelectFirstApi => "Select first (API)",
        TextKey::ClearSelectionApi => "Clear selection (API)",
        TextKey::UpdateFirstRow => "Update first row",
        TextKey::ReplaceFirstHeader => "Replace first header",
        TextKey::Cellvalue => "CellValue",
        TextKey::Header => "Header",
        TextKey::Selection => "Selection",
        TextKey::ColumnLayout => "Column Layout",
        TextKey::Virtualization => "Virtualization",
        TextKey::RowNumbersAndColumnHeadersStayFixed => {
            "Row numbers and column headers stay fixed on their own axis. Click any cell to select."
        }
        TextKey::StringNumberBoolProgressAndIconUse => {
            "String, Number, Bool, Progress and Icon use distinct renderers. Update first row to refresh its content."
        }
        TextKey::CustomHeadersCombineAStarAndLabel => {
            "Custom headers combine a star and label. Header and Cell content use independent renderers."
        }
        TextKey::ClickAColumnHeaderRowNumberOr => {
            "Click a column header, row number or cell. Arrow keys move the focus border without changing selection."
        }
        TextKey::NameStays180PxWideNumberAndBool => {
            "Name stays 180px wide; Number and Bool share remaining width at 2:1. Drag a header's right edge to resize."
        }
        TextKey::Text2000Rows40ColumnsScrollHorizontally => {
            "2,000 rows × 40 columns. Scroll horizontally and vertically; row/column ranges and visible cell count follow the viewport."
        }
        TextKey::InitiallyDisabledEnableToSelectAndNavigate => {
            "Initially disabled. Enable to select and navigate, then disable again: selection stays while input and hover stop."
        }
        TextKey::BasicTree => "Basic Tree",
        TextKey::TreeSharedA => "Shared TreeModel A",
        TextKey::TreeSharedB => "Shared TreeModel B: shorter viewport",
        TextKey::TreeSharedDescription => {
            "Expansion and selection are shared; scroll and enable / disable stay independent. Select a file to edit, remove or move between folder and root. Disabled views still receive data updates."
        }
        TextKey::TreeSharedFolder => "Shared folder",
        TextKey::TreeSharedFile => "File {index} (edits {edits})",
        TextKey::TreeInsertFile => "Insert file",
        TextKey::TreeReparentSelected => "Move file: folder / root",
        TextKey::ClickArrowsToExpandClickContentTo => {
            "Click arrows to expand; click content to select. Arrow/Home/End + Space/Enter navigate."
        }
        TextKey::EcsHeterogeneousTree => "ECS Heterogeneous Tree",
        TextKey::FolderAndFileComponentsDispatchDifferentRenderers => {
            "Folder and File Components dispatch different renderers. Business data stays on ECS nodes."
        }
        TextKey::LazyLoadingTree => "Lazy Loading Tree",
        TextKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse => {
            "Expand Remote folder: unloaded → loading → loaded. Collapse/re-expand keeps one request."
        }
        TextKey::VirtualizedTree => "Virtualized Tree",
        TextKey::Expand10000FilesWheelPagedownScroll => {
            "Expand 10,000 files; wheel/PageDown scroll. Only viewport rows exist."
        }
        TextKey::Workspace => "Workspace",
        TextKey::Projects => "Projects",
        TextKey::Widgetry => "Widgetry",
        TextKey::Photos => "Photos",
        TextKey::Notes => "Notes",
        TextKey::Assets => "Assets",
        TextKey::RemoteFolderUnloaded => "Remote folder (unloaded)",
        TextKey::RemoteFolderLoading => "Remote folder (loading...)",
        TextKey::RemoteFolderLoaded => "Remote folder (loaded)",
        TextKey::Text10000Files => "10,000 files",
        TextKey::OpenFile => "Open File",
        TextKey::OpenFiles => "Open Files",
        TextKey::SelectFolder => "Select Folder",
        TextKey::SelectFolders => "Select Folders",
        TextKey::OpenImage => "Open Image",
        TextKey::SaveFile => "Save File",
        TextKey::Images => "Images",
        TextKey::TextDocuments => "Text documents",
        TextKey::Background1 => "Background 1",
        TextKey::Background2 => "Background 2",
        TextKey::Stretch => "Stretch",
        TextKey::Cover => "Cover",
        TextKey::Cover50 => "Cover 50%",
        TextKey::Ok => "OK",
        TextKey::YesNo => "Yes / No",
        TextKey::YesNoCancel => "Yes / No / Cancel",
        TextKey::MessageBoxDemo => "MessageBox Demo",
        TextKey::Dark => "Dark",
        TextKey::Light => "Light",
        TextKey::PinkDream => "Pink Dream",
        TextKey::KamuriViolet => "Kamuri Violet",
        TextKey::Center => "Center",
        TextKey::Right => "Right",
        TextKey::Bottom => "Bottom",
        TextKey::WaitingForWaveformPage => "Waiting for Waveform page",
        TextKey::BasicLive100Hz4Channels5 => "Basic Live · 100 Hz · 4 channels · 5 s · Polyline",
        TextKey::BasicReplay8Khz4Channels5 => {
            "Basic Replay · 8 kHz · 4 channels · 5 s · fixed .wfrm · Envelope"
        }
        TextKey::StressLive64Khz64Channels10 => "Stress Live · 64 kHz · 64 channels · 10 s",
        TextKey::CursorRealWindowsMouseValidation => "Cursor · Real Windows Mouse validation",
        TextKey::Pointer12Frames8Fps64PxKeep => {
            "Pointer: 12 frames / 8 FPS / 64px. Keep the real mouse still to observe the loop."
        }
        TextKey::Wait24Frames8Fps64PxThe => {
            "Wait: 24 frames / 8 FPS / 64px. The loading ring rotates while Kamuri occasionally eats pudding."
        }
        TextKey::Text42StaticCursors48PxPointerAndWait => {
            "42 static cursors: 48px; Pointer and Wait: 64px. Compare each preview and semantic. BRP Custom Pointer does not change the OS cursor."
        }
        TextKey::WidgetryExtendedSemanticsKamuriSystemComparison => {
            "Widgetry extended semantics · Kamuri / System comparison"
        }
        TextKey::NestedNodeContentRootButtonDisabledAffects => {
            "Nested Node content: root Button Disabled affects WidgetryText / Icon; native fixed Text stays teal."
        }
        TextKey::ToggleRootButtonDisabled => "Toggle root Button Disabled",
        TextKey::DisabledContainersAffectDescendantsThroughPlainNodes => {
            "Disabled containers affect descendants through plain Nodes. After enabling the parent, only locally disabled CheckBoxes stay disabled."
        }
        TextKey::InheritParentDisabled => "Inherit parent Disabled",
        TextKey::LocallyDisabledStaysDisabledAfterParentIs => {
            "Locally Disabled; stays disabled after parent is enabled"
        }
        TextKey::ToggleContainerDisabled => "Toggle container Disabled",
        TextKey::PureTheme => "Pure Theme",
        TextKey::OverridesViaPublicApi => "Overrides via public API",
        TextKey::CurrentThemeParentEnabled => "Current: Theme; parent enabled",
        TextKey::OtherColorsFollowThemeClearOnlyAffects => {
            "Other colors follow Theme. Clear only affects the open override instance."
        }
        TextKey::ApplySampleOverrides => "Apply sample overrides",
        TextKey::ClearOverrides => "Clear overrides",
        TextKey::ToggleParentDisabled => "Toggle parent Disabled",
        TextKey::OpenPureTheme => "Open pure Theme",
        TextKey::OpenOverrideInstance => "Open override instance",
        TextKey::CycleTheme => "Cycle Theme",
        TextKey::ThemeSourceValuesPartStateFieldSwatch => {
            "Theme source values: part / state / field / swatch / HEX (including alpha), rather than final widget colors."
        }
        TextKey::RealWidgetsPureThemeOnTheLeft => {
            "Real widgets: pure Theme on the left, selected overrides on the right. Apply / clear through public API; clearing restores current Theme."
        }
        TextKey::YGutterInducesXOverflowNoCorner => "Y gutter induces X overflow; no Corner",
        TextKey::KeepBarAndGutterEvenWithoutOverflow => "Keep bar and gutter even without overflow",
        TextKey::ScrollableContentWithNoVisibleBar => "Scrollable content with no visible bar",
        TextKey::ToggleContentHeightToShowOrHide => "Toggle content height to show or hide the bar",
        TextKey::ScrollThenClickTheButtonToAlign => {
            "Scroll, then click the button to align Target to the top"
        }
        TextKey::FixedColorConvenienceApi0F766ERetainedAcross => {
            "Fixed color convenience API=#0F766E; retained across Theme and parent Disabled changes. Clear to inherit Button foreground."
        }
        TextKey::OnlyFieldOpenBorder7C3AedOpenPopup => {
            "Only field.open.border=#7C3AED; open Popup, list and indicator still use Theme."
        }
        TextKey::OnlyItemSelectedBackground0F766EMovePointer => {
            "Only item.selected.background=#0F766E; move pointer away after selection, active border stays unchanged."
        }
        TextKey::OnlyItemSelectedBackground0F766EMovePointer2 => {
            "Only item.selected.background=#0F766E; move pointer away after selection, expander still uses Theme."
        }
        TextKey::OnlyCellSelectedBackground0F766EMovePointer => {
            "Only cell.selected.background=#0F766E; move pointer away after selection, focus border stays unchanged."
        }
        TextKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D => {
            "Only normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D]; Disabled palette still uses Theme."
        }
        TextKey::OnlyHoveredBackground7C3AedRealHoverPressed => {
            "Only hovered.background=#7C3AED; real hover, Pressed still uses Theme."
        }
        TextKey::OnlyOptionCheckedNormalDot0F766EMove => {
            "Only option.checked.normal.dot=#0F766E; move pointer away after selection."
        }
        TextKey::OnlyCheckedNormalMark0F766EMovePointer => {
            "Only checked.normal.mark=#0F766E; move pointer away after Checked, Hover/Disabled still use Theme."
        }
        TextKey::OnlyEditableFocusedBorder7C3AedClickTo => {
            "Only editable.focused.border=#7C3AED; click to focus and type."
        }
        TextKey::OnlyReadOnlyFocusedBorder7C3AedSelect => {
            "Only read_only.focused.border=#7C3AED; select / copy allowed, editing blocked."
        }
        TextKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal => {
            "Only vertical.thumb.hovered.background=#7C3AED; horizontal thumb unchanged."
        }
        TextKey::OnlyPopupNormalBackground7C3AedRealHover => {
            "Only popup.normal.background=#7C3AED; real hover shows Tooltip, including on disabled anchors."
        }
        TextKey::WindowOnlyFrameNormalBorder7C3AedClear => {
            "Window: only frame.normal.border=#7C3AED; clear directly on the open instance."
        }
        TextKey::WindowImageSameBorderOverrideCover50 => {
            "Window Image: same border override; Cover / 50% opacity survive Theme changes and clear."
        }
        TextKey::MessageboxOnlyBodyNormalBackground0F766EApply => {
            "MessageBox: only body.normal.background=#0F766E; apply / clear and Theme switching inside modal."
        }
        TextKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal => {
            "FileDialog: only entry.selected.background=#0F766E; NonModal, clear after selecting a file without reopening."
        }
        TextKey::None => "<none>",
        TextKey::ComboStatus => {
            "Items: {count} | selected: {selected} | index: {index} | value: {label}\nFirst item disabled: {disabled} | ValueChange: {changes}"
        }
        TextKey::ListItem => "Item {index}  (edits: {edits})",
        TextKey::ListInsertedItem => "Inserted item  (edits: {edits})",
        TextKey::ListStatus => {
            "Model items: {count} | Rendered rows: {rows} | Rendered range: {start}..{end}\nselected: {selected} | active: {active} | ValueChange: {changes}"
        }
        TextKey::TreeFolder => "[Folder] {label}",
        TextKey::TreeFile => "[File] {label}  ({bytes} bytes)",
        TextKey::TreeStatus => {
            "Visible: {visible} | rows: {rows} {start}..{end} | selected: {selected} | {state}"
        }
        TextKey::Enabled => "enabled",
        TextKey::TableItem => "Item {index}",
        TextKey::TableUpdated => "Updated {edits}",
        TextKey::TableColumn => "Column {index}",
        TextKey::TableUpdatedHeader => "Updated header",
        TextKey::TableName => "Name",
        TextKey::TableNumber => "Number",
        TextKey::TableBool => "Bool",
        TextKey::TableProgress => "Progress",
        TextKey::TableStatus => {
            "Model: {rows} rows × {columns} columns | visible cells: {cells} | rows {row_start}..{row_end}, columns {column_start}..{column_end}\nselection: {selection} | cursor: {cursor} | scroll: {scroll} | {state} | events: select {select}, resize start/change/end/cancel {start}/{change}/{end}/{cancel}"
        }
        TextKey::TooltipCenter => "Center placement",
        TextKey::TooltipRight => "Right placement",
        TextKey::TooltipBottom => "Bottom placement",
        TextKey::ExpandThemeColors => "Expand / collapse Theme colors",
        TextKey::ExpandOverrides => "Expand / collapse explicit overrides",
        TextKey::ButtonActivations => "Button Activate: {count}",
        TextKey::OverrideStatus => "Last operation: {action}; changed: {changed}",
        TextKey::CursorTheme => "Current Cursor theme: {theme}",
        TextKey::WaveformStatus => {
            "64 kHz · 64 ch · 10 s · producer {producer} / displayed {displayed} · {fps} FPS / {ms} ms · dropped {dropped}"
        }
        TextKey::ResultWaiting => "Result: Waiting...",
        TextKey::ResultFailed => "Result: Could not open dialog",
        TextKey::ResultCancelled => "Result: Cancelled",
        TextKey::ResultPaths => "Result: {count} path(s){paths}",
        TextKey::WindowImageTitle => "Background {index} - {mode}",
        TextKey::OverrideWindowTitle => "{kind} · {sample}",
        TextKey::OverrideInstance => "Override instance",
        TextKey::V1 => "V 1",
        TextKey::V2 => "V 2",
        TextKey::V3 => "V 3",
        TextKey::V4 => "V 4",
        TextKey::V5 => "V 5",
        TextKey::V6 => "V 6",
        TextKey::H1 => "H 1",
        TextKey::H2 => "H 2",
        TextKey::H3 => "H 3",
        TextKey::H4 => "H 4",
        TextKey::H5 => "H 5",
    }
}
