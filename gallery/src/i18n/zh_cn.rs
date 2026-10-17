use super::TextKey;
use super::{ButtonKey, CheckBoxKey, ComboBoxKey, ScrollAreaKey, TextFieldKey, TooltipKey};
use super::{ColorShowcaseKey, CommonKey, HeaderKey, NavigationKey};
use super::{CursorKey, WaveformKey, WindowKey};
use super::{ListViewKey, TableKey, TreeKey};

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::Button(ButtonKey::LinkButton) => "链接按钮",
        TextKey::Button(ButtonKey::LinkDescription) => {
            "通过默认浏览器打开 HTTP / HTTPS 地址。悬停和按下时显示下划线，禁用时无法打开。"
        }
        TextKey::Button(ButtonKey::SwitchUrl) => "切换 URL",
        TextKey::Button(ButtonKey::ProgrammaticUrl) => {
            "在百度和 Google 地址间切换，不会启动浏览器；只有激活链接才打开当前地址。"
        }
        TextKey::Button(ButtonKey::LinkOverrideDescription) => {
            "仅覆盖链接的悬停颜色（#7C3AED），按下和禁用颜色仍跟随当前主题。"
        }
        TextKey::Window(WindowKey::FileFilterLabel) => "{label} ({suffixes})",
        TextKey::GalleryHeader(HeaderKey::WidgetGallery) => "Widget 展示",
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
        TextKey::Common(CommonKey::Normal) => "正常",
        TextKey::Common(CommonKey::Disabled) => "禁用",
        TextKey::Button(ButtonKey::RadioButton) => "Radio Button",
        TextKey::Button(ButtonKey::Horizontal) => "横向",
        TextKey::Button(ButtonKey::Grid) => "网格",
        TextKey::Common(CommonKey::Text) => "文本",
        TextKey::Common(CommonKey::IconText) => "图标 + 文本",
        TextKey::Button(ButtonKey::TextIcon) => "文本 + 图标",
        TextKey::Common(CommonKey::Icon) => "图标",
        TextKey::CheckBox(CheckBoxKey::ReceiveUpdates) => "接收更新",
        TextKey::CheckBox(CheckBoxKey::Unchecked) => "未勾选",
        TextKey::CheckBox(CheckBoxKey::Checked) => "已勾选",
        TextKey::CheckBox(CheckBoxKey::Indeterminate) => "不确定",
        TextKey::CheckBox(CheckBoxKey::TriState) => "三态",
        TextKey::CheckBox(CheckBoxKey::TriStateCheckBox) => "三态 CheckBox",
        TextKey::ComboBox(ComboBoxKey::Renderers) => "Renderer 展示",
        TextKey::ComboBox(ComboBoxKey::DataDriven) => "数据驱动",
        TextKey::ComboBox(ComboBoxKey::InsertMoveOrEditSelectionKeepsIts) => {
            "插入、移动或编辑时保留选中项身份。删除选中项后，Field 清空。\n香蕉初始禁用。禁用项仍可见但不可选。程序化选择与清空仅在变化时通知。"
        }
        TextKey::ScrollArea(ScrollAreaKey::Height80290) => "高度：80 / 290",
        TextKey::ScrollArea(ScrollAreaKey::ToggleContent) => "切换内容高度",
        TextKey::ScrollArea(ScrollAreaKey::ShowTarget) => "显示目标",
        TextKey::Common(CommonKey::Basic) => "基础",
        TextKey::Tooltip(TooltipKey::RichContent) => "丰富内容",
        TextKey::Tooltip(TooltipKey::Placement) => "位置",
        TextKey::Tooltip(TooltipKey::BasicTooltip) => "基础 Tooltip",
        TextKey::Tooltip(TooltipKey::HoverMe) => "悬停查看",
        TextKey::Tooltip(TooltipKey::RichTooltip) => "丰富 Tooltip",
        TextKey::Tooltip(TooltipKey::IconAndMultipleLines) => "图标与多行文本",
        TextKey::Tooltip(TooltipKey::RichContent2) => "丰富内容",
        TextKey::Tooltip(TooltipKey::DisabledControlsStillHaveTooltips) => "禁用的控件仍有 Tooltip",
        TextKey::Common(CommonKey::EnableDisable) => "启用 / 禁用",
        TextKey::Tree(TreeKey::SelectLastApi) => "选择最后一项（API）",
        TextKey::Window(WindowKey::FileDialog) => "文件对话框",
        TextKey::Window(WindowKey::ModalBlocksMainWindow) => "Modal（阻止主窗口输入）",
        TextKey::Window(WindowKey::ChooseFilesOrFoldersTypeAFile) => {
            "选择文件或文件夹。输入文件名以打开，使用文件类型筛选扩展名。多个 NonModal 对话框可同时打开。"
        }
        TextKey::Window(WindowKey::Result) => "结果：…",
        TextKey::Window(WindowKey::IndependentWindow) => "独立窗口",
        TextKey::Window(WindowKey::Theme) => "Theme",
        TextKey::Window(WindowKey::ThisIsAnIndependentWidgetryWindow) => {
            "这是一个独立 Widgetry 窗口。"
        }
        TextKey::Window(WindowKey::ClickMe) => "点击我",
        TextKey::Window(WindowKey::Clicked) => "已点击！",
        TextKey::Window(WindowKey::MessageBox) => "MessageBox",
        TextKey::Window(WindowKey::ChooseAResultBelow) => "请在下方选择结果。",
        TextKey::Window(WindowKey::ContentButtonKeepsDialogOpen) => "内容按钮（保持对话框打开）",
        TextKey::Button(ButtonKey::ManagedText) => "受管理的文本",
        TextKey::Button(ButtonKey::NativeFixedText) => "原生固定色文本",
        TextKey::Button(ButtonKey::TextInsideButton) => "Button 内的文本",
        TextKey::Button(ButtonKey::IconInsideButton) => "Button 内的图标",
        TextKey::Button(ButtonKey::HoverPress) => "悬停 / 按下",
        TextKey::Common(CommonKey::Apple) => "苹果",
        TextKey::Common(CommonKey::Banana) => "香蕉",
        TextKey::Common(CommonKey::Orange) => "橙子",
        TextKey::Common(CommonKey::Grape) => "葡萄",
        TextKey::ComboBox(ComboBoxKey::Pear) => "梨",
        TextKey::ComboBox(ComboBoxKey::Star) => "星星",
        TextKey::ComboBox(ComboBoxKey::Logo) => "Logo",
        TextKey::ComboBox(ComboBoxKey::Inserted) => "插入项",
        TextKey::CheckBox(CheckBoxKey::CheckMe) => "勾选我",
        TextKey::ScrollArea(ScrollAreaKey::ScrollHoverVerticalThumb) => "滚动 / 悬停纵向 thumb",
        TextKey::Tooltip(TooltipKey::TooltipReadableThemeForeground) => {
            "Tooltip：可读的 Theme 前景色"
        }
        TextKey::Tooltip(TooltipKey::HoverForTooltip) => "悬停显示 Tooltip",
        TextKey::Common(CommonKey::InsertFirst) => "插入首项",
        TextKey::Common(CommonKey::RemoveSelected) => "删除选中项",
        TextKey::ComboBox(ComboBoxKey::MoveSelectedLast) => "将选中项移到末尾",
        TextKey::Common(CommonKey::EditSelected) => "编辑选中项",
        TextKey::ComboBox(ComboBoxKey::EnableDisableFirst) => "启用 / 禁用首项",
        TextKey::ComboBox(ComboBoxKey::SetSelectedFirst) => "选择首项",
        TextKey::Common(CommonKey::ClearSelection) => "清空选择",
        TextKey::ComboBox(ComboBoxKey::ResetModel) => "重置 Model",
        TextKey::ListView(ListViewKey::ListSharedText) => "共享 Model A：文字",
        TextKey::ListView(ListViewKey::ListSharedIcon) => "共享 Model B：图标 + 文字",
        TextKey::ListView(ListViewKey::ListSharedDescription) => {
            "数据共享：编辑 / 插入 / 删除 / 移动。选择、导航、滚动与启用 / 禁用各自独立。"
        }
        TextKey::ListView(ListViewKey::SmallList) => "小列表",
        TextKey::ListView(ListViewKey::ArrowHomeEndActiveSpaceEnterSelect) => {
            "Arrow / Home / End 移动 active，Space / Enter 选择。可点击文本或整行。"
        }
        TextKey::ListView(ListViewKey::VirtualizedLargeList) => "虚拟化大列表",
        TextKey::ListView(ListViewKey::Text10000ItemsWheelPageupPagedownJump) => {
            "10,000 项。支持滚轮 / PageUp / PageDown、跳转、编辑、缩小和调整大小。"
        }
        TextKey::ListView(ListViewKey::DisabledListview) => "禁用的 ListView",
        TextKey::ListView(ListViewKey::UserInputBlockedModelSelectionAndScroll) => {
            "阻止用户输入，Model、选择和滚动按钮仍可用。"
        }
        TextKey::ListView(ListViewKey::DisabledListviewitem) => "禁用的 ListViewItem",
        TextKey::ListView(ListViewKey::InitiallyItems13DisabledArrowClick) => {
            "第 1 / 3 项初始禁用。Arrow / 点击可激活，Space / Enter 不可选择。"
        }
        TextKey::ListView(ListViewKey::SetSelected) => "设置选择",
        TextKey::ListView(ListViewKey::EditVisible) => "编辑可见项",
        TextKey::Common(CommonKey::RemoveFirst) => "删除首项",
        TextKey::Common(CommonKey::MoveFirstLast) => "首项移到末尾",
        TextKey::ListView(ListViewKey::ClearRepopulate) => "清空并重新填充",
        TextKey::ListView(ListViewKey::Edit7500) => "编辑第 7500 项",
        TextKey::ListView(ListViewKey::ShrinkTo3) => "缩小到 3 项",
        TextKey::Common(CommonKey::ResizeViewport) => "调整 viewport 大小",
        TextKey::ListView(ListViewKey::Scroll96Px) => "滚动 +96px",
        TextKey::ListView(ListViewKey::EnableDisable1) => "启用 / 禁用第 1 项",
        TextKey::ScrollArea(ScrollAreaKey::VerticalAuto) => "纵向 Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelYPageupPagedownHomeEnd) => {
            "Y 滚轮 / PageUp / PageDown / Home / End"
        }
        TextKey::ScrollArea(ScrollAreaKey::HorizontalAuto) => "横向 Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelXLeftRight) => "X 滚轮 / Left / Right",
        TextKey::ScrollArea(ScrollAreaKey::BothAuto) => "双向 Auto",
        TextKey::ScrollArea(ScrollAreaKey::Always) => "始终显示",
        TextKey::ScrollArea(ScrollAreaKey::Hidden) => "隐藏",
        TextKey::ScrollArea(ScrollAreaKey::AutoTransition) => "Auto 切换",
        TextKey::ScrollArea(ScrollAreaKey::Scrollintoview) => "ScrollIntoView",
        TextKey::ScrollArea(ScrollAreaKey::Text300290Content) => "300 × 290 内容",
        TextKey::ScrollArea(ScrollAreaKey::FitsViewport) => "内容适合 viewport",
        TextKey::ScrollArea(ScrollAreaKey::ScrollMeWithoutABar) => "无 bar 也可滚动",
        TextKey::ScrollArea(ScrollAreaKey::Before1) => "目标前 1",
        TextKey::ScrollArea(ScrollAreaKey::Before2) => "目标前 2",
        TextKey::ScrollArea(ScrollAreaKey::Before3) => "目标前 3",
        TextKey::ScrollArea(ScrollAreaKey::Before4) => "目标前 4",
        TextKey::ScrollArea(ScrollAreaKey::Target) => "目标",
        TextKey::ScrollArea(ScrollAreaKey::AfterTarget) => "目标后",
        TextKey::Table(TableKey::SelectFirstApi) => "选择首项（API）",
        TextKey::Table(TableKey::ClearSelectionApi) => "清空选择（API）",
        TextKey::Table(TableKey::UpdateFirstRow) => "更新首行",
        TextKey::Table(TableKey::TableShared) => "共享 Model",
        TextKey::Table(TableKey::TableSharedA) => "共享 Model A：列宽 180 px，行高 34 px",
        TextKey::Table(TableKey::TableSharedB) => {
            "共享 Model B：列宽 120 px，行高 28 px，紧凑内边距"
        }
        TextKey::Table(TableKey::TableSharedDescription) => {
            "共用 Model 和 Renderer 注册表，布局、列宽、选择、导航位置、滚动和禁用状态各自独立。两侧均可编辑、插入、删除和移动共享行。"
        }
        TextKey::Table(TableKey::TableInsertRow) => "插入首行",
        TextKey::Table(TableKey::ReplaceFirstHeader) => "替换首列 Header",
        TextKey::Table(TableKey::Cellvalue) => "CellValue",
        TextKey::Table(TableKey::Header) => "Header",
        TextKey::Table(TableKey::Selection) => "选择",
        TextKey::Table(TableKey::ColumnLayout) => "Column Layout",
        TextKey::Table(TableKey::Virtualization) => "虚拟化",
        TextKey::Table(TableKey::RowNumbersAndColumnHeadersStayFixed) => {
            "行号与列 Header 在各自轴向保持固定。点击任意 cell 选择。"
        }
        TextKey::Table(TableKey::StringNumberBoolProgressAndIconUse) => {
            "String、Number、Bool、Progress 和 Icon 使用不同 renderer。更新首行以刷新内容。"
        }
        TextKey::Table(TableKey::CustomHeadersCombineAStarAndLabel) => {
            "自定义 Header 组合星形图标与标签。Header 与 Cell 内容使用独立 renderer。"
        }
        TextKey::Table(TableKey::ClickAColumnHeaderRowNumberOr) => {
            "点击列 Header、行号或 cell。Arrow 移动 focus 边框而不改变选择。"
        }
        TextKey::Table(TableKey::NameStays180PxWideNumberAndBool) => {
            "Name 固定 180px，Number 与 Bool 按 2:1 分配剩余宽度。拖动 Header 右边缘调整宽度。"
        }
        TextKey::Table(TableKey::Text2000Rows40ColumnsScrollHorizontally) => {
            "2,000 行 × 40 列。可双向滚动，行列范围及可见 cell 数随 viewport 更新。"
        }
        TextKey::Table(TableKey::InitiallyDisabledEnableToSelectAndNavigate) => {
            "初始禁用。启用后可选择和导航，再禁用时保留选择，停止输入及 hover。"
        }
        TextKey::Tree(TreeKey::BasicTree) => "基础树",
        TextKey::Tree(TreeKey::TreeSharedA) => "共享 TreeModel A",
        TextKey::Tree(TreeKey::TreeSharedB) => "共享 TreeModel B：较矮视口",
        TextKey::Tree(TreeKey::TreeSharedDescription) => {
            "展开与选择共享；滚动、启用 / 禁用独立。选中文件后可编辑、删除或在目录与根之间移动。禁用侧仍接收数据更新。"
        }
        TextKey::Tree(TreeKey::TreeSharedFolder) => "共享目录",
        TextKey::Tree(TreeKey::TreeSharedFile) => "文件 {index}（编辑 {edits} 次）",
        TextKey::Tree(TreeKey::TreeInsertFile) => "插入文件",
        TextKey::Tree(TreeKey::TreeReparentSelected) => "移动文件：目录 / 根",
        TextKey::Tree(TreeKey::ClickArrowsToExpandClickContentTo) => {
            "点击箭头展开，点击内容选择。通过 Arrow / Home / End 和 Space / Enter 导航。"
        }
        TextKey::Tree(TreeKey::EcsHeterogeneousTree) => "ECS 异构树",
        TextKey::Tree(TreeKey::FolderAndFileComponentsDispatchDifferentRenderers) => {
            "Folder 与 File component 使用不同 renderer。业务数据保留在 ECS node 上。"
        }
        TextKey::Tree(TreeKey::LazyLoadingTree) => "按需加载树",
        TextKey::Tree(TreeKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse) => {
            "展开远程文件夹：未加载 → 加载中 → 已加载。折叠再展开仍保留单次请求。"
        }
        TextKey::Tree(TreeKey::VirtualizedTree) => "虚拟化树",
        TextKey::Tree(TreeKey::Expand10000FilesWheelPagedownScroll) => {
            "展开 10,000 个文件，使用滚轮 / PageDown 滚动。仅构造 viewport 内的行。"
        }
        TextKey::Tree(TreeKey::Workspace) => "工作区",
        TextKey::Tree(TreeKey::Projects) => "项目",
        TextKey::Tree(TreeKey::Widgetry) => "Widgetry",
        TextKey::Tree(TreeKey::Photos) => "照片",
        TextKey::Tree(TreeKey::Notes) => "笔记",
        TextKey::Tree(TreeKey::Assets) => "Assets",
        TextKey::Tree(TreeKey::RemoteFolderUnloaded) => "远程文件夹（未加载）",
        TextKey::Tree(TreeKey::RemoteFolderLoading) => "远程文件夹（加载中…）",
        TextKey::Tree(TreeKey::RemoteFolderLoaded) => "远程文件夹（已加载）",
        TextKey::Tree(TreeKey::Text10000Files) => "10,000 个文件",
        TextKey::Window(WindowKey::OpenFile) => "打开文件",
        TextKey::Window(WindowKey::OpenFiles) => "打开多个文件",
        TextKey::Window(WindowKey::SelectFolder) => "选择文件夹",
        TextKey::Window(WindowKey::SelectFolders) => "选择多个文件夹",
        TextKey::Window(WindowKey::OpenImage) => "打开图像",
        TextKey::Window(WindowKey::SaveFile) => "保存文件",
        TextKey::Window(WindowKey::Images) => "图像",
        TextKey::Window(WindowKey::TextDocuments) => "文本文档",
        TextKey::Window(WindowKey::Background1) => "背景 1",
        TextKey::Window(WindowKey::Background2) => "背景 2",
        TextKey::Window(WindowKey::Stretch) => "拉伸",
        TextKey::Window(WindowKey::Cover) => "覆盖",
        TextKey::Window(WindowKey::Cover50) => "覆盖 50%",
        TextKey::Window(WindowKey::Ok) => "确定",
        TextKey::Window(WindowKey::YesNo) => "是 / 否",
        TextKey::Window(WindowKey::YesNoCancel) => "是 / 否 / 取消",
        TextKey::Window(WindowKey::MessageBoxDemo) => "MessageBox 演示",
        TextKey::GalleryHeader(HeaderKey::Dark) => "深色",
        TextKey::GalleryHeader(HeaderKey::Light) => "浅色",
        TextKey::GalleryHeader(HeaderKey::PinkDream) => "粉色梦境",
        TextKey::GalleryHeader(HeaderKey::KamuriViolet) => "冠紫色",
        TextKey::Tooltip(TooltipKey::Center) => "中央",
        TextKey::Tooltip(TooltipKey::Right) => "右侧",
        TextKey::Tooltip(TooltipKey::Bottom) => "底部",
        TextKey::Waveform(WaveformKey::WaitingForWaveformPage) => "等待 Waveform 页面",
        TextKey::Waveform(WaveformKey::BasicLive100Hz4Channels5) => {
            "基础实时 · 100 Hz · 4 通道 · 5 s · Polyline"
        }
        TextKey::Waveform(WaveformKey::BasicReplay8Khz4Channels5) => {
            "基础回放 · 8 kHz · 4 通道 · 5 s · 固定 .wfrm · Envelope"
        }
        TextKey::Waveform(WaveformKey::StressLive64Khz64Channels10) => {
            "压力实时 · 64 kHz · 64 通道 · 10 s"
        }
        TextKey::Cursor(CursorKey::CursorRealWindowsMouseValidation) => {
            "Cursor · 真实 Windows Mouse 验证"
        }
        TextKey::Cursor(CursorKey::Pointer12Frames8Fps64PxKeep) => {
            "Pointer：12 帧 / 8 FPS / 64px。停住真实鼠标观察循环。"
        }
        TextKey::Cursor(CursorKey::Wait24Frames8Fps64PxThe) => {
            "Wait：24 帧 / 8 FPS / 64px。加载环旋转，冠偶尔吃一口布丁。"
        }
        TextKey::Cursor(CursorKey::Text42StaticCursors48PxPointerAndWait) => {
            "42 种静态 Cursor：48px；Pointer、Wait：64px。预览与语义逐格对照。BRP Custom Pointer 不改变 OS Cursor。"
        }
        TextKey::Cursor(CursorKey::WidgetryExtendedSemanticsKamuriSystemComparison) => {
            "Widgetry 扩展语义 · Kamuri / System 对照"
        }
        TextKey::Button(ButtonKey::NestedNodeContentRootButtonDisabledAffects) => {
            "多层 Node 内容：根 Button Disabled 影响 WidgetryText / Icon；原生固定 Text 保持青色。"
        }
        TextKey::Button(ButtonKey::ToggleRootButtonDisabled) => "切换根 Button Disabled",
        TextKey::Button(ButtonKey::DisabledContainersAffectDescendantsThroughPlainNodes) => {
            "禁用容器穿过普通 Node；恢复父级后，仅本地单独禁用的 CheckBox 继续禁用。"
        }
        TextKey::Button(ButtonKey::InheritParentDisabled) => "继承父级 Disabled",
        TextKey::Button(ButtonKey::LocallyDisabledStaysDisabledAfterParentIs) => {
            "本地也 Disabled，父级恢复后仍禁用"
        }
        TextKey::Button(ButtonKey::ToggleContainerDisabled) => "切换容器 Disabled",
        TextKey::ColorShowcase(ColorShowcaseKey::PureTheme) => "纯 Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::OverridesViaPublicApi) => "公开 API 覆盖实例",
        TextKey::ColorShowcase(ColorShowcaseKey::CurrentThemeParentEnabled) => {
            "当前：Theme；父级 enabled"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::OtherColorsFollowThemeClearOnlyAffects) => {
            "其他颜色继续跟随 Theme；清除只操作已打开的覆盖实例。"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::ApplySampleOverrides) => "应用示例覆盖",
        TextKey::ColorShowcase(ColorShowcaseKey::ClearOverrides) => "清除覆盖",
        TextKey::ColorShowcase(ColorShowcaseKey::ToggleParentDisabled) => "切换父级 Disabled",
        TextKey::Window(WindowKey::OpenPureTheme) => "打开纯 Theme",
        TextKey::Window(WindowKey::OpenOverrideInstance) => "打开覆盖实例",
        TextKey::Window(WindowKey::CycleTheme) => "循环切换 Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::ThemeSourceValuesPartStateFieldSwatch) => {
            "Theme 原值目录：部件 / state / 字段 / 色块 / HEX（含 alpha），不是控件最终颜色。"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::RealWidgetsPureThemeOnTheLeft) => {
            "真实控件：左侧纯 Theme，右侧只覆盖指定字段。应用 / 清除通过公开 API；清除回到当前 Theme。"
        }
        TextKey::ScrollArea(ScrollAreaKey::YGutterInducesXOverflowNoCorner) => {
            "Y gutter 诱发 X overflow；无 Corner"
        }
        TextKey::ScrollArea(ScrollAreaKey::KeepBarAndGutterEvenWithoutOverflow) => {
            "无 overflow 时仍保留 bar 和 gutter"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollableContentWithNoVisibleBar) => {
            "内容可滚动，但不显示 bar"
        }
        TextKey::ScrollArea(ScrollAreaKey::ToggleContentHeightToShowOrHide) => {
            "切换内容高度，观察 bar 增减"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollThenClickTheButtonToAlign) => {
            "滚动后点击按钮，将 Target 对齐到顶部"
        }
        TextKey::Button(ButtonKey::FixedColorConvenienceApi0F766ERetainedAcross) => {
            "固定色便利 API=#0F766E；Theme 和父级 Disabled 切换后保持。清除后继承 Button 前景色。"
        }
        TextKey::ComboBox(ComboBoxKey::OnlyFieldOpenBorder7C3AedOpenPopup) => {
            "仅 field.open.border=#7C3AED；展开 Popup，list 与 indicator 仍取 Theme。"
        }
        TextKey::ListView(ListViewKey::OnlyItemSelectedBackground0F766EMovePointer) => {
            "仅 item.selected.background=#0F766E；选中后移开 pointer，active border 不变。"
        }
        TextKey::Tree(TreeKey::OnlyItemSelectedBackground0F766EMovePointer2) => {
            "仅 item.selected.background=#0F766E；选中后移开 pointer，expander 仍取 Theme。"
        }
        TextKey::Table(TableKey::OnlyCellSelectedBackground0F766EMovePointer) => {
            "仅 cell.selected.background=#0F766E；选择后移开 pointer，focus border 不变。"
        }
        TextKey::Waveform(WaveformKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D) => {
            "仅 normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D]；Disabled palette 仍取 Theme。"
        }
        TextKey::Button(ButtonKey::OnlyHoveredBackground7C3AedRealHoverPressed) => {
            "仅 hovered.background=#7C3AED；真实 hover，Pressed 仍取 Theme。"
        }
        TextKey::Button(ButtonKey::OnlyOptionCheckedNormalDot0F766EMove) => {
            "仅 option.checked.normal.dot=#0F766E；选择后移开 pointer。"
        }
        TextKey::CheckBox(CheckBoxKey::OnlyCheckedNormalMark0F766EMovePointer) => {
            "仅 checked.normal.mark=#0F766E；Checked 后移开 pointer，Hover/Disabled 仍取 Theme。"
        }
        TextKey::TextField(TextFieldKey::OnlyEditableFocusedBorder7C3AedClickTo) => {
            "仅 editable.focused.border=#7C3AED；点击取得 focus 后输入。"
        }
        TextKey::TextField(TextFieldKey::OnlyReadOnlyFocusedBorder7C3AedSelect) => {
            "仅 read_only.focused.border=#7C3AED；可选择 / copy，不能编辑。"
        }
        TextKey::ScrollArea(ScrollAreaKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal) => {
            "仅 vertical.thumb.hovered.background=#7C3AED；横向 thumb 不变。"
        }
        TextKey::Tooltip(TooltipKey::OnlyPopupNormalBackground7C3AedRealHover) => {
            "仅 popup.normal.background=#7C3AED；真实 hover 出 Tooltip，禁用 anchor 仍可显示。"
        }
        TextKey::Window(WindowKey::WindowOnlyFrameNormalBorder7C3AedClear) => {
            "Window：仅 frame.normal.border=#7C3AED；已打开实例可直接清除。"
        }
        TextKey::Window(WindowKey::WindowImageSameBorderOverrideCover50) => {
            "Window Image：同一 border 覆盖；Cover / 50% opacity 不因切换 Theme 或清除覆盖改变。"
        }
        TextKey::Window(WindowKey::MessageboxOnlyBodyNormalBackground0F766EApply) => {
            "MessageBox：仅 body.normal.background=#0F766E；modal 内提供应用 / 清除和 Theme 切换。"
        }
        TextKey::Window(WindowKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal) => {
            "FileDialog：仅 entry.selected.background=#0F766E；NonModal，选中文件后清除，无需重新打开。"
        }
        TextKey::Common(CommonKey::None) => "<无>",
        TextKey::ComboBox(ComboBoxKey::ComboStatus) => {
            "项目：{count} | 选中：{selected} | 索引：{index} | 值：{label}\n首项禁用：{disabled} | ValueChange：{changes}"
        }
        TextKey::ListView(ListViewKey::ListItem) => "项目 {index}  （编辑：{edits}）",
        TextKey::ListView(ListViewKey::ListInsertedItem) => "插入的项目  （编辑：{edits}）",
        TextKey::ListView(ListViewKey::ListStatus) => {
            "Model 项目：{count} | 显示行：{rows} | 显示范围：{start}..{end}\n选中：{selected} | active：{active} | ValueChange：{changes}"
        }
        TextKey::Tree(TreeKey::TreeFolder) => "[文件夹] {label}",
        TextKey::Tree(TreeKey::TreeFile) => "[文件] {label}  （{bytes} 字节）",
        TextKey::Tree(TreeKey::TreeStatus) => {
            "可见：{visible} | 行：{rows} {start}..{end} | 选中：{selected} | {state}"
        }
        TextKey::Common(CommonKey::Enabled) => "启用",
        TextKey::Table(TableKey::TableItem) => "项目 {index}",
        TextKey::Table(TableKey::TableUpdated) => "已更新 {edits}",
        TextKey::Table(TableKey::TableColumn) => "列 {index}",
        TextKey::Table(TableKey::TableUpdatedHeader) => "已更新的 Header",
        TextKey::Table(TableKey::TableName) => "名称",
        TextKey::Table(TableKey::TableNumber) => "数值",
        TextKey::Table(TableKey::TableBool) => "Bool",
        TextKey::Table(TableKey::TableProgress) => "进度",
        TextKey::Table(TableKey::TableStatus) => {
            "Model：{rows} 行 × {columns} 列 | 可见 cell：{cells} | 行 {row_start}..{row_end}，列 {column_start}..{column_end}\n选择：{selection} | cursor：{cursor} | 滚动：{scroll} | {state} | event：选择 {select}，resize 开始/变化/结束/取消 {start}/{change}/{end}/{cancel}"
        }
        TextKey::Tooltip(TooltipKey::TooltipCenter) => "中央位置",
        TextKey::Tooltip(TooltipKey::TooltipRight) => "右侧位置",
        TextKey::Tooltip(TooltipKey::TooltipBottom) => "底部位置",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandThemeColors) => "展开 / 折叠 Theme 颜色",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandOverrides) => "展开 / 折叠显式覆盖",
        TextKey::Button(ButtonKey::ButtonActivations) => "Button Activate：{count}",
        TextKey::ColorShowcase(ColorShowcaseKey::OverrideStatus) => {
            "最近操作：{action}；实际变化：{changed}"
        }
        TextKey::Cursor(CursorKey::CursorTheme) => "当前 Cursor theme：{theme}",
        TextKey::Waveform(WaveformKey::WaveformStatus) => {
            "64 kHz · 64 通道 · 10 s · producer {producer} / 显示 {displayed} · {fps} FPS / {ms} ms · 丢弃 {dropped}"
        }
        TextKey::Window(WindowKey::ResultWaiting) => "结果：等待中…",
        TextKey::Window(WindowKey::ResultFailed) => "结果：无法打开对话框",
        TextKey::Window(WindowKey::ResultCancelled) => "结果：已取消",
        TextKey::Window(WindowKey::ResultPaths) => "结果：{count} 个路径{paths}",
        TextKey::Window(WindowKey::WindowImageTitle) => "背景 {index} - {mode}",
        TextKey::Window(WindowKey::OverrideWindowTitle) => "{kind} · {sample}",
        TextKey::Window(WindowKey::OverrideInstance) => "覆盖实例",
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
