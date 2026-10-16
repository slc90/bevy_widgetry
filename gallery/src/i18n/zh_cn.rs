use super::TextKey;
use super::{ColorShowcaseKey, CommonKey, HeaderKey, NavigationKey};

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::FileFilterLabel => "{label} ({suffixes})",
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
        TextKey::RadioGroup => "RadioGroup",
        TextKey::Common(CommonKey::ReadOnly) => "ReadOnly",
        TextKey::Common(CommonKey::Normal) => "正常",
        TextKey::Common(CommonKey::Disabled) => "禁用",
        TextKey::RadioButton => "Radio Button",
        TextKey::Horizontal => "横向",
        TextKey::Grid => "网格",
        TextKey::Common(CommonKey::Text) => "文本",
        TextKey::Common(CommonKey::IconText) => "图标 + 文本",
        TextKey::TextIcon => "文本 + 图标",
        TextKey::Common(CommonKey::Icon) => "图标",
        TextKey::ReceiveUpdates => "接收更新",
        TextKey::Unchecked => "未勾选",
        TextKey::Checked => "已勾选",
        TextKey::Indeterminate => "不确定",
        TextKey::TriState => "三态",
        TextKey::TriStateCheckBox => "三态 CheckBox",
        TextKey::Renderers => "Renderer 展示",
        TextKey::DataDriven => "数据驱动",
        TextKey::InsertMoveOrEditSelectionKeepsIts => {
            "插入、移动或编辑时保留选中项身份。删除选中项后，Field 清空。\n香蕉初始禁用。禁用项仍可见但不可选。程序化选择与清空仅在变化时通知。"
        }
        TextKey::Height80290 => "高度：80 / 290",
        TextKey::ToggleContent => "切换内容高度",
        TextKey::ShowTarget => "显示目标",
        TextKey::Common(CommonKey::Basic) => "基础",
        TextKey::RichContent => "丰富内容",
        TextKey::Placement => "位置",
        TextKey::BasicTooltip => "基础 Tooltip",
        TextKey::HoverMe => "悬停查看",
        TextKey::RichTooltip => "丰富 Tooltip",
        TextKey::IconAndMultipleLines => "图标与多行文本",
        TextKey::RichContent2 => "丰富内容",
        TextKey::DisabledControlsStillHaveTooltips => "禁用的控件仍有 Tooltip",
        TextKey::Common(CommonKey::EnableDisable) => "启用 / 禁用",
        TextKey::SelectLastApi => "选择最后一项（API）",
        TextKey::FileDialog => "文件对话框",
        TextKey::ModalBlocksMainWindow => "Modal（阻止主窗口输入）",
        TextKey::ChooseFilesOrFoldersTypeAFile => {
            "选择文件或文件夹。输入文件名以打开，使用文件类型筛选扩展名。多个 NonModal 对话框可同时打开。"
        }
        TextKey::Result => "结果：…",
        TextKey::IndependentWindow => "独立窗口",
        TextKey::Theme => "Theme",
        TextKey::ThisIsAnIndependentWidgetryWindow => "这是一个独立 Widgetry 窗口。",
        TextKey::ClickMe => "点击我",
        TextKey::Clicked => "已点击！",
        TextKey::MessageBox => "MessageBox",
        TextKey::ChooseAResultBelow => "请在下方选择结果。",
        TextKey::ContentButtonKeepsDialogOpen => "内容按钮（保持对话框打开）",
        TextKey::ManagedText => "受管理的文本",
        TextKey::NativeFixedText => "原生固定色文本",
        TextKey::TextInsideButton => "Button 内的文本",
        TextKey::IconInsideButton => "Button 内的图标",
        TextKey::HoverPress => "悬停 / 按下",
        TextKey::Common(CommonKey::Apple) => "苹果",
        TextKey::Common(CommonKey::Banana) => "香蕉",
        TextKey::Common(CommonKey::Orange) => "橙子",
        TextKey::Common(CommonKey::Grape) => "葡萄",
        TextKey::Pear => "梨",
        TextKey::Star => "星星",
        TextKey::Logo => "Logo",
        TextKey::Inserted => "插入项",
        TextKey::CheckMe => "勾选我",
        TextKey::ScrollHoverVerticalThumb => "滚动 / 悬停纵向 thumb",
        TextKey::TooltipReadableThemeForeground => "Tooltip：可读的 Theme 前景色",
        TextKey::HoverForTooltip => "悬停显示 Tooltip",
        TextKey::Common(CommonKey::InsertFirst) => "插入首项",
        TextKey::Common(CommonKey::RemoveSelected) => "删除选中项",
        TextKey::MoveSelectedLast => "将选中项移到末尾",
        TextKey::Common(CommonKey::EditSelected) => "编辑选中项",
        TextKey::EnableDisableFirst => "启用 / 禁用首项",
        TextKey::SetSelectedFirst => "选择首项",
        TextKey::Common(CommonKey::ClearSelection) => "清空选择",
        TextKey::ResetModel => "重置 Model",
        TextKey::ListSharedText => "共享 Model A：文字",
        TextKey::ListSharedIcon => "共享 Model B：图标 + 文字",
        TextKey::ListSharedDescription => {
            "数据共享：编辑 / 插入 / 删除 / 移动。选择、导航、滚动与启用 / 禁用各自独立。"
        }
        TextKey::SmallList => "小列表",
        TextKey::ArrowHomeEndActiveSpaceEnterSelect => {
            "Arrow / Home / End 移动 active，Space / Enter 选择。可点击文本或整行。"
        }
        TextKey::VirtualizedLargeList => "虚拟化大列表",
        TextKey::Text10000ItemsWheelPageupPagedownJump => {
            "10,000 项。支持滚轮 / PageUp / PageDown、跳转、编辑、缩小和调整大小。"
        }
        TextKey::DisabledListview => "禁用的 ListView",
        TextKey::UserInputBlockedModelSelectionAndScroll => {
            "阻止用户输入，Model、选择和滚动按钮仍可用。"
        }
        TextKey::DisabledListviewitem => "禁用的 ListViewItem",
        TextKey::InitiallyItems13DisabledArrowClick => {
            "第 1 / 3 项初始禁用。Arrow / 点击可激活，Space / Enter 不可选择。"
        }
        TextKey::SetSelected => "设置选择",
        TextKey::EditVisible => "编辑可见项",
        TextKey::Common(CommonKey::RemoveFirst) => "删除首项",
        TextKey::Common(CommonKey::MoveFirstLast) => "首项移到末尾",
        TextKey::ClearRepopulate => "清空并重新填充",
        TextKey::Edit7500 => "编辑第 7500 项",
        TextKey::ShrinkTo3 => "缩小到 3 项",
        TextKey::Common(CommonKey::ResizeViewport) => "调整 viewport 大小",
        TextKey::Scroll96Px => "滚动 +96px",
        TextKey::EnableDisable1 => "启用 / 禁用第 1 项",
        TextKey::VerticalAuto => "纵向 Auto",
        TextKey::WheelYPageupPagedownHomeEnd => "Y 滚轮 / PageUp / PageDown / Home / End",
        TextKey::HorizontalAuto => "横向 Auto",
        TextKey::WheelXLeftRight => "X 滚轮 / Left / Right",
        TextKey::BothAuto => "双向 Auto",
        TextKey::Always => "始终显示",
        TextKey::Hidden => "隐藏",
        TextKey::AutoTransition => "Auto 切换",
        TextKey::Scrollintoview => "ScrollIntoView",
        TextKey::Text300290Content => "300 × 290 内容",
        TextKey::FitsViewport => "内容适合 viewport",
        TextKey::ScrollMeWithoutABar => "无 bar 也可滚动",
        TextKey::Before1 => "目标前 1",
        TextKey::Before2 => "目标前 2",
        TextKey::Before3 => "目标前 3",
        TextKey::Before4 => "目标前 4",
        TextKey::Target => "目标",
        TextKey::AfterTarget => "目标后",
        TextKey::SelectFirstApi => "选择首项（API）",
        TextKey::ClearSelectionApi => "清空选择（API）",
        TextKey::UpdateFirstRow => "更新首行",
        TextKey::TableShared => "共享 Model",
        TextKey::TableSharedA => "共享 Model A：列宽 180 px，行高 34 px",
        TextKey::TableSharedB => "共享 Model B：列宽 120 px，行高 28 px，紧凑内边距",
        TextKey::TableSharedDescription => {
            "共用 Model 和 Renderer 注册表，布局、列宽、选择、导航位置、滚动和禁用状态各自独立。两侧均可编辑、插入、删除和移动共享行。"
        }
        TextKey::TableInsertRow => "插入首行",
        TextKey::ReplaceFirstHeader => "替换首列 Header",
        TextKey::Cellvalue => "CellValue",
        TextKey::Header => "Header",
        TextKey::Selection => "选择",
        TextKey::ColumnLayout => "Column Layout",
        TextKey::Virtualization => "虚拟化",
        TextKey::RowNumbersAndColumnHeadersStayFixed => {
            "行号与列 Header 在各自轴向保持固定。点击任意 cell 选择。"
        }
        TextKey::StringNumberBoolProgressAndIconUse => {
            "String、Number、Bool、Progress 和 Icon 使用不同 renderer。更新首行以刷新内容。"
        }
        TextKey::CustomHeadersCombineAStarAndLabel => {
            "自定义 Header 组合星形图标与标签。Header 与 Cell 内容使用独立 renderer。"
        }
        TextKey::ClickAColumnHeaderRowNumberOr => {
            "点击列 Header、行号或 cell。Arrow 移动 focus 边框而不改变选择。"
        }
        TextKey::NameStays180PxWideNumberAndBool => {
            "Name 固定 180px，Number 与 Bool 按 2:1 分配剩余宽度。拖动 Header 右边缘调整宽度。"
        }
        TextKey::Text2000Rows40ColumnsScrollHorizontally => {
            "2,000 行 × 40 列。可双向滚动，行列范围及可见 cell 数随 viewport 更新。"
        }
        TextKey::InitiallyDisabledEnableToSelectAndNavigate => {
            "初始禁用。启用后可选择和导航，再禁用时保留选择，停止输入及 hover。"
        }
        TextKey::BasicTree => "基础树",
        TextKey::TreeSharedA => "共享 TreeModel A",
        TextKey::TreeSharedB => "共享 TreeModel B：较矮视口",
        TextKey::TreeSharedDescription => {
            "展开与选择共享；滚动、启用 / 禁用独立。选中文件后可编辑、删除或在目录与根之间移动。禁用侧仍接收数据更新。"
        }
        TextKey::TreeSharedFolder => "共享目录",
        TextKey::TreeSharedFile => "文件 {index}（编辑 {edits} 次）",
        TextKey::TreeInsertFile => "插入文件",
        TextKey::TreeReparentSelected => "移动文件：目录 / 根",
        TextKey::ClickArrowsToExpandClickContentTo => {
            "点击箭头展开，点击内容选择。通过 Arrow / Home / End 和 Space / Enter 导航。"
        }
        TextKey::EcsHeterogeneousTree => "ECS 异构树",
        TextKey::FolderAndFileComponentsDispatchDifferentRenderers => {
            "Folder 与 File component 使用不同 renderer。业务数据保留在 ECS node 上。"
        }
        TextKey::LazyLoadingTree => "按需加载树",
        TextKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse => {
            "展开远程文件夹：未加载 → 加载中 → 已加载。折叠再展开仍保留单次请求。"
        }
        TextKey::VirtualizedTree => "虚拟化树",
        TextKey::Expand10000FilesWheelPagedownScroll => {
            "展开 10,000 个文件，使用滚轮 / PageDown 滚动。仅构造 viewport 内的行。"
        }
        TextKey::Workspace => "工作区",
        TextKey::Projects => "项目",
        TextKey::Widgetry => "Widgetry",
        TextKey::Photos => "照片",
        TextKey::Notes => "笔记",
        TextKey::Assets => "Assets",
        TextKey::RemoteFolderUnloaded => "远程文件夹（未加载）",
        TextKey::RemoteFolderLoading => "远程文件夹（加载中…）",
        TextKey::RemoteFolderLoaded => "远程文件夹（已加载）",
        TextKey::Text10000Files => "10,000 个文件",
        TextKey::OpenFile => "打开文件",
        TextKey::OpenFiles => "打开多个文件",
        TextKey::SelectFolder => "选择文件夹",
        TextKey::SelectFolders => "选择多个文件夹",
        TextKey::OpenImage => "打开图像",
        TextKey::SaveFile => "保存文件",
        TextKey::Images => "图像",
        TextKey::TextDocuments => "文本文档",
        TextKey::Background1 => "背景 1",
        TextKey::Background2 => "背景 2",
        TextKey::Stretch => "拉伸",
        TextKey::Cover => "覆盖",
        TextKey::Cover50 => "覆盖 50%",
        TextKey::Ok => "确定",
        TextKey::YesNo => "是 / 否",
        TextKey::YesNoCancel => "是 / 否 / 取消",
        TextKey::MessageBoxDemo => "MessageBox 演示",
        TextKey::GalleryHeader(HeaderKey::Dark) => "深色",
        TextKey::GalleryHeader(HeaderKey::Light) => "浅色",
        TextKey::GalleryHeader(HeaderKey::PinkDream) => "粉色梦境",
        TextKey::GalleryHeader(HeaderKey::KamuriViolet) => "冠紫色",
        TextKey::Center => "中央",
        TextKey::Right => "右侧",
        TextKey::Bottom => "底部",
        TextKey::WaitingForWaveformPage => "等待 Waveform 页面",
        TextKey::BasicLive100Hz4Channels5 => "基础实时 · 100 Hz · 4 通道 · 5 s · Polyline",
        TextKey::BasicReplay8Khz4Channels5 => {
            "基础回放 · 8 kHz · 4 通道 · 5 s · 固定 .wfrm · Envelope"
        }
        TextKey::StressLive64Khz64Channels10 => "压力实时 · 64 kHz · 64 通道 · 10 s",
        TextKey::CursorRealWindowsMouseValidation => "Cursor · 真实 Windows Mouse 验证",
        TextKey::Pointer12Frames8Fps64PxKeep => {
            "Pointer：12 帧 / 8 FPS / 64px。停住真实鼠标观察循环。"
        }
        TextKey::Wait24Frames8Fps64PxThe => {
            "Wait：24 帧 / 8 FPS / 64px。加载环旋转，冠偶尔吃一口布丁。"
        }
        TextKey::Text42StaticCursors48PxPointerAndWait => {
            "42 种静态 Cursor：48px；Pointer、Wait：64px。预览与语义逐格对照。BRP Custom Pointer 不改变 OS Cursor。"
        }
        TextKey::WidgetryExtendedSemanticsKamuriSystemComparison => {
            "Widgetry 扩展语义 · Kamuri / System 对照"
        }
        TextKey::NestedNodeContentRootButtonDisabledAffects => {
            "多层 Node 内容：根 Button Disabled 影响 WidgetryText / Icon；原生固定 Text 保持青色。"
        }
        TextKey::ToggleRootButtonDisabled => "切换根 Button Disabled",
        TextKey::DisabledContainersAffectDescendantsThroughPlainNodes => {
            "禁用容器穿过普通 Node；恢复父级后，仅本地单独禁用的 CheckBox 继续禁用。"
        }
        TextKey::InheritParentDisabled => "继承父级 Disabled",
        TextKey::LocallyDisabledStaysDisabledAfterParentIs => "本地也 Disabled，父级恢复后仍禁用",
        TextKey::ToggleContainerDisabled => "切换容器 Disabled",
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
        TextKey::OpenPureTheme => "打开纯 Theme",
        TextKey::OpenOverrideInstance => "打开覆盖实例",
        TextKey::CycleTheme => "循环切换 Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::ThemeSourceValuesPartStateFieldSwatch) => {
            "Theme 原值目录：部件 / state / 字段 / 色块 / HEX（含 alpha），不是控件最终颜色。"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::RealWidgetsPureThemeOnTheLeft) => {
            "真实控件：左侧纯 Theme，右侧只覆盖指定字段。应用 / 清除通过公开 API；清除回到当前 Theme。"
        }
        TextKey::YGutterInducesXOverflowNoCorner => "Y gutter 诱发 X overflow；无 Corner",
        TextKey::KeepBarAndGutterEvenWithoutOverflow => "无 overflow 时仍保留 bar 和 gutter",
        TextKey::ScrollableContentWithNoVisibleBar => "内容可滚动，但不显示 bar",
        TextKey::ToggleContentHeightToShowOrHide => "切换内容高度，观察 bar 增减",
        TextKey::ScrollThenClickTheButtonToAlign => "滚动后点击按钮，将 Target 对齐到顶部",
        TextKey::FixedColorConvenienceApi0F766ERetainedAcross => {
            "固定色便利 API=#0F766E；Theme 和父级 Disabled 切换后保持。清除后继承 Button 前景色。"
        }
        TextKey::OnlyFieldOpenBorder7C3AedOpenPopup => {
            "仅 field.open.border=#7C3AED；展开 Popup，list 与 indicator 仍取 Theme。"
        }
        TextKey::OnlyItemSelectedBackground0F766EMovePointer => {
            "仅 item.selected.background=#0F766E；选中后移开 pointer，active border 不变。"
        }
        TextKey::OnlyItemSelectedBackground0F766EMovePointer2 => {
            "仅 item.selected.background=#0F766E；选中后移开 pointer，expander 仍取 Theme。"
        }
        TextKey::OnlyCellSelectedBackground0F766EMovePointer => {
            "仅 cell.selected.background=#0F766E；选择后移开 pointer，focus border 不变。"
        }
        TextKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D => {
            "仅 normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D]；Disabled palette 仍取 Theme。"
        }
        TextKey::OnlyHoveredBackground7C3AedRealHoverPressed => {
            "仅 hovered.background=#7C3AED；真实 hover，Pressed 仍取 Theme。"
        }
        TextKey::OnlyOptionCheckedNormalDot0F766EMove => {
            "仅 option.checked.normal.dot=#0F766E；选择后移开 pointer。"
        }
        TextKey::OnlyCheckedNormalMark0F766EMovePointer => {
            "仅 checked.normal.mark=#0F766E；Checked 后移开 pointer，Hover/Disabled 仍取 Theme。"
        }
        TextKey::OnlyEditableFocusedBorder7C3AedClickTo => {
            "仅 editable.focused.border=#7C3AED；点击取得 focus 后输入。"
        }
        TextKey::OnlyReadOnlyFocusedBorder7C3AedSelect => {
            "仅 read_only.focused.border=#7C3AED；可选择 / copy，不能编辑。"
        }
        TextKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal => {
            "仅 vertical.thumb.hovered.background=#7C3AED；横向 thumb 不变。"
        }
        TextKey::OnlyPopupNormalBackground7C3AedRealHover => {
            "仅 popup.normal.background=#7C3AED；真实 hover 出 Tooltip，禁用 anchor 仍可显示。"
        }
        TextKey::WindowOnlyFrameNormalBorder7C3AedClear => {
            "Window：仅 frame.normal.border=#7C3AED；已打开实例可直接清除。"
        }
        TextKey::WindowImageSameBorderOverrideCover50 => {
            "Window Image：同一 border 覆盖；Cover / 50% opacity 不因切换 Theme 或清除覆盖改变。"
        }
        TextKey::MessageboxOnlyBodyNormalBackground0F766EApply => {
            "MessageBox：仅 body.normal.background=#0F766E；modal 内提供应用 / 清除和 Theme 切换。"
        }
        TextKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal => {
            "FileDialog：仅 entry.selected.background=#0F766E；NonModal，选中文件后清除，无需重新打开。"
        }
        TextKey::Common(CommonKey::None) => "<无>",
        TextKey::ComboStatus => {
            "项目：{count} | 选中：{selected} | 索引：{index} | 值：{label}\n首项禁用：{disabled} | ValueChange：{changes}"
        }
        TextKey::ListItem => "项目 {index}  （编辑：{edits}）",
        TextKey::ListInsertedItem => "插入的项目  （编辑：{edits}）",
        TextKey::ListStatus => {
            "Model 项目：{count} | 显示行：{rows} | 显示范围：{start}..{end}\n选中：{selected} | active：{active} | ValueChange：{changes}"
        }
        TextKey::TreeFolder => "[文件夹] {label}",
        TextKey::TreeFile => "[文件] {label}  （{bytes} 字节）",
        TextKey::TreeStatus => {
            "可见：{visible} | 行：{rows} {start}..{end} | 选中：{selected} | {state}"
        }
        TextKey::Common(CommonKey::Enabled) => "启用",
        TextKey::TableItem => "项目 {index}",
        TextKey::TableUpdated => "已更新 {edits}",
        TextKey::TableColumn => "列 {index}",
        TextKey::TableUpdatedHeader => "已更新的 Header",
        TextKey::TableName => "名称",
        TextKey::TableNumber => "数值",
        TextKey::TableBool => "Bool",
        TextKey::TableProgress => "进度",
        TextKey::TableStatus => {
            "Model：{rows} 行 × {columns} 列 | 可见 cell：{cells} | 行 {row_start}..{row_end}，列 {column_start}..{column_end}\n选择：{selection} | cursor：{cursor} | 滚动：{scroll} | {state} | event：选择 {select}，resize 开始/变化/结束/取消 {start}/{change}/{end}/{cancel}"
        }
        TextKey::TooltipCenter => "中央位置",
        TextKey::TooltipRight => "右侧位置",
        TextKey::TooltipBottom => "底部位置",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandThemeColors) => "展开 / 折叠 Theme 颜色",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandOverrides) => "展开 / 折叠显式覆盖",
        TextKey::ButtonActivations => "Button Activate：{count}",
        TextKey::ColorShowcase(ColorShowcaseKey::OverrideStatus) => {
            "最近操作：{action}；实际变化：{changed}"
        }
        TextKey::CursorTheme => "当前 Cursor theme：{theme}",
        TextKey::WaveformStatus => {
            "64 kHz · 64 通道 · 10 s · producer {producer} / 显示 {displayed} · {fps} FPS / {ms} ms · 丢弃 {dropped}"
        }
        TextKey::ResultWaiting => "结果：等待中…",
        TextKey::ResultFailed => "结果：无法打开对话框",
        TextKey::ResultCancelled => "结果：已取消",
        TextKey::ResultPaths => "结果：{count} 个路径{paths}",
        TextKey::WindowImageTitle => "背景 {index} - {mode}",
        TextKey::OverrideWindowTitle => "{kind} · {sample}",
        TextKey::OverrideInstance => "覆盖实例",
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
