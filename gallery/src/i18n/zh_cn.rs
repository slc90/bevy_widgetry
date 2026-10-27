use super::TextKey;
use super::{ButtonKey, CheckBoxKey, ComboBoxKey, ScrollAreaKey, TextFieldKey, TooltipKey};
use super::{ColorShowcaseKey, CommonKey, HeaderKey, NavigationKey};
use super::{CursorKey, WaveformKey, WindowKey};
use super::{ListViewKey, TableKey, TreeKey};

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::Navigation(NavigationKey::DatePicker) => "日期选择",
        TextKey::DatePicker(key) => match key {
            super::DatePickerKey::Description => {
                "浏览月份不改变选择；点击标题进入月份与年份视图。历史只记录已提交的真实变化，Today 不会自动选择日期。"
            }
            super::DatePickerKey::Main => "完整交互日历",
            super::DatePickerKey::Unselected => "初始未选择",
            super::DatePickerKey::Bounded => "可选范围：2026-10-10 至 2026-10-20",
            super::DatePickerKey::Disabled => "禁用日历 · 仍可程序化修改",
            super::DatePickerKey::SetDate => "设置 2026-10-26",
            super::DatePickerKey::Same => "重复设置当前值",
            super::DatePickerKey::Clear => "清空选择",
            super::DatePickerKey::Previous => "上个月",
            super::DatePickerKey::Next => "下个月",
            super::DatePickerKey::Jump => "跳转到 2027 年 1 月",
            super::DatePickerKey::SetDisabled => "程序设置禁用实例为 2026-11-06",
            super::DatePickerKey::Status => {
                "选中日期：{selected}\n显示年月：{month}\n选择事件：{selection_events} · 月份事件：{month_events}"
            }
            super::DatePickerKey::History => "最近事件（最多 8 条）：\n{events}",
            super::DatePickerKey::Operation => "最近 API：{action} · {changed}",
            super::DatePickerKey::Ready => "操作日历或使用公开 API 按钮。",
            super::DatePickerKey::None => "未选择",
            super::DatePickerKey::Changed => "状态已变化",
            super::DatePickerKey::Unchanged => "状态未变化 · 无新事件",
            super::DatePickerKey::Overrides => {
                "覆盖文字、边框及选中底色。Today 笔圈在四套主题和禁用状态下均保留原色。"
            }
        },
        TextKey::Navigation(NavigationKey::PowerInhibit) => "休眠抑制",
        TextKey::PowerInhibit(key) => match key {
            super::PowerInhibitKey::Description => {
                "在 Gallery 运行期间请求 Windows 保持显示器和系统运行。默认关闭，按钮直接调用真实的 Windows 电源请求 API。"
            }
            super::PowerInhibitKey::Enable => "开启禁止休眠",
            super::PowerInhibitKey::Disable => "关闭禁止休眠",
            super::PowerInhibitKey::Enabled => {
                "已开启 · Windows 已接收防止自动息屏和系统休眠的两类请求。"
            }
            super::PowerInhibitKey::Disabled => {
                "已关闭 · Gallery 当前没有持有防止自动息屏或系统休眠的请求。"
            }
            super::PowerInhibitKey::CleanupRequired => {
                "清理未完成 · 可能仍有请求保留，请点击关闭按钮重试。"
            }
            super::PowerInhibitKey::NoError => "",
            super::PowerInhibitKey::Error => "上次操作失败：{error}",
            super::PowerInhibitKey::Instructions => {
                "1. 先在 Windows 设置中缩短自动关闭显示器和睡眠的等待时间。\n2. 在此开启禁止休眠，然后停止操作鼠标和键盘，等待超过系统设定的时间。\n3. 关闭禁止休眠后再次等待，对比正常空闲行为。测试结束后恢复你原来的系统等待时间。"
            }
            super::PowerInhibitKey::Lifecycle => {
                "切换页面、失去焦点、最小化 Gallery 或隐藏到托盘都会保留请求。请返回本页关闭，或彻底退出 Gallery；关闭到托盘不会释放请求。下次启动仍默认关闭。"
            }
            super::PowerInhibitKey::Limits => {
                "仅抑制空闲导致的自动息屏和系统睡眠。手动睡眠、合盖和电源按钮仍按 Windows 策略执行；系统电源策略也可能限制已接收的请求。"
            }
        },
        TextKey::Navigation(NavigationKey::Job) => "后台任务",
        TextKey::Job(key) => match key {
            super::JobKey::Description => {
                "真实I/O、CPU与命名独立线程Job。先选择一行，再取消、订阅或移除Manager记录。B只接收加入后的新数据；慢B会对生产者施加背压，丢弃B可解除阻塞。进度条只根据实际消费的Progress更新；Manager终态不代表各接收器已读到Finished。缓冲容量4，最多8行。"
            }
            super::JobKey::StartIo => "启动短I/O",
            super::JobKey::StartCpu => "启动CPU计算",
            super::JobKey::StartThread => "启动命名线程",
            super::JobKey::StartFailure => "启动失败任务",
            super::JobKey::StartLateComplete => "启动3秒不可中断调用",
            super::JobKey::SubscribeB => "订阅B",
            super::JobKey::DropA => "丢弃A",
            super::JobKey::DropB => "丢弃B",
            super::JobKey::ToggleSlowB => "切换B快慢",
            super::JobKey::Cancel => "取消所选Job",
            super::JobKey::Remove => "移除Manager记录",
            super::JobKey::Ready => "请选择一种任务启动。离页会取消本页Job，不阻塞UI。",
            super::JobKey::Selection => "所选#{id} · {kind} · Manager {state} · B：{speed}",
            super::JobKey::Stats => {
                "Manager：{state} · 已生产批次：{produced}/{total} · 双方最后Batch共享Arc：{shared}
A：条目{items_a} · 批次{a} · 首/末序号{first_a}/{last_a} · 进度{progress_a}/{total} · {flow_a}
B：条目{items_b} · 批次{b} · 首/末序号{first_b}/{last_b} · 进度{progress_b}/{total} · {flow_b}"
            }
            super::JobKey::Select => "选择#{id} · {kind}",
            super::JobKey::Operation => "{action}：{result}",
            super::JobKey::Receiving => "尚未读到Finished",
            super::JobKey::Detached => "无接收器",
            super::JobKey::Slow => "每200ms读取一条",
            super::JobKey::Fast => "每帧最多8条",
            super::JobKey::Limit => "已存在8行，请离页后重新进入以开始新的演示。",
        },
        TextKey::GalleryClose(key) => match key {
            super::CloseKey::Title => "关闭 Widget Gallery",
            super::CloseKey::Prompt => "请选择关闭方式：",
            super::CloseKey::Exit => "退出程序",
            super::CloseKey::Hide => "隐藏窗口，进入系统托盘",
            super::CloseKey::Confirm => "确定",
            super::CloseKey::Cancel => "取消",
            super::CloseKey::Waiting => "正在等待后台任务停止…",
            super::CloseKey::TimedOut => "后台任务收尾已超时，请选择继续等待或强制退出。",
            super::CloseKey::Continuing => "正在继续等待后台任务，原始截止时间保持不变。",
            super::CloseKey::ShutdownDetails => {
                "仍在执行或等待线程回收的任务：{remaining}；全局超时：{timeout} 毫秒。已停止接纳新任务。"
            }
            super::CloseKey::ContinueWaiting => "继续等待",
            super::CloseKey::ForceExit => "强制退出",
            super::CloseKey::ForceWarning => {
                "强制退出不保证完成后台清理、设备结束握手、缓存刷盘及外部副作用一致性。"
            }
        },
        TextKey::Navigation(NavigationKey::Canvas) => "画布",
        TextKey::Canvas(key) => match key {
            super::CanvasKey::Description => {
                "点击单选，Ctrl+点击切换多选，按住所选图形整批平移、Delete整批删除。八个方点整体缩放或镜像，圆点旋转（Shift：15°吸附），内侧菱形改宽高/半径，内侧圆点改矩形圆角。Delete 删除。矩形/椭圆工具一次绘制，Esc 取消绘制。Space + 左键平移至左键释放；滚轮围绕指针缩放。工具与缩放按钮仅操作主视图。路径节点模式：先选Path，再拖方点锚点或菱形控制点；Esc不回滚。Pen点按输入：L直线、Q二次（控点/终点）、B三次（双控点/终点）、A弧线；C闭合子路径、N新子路径、Enter完成整条Path并保持Pen、Esc丢弃草稿。"
            }
            super::CanvasKey::Select => "选择",
            super::CanvasKey::Nodes => "路径节点",
            super::CanvasKey::Rectangle => "矩形",
            super::CanvasKey::Ellipse => "椭圆",
            super::CanvasKey::Pen => "Pen",
            super::CanvasKey::Add => "添加示例",
            super::CanvasKey::ZoomIn => "放大 +",
            super::CanvasKey::ZoomOut => "缩小 −",
            super::CanvasKey::Reset => "重置主视图",
            super::CanvasKey::Auxiliary => "隐藏 / 显示辅助视图",
            super::CanvasKey::Region => "切换导出区域",
            super::CanvasKey::Scale => "切换 PNG 倍率",
            super::CanvasKey::Svg => "导出 SVG",
            super::CanvasKey::Png => "导出 PNG",
            super::CanvasKey::Main => "主视图",
            super::CanvasKey::Secondary => "共享 Document · 独立视图",
            super::CanvasKey::Paths => {
                "文件：{path} / canvas.svg 与 canvas.png · 直接覆盖已有文件。"
            }
            super::CanvasKey::Ready => "选择 SVG 或 PNG，以透明背景导出已提交图形。",
            super::CanvasKey::Readout => {
                "主视图：{zoom_a}× · 中心（{center_a}）· {mode}\n辅助视图：{zoom_b}× · 中心（{center_b}）\n导出：{region} · PNG {scale}×"
            }
            super::CanvasKey::AutoRegion => "内容 AABB",
            super::CanvasKey::FixedRegion => "区域 (−100, −75) … (100, 75)",
            super::CanvasKey::Saved => "已保存 {path} · {size}",
            super::CanvasKey::Busy => "已有 PNG 正在导出，请等待完成后重试。",
            super::CanvasKey::Pending => "PNG 正在导出…",
            super::CanvasKey::Failed => "导出失败：{error}",
        },
        TextKey::Navigation(NavigationKey::Collapsible) => "折叠卡片",
        TextKey::Collapsible(key) => match key {
            super::CollapsibleKey::Description => {
                "只有箭头切换卡片。修改控件后折叠再展开，值仍保留。F8 / F9 分别切换基本 / 高级设置，不提前抢焦点。依次禁用根、禁用 Header、恢复根，Header 仍禁用。离开页面后示例重置。"
            }
            super::CollapsibleKey::Basic => "基本设置",
            super::CollapsibleKey::BasicHint => "初始展开 · 保留勾选与滑块值",
            super::CollapsibleKey::Advanced => "高级设置",
            super::CollapsibleKey::AdvancedHint => "初始折叠 · 编辑时可按 F9 折叠",
            super::CollapsibleKey::Enabled => "接收更新",
            super::CollapsibleKey::Volume => "音量",
            super::CollapsibleKey::EditHint => "在此输入，折叠再展开后文字保留。",
            super::CollapsibleKey::Expand => "展开",
            super::CollapsibleKey::Collapse => "折叠",
            super::CollapsibleKey::Same => "再次设置当前值",
            super::CollapsibleKey::RootDisabled => "切换根禁用",
            super::CollapsibleKey::HeaderDisabled => "切换 Header 禁用",
            super::CollapsibleKey::Status => {
                "展开：{expanded} · 通知：{events} · 最近：{last}\n根禁用：{root_disabled} · Header 请求：{header_disabled}"
            }
        },
        TextKey::Navigation(NavigationKey::NumberInput) => "数值输入",
        TextKey::NumberInput(key) => match key {
            super::NumberInputKey::Description => {
                "输入草稿，按 Enter 或离开后提交，Esc 取消。方向键或按住 −/+ 可步进。正式值与事件次数仅在真实变化时更新。预设按钮覆盖草稿，只读与禁用时仍可设值。"
            }
            super::NumberInputKey::Signed => "i32 · 0..10 · 步长 3 · 端点可达",
            super::NumberInputKey::Unsigned => "u8 · 无显式边界 · 类型最大值 255",
            super::NumberInputKey::Float => "f64 · 步长 0.1 · 两位小数 · 聚焦显示完整精度",
            super::NumberInputKey::ReadOnly => "只读 · 可选择/复制 · 预设仍可设值",
            super::NumberInputKey::Disabled => "控件本地禁用 · 预设仍可设值",
            super::NumberInputKey::InheritedDisabled => "继承父级禁用 · 预设仍可设值",
            super::NumberInputKey::Fixed => "固定范围 · 最小值 = 最大值 = 7 · 不发变化事件",
            super::NumberInputKey::Limits => "i64 · 类型极限 · 步长 i64::MAX",
            super::NumberInputKey::Narrow => "父级 50px · 控件最小宽 134px · 外层 180px 裁剪",
            super::NumberInputKey::Status => {
                "正式值：{value} · 变化次数：{events} · 最近事件：{last}"
            }
            super::NumberInputKey::Set => "设为 {value}",
        },
        TextKey::Navigation(NavigationKey::Splitter) => "分割面板",
        TextKey::Splitter(key) => match key {
            super::SplitterKey::Description => {
                "按住细线拖动，移出面板后仍可调整。A 初始 30%（1008px 容器内为 300 / 700px）。主动设置受双方当前尺寸限制；缩放父级保留目标比例，不增加数值事件。Esc 与方向键不控制 Handle。"
            }
            super::SplitterKey::Status => {
                "A 目标：{target}% · 实际：{actual}% · 数值事件：{events} · 面板按钮：{clicks} · 拒绝：{errors} · resizable：{resizable} · Disabled：{disabled}"
            }
            super::SplitterKey::PaneA => "面板 A · 初始最小 300 / 最大 800px",
            super::SplitterKey::PaneB => "面板 B · 最小 200 / 最大 700px",
            super::SplitterKey::PaneAction => "面板内按钮",
            super::SplitterKey::Nested => "嵌套纵向 · 50% · 命中区 12 / 细线 4px",
            super::SplitterKey::Top => "上方 A",
            super::SplitterKey::Bottom => "下方 B",
            super::SplitterKey::Left => "左侧 A",
            super::SplitterKey::Right => "右侧 B",
            super::SplitterKey::Low => "World 10%（钳制）",
            super::SplitterKey::High => "World 90%（钳制）",
            super::SplitterKey::Queued => "Commands 60%",
            super::SplitterKey::Same => "设置相同比例",
            super::SplitterKey::Invalid => "拒绝 NaN",
            super::SplitterKey::Resize => "父级 1008 / 865px",
            super::SplitterKey::Constraints => "A 最小值 300 / 100px",
            super::SplitterKey::Resizable => "切换 resizable",
            super::SplitterKey::Disabled => "切换根 Disabled",
            super::SplitterKey::Override => "覆盖 Handle 配色",
            super::SplitterKey::Clear => "恢复主题配色",
            super::SplitterKey::Maximum => {
                "有限最大值 · A / B 各最大 300px · Splitter 在 1000px 父级中只占 608px"
            }
            super::SplitterKey::Minimum => {
                "外层横向 ScrollArea · 视口 360px，双方最小值 300 + 400 + 8 = 708px；滚动查看 B"
            }
            super::SplitterKey::Fixed => "resizable=false · 面板内按钮仍可操作",
            super::SplitterKey::DisabledExample => "根 Disabled · 面板内按钮继承禁用",
        },
        TextKey::Navigation(NavigationKey::ProgressBar) => "进度条",
        TextKey::ProgressBar(key) => match key {
            super::ProgressBarKey::Description => {
                "使用外部按钮调整进度，观察平滑填充、倒退和未知进度循环。未知模式下加减会调整示例的模拟输入；切回确定模式后直接显示该比例。"
            }
            super::ProgressBarKey::Increase => "+10%",
            super::ProgressBarKey::Decrease => "-10%",
            super::ProgressBarKey::Reset => "重置",
            super::ProgressBarKey::Mode => "切换模式",
            super::ProgressBarKey::Percentage => "{percent}%",
            super::ProgressBarKey::Running => "正在运行，进度未知",
            super::ProgressBarKey::Override => "应用填充色覆盖",
            super::ProgressBarKey::Clear => "恢复主题配色",
        },
        TextKey::Navigation(NavigationKey::Tabs) => "Tabs",
        TextKey::Tabs(key) => match key {
            super::TabsKey::Description => {
                "点击标签后使用对应方向键，第 1 项禁用，导航跳过它并在首尾停止。输入后切页再返回，正文和页面滚动位置保留。F8 / F9 在不移动焦点的情况下程序选择横向 / 纵向末页；离开整个 Gallery 页面后，两组演示重置。"
            }
            super::TabsKey::Horizontal => "横向 · 独立页面与标签栏自动滚动",
            super::TabsKey::Vertical => "纵向 · 长标题单行省略与完整提示",
            super::TabsKey::Title => "工作区 {index}",
            super::TabsKey::LongTitle => "高级渲染与图形设备兼容性设置及详细配置",
            super::TabsKey::Page => "第 {index} 页 · 在下方输入，再切页返回",
            super::TabsKey::Retained => "页面滚动与标签栏滚动互相独立。",
            super::TabsKey::First => "选择首项",
            super::TabsKey::Last => "选择末项",
            super::TabsKey::Reselect => "再次选择当前项",
            super::TabsKey::ToggleRoot => "切换整组禁用",
            super::TabsKey::ToggleHeader => "切换已选标签禁用",
            super::TabsKey::ToggleSpace => "切换紧凑 / 宽敞布局",
            super::TabsKey::Status => {
                "选中 {index} · 通知 {events} 次 · 最近通知 {last} · 整组禁用：{root_disabled} · 已选标签禁用：{header_disabled}"
            }
            super::TabsKey::Overrides => {
                "两个短标签无需滚动槽。应用整组选中颜色和第二个标签的局部覆盖；清除后恢复主题。"
            }
        },
        TextKey::Navigation(NavigationKey::Segmented) => "Segmented",
        TextKey::Segmented(super::SegmentedKey::Description) => {
            "点击选项获得组焦点；横向用左右键，竖向用上下键，跳过禁用项并在首尾停止。Tab 跳过所有分段组。"
        }
        TextKey::Segmented(super::SegmentedKey::Basic) => "基础单选 · 独立用户事件计数",
        TextKey::Segmented(super::SegmentedKey::Natural) => "内容自适应 · 不同长度文字",
        TextKey::Segmented(super::SegmentedKey::Equal) => "等宽 · 相同文字均分 360px",
        TextKey::Segmented(super::SegmentedKey::OptionDisabled) => "单项禁用 · 中间项不可交互",
        TextKey::Segmented(super::SegmentedKey::SelectedDisabled) => "已选项禁用 · 保留选中高亮",
        TextKey::Segmented(super::SegmentedKey::GroupDisabled) => "整组禁用 · 允许程序化选择",
        TextKey::Segmented(super::SegmentedKey::Vertical) => "竖向 · 上下键，中间项禁用",
        TextKey::Segmented(super::SegmentedKey::Day) => "日",
        TextKey::Segmented(super::SegmentedKey::Week) => "周",
        TextKey::Segmented(super::SegmentedKey::Month) => "月",
        TextKey::Segmented(super::SegmentedKey::Year) => "年",
        TextKey::Segmented(super::SegmentedKey::Today) => "今天",
        TextKey::Segmented(super::SegmentedKey::Last7) => "最近7天",
        TextKey::Segmented(super::SegmentedKey::Last30) => "最近30天",
        TextKey::Segmented(super::SegmentedKey::Programmatic) => "程序选择禁用组的下一项",
        TextKey::Segmented(super::SegmentedKey::ToggleDisabled) => "切换整组禁用",
        TextKey::Segmented(super::SegmentedKey::FocusBefore) => "组前的 Tab 目标",
        TextKey::Segmented(super::SegmentedKey::FocusAfter) => "组后的 Tab 目标",
        TextKey::Segmented(super::SegmentedKey::Status) => {
            "选中索引：{index} · 用户事件：{events} · 程序操作：{programmatic}"
        }
        TextKey::Segmented(super::SegmentedKey::Overrides) => {
            "覆盖组级已选背景，并局部覆盖中间 Option；其他字段继续跟随主题。"
        }
        TextKey::Navigation(NavigationKey::Slider) => "Slider",
        TextKey::Slider(super::SliderKey::Description) => {
            "拖动或使用方向键，控件旁数值实时更新；垂直滑块向上增大。"
        }
        TextKey::Slider(super::SliderKey::Continuous) => "连续数值 · 默认轨道 6px / Thumb 20px",
        TextKey::Slider(super::SliderKey::Stepped) => {
            "步长 3 · 0 → 3 → 6 → 9 → 10 / 10 → 7 → 4 → 1 → 0"
        }
        TextKey::Slider(super::SliderKey::Vertical) => "垂直 · 自下向上增大",
        TextKey::Slider(super::SliderKey::Custom) => "自定义尺寸 · 轨道 10px / Thumb 28px",
        TextKey::Slider(super::SliderKey::Disabled) => "禁用 · 拒绝用户输入，保留当前位置",
        TextKey::Slider(super::SliderKey::Overrides) => {
            "通过公开 API 覆盖 Fill、Thumb 与焦点环颜色，清除后恢复当前主题。"
        }
        TextKey::Button(ButtonKey::ToggleDescription) => {
            "二态按钮：点击或按空格 / 回车切换并保持选中外观。"
        }
        TextKey::Button(ButtonKey::ToggleButton) => "切换按钮",
        TextKey::Button(ButtonKey::ToggleProgrammatic) => "程序化选中 / 取消",
        TextKey::Button(ButtonKey::HoldTitle) => "长按确认 · 64 / 112 / 160 px",
        TextKey::Button(ButtonKey::HoldDescription) => {
            "按住 3 秒确认，提前松开在 1.2 秒内衰减。Enter / Space；按 F7 切换本地禁用、F8 父级禁用、F9 显示。"
        }
        TextKey::Button(ButtonKey::HoldConfirm) => "确认",
        TextKey::Button(ButtonKey::HoldFocus) => "聚焦 112px 按钮",
        TextKey::Button(ButtonKey::HoldBlur) => "移开焦点",
        TextKey::Button(ButtonKey::HoldDisable) => "切换禁用 (F7)",
        TextKey::Button(ButtonKey::HoldParentDisable) => "父级禁用 (F8)",
        TextKey::Button(ButtonKey::HoldVisibility) => "隐藏 / 显示 (F9)",
        TextKey::Button(ButtonKey::HoldUnderlay) => "透明角落：下层按钮",
        TextKey::Button(ButtonKey::HoldOverrides) => "独立液体颜色覆写，清除后恢复当前主题",
        TextKey::Button(ButtonKey::LoadingDescription) => {
            "Loading：点击后阻止重复激活。点击结束按钮或按 F1，观察 Wait → Pointer 光标切换。"
        }
        TextKey::Button(ButtonKey::StartLoading) => "开始 Loading",
        TextKey::Button(ButtonKey::FinishLoading) => "结束 Loading",
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
        TextKey::ComboBox(ComboBoxKey::LocalSearch) => "本地搜索（共享数据）",
        TextKey::ComboBox(ComboBoxKey::SearchPrompt) => "搜索",
        TextKey::ComboBox(ComboBoxKey::NoResults) => "没有匹配的选项",
        TextKey::ComboBox(ComboBoxKey::SearchStatus) => {
            "查询：{query} | 匹配：{matches} | 已选 ID：{selected} | Active：{active}"
        }
        TextKey::ComboBox(ComboBoxKey::SearchInstructions) => {
            "支持中英文水果名。方向键浏览，Enter 确认，Esc 取消；重开清空查询。禁用搜索示例拒绝输入。"
        }
        TextKey::ComboBox(ComboBoxKey::Placeholder) => "请选择…",
        TextKey::ComboBox(ComboBoxKey::Renderers) => "Renderer 展示",
        TextKey::ComboBox(ComboBoxKey::DataDriven) => "数据驱动",
        TextKey::ComboBox(ComboBoxKey::InsertMoveOrEditSelectionKeepsIts) => {
            "初始未选择并显示提示；插入、移动或编辑保留选中项身份。删除选中项或清空后恢复提示，不自动首选。\n香蕉初始禁用。禁用项仍可见但不可选。程序化选择与清空仅在变化时通知。"
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
        TextKey::Tree(TreeKey::SelectVisible) => "选择可见节点（Ctrl+A）",
        TextKey::Tree(TreeKey::ExpandAll) => "展开已加载节点",
        TextKey::Tree(TreeKey::CollapseAll) => "全部折叠",
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
        TextKey::ListView(ListViewKey::ResetViewOrder) => "重置 View 顺序",
        TextKey::ListView(ListViewKey::ToggleDrag) => "切换拖拽",
        TextKey::ListView(ListViewKey::ToggleFilter) => "切换 B 偶数过滤",
        TextKey::ListView(ListViewKey::ListSharedText) => "共享 Model A：文字",
        TextKey::ListView(ListViewKey::ListSharedIcon) => "共享 Model B：图标 + 文字",
        TextKey::ListView(ListViewKey::ListSharedDescription) => {
            "右侧偶数标签倒序。拖动一项到行前/行后只重排当前 View，共享 Model 与另一侧顺序不变，多选集合保持；Ctrl/Shift 选择，两侧选择独立。"
        }
        TextKey::ListView(ListViewKey::SmallList) => "小列表",
        TextKey::ListView(ListViewKey::ArrowHomeEndActiveSpaceEnterSelect) => {
            "已开启拖拽：每次只移动一项。单击单选，Ctrl 切换，Shift 选区；方向/Home/End 移动 active，Space/Enter 确认。"
        }
        TextKey::ListView(ListViewKey::VirtualizedLargeList) => "虚拟化大列表",
        TextKey::ListView(ListViewKey::Text10000ItemsWheelPageupPagedownJump) => {
            "未开启拖拽。10,000 项；Ctrl+A 全选并显示数量，滚动检查离屏选中项，支持 PageUp/PageDown、编辑与调整大小。"
        }
        TextKey::ListView(ListViewKey::DisabledListview) => "禁用的 ListView",
        TextKey::ListView(ListViewKey::UserInputBlockedModelSelectionAndScroll) => {
            "阻止用户输入，Model、选择和滚动按钮仍可用。"
        }
        TextKey::ListView(ListViewKey::DisabledListviewitem) => "禁用的 ListViewItem",
        TextKey::ListView(ListViewKey::InitiallyItems13DisabledArrowClick) => {
            "第 1 / 3 项初始禁用，不参与用户选择、Shift 范围与 Ctrl+A。Arrow / 点击仍可激活。"
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
        TextKey::Table(TableKey::SelectAllApi) => "全选当前 View（API）",
        TextKey::Table(TableKey::ToggleEditing) => "切换编辑开关",
        TextKey::Table(TableKey::ToggleColumnReordering) => "启用 / 停用列拖拽",
        TextKey::Table(TableKey::ResetColumnOrder) => "重置 View 列顺序",
        TextKey::Table(TableKey::InsertColumn) => "在 Model 第二列位置插入列",
        TextKey::Table(TableKey::RemoveColumn) => "删除 Model 首列",
        TextKey::Table(TableKey::MoveColumn) => "将 Model 首列移到末尾",
        TextKey::Table(TableKey::ReplaceModel) => "替换共享 Model（两个 View）",
        TextKey::Table(TableKey::ColumnReorderingDescription) => {
            "列拖拽默认关闭。启用后拖动标题主体，沿插入线释放提交；在标题区外释放则取消。最右侧 6px 调整列宽，排序 / 筛选按钮保持各自操作。仅当前 View 改序，仍冻结最前 N 个位置。共享 Model 按钮可观察：新列追加到手动 View 末尾、删列修复、Model 移列和行编辑保留手动顺序、显式重置恢复当前 Model 次序，以及替换 Model 后两个 View 清除旧偏好。"
        }
        TextKey::Table(TableKey::ColumnOrderStatus) => {
            "\n列拖拽：{state}\nModel 列顺序：{model}\nView 列顺序：{view}"
        }
        TextKey::Table(TableKey::CycleFrozen) => "冻结列数：0 / 1 / 2 / 99",
        TextKey::Table(TableKey::FrozenDescription) => {
            "每个 View 冻结前 N 个显示列，横滚只移动其余列，两区纵向同步。列宽超过视口也不减少 N。共享 View B 初始冻结一列；可调整视口和列宽观察严格冻结与裁剪。"
        }
        TextKey::Table(TableKey::FrozenStatus) => "\n冻结列数：配置 {configured}，实际 {effective}",
        TextKey::Table(TableKey::CopySelection) => "复制选区（TSV）",
        TextKey::Table(TableKey::ToggleCopyFormatter) => "启用 / 禁用布尔复制格式化器",
        TextKey::Table(TableKey::CopyDescription) => {
            "Ctrl+C 复制当前 View 完整逻辑选区（含离屏单元格），不含表头。数字使用列覆盖格式，布尔输出 yes/no，进度使用强类型格式化器。备注保留真实 Tab、换行与引号。编辑器内 Ctrl+C 归文本选区所有。禁用布尔格式化器后，复制含布尔值的选区会明确失败并保留原剪贴板。"
        }
        TextKey::Table(TableKey::CopyPasteHint) => "在此处或外部表格中粘贴检查 TSV：",
        TextKey::Table(TableKey::CopySucceeded) => "选区已复制，剪贴板只包含数据。",
        TextKey::Table(TableKey::CopySkipped) => "未复制：请选择数据、退出编辑并启用 Table。",
        TextKey::Table(TableKey::CopyFailed) => "复制失败，保留原有剪贴板：",
        TextKey::Table(TableKey::SingleLineNotes) => "备注（Enter）",
        TextKey::Table(TableKey::MultiLineNotes) => "备注（Ctrl+Enter）",
        TextKey::Table(TableKey::InvalidEditValue) => {
            "请输入有效的非负整数，或 0 到 1 之间的进度值。"
        }
        TextKey::Table(TableKey::EditRejected) => {
            "应用拒绝本次修改。数字不可超过 1000；请修改后重新提交。"
        }
        TextKey::Table(TableKey::EditingDescription) => {
            "Enter/双击进入编辑；文本用 Enter 保存，多行备注用 Ctrl+Enter，布尔值切换选项后保存。校验通过后自动写回，数字大于 1000 会拒绝，可修改后重试或按 Esc 丢弃草稿。Esc/外部点击丢弃未保存草稿。只读单元格保持正常文本和选择能力。"
        }
        TextKey::Table(TableKey::RangeSelectionDescription) => {
            "拖动单元格选择矩形；Shift 点击扩展同类型的单元格、行或列，切换类型则重设锚点。Ctrl+A 或左上角全选当前 View 的投影行和列（含离屏 Cell），不含过滤行。"
        }
        TextKey::Table(TableKey::SelectionRangeStatus) => "\n选区 {kind}：行 {rows}，列 {columns}",
        TextKey::Table(TableKey::SelectionEmpty) => "空",
        TextKey::Table(TableKey::SelectionRows) => "整行",
        TextKey::Table(TableKey::SelectionColumns) => "整列",
        TextKey::Table(TableKey::SelectionCells) => "单元格",
        TextKey::Table(TableKey::SelectionAll) => "当前 View 全部",
        TextKey::Table(TableKey::SortFilterDescription) => {
            "表头箭头请求 无排序 → 升序 → 降序 → 无排序；筛选按钮切换条件：名称=偶数序号、数值=3 的倍数、Bool=true、进度至少 50%。多列条件取交集，共享 Model 的 View 各自独立。"
        }
        TextKey::Table(TableKey::SortFilterStatus) => "\n排序：{sort} | 活跃筛选：{filters}",
        TextKey::Table(TableKey::ToggleProjection) => "切换倒序/偶数投影",
        TextKey::Table(TableKey::ProjectionStatus) => "\nView 行数：{count} | 显示样本：{order}",
        TextKey::Table(TableKey::TableSharedDescription) => {
            "共用 Model 和 Renderer；行投影、布局、选择与滚动独立。B 初始为偶数行倒序，应用在数据变化后重新提交投影。两侧均可编辑、插入、删除和移动共享行。"
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
        TextKey::Tree(TreeKey::ToggleDrag) => "切换拖拽",
        TextKey::Tree(TreeKey::TreeSharedDescription) => {
            "展开、选择、滚动与拖拽开关独立。A 初始开启拖拽，B 关闭。Ctrl 切换、Shift 范围选择、Ctrl+A 并入可见节点。拖至行上/中/下三分之一区域表示之前/内部/之后；整框高亮表示内部。灰框和禁止光标表示循环或禁用落点。两侧同步原树数据。"
        }
        TextKey::Tree(TreeKey::TreeSharedFolder) => "共享目录",
        TextKey::Tree(TreeKey::TreeSharedFile) => "文件 {index}（编辑 {edits} 次）",
        TextKey::Tree(TreeKey::TreeInsertFile) => "插入文件",
        TextKey::Tree(TreeKey::TreeReparentSelected) => "移动文件：目录 / 根",
        TextKey::Tree(TreeKey::ClickArrowsToExpandClickContentTo) => {
            "箭头展开；Ctrl/Shift 多选，Ctrl+A 并入可见节点。拖至行上/中/下三分之一表示之前/内部/之后。父节点携子树移动，多选仍只移动一个节点，循环落点被拒绝。"
        }
        TextKey::Tree(TreeKey::EcsHeterogeneousTree) => "ECS 异构树",
        TextKey::Tree(TreeKey::FolderAndFileComponentsDispatchDifferentRenderers) => {
            "Folder 与 File component 使用不同 renderer。业务数据保留在 ECS node 上。"
        }
        TextKey::Tree(TreeKey::LazyLoadingTree) => "按需加载树",
        TextKey::Tree(TreeKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse) => {
            "将 Local file 拖至未加载/加载中的 Remote 上下方可排序，中间禁止（灰框与禁止光标）。展开 Remote 加载一次，完成后可拖入内部。Locked file 禁止放置。"
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
        TextKey::Waveform(WaveformKey::ModeLive) => "实时",
        TextKey::Waveform(WaveformKey::ModeReplay) => "回放",
        TextKey::Waveform(WaveformKey::ModeDisk) => "本地大文件回放",
        TextKey::Waveform(WaveformKey::DiskMissing) => {
            "本地记录不存在：{path}。请按 docs/gallery.md 的命令生成后重新加载；其他演示仍可使用。"
        }
        TextKey::Waveform(WaveformKey::DiskLoading) => {
            "正在打开本地记录：{path}。加载完成前保留当前记录。"
        }
        TextKey::Waveform(WaveformKey::DiskReady) => {
            "本地回放 · {rate} Hz · {channels} 通道 · {seconds} 秒 · {path}"
        }
        TextKey::Waveform(WaveformKey::DiskFailed) => {
            "无法打开 {path}：{error}。当前记录保留，可重新加载重试。"
        }
        TextKey::Waveform(WaveformKey::DiskReload) => "重新加载本地记录",
        TextKey::Waveform(WaveformKey::MeasurementHint) => {
            "Δt / ΔV：开启测量后框选，可拖动内部或边角编辑；波形继续运行。"
        }
        TextKey::Waveform(WaveformKey::ShowTimeGrid) => "显示时间网格",
        TextKey::Waveform(WaveformKey::Algorithm) => "降采样算法",
        TextKey::Waveform(WaveformKey::AlgorithmHint) => {
            "Min/Max 保留尖峰，LTTB 可能漏掉极值；Stress Live 注入短促尖峰供比较。"
        }
        TextKey::Waveform(WaveformKey::BasicLive100Hz4Channels5) => {
            "基础实时 · 100 Hz · 4 通道 · 第 2/3 通道含缺口"
        }
        TextKey::Waveform(WaveformKey::BasicReplay8Khz4Channels5) => "基础回放 · 8 kHz · 4 通道",
        TextKey::Waveform(WaveformKey::StressLive64Khz64Channels10) => {
            "压力实时 · 64 kHz · 64 通道"
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
        TextKey::Cursor(CursorKey::StaticCursorSizes) => {
            "45 种语义：34 种系统光标与 11 种 Hint。43 种静态 Cursor 为 48px；Pointer、Wait 为 64px。BRP Custom Pointer 不改变 OS Cursor。"
        }
        TextKey::Cursor(CursorKey::ReadOnlySemantic) => {
            "ReadOnly：仍可选择和复制，仅不可编辑。Kamuri 使用蓝白挂锁，System 使用 Cell；可与下方祖先 Disabled 样例对照。"
        }
        TextKey::Cursor(CursorKey::DisabledOverridesHints) => {
            "Disabled 优先于 Pointer 和 ReadOnly：下方两个样例悬停时均显示 NotAllowed 禁用光标，包括继承父级禁用的 ReadOnly。"
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
        TextKey::ListView(ListViewKey::DragDisabled) => {
            "未开启拖拽：此小列表与已开启的小列表共享数据；点击选择仍正常工作。"
        }
        TextKey::ListView(ListViewKey::ListOrders) => {
            "\nModel 原顺序：{original}\nProjection 显示：{displayed}"
        }
        TextKey::ListView(ListViewKey::ListStatus) => {
            "Model 项目：{count} | 显示行：{rows} | 显示范围：{start}..{end}\n选中：{selected} | active：{active} | SelectionChanged：{changes}"
        }
        TextKey::Tree(TreeKey::TreeFolder) => "[文件夹] {label}",
        TextKey::Tree(TreeKey::TreeFile) => "[文件] {label}  （{bytes} 字节）",
        TextKey::Tree(TreeKey::TreeStatus) => {
            "可见：{visible} | 已展开：{expanded} | 行：{rows} {start}..{end} | 选中（{selected_count}）：{selected} | {state} | 拖拽：{drag} | 根顺序：{order}"
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
