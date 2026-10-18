# Widget Gallery

## 功能概述

Widget Gallery 是 bevy_widgetry 的独立展示应用，用于浏览各 Widget 的功能、体验实际输入交互，并观察不同 Theme、状态和组合场景下的界面表现。

Gallery 不只是静态控件预览。多数页面提供可操作的演示场景，包括状态变化、数据修改、Disabled 行为、虚拟化内容以及多窗口交互。

应用还集成了 BRP Runtime，支持通过外部工具执行 GUI 输入、查询 ECS 状态和获取 Screenshot，用于开发过程中的运行时调试与验证。

## 当前设计思路

Gallery 使用 Widgetry 自身提供的 Widget 和 Bevy BSN 构造界面，以实际使用场景展示库能力，而不是建立独立的模拟控件实现。

宿主日志通过 Facade 的可选 `app_logging` Feature 装配：先准备日志并 `install`，再将 Terminal / File Layer 接入官方 LogPlugin，持有 WorkerGuard 直到应用退出。日志实现保留在独立的 app_logging crate，Gallery 负责日志目录和过滤策略。

整体采用**常驻应用框架与按需创建的页面内容**相结合的设计：

- 主窗口、Header、Sidebar 和页面承载区域在页面切换期间保持存在。
- 同一时刻只挂载一个 Gallery 页面。
- 进入页面时创建对应的演示 UI 和必要数据，离开页面时销毁当前页面内容。
- 再次进入页面时重新创建演示内容，不将各页面的临时状态长期保留在全局应用中。
- Theme、Language 和 Scale Factor 等全局设置独立于页面生命周期。

Gallery 的页面按 Widget 功能组织，复杂控件通过不同场景展示其主要行为和边界条件。

各页面还可以展开 Theme Colors 和颜色覆盖示例，用于对比默认配色与局部 Override 的实际效果。

## 主要功能

### 页面导航

Sidebar 提供 12 个页面入口，支持切换当前展示内容。

| 页面 | 主要演示内容 |
| --- | --- |
| Button | 显式 Loading 演示、普通与 Disabled Button、文字与 Icon 组合、LinkButton 及程序化 URL 切换，以及 RadioGroup 的不同排列方式。 |
| CheckBox | 二态、三态 CheckBox，以及不同 Checked、Indeterminate 和 Disabled 状态。 |
| ComboBox | 文字、Icon 与混合 Renderer，选项数据修改、程序化选择及 Disabled 行为。 |
| ScrollArea | 垂直、水平和双轴滚动，Scrollbar 显示策略、内容尺寸变化和 Scroll Into View。 |
| ListView | 小型列表、大规模虚拟化列表、Model 数据操作、Selection 及 Disabled 行为，以及文字 / 图标文字 Renderer 的共享 Model 双 View。 |
| Tree | 共享 TreeModel 双 View、基础树、异构 ECS 节点、延迟加载、大规模虚拟化及节点交互。 |
| Table | 共享 Model 双 View 的独立布局、列宽和交互，以及不同 Cell Value、Header、Selection、虚拟化与 Disabled 场景。 |
| TextField | 单行、固定多行、Read-only、Disabled、最多 5 个 Unicode 字素簇的输入、独立及带上限的字数展示、Prefix / Suffix 和 Clear 按钮组合，以及默认 2–6 行和只读 1–3 行的 AutoSize。支持常显警告色 HintText；AutoSize 示例组合全部五项能力，并提供窄/宽切换以观察软换行和独立提示布局。 |
| Tooltip | 基础与富内容 Tooltip、Disabled 控件提示和不同位置的展示行为。 |
| Window | 独立窗口、不同背景、MessageBox 及 FileDialog。 |
| Waveform | 多通道实时波形、数据回放以及高负载波形展示。 |
| Cursor | System / Kamuri 光标切换、标准与扩展 Cursor 语义、动画及 Disabled 场景。 |

ListView 页顶部的两个 View 共用一个 Model，分别使用纯文字和图标加文字 Renderer。右侧使用独立投影，仅显示偶数标签并倒序排列，不复制原始条目。任一侧的数据按钮修改共享数据；选择、Active、滚动和启用 / 禁用各自独立，禁用侧仍显示数据更新。离开页面时两个 View 与共享 Model 一起清理。

Tree 页顶部两个不同高度的 View 共用一个 TreeModel，展开与 Selection 同步，滚动和启用 / 禁用各自独立。选中文件后，可编辑、删除或在目录与根之间移动，也可插入新文件；禁用侧继续反映共享数据和程序化选择。两个 View 使用相同的 App 级 Renderer，语言切换原位更新文字并保留节点身份、选择、展开和滚动。离开页面时清理所有临时 View、Model 与业务节点。

Table 页的 Shared Model 示例让两个 View 共用 64 行、8 列及 App 级 Renderer。A 的初始列宽 / 行高为 180 / 34 px，B 为 120 / 28 px 并采用较紧凑的 Cell Padding 和不同 Border。两侧可拖动调整各自列宽、独立选择、键盘导航和双轴滚动；单侧禁用仍同步共享的编辑、插入、删除和移动。移动保留 Stable ID，删除无效 Selection / Focused Cell 时静默清理。语言和 Theme 切换保持现有 View、Model 与交互状态，离页回收全部临时数据。

Button 页提供 Loading 演示：点击开始按钮只激活一次并显示 Spinner，点击结束按钮恢复；F1 仅在 Button 页面且目标处于 Loading 时执行结束操作，便于保持鼠标原位观察 Wait → Pointer，包括 Kamuri 动画。离页后演示清理，重新进入恢复默认状态；说明和按钮纳入中英文切换。

Button 页的 LinkButton 区域展示正常、Disabled 链接，以及在 `https://www.baidu.com` 与 `https://www.google.com` 间切换的链接。切换按钮通过公开 API 同时更新 URL 文字和跳转目标，只有激活链接才请求系统浏览器。标题、说明及按钮支持中英文，URL 保留原文。Theme Colors 包含 `link_button` 的四态配色，Overrides 提供仅覆盖 Hover 颜色的对比、清除及父级禁用操作；离页清理示例，重新进入恢复初始 URL。

### 全局界面设置

Header 提供三个常驻选择器。

**Theme**

支持四种主题：

- Dark
- Light
- Pink Dream
- Kamuri Violet

切换 Theme 后，当前界面及使用 Widgetry Theme 的相关控件更新配色。Theme 选择器同步反映当前生效的主题。

**Language**

支持 English 和简体中文，默认 English。

切换语言时，已绑定国际化文本的界面内容原地更新，包括导航、页面说明、状态文本及相关窗口标题。

语言切换不重新创建当前页面，也不主动清除用户输入或控件状态。

**Scale Factor**

支持 `1.0`、`1.25`、`1.5` 和 `2.0` 四档缩放。

选择后立即调整主窗口的 Scale Factor Override，用于观察不同 UI 缩放条件下的布局与显示效果。

### Theme Colors 与 Override 展示

常规控件页面提供按需展开的颜色展示区域：

- **Theme Colors**：查看当前 Theme 下相关控件的配色值和状态分类。
- **Overrides**：通过实际 Widget 对比默认 Theme 与局部颜色覆盖的效果。

颜色展示区域默认折叠，展开后才创建对应内容。

切换 Theme 时，已展开的 Theme Colors 内容同步刷新。

### Window 与 Dialog

Window 页面包含三类演示。

**独立窗口**

可以创建具有 Theme 背景或 Image 背景的独立 Widgetry Window。

Image 背景提供 Stretch、Cover 及不同透明度的组合示例。新窗口拥有独立内容和可交互 Button。

**MessageBox**

可以打开不同结果按钮组合的 MessageBox，包括 OK、Yes / No、Yes / No / Cancel。

Gallery 通过结果事件观察用户选择，并记录相应操作结果。

**FileDialog**

提供以下操作：

- 打开单个文件。
- 打开多个文件。
- 选择单个文件夹。
- 选择多个文件夹。
- 使用 Image Filter 打开文件。
- 保存文件。

可以选择 Modal 或 Non-modal 模式。

操作结果在对应的 Gallery 区域显示，包括取消状态、返回路径及选择数量。

### Waveform

Waveform 页面展示不同规模和来源的波形数据：

- 100 Hz、4 Channel 的基础实时波形。
- 8 kHz、4 Channel 的回放波形。
- 64 kHz、64 Channel 的高负载实时波形。

这些示例用于观察不同数据密度下的实际显示和持续更新行为。

### Cursor

Cursor 页面可以独立选择 System 或 Kamuri Cursor，并通过不同语义的目标区域观察光标变化。

Kamuri 提供 44 种光标图像，包括静态样式与 Pointer、Wait 动画。

页面包含官方 System Cursor 语义、Widgetry 扩展 Hint 以及 Disabled 示例，用于比较不同 Cursor Theme 下的表现。

### 启动画面

Gallery 默认在启动时播放一次 Splash。

Splash 在主界面准备期间展示启动画面，并在主界面可呈现后播放揭幕过渡。过渡期间隔离主窗口输入，避免用户操作尚未正式进入的界面。

开发和 Benchmark 环境可以按需跳过 Splash 或启用专用预览模式。

`GALLERY_SPLASH_DISABLE` 或专用 Gallery Benchmark 环境可跳过 Splash。`GALLERY_SPLASH_PREVIEW` 支持 `play`、`static`、`ripple`、`baseline`、`benchmark` 以及 `0–800ms` 定帧预览。

### 运行时调试与 Benchmark

Gallery 可选启用 BRP Runtime，供外部调试工具连接。

支持通过 BRP 进行 GUI 输入、运行时状态检查及 Screenshot 获取。

仓库还提供与 Gallery 相关的专项 Benchmark，包括 Waveform、FileDialog、页面生命周期和 Splash 场景。

这些 Benchmark 用于特定性能路径的测量，不代表普通页面操作会自动执行性能验收。

## 行为与状态契约

### 启动与页面生命周期

Gallery 初始化完成后，默认进入 Button 页面。

Header、Sidebar 和主窗口保持常驻，切换页面时仅替换当前页面内容。

页面重新进入时：

- 演示控件重新创建。
- 属于页面的临时状态和演示数据重新初始化。
- 颜色展示区域恢复初始折叠状态。
- 全局 Theme、Language 和 Scale Factor 继续保持。

ComboBox、ListView、Tree、Table 等页面的演示 Model 不承诺在离开后保留。Waveform 页面使用的实时数据 Producer 也随页面生命周期管理。

### 输入与交互

Gallery 通过 Widgetry 控件的实际交互和公开 API 驱动演示行为。

用户操作与程序化演示操作可以改变相应控件或 Model 状态，并通过界面状态文本、选择结果或日志观察变化。

Disabled、Read-only、Modal 等限制遵循对应 Widget 的真实行为契约，不由 Gallery 另行定义一套控件语义。

### 国际化

Language 是全局状态。

公共文案由 Common、Header、Navigation 与 ColorShowcase 的强类型 Key 分类；页面专属文案由 Button、CheckBox、ComboBox、TextField、Tooltip、ScrollArea、ListView、Tree、Table、Window、Waveform 与 Cursor 的 Key 分类。Window 分类包含独立窗口、MessageBox 和 FileDialog 演示。`TextKey` 只组合这些类别，中英文按 Key 穷尽匹配，命名参数只替换一次，原始参数中的花括号不作为新模板求值。

切换语言时，具有 `LocalizedText` 绑定的内容更新为对应语言，不重建页面。

用户输入、文件路径及其他原始业务数据不因语言切换而被翻译。

新建独立窗口和 Dialog 使用当前语言生成相应标题，已绑定本地化的窗口标题也会响应语言变化。

`LocalizedWindowTitle` 将强类型文案与命名参数渲染为标题，通过 Window 的 `WidgetryWindowTitle` API 同步原生标题和明确标记的可见文字。主窗口、独立窗口、MessageBox 和 FileDialog 采用该绑定；标题文字不再由普通 `LocalizedText` 重复写入。目标识别不依赖子节点顺序或旧字符串，正文同名文字、用户文件名和路径保持原始内容。

收到 `WindowClosing` 后解除对应的本地化标题绑定，语言切换不会再尝试更新正在回收的窗口；正常结束与真正失效的活动窗口绑定分别处理。

### 配置持久化

Gallery 从 Executable 同级的 `config/config.ron` 读取配置。

默认配置为：

| 配置 | 默认值 |
| --- | --- |
| Window Width | 1920 |
| Window Height | 1080 |
| Scale Factor | 1.25 |
| Theme | Dark |
| Language | English |
| BRP Enabled | true |
| BRP Port | 15702 |

Theme、Language 或 Scale Factor 实际切换成功后，Gallery 尝试保存完整配置。

窗口 Resize 不会自动更新持久化的窗口尺寸。

BRP 的启用状态和 Port 通过配置控制，修改后需要重新启动应用才能生效。

配置端口允许 `1024..=65534`，保留 Render World 的 `15703`。合法 `BRP_EXTRAS_PORT` 可覆盖配置端口，直接启动 Executable 后也可由 Agent 连接该端口，不要求通过 Agent 启动。

## 边界情况与错误处理

### 配置读取与保存

配置文件不存在时，Gallery 使用默认配置并尝试创建文件。

读取、解析或校验失败时，整体回退默认配置，并保留原有文件，不使用部分有效字段覆盖默认值。

配置保存失败时，已经生效的 UI 修改不会回滚，但新的设置可能无法在下次启动时恢复。

配置异常通过日志报告，不要求用户在启动前手动修复配置才能进入 Gallery。

### 页面与 UI 构造失败

Gallery 要求存在唯一的 Page Host，且同一时刻只能挂载一个页面。

页面构造失败、页面生命周期不一致或必要 Entity 缺失时，应记录错误并通过 Bevy 错误处理机制报告，不将内部结构错误视为正常 No-op。

### Dialog 生命周期

FileDialog 的结果与对应 Session 和页面归属关联，避免已失效的结果更新不再存在的演示页面。

同一演示入口已有活动 Dialog 时，不重复启动对应操作。

页面离开或相关 Owner 消失后，Gallery 会处理相应 Dialog 的清理，避免将失效页面继续作为结果接收目标。

FileDialog 的结果展示属于摘要视图。扩展盘符与 UNC 路径在展示时隐藏 Windows 扩展长度前缀，其他前缀保持原样，返回的原始路径不变。多路径结果最多列出前 8 条路径，每条路径文字最多保留 240 个字符，超出部分使用省略标记。

### BRP 与原生输入

BRP 的普通 Pointer 输入不等同于真实 Windows Mouse。

Native Window 移动、Resize 及需要原生 Cursor 行为的场景，不能仅凭 BRP Custom Pointer 的执行结果判断正确性。

GUI 自动验证的具体能力边界与操作要求遵循 `rules/gui-debugging.md`。

## 已知限制

- Gallery 当前仅面向项目规定的 Windows x64 / MSVC / DX12 环境。
- 界面语言目前仅提供 English 和简体中文。
- 页面演示状态不跨页面离开与重新进入保留，Gallery 不提供演示 Session 的持久化恢复。
- Gallery 只展示已经实现的代表性功能和交互场景，不保证覆盖各 Widget 的所有公开 API、状态组合和边界条件。完整行为契约以对应 Crate 的功能文档及实际实现为准。
- BRP 无法完整替代涉及原生操作系统行为的真实 Mouse 和 Window 交互验证。

Button 页包含 HoldButton 展示区：64 / 112 / 160px、独立 Disabled、实际 Activate 次数、112px 按钮的 Focus / Blur 入口，以及本地禁用 (F7)、父级禁用 (F8)、隐藏/恢复 (F9)。快捷键可在保持鼠标或键盘充能时中断输入。下层按钮与上方圆形控件组合用于验证透明角落穿透，另显示下层激活次数。所有实例使用默认 3 秒充能 / 1.2 秒衰减。Theme 颜色目录与 Overrides 区域包含 HoldButton。
