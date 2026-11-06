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
├── diagnostics/
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

Button 直接依赖 Diagnostics 的统一诊断和 Cursor 的公开 `WidgetryCursorLoading` 声明，Cursor 不依赖 Button；可组合 Headless、可选 Loading 状态与默认 Style（含 Spinner 和 Wait 声明）实现归属 Button，Disabled 理由组合继续由 Core 管理。 LinkButton 同根组合 Button Headless、可选 OpenUrl 能力与链接专属 Style；URL 校验、受控状态 API 和原生浏览器请求归属 OpenUrl，默认文字与四态配色归属链接 Style。

ToggleButton 直接使用 Diagnostics 的领域诊断、持续异常状态和批量错误桥接，选中状态继续采用官方 Checked。

HoldButton 直接使用 Diagnostics 保存构造、颜色及已拥有材质/渲染结构失败。公开 Headless 与独立 HeadlessPlugin 拥有长按输入、计时、只读进度/尝试状态及行为重绘请求；默认 Style 同根组合 Headless，独立管理圆形材质、内容阴影与视觉动画，不作为外部长按行为的依赖。

ToggleButton 的 Headless/独立 HeadlessPlugin 组合官方 Checkbox 输入与唯一 Checked 状态，拥有查询、World/Commands 设置和提交后 Changed 通知；默认八态颜色及前景传播仅识别同根组合 Headless 的默认控件。Waveform 的测量工具通过 Headless 状态 API 操作其默认按钮。

CheckBox 二态 Headless/独立 HeadlessPlugin 复用官方 Checkbox、Checked 和自更新 Observer；默认 Plugin 组合这套行为与原有二态/三态 Indicator、颜色及 Icon 管理。只安装二态行为不要求默认 Icon/Indicator，默认样式结构诊断继续按默认控件身份执行。

三态 CheckBox 的 Headless/独立 Plugin 持有唯一 WidgetryCheckState、原有输入与 Commands Set/Cycle、提交后 ValueChange 和无障碍同步，独立于默认颜色阶段装配。二态与三态分别维护原有权威，默认 Style 继续共享 Indicator；Facade 导出两套 Headless 能力。

CheckBox 直接使用 Diagnostics 保留二态/三态的领域错误与 mark/colors 持续异常，批量失败完整传播到宿主。

RadioGroup 直接使用 Diagnostics 保存 Group/Option 归属、选择和样式诊断；选择历史与 Checked 修复继续复用 Core，结构和配色仍由各自的变更集合驱动。

RadioGroup 的独立 Headless Plugin 管理原生 Group / Option 输入、严格直接成员和 Checked 修复；默认 Plugin 组合行为并安装默认样式。默认选项与自定义选项可混合，默认选项在纯 Headless 父级下以 Theme 作为配色来源。

各 Crate 的实际依赖关系以 Cargo Manifest 为准，职责边界与依赖方向遵循 `rules/architecture.md`。

公共错误载体位于独立的 [diagnostics](crates/diagnostics.md) crate，直接依赖轻量 internal_log；internal_log 的生产依赖只有 Tracing。其默认组合不依赖 Core、Bevy ECS 或渲染栈，可用于 Job、原生平台能力及构建期工具。默认关闭的 `bevy` Feature 只增加 Bevy ECS 的边界适配；Facade 显式启用并经 `diagnostics` 模块重导出。载体提供领域失败、共享 origin、原始线程回溯及传播上下文，不安装宿主处理器。

Theme 直接依赖 diagnostics 的 Bevy 桥接，为同步切换提供领域失败，并在 Commands 执行边界转换；颜色常量与查表保持独立的纯数据。

Core 的 Scene 适配依赖 diagnostics 的 Bevy 桥接，在原生 Scene 多层包装中保留诊断及完整原对象；同步 API 返回 SceneFailure，Deferred 才显式交给宿主 Handler。Facade 只重导出这些入口。

Window 直接依赖 diagnostics 的 Bevy 桥接，为原生图标、透明 DX12 初始化、标题及颜色 API 返回领域失败；运行期图标故障按主窗口集合保存持续诊断，标题 / 样式故障按 Root 和结构身份保存，全部窗口失败在 ECS 边界聚合传播。启动前失败由宿主入口显式报告，库不安装宿主处理器。

MessageBox 直接组合 Window、Button 与 ScrollArea，正文在可滚动区域显示，结果按钮独立保留。它直接依赖 diagnostics 的 Bevy 桥接，决议与颜色 API 返回领域失败；样式归属、派生状态和受管部件按自身生命周期保存持续诊断，组合下层控件时保留原始 cause 和 origin，多实例失败在边界聚合。

Gallery 的独立 Canvas 绘画会话由应用内部管理；Gallery 直接使用 Core 的 `WidgetryUiSystems::Build` 排序点，在 Canvas 相机准备前提交窗口适配状态。

Core 的 `offscreen` 提供统一的离屏 RenderLayers 分配域、Image 目标、Viewport 关联和生命周期回收，Waveform 使用该公共层并保留自身相机配置、Mesh、Material 和波形投影逻辑。Canvas 和 Waveform 直接依赖 diagnostics 的 Bevy 桥接，在各自 Renderer 边界聚合 Core 的共享诊断并补充业务 Owner，不重新生成错误身份。

Facade 为 25 个控件提供默认全部启用的独立可选依赖与 Feature，按需使用时可关闭默认集合。Core、Theme、Cursor 与 Core 使用的内建 Asset 仍为基础依赖；控件 Feature 不取消各 Widget 自身的组合依赖，也不自动公开对应的下层 Facade 模块。

宿主日志能力由 Facade 独立且默认关闭的 `app_logging` Feature 重导出，实现仍在 app_logging crate。Gallery 显式启用该 Feature 并通过 Facade 导入，内建 Asset 和构建期 app_icon_build 保持各自的直接依赖。

Job 控制能力经 Facade 默认关闭的 `job` Feature 导出，业务 crate 可直接依赖基础设施 job，不反向依赖 Facade。Job 直接依赖 diagnostics 的 Bevy 桥接，提交、访问和收发失败保留领域分类与统一诊断，由直接消费者在 ECS 边界显式转换。Gallery 显式启用该 Feature 并安装唯一 JobManagerPlugin，后台结果可唤醒 Reactive 主循环。

Windows 托盘能力由独立的 [tray](crates/tray.md) crate 提供，经 Facade 默认关闭的 `tray` Feature 导出。Tray 直接依赖 Bevy、diagnostics 的 Bevy 桥接、内部日志、ICO 解码所需的 Image、tray-icon、Crossbeam Channel 和 windows-sys，不依赖 Window 或 Gallery，也不管理窗口。可选原生右键菜单使用宿主 Resource 描述，点击经实例隔离路由转为消息；私有 Windows FFI 在主线程原生菜单输入循环中消费关闭手势，unsafe 仅允许在私有 native Module。原生回调的诊断由实例收件箱无损保留，ECS 边界批量传播；Gallery 显式启用 `tray`，在应用内部组合关闭确认、隐藏会话、恢复及退出策略。

Windows 空闲息屏与休眠抑制由独立的 [power_inhibit](crates/power_inhibit.md) crate 提供，经 Facade 默认关闭的 `power_inhibit` Feature 导出。它直接依赖 Bevy、diagnostics 的 Bevy 桥接、内部日志和 windows-sys，通过同步 Resource API 管理进程电源请求，不依赖 Window、Tray 或 Gallery。领域集合保留主操作、回滚和清理失败各自的原因与 origin，由调用方在执行边界转换。私有 FFI 使用标准库 OwnedHandle 管理句柄，新 crate 单独配置 unsafe lint，其他成员继续继承 Workspace 的 forbid。

Gallery 显式启用 `power_inhibit` Feature 并安装对应 Plugin；独立页面以应用级状态保存本次手动开关结果，不因页面或窗口可见性变化自动撤销请求，也不持久化到配置。

## Workspace Roles

Canvas 默认保留 Document、View/Renderer 与 SVG；默认关闭的 `png_export` 控制异步 PNG API、导出系统及直接的 `image`、`png`、`event-listener` 依赖。Facade 的 `canvas_png_export` 同时启用 Canvas 并转发该开关，Gallery 显式选择。共享 JobFailure/SVG 仍依赖 Job，正常绘制仍安装 StraightAlpha Shader。Bevy 等依赖可能继续使用图像库，关闭 Canvas PNG 不保证整个应用排除这些传递依赖。

Theme 默认仅包含 Dark 配色，Facade 的 `light`、`pink_dream` 独立转发到 Theme 同名 Feature；它们不控制 Cursor。Gallery 显式选择这两个可选预设，并在应用主题模块中完整定义 Kamuri Violet，库不再提供该配色专用 Feature 或数据入口。`test_utils` 通过默认关闭的主题转发和 `all_themes` 集合供各 crate 的测试依赖选择，不向库的生产依赖路径启用额外主题。

Cursor 的 Asset 直接依赖由默认关闭的 `kamuri_cursor` Feature 激活，并传递到 Asset 的同名嵌入资源 Feature。Facade 提供同名入口，Gallery 显式启用；Core 使用 Cursor 不会自动启用 Kamuri。System-only Cursor 不安装图集支持，Core 原有的字体、图标等 Asset 依赖保留。

本节概述各 Workspace 成员的职责。各库 crate 的具体功能、设计思路、行为契约和已知限制见对应的独立功能文档；Gallery 的展示与交互以应用实现为准。

| 成员 | 职责 |
| --- | --- |
| [gallery](../gallery/) | Widgetry 的功能展示与 GUI 运行时验证应用。 |
| [bevy_widgetry](crates/bevy_widgetry.md) | 聚合并导出 Widgetry 公共 API 的顶层 Facade。 |
| [core](crates/core.md) | 提供跨 Widget 共享的 UI 基础设施、交互状态投影、原生 Mouse 有序失焦取消适配、组合控件输入目标元数据与组合控件支持。 |
| [canvas](crates/canvas.md) | 提供独立 CanvasDocument、统一 CanvasShape/CanvasGeometry、矩形、参数椭圆与多子路径命令模型及普通 UI CanvasView，使用 Core 公共离屏层显示图形，依赖 Asset 的颜色表示转换 Shader；椭圆与 Path 通过 lyon 细分共享 Mesh。View 引用 Document，专属代理不进入业务模型。三类图形支持保留参数及原始命令的独立SVG导出；三类图形使用与View相同的Mesh管线分块异步导出PNG；通过png流式编码、image转换读回块，整个PNG导出通过共享JobManager纳管，ECS/GPU仍负责渲染与截图。 |
| [internal_log](crates/internal_log.md) | 提供 Widgetry 内部统一的日志 Macro 与 Target。 |
| [diagnostics](crates/diagnostics.md) | 不依赖 Bevy 的统一领域失败、唯一身份、源头回溯、拥有的传播上下文及显式处理记录。 |
| [job](crates/job.md) | 实例级 Job 控制核心，基于 Bevy I/O、CPU 任务池及受管命名线程提供统一身份、状态、协作取消、真实退出收割、全局关闭与显式记录移除；Plugin 安装唯一 App Resource 并合并 Winit Reactive 唤醒。通过 async-broadcast 提供有界 Arc 结果广播与独立订阅；依赖 Bevy、diagnostics 的 Bevy 桥接、内部日志、Tracing、Futures Util/Timer、Event Listener、async-broadcast 与安全 Windows 句柄等待 win-custody。 |
| [app_logging](crates/app_logging.md) | 提供宿主可选的日志配置与输出基础设施。 |
| [app_icon_build](crates/app_icon_build.md) | 提供应用构建期间嵌入 Windows EXE Icon 的辅助能力。 |
| [asset](crates/asset.md) | 统一管理 Widgetry 内建 Asset、Embedded 注册及语义标识，包括 Kamuri 45 种动画的共享时序、热点与图集目录。 |
| [theme](crates/theme.md) | 提供完整 Theme 数据、预设配色及原子主题切换；Gallery 自行管理预设名称与配置。 |
| [cursor](crates/cursor.md) | 管理 Native Window 的 Cursor 状态及可选 Kamuri Cursor。 |
| [tray](crates/tray.md) | 可选 Windows 常驻托盘图标、静态嵌入 ICO 与 Tooltip、原生右键菜单、主线程生命周期及唤醒反应式循环的点击通知。 |
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
| [tooltip](crates/tooltip.md) | 依赖 Core、Theme、Diagnostics 和内部日志，提供 Tooltip 的内容展示、交互触发、Popup 失败批量传播与持续异常诊断。 |
| [scroll_area](crates/scroll_area.md) | 提供可滚动内容区域及 Scrollbar 交互。 |
| [list_view](crates/list_view.md) | 提供稳定身份 List Model、泛型读取、逐 View 投影与独立手动排序、虚拟化列表展示及条目交互；为 Tree 提供内部结构拖放桥接。 |
| [tree](crates/tree.md) | 提供层级 Tree Model、逐 View 展开与选择，以及经校验的真实 ECS 节点拖放移动。 |
| [table](crates/table.md) | 提供 Table Model、表格布局、虚拟化单元格展示与交互。 |
| [window](crates/window.md) | 提供自定义 Window、Native Window 生命周期及多窗口支持。 |
| [message_box](crates/message_box.md) | 提供 MessageBox 的展示、操作结果与关闭行为。 |
| [file_dialog](crates/file_dialog.md) | 提供 FileDialog 的文件浏览、选择、验证及后台文件系统处理；直接依赖 job 纳管全部文件系统/偏好操作、CPU投影/选择计算和大快照生命周期，自动确保共享 JobManagerPlugin；不再持有专有三线程 Worker。 |
| [waveform](crates/waveform.md) | 提供 Waveform 数据处理与可视化展示；依赖 job 纳管完整后台降采样重建，WaveformPlugin 自动复用共享 JobManagerPlugin。 |
| [waveform_file](crates/waveform_file.md) | 外部 Replay 数据源适配器，流式构建记录及独立 MinMax/LTTB 磁盘索引；依赖 Waveform、job 与 diagnostics 的 Bevy 桥接，以受管长期线程服务后台局部随机读取并保留队列和工作结果诊断，Widget 不反向依赖文件格式。 |
| [test_utils](crates/test_utils.md) | 提供跨 Crate 共享的测试与 Benchmark 基础设施，使用 diagnostics 的 Bevy 适配保留受管错误与真实 ECS context，仅进入测试依赖路径。 |

Table 直接依赖 Asset 的表头排序/筛选 SVG 语义标识，使用 Core 的 WidgetryIcon 组合独立请求按钮，并自动安装相应 Asset 与 Icon Plugin。

Waveform 的顶部时间、幅值和每屏通道数选择组合 ComboBox 与 ListView Model；标签列与波形共用 ScrollArea 的原生滚动状态，渲染 Plugin 自动安装这些 UI 能力，独立数据更新 Plugin 保持无 UI 用法。

Toggle Button 直接依赖 Core、Theme 和内部日志，不依赖 CheckBox 的固定视觉或 Button 的瞬时 Activate 行为。

RadioGroup 与 Segmented 共用 Core 的 `exclusive_selection` 结构回退历史和 Checked 提交，两个 Widget 无相互生产依赖。Segmented 自身处理有焦点、不循环的方向键及点击选择，以免原生 RadioGroup 键盘与祖先事件路由破坏其独立交互契约。

Waveform 测量层依赖通用 Toggle Button 的唯一选中状态，在可视绘图区之上组合独立 UI 矩形及读数；不参与 Source 或 Reducer。

Waveform 的 Replay 控制栏组合 Button、Slider、ComboBox 与 Asset 的 PlaybackPlay/PlaybackPause SVG，自动安装对应 Plugin。库传递播放、倍率及定位手势意图并投影外部提交状态，播放时钟及拖动前状态的保存/恢复由宿主控制器负责。Gallery 分开组织 Live、小型 Replay 与大型本地 Replay，后者在后台打开/重载 waveform_file 的真实磁盘源；演示录制生成 Example 归属 Gallery，文件读写与索引构建归属 waveform_file，文件和缓存保留在 Git 忽略目录。

Segmented 通过 Theme 的独立 `segmented` 配色解析容器和选项状态，并使用 Core 的前景传播能力。其错误直接依赖 diagnostics 的 Bevy 桥接，领域失败与按 Owner 保存的持续诊断在 ECS 边界聚合，选择提交仍由 Core 负责。

[DatePicker](crates/date_picker.md) 归属独立 `date_picker` crate，依赖 Core、Theme、Asset、Diagnostics、内部日志和 Workspace `time`，由 Facade 的独立 `date_picker` Feature 聚合。私有选中日期和显示年月是唯一业务状态，42 格月历按需派生。BSN 日期视图复用 Core Pointer 与有效 Disabled 继承，常规颜色由 Theme 的独立 `date_picker` 配色驱动。共享本地 Today 缓存通过一分钟检查与 Winit Reactive 等待上限维护，固定笔圈 SVG 由 Asset 管理、Core Icon 渲染。

DatePicker 的 App 级呈现语言 Resource 独立于业务状态，支持原位切换英文与简体中文。Gallery 的独立 DatePicker 页面经 Facade 消费控件，并直接使用 Workspace `time` 决定演示初始本地日期；页面状态查询和操作均走公开 API，真实事件历史由页面实体持有。

ScrollArea 直接依赖 diagnostics 的 Bevy 桥接，领域颜色失败、持久的几何 / 样式诊断和批量边界传播共享原始错误身份；组合控件消费滚动层诊断时只补充 Owner / Request 上下文。

FileDialog 的组合配色更新直接使用 diagnostics 的 Bevy 桥接汇总跨部件和跨 Dialog 的受管失败；尚未迁移的原生 Bevy 失败分别在队列边界交给宿主，保留原有 Severity 与 payload。

FileDialog 的状态、选择和存储契约使用同一 diagnostics 领域载体；组合公开 API 单独承接领域诊断与 Job 失败，在 Template / ECS 边界保留原始身份并显式转换。Facade 重导出这些错误类别，不参与生命周期处理。

FileDialog 的样式层直接使用 windows-sys 获取 Windows Shell 文件图标，FFI 限制在私有 native 模块；图标请求由 JobManager 受管线程批量执行，缓存和 ImageNode 投影归属 FileDialog，不进入业务 Backend、快照或选择结果。图像注册使用 Bevy Assets，销毁后的在途任务复用 Runtime 的终态收尾能力；Core 的 WidgetryIcon 继续只承担 SVG。FileDialog 与 PowerInhibit 一样在 crate 内采用 unsafe deny，仅允许各自私有 FFI 模块使用 unsafe，其余 lint 与 Workspace 一致。

Splitter 直接依赖 diagnostics 的 Bevy 桥接。比例 / 配置 API 返回领域失败，布局和样式按业务键惰性记录持续诊断并在执行边界聚合多个 Root；底层数值与原生查询原因由 Splitter 领域类型保留。

Tabs 直接依赖 Core、Theme、ScrollArea 和内部日志，使用 Theme 独立 `tabs` 配色及 Core 的 Pointer、有效 Disabled 和 Foreground。标签栏以 ScrollArea 的 Auto 策略承载 Header，选中定位复用最近 Viewport 的 ScrollIntoView；页面位于独立内容容器，不默认套滚动层。Tabs 样式仅管理自身外壳与标签头，不修改页面业务样式。

Tabs 直接依赖 diagnostics 的 Bevy 桥接，公开查询、选择与颜色操作返回领域失败。初始化、选中定位与样式执行边界聚合多个 Owner 的诊断；样式以结构业务键保存持续失败，Core 与 Scene 传播保留源头 origin。

Collapsible 直接组合 Button 与 Core Icon，依赖 Asset、Theme、Core 与内部日志；以私有 Expanded 管理常驻 Header/Content 的显示，通过公开 API 提交并通知。卡片颜色由 Collapsible 自身投影，按钮交互配色及箭头前景色复用 Button / Icon。

Collapsible 直接依赖 diagnostics 的 Bevy 桥接。查询、展开、Header 禁用与颜色 API 返回领域失败，保留组合部件身份、Icon / Core 原始原因及 origin；初始化聚合多个 Root，样式按结构业务键惰性保存持续诊断。

ProgressBar 直接依赖 diagnostics 的 Bevy 桥接。状态、尺寸与颜色失败保留请求值、Root 和 Core 原因，样式结构异常按业务键保存诊断并在 System 边界聚合，正常动画算法保持不变。

Slider 直接依赖 diagnostics 的 Bevy 桥接，公开数值 / 颜色 API 返回领域失败并保留完整 f32 配置和判定类别；输入与样式边界保留原诊断，样式按结构键去重并聚合所有 Root。Waveform 进度投影在消费 Slider 失败时补充自身上下文。

TextField 直接依赖 diagnostics 的 Bevy 桥接，普通与只读控件共享内容、映射、AutoSize 和颜色领域诊断。NumberInput、ComboBox、FileDialog 消费时保留原诊断并补充调用上下文。Table 也直接依赖该桥接，其 Editor read 回调区分正常业务拒绝与下层诊断失败，避免 TextField 错误被降成字符串反馈。

NumberInput 直接依赖 diagnostics 的 Bevy 桥接，配置和状态 API 返回领域失败；数值快照保留原类型及精度，TextField 组合失败通过 map_kind 继承原 source / origin，输入和样式只在执行边界转换。

Cursor 直接依赖 diagnostics 的 Bevy 桥接，共用按业务键保存原诊断的 `FailureState`；该通用状态实现位于 diagnostics，Core 保留重导出入口。Cursor 的资源缓存拥有图集失败身份，各 Window 的实际静态/System 回退消费不新建错误身份。

ListView 直接依赖 Diagnostics 保存 Model、数据源配置及选择 API 的领域失败和原 Key 快照；Facade 聚合相应诊断类型。

Tree 直接依赖 diagnostics 的 Bevy 桥接，节点移动与拖放开关返回领域失败；纯移动拒绝与公开请求诊断分离，下层 ListView 配置错误继承原始原因与 origin。

ComboBox 直接依赖 diagnostics 的 Bevy 桥接。注册、查询及颜色 API 返回领域失败；组合 ListView、TextField、Button 和 Icon 的失败保留原始原因与 origin。Field、搜索、图标和自有配色结构异常由实例保存持续诊断，多个实例的未处理失败全部传至宿主；实际 Scene 重试保留独立诊断及原生策略。

构建期 app_icon_build 直接依赖默认无 Bevy 的 diagnostics 和 winresource；Gallery build.rs 独立使用 Tracing 接收与终结构建失败，不接入运行期日志或 ECS。

app_logging 直接依赖 diagnostics 的 Bevy 桥接，用共享诊断保存初始化与输出故障；其 lossless 文件writer和LoggingGuard归宿主生命周期管理，库不替宿主安装Subscriber或ECS Handler。Facade仅重导出配置与类型。

Gallery 的错误终结策略位于应用内部 diagnostics 模块，通过 Facade 的统一诊断桥读取原始 ID、原因与栈。Gallery 显式开启 bevy_ecs/backtrace 以保留原生 Bevy 可用栈；受管诊断仍使用自己的 force_capture，与该 Feature 和环境变量独立。发行安装脚本归 Gallery，匹配 PDB 与 EXE 一起部署。
