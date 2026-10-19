use super::TextKey;
use super::{ButtonKey, CheckBoxKey, ComboBoxKey, ScrollAreaKey, TextFieldKey, TooltipKey};
use super::{ColorShowcaseKey, CommonKey, HeaderKey, NavigationKey};
use super::{CursorKey, WaveformKey, WindowKey};
use super::{ListViewKey, TableKey, TreeKey};

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::Button(ButtonKey::HoldTitle) => "Hold to confirm · 64 / 112 / 160 px",
        TextKey::Button(ButtonKey::HoldDescription) => {
            "Hold for 3 s; early release decays over 1.2 s. Enter / Space; F7 local disabled, F8 parent disabled, F9 visibility."
        }
        TextKey::Button(ButtonKey::HoldConfirm) => "Confirm",
        TextKey::Button(ButtonKey::HoldFocus) => "Focus 112px button",
        TextKey::Button(ButtonKey::HoldBlur) => "Move focus away",
        TextKey::Button(ButtonKey::HoldDisable) => "Toggle disabled (F7)",
        TextKey::Button(ButtonKey::HoldParentDisable) => "Parent disabled (F8)",
        TextKey::Button(ButtonKey::HoldVisibility) => "Hide / show (F9)",
        TextKey::Button(ButtonKey::HoldUnderlay) => "Transparent corners: underlay",
        TextKey::Button(ButtonKey::HoldOverrides) => {
            "Liquid color overrides; clear restores current theme"
        }
        TextKey::Button(ButtonKey::LoadingDescription) => {
            "Loading blocks repeated activation. Finish with the button or F1 to observe the Wait → Pointer cursor change."
        }
        TextKey::Button(ButtonKey::StartLoading) => "Start Loading",
        TextKey::Button(ButtonKey::FinishLoading) => "Finish Loading",
        TextKey::Button(ButtonKey::LinkButton) => "LinkButton",
        TextKey::Button(ButtonKey::LinkDescription) => {
            "Open an HTTP / HTTPS URL in the default browser. Hover or press to underline; disabled links cannot open."
        }
        TextKey::Button(ButtonKey::SwitchUrl) => "Switch URL",
        TextKey::Button(ButtonKey::ProgrammaticUrl) => {
            "Switch between Baidu and Google without opening a browser. Only activating the link opens its current URL."
        }
        TextKey::Button(ButtonKey::LinkOverrideDescription) => {
            "Override only the hovered link color (#7C3AED). Pressed and disabled states follow the current theme."
        }
        TextKey::Window(WindowKey::FileFilterLabel) => "{label} ({suffixes})",
        TextKey::GalleryHeader(HeaderKey::WidgetGallery) => "Widget Gallery",
        TextKey::Navigation(NavigationKey::Button) => "Button",
        TextKey::Navigation(NavigationKey::CheckBox) => "CheckBox",
        TextKey::Navigation(NavigationKey::ComboBox) => "ComboBox",
        TextKey::Navigation(NavigationKey::ScrollArea) => "ScrollArea",
        TextKey::Navigation(NavigationKey::ListView) => "ListView",
        TextKey::Navigation(NavigationKey::Tree) => "Tree",
        TextKey::Navigation(NavigationKey::Table) => "Table",
        TextKey::Navigation(NavigationKey::TextField) => "TextField",
        TextKey::Navigation(NavigationKey::Tooltip) => "Tooltip",
        TextKey::Navigation(NavigationKey::Window) => "Window",
        TextKey::Navigation(NavigationKey::Waveform) => "Waveform",
        TextKey::Navigation(NavigationKey::Cursor) => "Cursor",
        TextKey::Button(ButtonKey::RadioGroup) => "RadioGroup",
        TextKey::Common(CommonKey::ReadOnly) => "ReadOnly",
        TextKey::Common(CommonKey::Normal) => "Normal",
        TextKey::Common(CommonKey::Disabled) => "Disabled",
        TextKey::Button(ButtonKey::RadioButton) => "Radio Button",
        TextKey::Button(ButtonKey::Horizontal) => "Horizontal",
        TextKey::Button(ButtonKey::Grid) => "Grid",
        TextKey::Common(CommonKey::Text) => "Text",
        TextKey::Common(CommonKey::IconText) => "Icon + Text",
        TextKey::Button(ButtonKey::TextIcon) => "Text + Icon",
        TextKey::Common(CommonKey::Icon) => "Icon",
        TextKey::CheckBox(CheckBoxKey::ReceiveUpdates) => "Receive updates",
        TextKey::CheckBox(CheckBoxKey::Unchecked) => "Unchecked",
        TextKey::CheckBox(CheckBoxKey::Checked) => "Checked",
        TextKey::CheckBox(CheckBoxKey::Indeterminate) => "Indeterminate",
        TextKey::CheckBox(CheckBoxKey::TriState) => "Tri-State",
        TextKey::CheckBox(CheckBoxKey::TriStateCheckBox) => "Tri-State CheckBox",
        TextKey::ComboBox(ComboBoxKey::LocalSearch) => "Local search (shared data)",
        TextKey::ComboBox(ComboBoxKey::SearchPrompt) => "Search",
        TextKey::ComboBox(ComboBoxKey::NoResults) => "No matching options",
        TextKey::ComboBox(ComboBoxKey::SearchStatus) => {
            "Query: {query} | Matches: {matches} | Selected ID: {selected} | Active: {active}"
        }
        TextKey::ComboBox(ComboBoxKey::SearchInstructions) => {
            "Search fruit names in English or Chinese. Arrow keys browse; Enter confirms; Esc cancels. Reopening clears the query. The disabled search example blocks input."
        }
        TextKey::ComboBox(ComboBoxKey::Placeholder) => "Choose an option…",
        TextKey::ComboBox(ComboBoxKey::Renderers) => "Renderers",
        TextKey::ComboBox(ComboBoxKey::DataDriven) => "Data-Driven",
        TextKey::ComboBox(ComboBoxKey::InsertMoveOrEditSelectionKeepsIts) => {
            "Starts unselected with a placeholder. Insert, move or edit preserves identity. Remove selected or clear restores the placeholder without selecting another item.\nBanana starts disabled. Disabled items stay visible but cannot be selected. Program selection and clear notify on change."
        }
        TextKey::ScrollArea(ScrollAreaKey::Height80290) => "Height: 80 / 290",
        TextKey::ScrollArea(ScrollAreaKey::ToggleContent) => "Toggle content",
        TextKey::ScrollArea(ScrollAreaKey::ShowTarget) => "Show target",
        TextKey::Common(CommonKey::Basic) => "Basic",
        TextKey::Tooltip(TooltipKey::RichContent) => "Rich Content",
        TextKey::Tooltip(TooltipKey::Placement) => "Placement",
        TextKey::Tooltip(TooltipKey::BasicTooltip) => "Basic tooltip",
        TextKey::Tooltip(TooltipKey::HoverMe) => "Hover me",
        TextKey::Tooltip(TooltipKey::RichTooltip) => "Rich tooltip",
        TextKey::Tooltip(TooltipKey::IconAndMultipleLines) => "Icon and multiple lines",
        TextKey::Tooltip(TooltipKey::RichContent2) => "Rich content",
        TextKey::Tooltip(TooltipKey::DisabledControlsStillHaveTooltips) => {
            "Disabled controls still have tooltips"
        }
        TextKey::Common(CommonKey::EnableDisable) => "Enable / Disable",
        TextKey::Tree(TreeKey::SelectLastApi) => "Select last (API)",
        TextKey::Tree(TreeKey::SelectVisible) => "Select visible (Ctrl+A)",
        TextKey::Tree(TreeKey::ExpandAll) => "Expand loaded",
        TextKey::Tree(TreeKey::CollapseAll) => "Collapse all",
        TextKey::Window(WindowKey::FileDialog) => "File Dialog",
        TextKey::Window(WindowKey::ModalBlocksMainWindow) => "Modal (blocks main window)",
        TextKey::Window(WindowKey::ChooseFilesOrFoldersTypeAFile) => {
            "Choose files or folders. Type a file name to open; use File type to filter extensions. Nonmodal dialogs can stay open together."
        }
        TextKey::Window(WindowKey::Result) => "Result: ...",
        TextKey::Window(WindowKey::IndependentWindow) => "Independent Window",
        TextKey::Window(WindowKey::Theme) => "Theme",
        TextKey::Window(WindowKey::ThisIsAnIndependentWidgetryWindow) => {
            "This is an independent Widgetry window."
        }
        TextKey::Window(WindowKey::ClickMe) => "Click me",
        TextKey::Window(WindowKey::Clicked) => "Clicked!",
        TextKey::Window(WindowKey::MessageBox) => "MessageBox",
        TextKey::Window(WindowKey::ChooseAResultBelow) => "Choose a result below.",
        TextKey::Window(WindowKey::ContentButtonKeepsDialogOpen) => {
            "Content button (keeps dialog open)"
        }
        TextKey::Button(ButtonKey::ManagedText) => "Managed text",
        TextKey::Button(ButtonKey::NativeFixedText) => "Native fixed text",
        TextKey::Button(ButtonKey::TextInsideButton) => "Text inside Button",
        TextKey::Button(ButtonKey::IconInsideButton) => "Icon inside Button",
        TextKey::Button(ButtonKey::HoverPress) => "Hover / Press",
        TextKey::Common(CommonKey::Apple) => "Apple",
        TextKey::Common(CommonKey::Banana) => "Banana",
        TextKey::Common(CommonKey::Orange) => "Orange",
        TextKey::Common(CommonKey::Grape) => "Grape",
        TextKey::ComboBox(ComboBoxKey::Pear) => "Pear",
        TextKey::ComboBox(ComboBoxKey::Star) => "Star",
        TextKey::ComboBox(ComboBoxKey::Logo) => "Logo",
        TextKey::ComboBox(ComboBoxKey::Inserted) => "Inserted",
        TextKey::CheckBox(CheckBoxKey::CheckMe) => "Check me",
        TextKey::ScrollArea(ScrollAreaKey::ScrollHoverVerticalThumb) => {
            "Scroll / hover vertical thumb"
        }
        TextKey::Tooltip(TooltipKey::TooltipReadableThemeForeground) => {
            "Tooltip: readable Theme foreground"
        }
        TextKey::Tooltip(TooltipKey::HoverForTooltip) => "Hover for tooltip",
        TextKey::Common(CommonKey::InsertFirst) => "Insert first",
        TextKey::Common(CommonKey::RemoveSelected) => "Remove selected",
        TextKey::ComboBox(ComboBoxKey::MoveSelectedLast) => "Move selected → last",
        TextKey::Common(CommonKey::EditSelected) => "Edit selected",
        TextKey::ComboBox(ComboBoxKey::EnableDisableFirst) => "Enable/disable first",
        TextKey::ComboBox(ComboBoxKey::SetSelectedFirst) => "Set selected first",
        TextKey::Common(CommonKey::ClearSelection) => "Clear selection",
        TextKey::ComboBox(ComboBoxKey::ResetModel) => "Reset model",
        TextKey::ListView(ListViewKey::ListSharedText) => "Shared Model A: text",
        TextKey::ListView(ListViewKey::ListSharedIcon) => "Shared Model B: icon + text",
        TextKey::ListView(ListViewKey::ListSharedDescription) => {
            "B reverses even labels. Drag one item before/after a row to reorder the shared Model; multi-selection is preserved. Ctrl/Shift select; each view keeps its own selection."
        }
        TextKey::ListView(ListViewKey::SmallList) => "Small List",
        TextKey::ListView(ListViewKey::ArrowHomeEndActiveSpaceEnterSelect) => {
            "Drag enabled: move one item at a time. Click selects one; Ctrl toggles; Shift selects a range. Arrow/Home/End: active; Space/Enter: confirm."
        }
        TextKey::ListView(ListViewKey::VirtualizedLargeList) => "Virtualized Large List",
        TextKey::ListView(ListViewKey::Text10000ItemsWheelPageupPagedownJump) => {
            "Drag disabled. 10,000 items: Ctrl+A shows the full selection count. Scroll, PageUp/PageDown, edit and resize."
        }
        TextKey::ListView(ListViewKey::DisabledListview) => "Disabled ListView",
        TextKey::ListView(ListViewKey::UserInputBlockedModelSelectionAndScroll) => {
            "User input blocked; model, selection and scroll buttons remain available."
        }
        TextKey::ListView(ListViewKey::DisabledListviewitem) => "Disabled ListViewItem",
        TextKey::ListView(ListViewKey::InitiallyItems13DisabledArrowClick) => {
            "Items 1/3 disabled: excluded from clicks, Shift ranges and Ctrl+A. Arrow/click can still activate."
        }
        TextKey::ListView(ListViewKey::SetSelected) => "Set selected",
        TextKey::ListView(ListViewKey::EditVisible) => "Edit visible",
        TextKey::Common(CommonKey::RemoveFirst) => "Remove first",
        TextKey::Common(CommonKey::MoveFirstLast) => "Move first → last",
        TextKey::ListView(ListViewKey::ClearRepopulate) => "Clear + repopulate",
        TextKey::ListView(ListViewKey::Edit7500) => "Edit #7500",
        TextKey::ListView(ListViewKey::ShrinkTo3) => "Shrink to 3",
        TextKey::Common(CommonKey::ResizeViewport) => "Resize viewport",
        TextKey::ListView(ListViewKey::Scroll96Px) => "Scroll +96px",
        TextKey::ListView(ListViewKey::EnableDisable1) => "Enable/disable #1",
        TextKey::ScrollArea(ScrollAreaKey::VerticalAuto) => "Vertical Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelYPageupPagedownHomeEnd) => {
            "wheel Y / PageUp / PageDown / Home / End"
        }
        TextKey::ScrollArea(ScrollAreaKey::HorizontalAuto) => "Horizontal Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelXLeftRight) => "wheel X / Left / Right",
        TextKey::ScrollArea(ScrollAreaKey::BothAuto) => "Both Auto",
        TextKey::ScrollArea(ScrollAreaKey::Always) => "Always",
        TextKey::ScrollArea(ScrollAreaKey::Hidden) => "Hidden",
        TextKey::ScrollArea(ScrollAreaKey::AutoTransition) => "Auto transition",
        TextKey::ScrollArea(ScrollAreaKey::Scrollintoview) => "ScrollIntoView",
        TextKey::ScrollArea(ScrollAreaKey::Text300290Content) => "300 × 290 content",
        TextKey::ScrollArea(ScrollAreaKey::FitsViewport) => "Fits viewport",
        TextKey::ScrollArea(ScrollAreaKey::ScrollMeWithoutABar) => "Scroll me without a bar",
        TextKey::ScrollArea(ScrollAreaKey::Before1) => "Before 1",
        TextKey::ScrollArea(ScrollAreaKey::Before2) => "Before 2",
        TextKey::ScrollArea(ScrollAreaKey::Before3) => "Before 3",
        TextKey::ScrollArea(ScrollAreaKey::Before4) => "Before 4",
        TextKey::ScrollArea(ScrollAreaKey::Target) => "Target",
        TextKey::ScrollArea(ScrollAreaKey::AfterTarget) => "After target",
        TextKey::Table(TableKey::SelectFirstApi) => "Select first (API)",
        TextKey::Table(TableKey::ClearSelectionApi) => "Clear selection (API)",
        TextKey::Table(TableKey::UpdateFirstRow) => "Update first row",
        TextKey::Table(TableKey::TableShared) => "Shared Model",
        TextKey::Table(TableKey::TableSharedA) => "Shared Model A: 180 px columns, 34 px rows",
        TextKey::Table(TableKey::TableSharedB) => {
            "Shared Model B: 120 px columns, 28 px rows, compact padding"
        }
        TextKey::Table(TableKey::TableSharedDescription) => {
            "One Model and Renderer registry; independent layout, column widths, selection, cursor, scroll and disabled state. Edit, insert, remove or move rows from either side."
        }
        TextKey::Table(TableKey::TableInsertRow) => "Insert first row",
        TextKey::Table(TableKey::ReplaceFirstHeader) => "Replace first header",
        TextKey::Table(TableKey::Cellvalue) => "CellValue",
        TextKey::Table(TableKey::Header) => "Header",
        TextKey::Table(TableKey::Selection) => "Selection",
        TextKey::Table(TableKey::ColumnLayout) => "Column Layout",
        TextKey::Table(TableKey::Virtualization) => "Virtualization",
        TextKey::Table(TableKey::RowNumbersAndColumnHeadersStayFixed) => {
            "Row numbers and column headers stay fixed on their own axis. Click any cell to select."
        }
        TextKey::Table(TableKey::StringNumberBoolProgressAndIconUse) => {
            "String, Number, Bool, Progress and Icon use distinct renderers. Update first row to refresh its content."
        }
        TextKey::Table(TableKey::CustomHeadersCombineAStarAndLabel) => {
            "Custom headers combine a star and label. Header and Cell content use independent renderers."
        }
        TextKey::Table(TableKey::ClickAColumnHeaderRowNumberOr) => {
            "Click a column header, row number or cell. Arrow keys move the focus border without changing selection."
        }
        TextKey::Table(TableKey::NameStays180PxWideNumberAndBool) => {
            "Name stays 180px wide; Number and Bool share remaining width at 2:1. Drag a header's right edge to resize."
        }
        TextKey::Table(TableKey::Text2000Rows40ColumnsScrollHorizontally) => {
            "2,000 rows × 40 columns. Scroll horizontally and vertically; row/column ranges and visible cell count follow the viewport."
        }
        TextKey::Table(TableKey::InitiallyDisabledEnableToSelectAndNavigate) => {
            "Initially disabled. Enable to select and navigate, then disable again: selection stays while input and hover stop."
        }
        TextKey::Tree(TreeKey::BasicTree) => "Basic Tree",
        TextKey::Tree(TreeKey::TreeSharedA) => "Shared TreeModel A",
        TextKey::Tree(TreeKey::TreeSharedB) => "Shared TreeModel B: shorter viewport",
        TextKey::Tree(TreeKey::TreeSharedDescription) => {
            "Expansion, selection and scroll are independent per view; original tree data is shared. Ctrl toggles, Shift selects a visible range, Ctrl+A adds visible nodes and keeps hidden selections. Edit/remove/move require one selected file. Disabled views still receive data updates."
        }
        TextKey::Tree(TreeKey::TreeSharedFolder) => "Shared folder",
        TextKey::Tree(TreeKey::TreeSharedFile) => "File {index} (edits {edits})",
        TextKey::Tree(TreeKey::TreeInsertFile) => "Insert file",
        TextKey::Tree(TreeKey::TreeReparentSelected) => "Move file: folder / root",
        TextKey::Tree(TreeKey::ClickArrowsToExpandClickContentTo) => {
            "Click arrows to expand; click content to select. Arrow/Home/End + Space/Enter navigate."
        }
        TextKey::Tree(TreeKey::EcsHeterogeneousTree) => "ECS Heterogeneous Tree",
        TextKey::Tree(TreeKey::FolderAndFileComponentsDispatchDifferentRenderers) => {
            "Folder and File Components dispatch different renderers. Business data stays on ECS nodes."
        }
        TextKey::Tree(TreeKey::LazyLoadingTree) => "Lazy Loading Tree",
        TextKey::Tree(TreeKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse) => {
            "Expand Remote folder: unloaded → loading → loaded. Collapse/re-expand keeps one request."
        }
        TextKey::Tree(TreeKey::VirtualizedTree) => "Virtualized Tree",
        TextKey::Tree(TreeKey::Expand10000FilesWheelPagedownScroll) => {
            "Expand 10,000 files; wheel/PageDown scroll. Only viewport rows exist."
        }
        TextKey::Tree(TreeKey::Workspace) => "Workspace",
        TextKey::Tree(TreeKey::Projects) => "Projects",
        TextKey::Tree(TreeKey::Widgetry) => "Widgetry",
        TextKey::Tree(TreeKey::Photos) => "Photos",
        TextKey::Tree(TreeKey::Notes) => "Notes",
        TextKey::Tree(TreeKey::Assets) => "Assets",
        TextKey::Tree(TreeKey::RemoteFolderUnloaded) => "Remote folder (unloaded)",
        TextKey::Tree(TreeKey::RemoteFolderLoading) => "Remote folder (loading...)",
        TextKey::Tree(TreeKey::RemoteFolderLoaded) => "Remote folder (loaded)",
        TextKey::Tree(TreeKey::Text10000Files) => "10,000 files",
        TextKey::Window(WindowKey::OpenFile) => "Open File",
        TextKey::Window(WindowKey::OpenFiles) => "Open Files",
        TextKey::Window(WindowKey::SelectFolder) => "Select Folder",
        TextKey::Window(WindowKey::SelectFolders) => "Select Folders",
        TextKey::Window(WindowKey::OpenImage) => "Open Image",
        TextKey::Window(WindowKey::SaveFile) => "Save File",
        TextKey::Window(WindowKey::Images) => "Images",
        TextKey::Window(WindowKey::TextDocuments) => "Text documents",
        TextKey::Window(WindowKey::Background1) => "Background 1",
        TextKey::Window(WindowKey::Background2) => "Background 2",
        TextKey::Window(WindowKey::Stretch) => "Stretch",
        TextKey::Window(WindowKey::Cover) => "Cover",
        TextKey::Window(WindowKey::Cover50) => "Cover 50%",
        TextKey::Window(WindowKey::Ok) => "OK",
        TextKey::Window(WindowKey::YesNo) => "Yes / No",
        TextKey::Window(WindowKey::YesNoCancel) => "Yes / No / Cancel",
        TextKey::Window(WindowKey::MessageBoxDemo) => "MessageBox Demo",
        TextKey::GalleryHeader(HeaderKey::Dark) => "Dark",
        TextKey::GalleryHeader(HeaderKey::Light) => "Light",
        TextKey::GalleryHeader(HeaderKey::PinkDream) => "Pink Dream",
        TextKey::GalleryHeader(HeaderKey::KamuriViolet) => "Kamuri Violet",
        TextKey::Tooltip(TooltipKey::Center) => "Center",
        TextKey::Tooltip(TooltipKey::Right) => "Right",
        TextKey::Tooltip(TooltipKey::Bottom) => "Bottom",
        TextKey::Waveform(WaveformKey::WaitingForWaveformPage) => "Waiting for Waveform page",
        TextKey::Waveform(WaveformKey::BasicLive100Hz4Channels5) => {
            "Basic Live · 100 Hz · 4 channels · 5 s · Polyline"
        }
        TextKey::Waveform(WaveformKey::BasicReplay8Khz4Channels5) => {
            "Basic Replay · 8 kHz · 4 channels · 5 s · fixed .wfrm · Envelope"
        }
        TextKey::Waveform(WaveformKey::StressLive64Khz64Channels10) => {
            "Stress Live · 64 kHz · 64 channels · 10 s"
        }
        TextKey::Cursor(CursorKey::CursorRealWindowsMouseValidation) => {
            "Cursor · Real Windows Mouse validation"
        }
        TextKey::Cursor(CursorKey::Pointer12Frames8Fps64PxKeep) => {
            "Pointer: 12 frames / 8 FPS / 64px. Keep the real mouse still to observe the loop."
        }
        TextKey::Cursor(CursorKey::Wait24Frames8Fps64PxThe) => {
            "Wait: 24 frames / 8 FPS / 64px. The loading ring rotates while Kamuri occasionally eats pudding."
        }
        TextKey::Cursor(CursorKey::Text42StaticCursors48PxPointerAndWait) => {
            "42 static cursors: 48px; Pointer and Wait: 64px. Compare each preview and semantic. BRP Custom Pointer does not change the OS cursor."
        }
        TextKey::Cursor(CursorKey::WidgetryExtendedSemanticsKamuriSystemComparison) => {
            "Widgetry extended semantics · Kamuri / System comparison"
        }
        TextKey::Button(ButtonKey::NestedNodeContentRootButtonDisabledAffects) => {
            "Nested Node content: root Button Disabled affects WidgetryText / Icon; native fixed Text stays teal."
        }
        TextKey::Button(ButtonKey::ToggleRootButtonDisabled) => "Toggle root Button Disabled",
        TextKey::Button(ButtonKey::DisabledContainersAffectDescendantsThroughPlainNodes) => {
            "Disabled containers affect descendants through plain Nodes. After enabling the parent, only locally disabled CheckBoxes stay disabled."
        }
        TextKey::Button(ButtonKey::InheritParentDisabled) => "Inherit parent Disabled",
        TextKey::Button(ButtonKey::LocallyDisabledStaysDisabledAfterParentIs) => {
            "Locally Disabled; stays disabled after parent is enabled"
        }
        TextKey::Button(ButtonKey::ToggleContainerDisabled) => "Toggle container Disabled",
        TextKey::ColorShowcase(ColorShowcaseKey::PureTheme) => "Pure Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::OverridesViaPublicApi) => {
            "Overrides via public API"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::CurrentThemeParentEnabled) => {
            "Current: Theme; parent enabled"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::OtherColorsFollowThemeClearOnlyAffects) => {
            "Other colors follow Theme. Clear only affects the open override instance."
        }
        TextKey::ColorShowcase(ColorShowcaseKey::ApplySampleOverrides) => "Apply sample overrides",
        TextKey::ColorShowcase(ColorShowcaseKey::ClearOverrides) => "Clear overrides",
        TextKey::ColorShowcase(ColorShowcaseKey::ToggleParentDisabled) => "Toggle parent Disabled",
        TextKey::Window(WindowKey::OpenPureTheme) => "Open pure Theme",
        TextKey::Window(WindowKey::OpenOverrideInstance) => "Open override instance",
        TextKey::Window(WindowKey::CycleTheme) => "Cycle Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::ThemeSourceValuesPartStateFieldSwatch) => {
            "Theme source values: part / state / field / swatch / HEX (including alpha), rather than final widget colors."
        }
        TextKey::ColorShowcase(ColorShowcaseKey::RealWidgetsPureThemeOnTheLeft) => {
            "Real widgets: pure Theme on the left, selected overrides on the right. Apply / clear through public API; clearing restores current Theme."
        }
        TextKey::ScrollArea(ScrollAreaKey::YGutterInducesXOverflowNoCorner) => {
            "Y gutter induces X overflow; no Corner"
        }
        TextKey::ScrollArea(ScrollAreaKey::KeepBarAndGutterEvenWithoutOverflow) => {
            "Keep bar and gutter even without overflow"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollableContentWithNoVisibleBar) => {
            "Scrollable content with no visible bar"
        }
        TextKey::ScrollArea(ScrollAreaKey::ToggleContentHeightToShowOrHide) => {
            "Toggle content height to show or hide the bar"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollThenClickTheButtonToAlign) => {
            "Scroll, then click the button to align Target to the top"
        }
        TextKey::Button(ButtonKey::FixedColorConvenienceApi0F766ERetainedAcross) => {
            "Fixed color convenience API=#0F766E; retained across Theme and parent Disabled changes. Clear to inherit Button foreground."
        }
        TextKey::ComboBox(ComboBoxKey::OnlyFieldOpenBorder7C3AedOpenPopup) => {
            "Only field.open.border=#7C3AED; open Popup, list and indicator still use Theme."
        }
        TextKey::ListView(ListViewKey::OnlyItemSelectedBackground0F766EMovePointer) => {
            "Only item.selected.background=#0F766E; move pointer away after selection, active border stays unchanged."
        }
        TextKey::Tree(TreeKey::OnlyItemSelectedBackground0F766EMovePointer2) => {
            "Only item.selected.background=#0F766E; move pointer away after selection, expander still uses Theme."
        }
        TextKey::Table(TableKey::OnlyCellSelectedBackground0F766EMovePointer) => {
            "Only cell.selected.background=#0F766E; move pointer away after selection, focus border stays unchanged."
        }
        TextKey::Waveform(WaveformKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D) => {
            "Only normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D]; Disabled palette still uses Theme."
        }
        TextKey::Button(ButtonKey::OnlyHoveredBackground7C3AedRealHoverPressed) => {
            "Only hovered.background=#7C3AED; real hover, Pressed still uses Theme."
        }
        TextKey::Button(ButtonKey::OnlyOptionCheckedNormalDot0F766EMove) => {
            "Only option.checked.normal.dot=#0F766E; move pointer away after selection."
        }
        TextKey::CheckBox(CheckBoxKey::OnlyCheckedNormalMark0F766EMovePointer) => {
            "Only checked.normal.mark=#0F766E; move pointer away after Checked, Hover/Disabled still use Theme."
        }
        TextKey::TextField(TextFieldKey::OnlyEditableFocusedBorder7C3AedClickTo) => {
            "Only editable.focused.border=#7C3AED; click to focus and type."
        }
        TextKey::TextField(TextFieldKey::OnlyReadOnlyFocusedBorder7C3AedSelect) => {
            "Only read_only.focused.border=#7C3AED; select / copy allowed, editing blocked."
        }
        TextKey::ScrollArea(ScrollAreaKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal) => {
            "Only vertical.thumb.hovered.background=#7C3AED; horizontal thumb unchanged."
        }
        TextKey::Tooltip(TooltipKey::OnlyPopupNormalBackground7C3AedRealHover) => {
            "Only popup.normal.background=#7C3AED; real hover shows Tooltip, including on disabled anchors."
        }
        TextKey::Window(WindowKey::WindowOnlyFrameNormalBorder7C3AedClear) => {
            "Window: only frame.normal.border=#7C3AED; clear directly on the open instance."
        }
        TextKey::Window(WindowKey::WindowImageSameBorderOverrideCover50) => {
            "Window Image: same border override; Cover / 50% opacity survive Theme changes and clear."
        }
        TextKey::Window(WindowKey::MessageboxOnlyBodyNormalBackground0F766EApply) => {
            "MessageBox: only body.normal.background=#0F766E; apply / clear and Theme switching inside modal."
        }
        TextKey::Window(WindowKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal) => {
            "FileDialog: only entry.selected.background=#0F766E; NonModal, clear after selecting a file without reopening."
        }
        TextKey::Common(CommonKey::None) => "<none>",
        TextKey::ComboBox(ComboBoxKey::ComboStatus) => {
            "Items: {count} | selected: {selected} | index: {index} | value: {label}\nFirst item disabled: {disabled} | ValueChange: {changes}"
        }
        TextKey::ListView(ListViewKey::ListItem) => "Item {index}  (edits: {edits})",
        TextKey::ListView(ListViewKey::ListInsertedItem) => "Inserted item  (edits: {edits})",
        TextKey::ListView(ListViewKey::DragDisabled) => {
            "Drag disabled: this small list shares data with the enabled list; clicks and selection still work."
        }
        TextKey::ListView(ListViewKey::ListOrders) => {
            "\nModel: {original}\nProjection: {displayed}"
        }
        TextKey::ListView(ListViewKey::ListStatus) => {
            "Model items: {count} | Rendered rows: {rows} | Rendered range: {start}..{end}\nselected: {selected} | active: {active} | SelectionChanged: {changes}"
        }
        TextKey::Tree(TreeKey::TreeFolder) => "[Folder] {label}",
        TextKey::Tree(TreeKey::TreeFile) => "[File] {label}  ({bytes} bytes)",
        TextKey::Tree(TreeKey::TreeStatus) => {
            "Visible: {visible} | expanded: {expanded} | rows: {rows} {start}..{end} | selected ({selected_count}): {selected} | {state}"
        }
        TextKey::Common(CommonKey::Enabled) => "enabled",
        TextKey::Table(TableKey::TableItem) => "Item {index}",
        TextKey::Table(TableKey::TableUpdated) => "Updated {edits}",
        TextKey::Table(TableKey::TableColumn) => "Column {index}",
        TextKey::Table(TableKey::TableUpdatedHeader) => "Updated header",
        TextKey::Table(TableKey::TableName) => "Name",
        TextKey::Table(TableKey::TableNumber) => "Number",
        TextKey::Table(TableKey::TableBool) => "Bool",
        TextKey::Table(TableKey::TableProgress) => "Progress",
        TextKey::Table(TableKey::TableStatus) => {
            "Model: {rows} rows × {columns} columns | visible cells: {cells} | rows {row_start}..{row_end}, columns {column_start}..{column_end}\nselection: {selection} | cursor: {cursor} | scroll: {scroll} | {state} | events: select {select}, resize start/change/end/cancel {start}/{change}/{end}/{cancel}"
        }
        TextKey::Tooltip(TooltipKey::TooltipCenter) => "Center placement",
        TextKey::Tooltip(TooltipKey::TooltipRight) => "Right placement",
        TextKey::Tooltip(TooltipKey::TooltipBottom) => "Bottom placement",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandThemeColors) => {
            "Expand / collapse Theme colors"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandOverrides) => {
            "Expand / collapse explicit overrides"
        }
        TextKey::Button(ButtonKey::ButtonActivations) => "Button Activate: {count}",
        TextKey::ColorShowcase(ColorShowcaseKey::OverrideStatus) => {
            "Last operation: {action}; changed: {changed}"
        }
        TextKey::Cursor(CursorKey::CursorTheme) => "Current Cursor theme: {theme}",
        TextKey::Waveform(WaveformKey::WaveformStatus) => {
            "64 kHz · 64 ch · 10 s · producer {producer} / displayed {displayed} · {fps} FPS / {ms} ms · dropped {dropped}"
        }
        TextKey::Window(WindowKey::ResultWaiting) => "Result: Waiting...",
        TextKey::Window(WindowKey::ResultFailed) => "Result: Could not open dialog",
        TextKey::Window(WindowKey::ResultCancelled) => "Result: Cancelled",
        TextKey::Window(WindowKey::ResultPaths) => "Result: {count} path(s){paths}",
        TextKey::Window(WindowKey::WindowImageTitle) => "Background {index} - {mode}",
        TextKey::Window(WindowKey::OverrideWindowTitle) => "{kind} · {sample}",
        TextKey::Window(WindowKey::OverrideInstance) => "Override instance",
        TextKey::ScrollArea(ScrollAreaKey::V1) => "V 1",
        TextKey::ScrollArea(ScrollAreaKey::V2) => "V 2",
        TextKey::ScrollArea(ScrollAreaKey::V3) => "V 3",
        TextKey::ScrollArea(ScrollAreaKey::V4) => "V 4",
        TextKey::ScrollArea(ScrollAreaKey::V5) => "V 5",
        TextKey::ScrollArea(ScrollAreaKey::V6) => "V 6",
        TextKey::ScrollArea(ScrollAreaKey::H1) => "H 1",
        TextKey::ScrollArea(ScrollAreaKey::H2) => "H 2",
        TextKey::ScrollArea(ScrollAreaKey::H3) => "H 3",
        TextKey::ScrollArea(ScrollAreaKey::H4) => "H 4",
        TextKey::ScrollArea(ScrollAreaKey::H5) => "H 5",
    }
}
