# Bevy Widgetry Architecture

## Architecture Overview

bevy_widgetry 使用 Cargo Workspace 组织，由 Gallery 应用、顶层 Facade、各 Widget Crate、共享基础设施及测试基础设施组成。

当前 Workspace 结构：

```text
gallery/

crates/
├── bevy_widgetry/
├── core/
├── canvas/
├── internal_log/
├── job/
├── app_logging/
├── app_icon_build/
├── asset/
├── theme/
├── cursor/
├── button/
├── hold_button/
├── toggle_button/
├── check_box/
├── radio_group/
├── segmented/
├── date_picker/
├── tabs/
├── collapsible/
├── slider/
├── number_input/
├── progress_bar/
├── splitter/
├── text_field/
├── combo_box/
├── tooltip/
├── scroll_area/
├── list_view/
├── tree/
├── table/
├── window/
├── tray/
├── power_inhibit/
├── message_box/
├── file_dialog/
├── waveform/
├── waveform_file/
└── test_utils/
```

Button 直接依赖 Cursor 的内部忙碌光标标记，Cursor 不依赖 Button；Loading 状态和 Spinner 实现归属 Button，Disabled 理由组合继续由 Core 管理。

各 Crate 的实际依赖关系以 Cargo Manifest 为准，职责边界与依赖方向遵循 `rules/architecture.md`。

Gallery 的独立 Canvas 绘画会话由应用内部管理；Gallery 直接使用 Core 的 `WidgetryUiSystems::Build` 排序点，在 Canvas 相机准备前提交窗口适配状态。

Core 的 `offscreen` 提供统一的离屏 RenderLayers 分配域、Image 目标、Viewport 关联和生命周期回收，Waveform 使用该公共层并保留自身相机配置、Mesh、Material 和波形投影逻辑。

Facade 为 25 个控件提供默认全部启用的独立可选依赖与 Feature，按需使用时可关闭默认集合。Core、Theme、Cursor 与 Core 使用的内建 Asset 仍为基础依赖；控件 Feature 不取消各 Widget 自身的组合依赖，也不自动公开对应的下层 Facade 模块。

宿主日志能力由 Facade 独立且默认关闭的 `app_logging` Feature 重导出，实现仍在 app_logging crate。Gallery 显式启用该 Feature 并通过 Facade 导入，内建 Asset 和构建期 app_icon_build 保持各自的直接依赖。

Job 控制能力经 Facade 默认关闭的 `job` Feature 导出，业务 crate 可直接依赖基础设施 job，不反向依赖 Facade。Gallery 显式启用该 Feature 并安装唯一 JobManagerPlugin，后台结果可唤醒 Reactive 主循环。

Windows 托盘能力由独立的 [tray](crates/tray.md) crate 提供，经 Facade 默认关闭的 `tray` Feature 导出。Tray 直接依赖 Bevy、内部日志、ICO 解码所需的 Image、tray-icon 与 Crossbeam Channel，不依赖 Window 或 Gallery，也不管理窗口。Gallery 显式启用 `tray`，在应用内部组合关闭确认、隐藏会话、恢复及退出策略。

Windows 空闲息屏与休眠抑制由独立的 [power_inhibit](crates/power_inhibit.md) crate 提供，经 Facade 默认关闭的 `power_inhibit` Feature 导出。它仅依赖 Bevy、内部日志和 windows-sys，通过同步 Resource API 管理进程电源请求，不依赖 Window、Tray 或 Gallery。私有 FFI 使用标准库 OwnedHandle 管理句柄，新 crate 单独配置 unsafe lint，其他成员继续继承 Workspace 的 forbid。

Gallery 显式启用 `power_inhibit` Feature 并安装对应 Plugin；独立页面以应用级状态保存本次手动开关结果，不因页面或窗口可见性变化自动撤销请求，也不持久化到配置。

## Workspace Roles

本节概述各 Workspace 成员的职责。具体功能、设计思路、行为契约和已知限制见对应的独立功能文档。

| 成员 | 职责 |
| --- | --- |
| [gallery](gallery.md) | Widgetry 的功能展示与 GUI 运行时验证应用。 |
| [bevy_widgetry](crates/bevy_widgetry.md) | 聚合并导出 Widgetry 公共 API 的顶层 Facade。 |
| [core](crates/core.md) | 提供跨 Widget 共享的 UI 基础设施、交互状态投影、原生 Mouse 有序失焦取消适配、组合控件输入目标元数据与组合控件支持。 |
| [canvas](crates/canvas.md) | 提供独立 CanvasDocument、统一 CanvasShape/CanvasGeometry、矩形、参数椭圆与多子路径命令模型及普通 UI CanvasView，使用 Core 公共离屏层显示图形，依赖 Asset 的颜色表示转换 Shader；椭圆与 Path 通过 lyon 细分共享 Mesh。View 引用 Document，专属代理不进入业务模型。三类图形支持保留参数及原始命令的独立SVG导出；三类图形使用与View相同的Mesh管线分块异步导出PNG；通过png流式编码、image转换读回块，整个PNG导出通过共享JobManager纳管，ECS/GPU仍负责渲染与截图。 |
| [internal_log](crates/internal_log.md) | 提供 Widgetry 内部统一的日志 Macro 与 Target。 |
| [job](crates/job.md) | 实例级 Job 控制核心，基于 Bevy I/O、CPU 任务池及受管命名线程提供统一身份、状态、协作取消、真实退出收割、全局关闭与显式记录移除；Plugin 安装唯一 App Resource 并合并 Winit Reactive 唤醒。通过 async-broadcast 提供有界 Arc 结果广播与独立订阅；依赖 Bevy、内部日志、Tracing、Futures Util/Timer、Event Listener、async-broadcast 与安全 Windows 句柄等待 win-custody。 |
| [app_logging](crates/app_logging.md) | 提供宿主可选的日志配置与输出基础设施。 |
| [app_icon_build](crates/app_icon_build.md) | 提供应用构建期间嵌入 Windows EXE Icon 的辅助能力。 |
| [asset](crates/asset.md) | 统一管理 Widgetry 内建 Asset、Embedded 注册及语义标识，包括 Kamuri 45 种动画的共享时序、热点与图集目录。 |
| [theme](crates/theme.md) | 提供预定义 Theme、Widget 配色及 Theme 状态管理。 |
| [cursor](crates/cursor.md) | 管理 Native Window 的 Cursor 状态及可选 Kamuri Cursor。 |
| [tray](crates/tray.md) | 可选 Windows 常驻托盘图标、静态嵌入 ICO 与 Tooltip、主线程生命周期及唤醒反应式循环的左键单击通知。 |
| [power_inhibit](crates/power_inhibit.md) | 默认关闭的 Windows Display/System 电源请求、同步开关及 Resource RAII 清理。 |
| [hold_button](crates/hold_button.md) | 提供独立长按确认、输入取消、圆形 Picking 与液面材质。 |
| [toggle_button](crates/toggle_button.md) | 独立二态按钮，复用官方 Checkbox / Checked 输入与状态，提供持续选中样式和提交后通知。 |
| [button](crates/button.md) | 提供 Button、LinkButton 控件及其交互、URL 打开与外观行为。 |
| [check_box](crates/check_box.md) | 提供 CheckBox 与 TriStateCheckBox 控件。 |
| [progress_bar](crates/progress_bar.md) | 提供只读展示进度的状态、BSN 初始化、同步 / 延迟更新及水平轨道与主题样式。 |
| [slider](crates/slider.md) | 提供 Slider 单值状态、输入、主题样式、尺寸配置与程序化更新。 |
| [number_input](crates/number_input.md) | 提供原类型泛型数值状态、TextField 延迟编辑、按钮/方向键步进与长按、只读限制、三段布局与统一主题反馈；直接依赖 core、text_field、theme 与内部日志。 |
| [splitter](crates/splitter.md) | 提供 BSN 双 Pane 组合、唯一比例状态、受原生约束限制的主轴布局、Handle 拖动与主题细线样式；直接依赖 core、theme。 |
| [radio_group](crates/radio_group.md) | 提供 RadioGroup 的选项组织与单选交互。 |
| [segmented](crates/segmented.md) | 提供独立严格单选分段控件的横向 / 竖向输入、动态选项维护和浮块式主题外观。 |
| [collapsible](crates/collapsible.md) | BSN 常驻 Header/Content、唯一 Expanded 与同步/延迟状态 API。 |
| [tabs](crates/tabs.md) | 固定 Header / Content 配对、唯一选择、常驻页面切换、方向导航、经典主题外观及标签栏 Auto 滚动。 |
| [text_field](crates/text_field.md) | 提供 TextField 与 ReadOnlyTextField 控件。 |
| [combo_box](crates/combo_box.md) | 提供 ComboBox 的选项展示、选择及可选本地搜索；搜索输入组合 text_field，结果组合 list_view。 |
| [tooltip](crates/tooltip.md) | 提供 Tooltip 的内容展示与交互触发行为。 |
| [scroll_area](crates/scroll_area.md) | 提供可滚动内容区域及 Scrollbar 交互。 |
| [list_view](crates/list_view.md) | 提供稳定身份 List Model、泛型读取、逐 View 投影与独立手动排序、虚拟化列表展示及条目交互；为 Tree 提供内部结构拖放桥接。 |
| [tree](crates/tree.md) | 提供层级 Tree Model、逐 View 展开与选择，以及经校验的真实 ECS 节点拖放移动。 |
| [table](crates/table.md) | 提供 Table Model、表格布局、虚拟化单元格展示与交互。 |
| [window](crates/window.md) | 提供自定义 Window、Native Window 生命周期及多窗口支持。 |
| [message_box](crates/message_box.md) | 提供 MessageBox 的展示、操作结果与关闭行为。 |
| [file_dialog](crates/file_dialog.md) | 提供 FileDialog 的文件浏览、选择、验证及后台文件系统处理；直接依赖 job 纳管全部文件系统/偏好操作、CPU投影/选择计算和大快照生命周期，自动确保共享 JobManagerPlugin；不再持有专有三线程 Worker。 |
| [waveform](crates/waveform.md) | 提供 Waveform 数据处理与可视化展示；依赖 job 纳管完整后台降采样重建，WaveformPlugin 自动复用共享 JobManagerPlugin。 |
| [waveform_file](crates/waveform_file.md) | 外部 Replay 数据源适配器，流式构建记录及独立 MinMax/LTTB 磁盘索引；依赖 Waveform 与 job，以受管长期线程服务后台局部随机读取，Widget 不反向依赖文件格式。 |
| [test_utils](crates/test_utils.md) | 提供跨 Crate 共享的测试与 Benchmark 基础设施。 |

Table 直接依赖 Asset 的表头排序/筛选 SVG 语义标识，使用 Core 的 WidgetryIcon 组合独立请求按钮，并自动安装相应 Asset 与 Icon Plugin。

Waveform 的顶部时间、幅值和每屏通道数选择组合 ComboBox 与 ListView Model；标签列与波形共用 ScrollArea 的原生滚动状态，渲染 Plugin 自动安装这些 UI 能力，独立数据更新 Plugin 保持无 UI 用法。

Toggle Button 直接依赖 Core、Theme 和内部日志，不依赖 CheckBox 的固定视觉或 Button 的瞬时 Activate 行为。

RadioGroup 与 Segmented 共用 Core 的 `exclusive_selection` 结构回退历史和 Checked 提交，两个 Widget 无相互生产依赖。Segmented 自身处理有焦点、不循环的方向键及点击选择，以免原生 RadioGroup 键盘与祖先事件路由破坏其独立交互契约。

Waveform 测量层依赖通用 Toggle Button 的唯一选中状态，在可视绘图区之上组合独立 UI 矩形及读数；不参与 Source 或 Reducer。

Waveform 的 Replay 控制栏组合 Button、Slider、ComboBox 与 Asset 的 PlaybackPlay/PlaybackPause SVG，自动安装对应 Plugin。库传递播放、倍率及定位手势意图并投影外部提交状态，播放时钟及拖动前状态的保存/恢复由宿主控制器负责。Gallery 分开组织 Live、小型 Replay 与大型本地 Replay，后者在后台打开/重载 waveform_file 的真实磁盘源；演示录制生成 Example 归属 Gallery，文件读写与索引构建归属 waveform_file，文件和缓存保留在 Git 忽略目录。

Segmented 通过 Theme 的独立 `segmented` 配色解析容器和选项状态，并使用 Core 的前景传播能力。

[DatePicker](crates/date_picker.md) 归属独立 `date_picker` crate，依赖 Core、Theme、Asset、内部日志和 Workspace `time`，由 Facade 的独立 `date_picker` Feature 聚合。私有选中日期和显示年月是唯一业务状态，42 格月历按需派生。BSN 日期视图复用 Core Pointer 与有效 Disabled 继承，常规颜色由 Theme 的独立 `date_picker` 配色驱动。共享本地 Today 缓存通过一分钟检查与 Winit Reactive 等待上限维护，固定笔圈 SVG 由 Asset 管理、Core Icon 渲染。

DatePicker 的 App 级呈现语言 Resource 独立于业务状态，支持原位切换英文与简体中文。Gallery 的独立 DatePicker 页面经 Facade 消费控件，并直接使用 Workspace `time` 决定演示初始本地日期；页面状态查询和操作均走公开 API，真实事件历史由页面实体持有。

Tabs 直接依赖 Core、Theme、ScrollArea 和内部日志，使用 Theme 独立 `tabs` 配色及 Core 的 Pointer、有效 Disabled 和 Foreground。标签栏以 ScrollArea 的 Auto 策略承载 Header，选中定位复用最近 Viewport 的 ScrollIntoView；页面位于独立内容容器，不默认套滚动层。Tabs 样式仅管理自身外壳与标签头，不修改页面业务样式。

Collapsible 直接组合 Button 与 Core Icon，依赖 Asset、Theme、Core 与内部日志；以私有 Expanded 管理常驻 Header/Content 的显示，通过公开 API 提交并通知。卡片颜色由 Collapsible 自身投影，按钮交互配色及箭头前景色复用 Button / Icon。
