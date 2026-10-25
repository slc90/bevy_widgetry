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

Sidebar 提供按 Widget 分类的页面入口，支持切换当前展示内容。

| 页面 | 主要演示内容 |
| --- | --- |
| Button | 二态 Toggle Button（普通、禁用选中、程序化设置）、显式 Loading 演示、普通与 Disabled Button、文字与 Icon 组合、LinkButton 及程序化 URL 切换，以及 RadioGroup 的不同排列方式。 |
| Segmented | 严格单选、内容自适应与 360px 等宽对照、横向 / 竖向、普通 / 已选 Option 禁用、整组禁用、程序选择与用户事件计数、Tab 前后目标，以及主题和局部颜色覆盖。 |
| Tabs | 横纵两个独立的十项标签组、方向键跳过禁用项及首尾停止、真实选择通知、程序选择与同值静默、整组和已选 Header 禁用、标签栏 Auto 滚动与长标题 Tooltip，以及页面内 TextField 和独立滚动的状态保留。 |
| Collapsible | 默认展开的基本设置与初始折叠的高级设置；固定 SVG 箭头切换、CheckBox / Slider / TextField 状态保留、程序化展开、两级禁用、变化通知及卡片 Theme 配色目录。 |
| CheckBox | 二态、三态 CheckBox，以及不同 Checked、Indeterminate 和 Disabled 状态。 |
| ComboBox | 文字、Icon 与混合 Renderer，初始空选择和本地化 Placeholder，可选本地搜索、无结果提示、共享数据隔离，选项数据修改、按 ID / 原始索引程序化选择及 Disabled 行为。 |
| ScrollArea | 垂直、水平和双轴滚动，Scrollbar 显示策略、内容尺寸变化和 Scroll Into View。 |
| ProgressBar | 单条真实进度条与外部 +10%/-10%/重置/模式切换，外部状态文字、平滑过渡与不确定循环，同实例局部填充色覆盖。 |
| Slider | 水平连续值、双端点方向步进、垂直自下向上填充、Disabled、默认 6/20px 与自定义 10/28px 尺寸，控件外实时数值及局部配色对照。根上的反射状态 SliderDemoState 记录拖动开始、完成、取消次数、活动状态和当前值，可通过 BRP 检查生命周期。 |
| Splitter | 横向 30% 双 Pane 与嵌套纵向 50% 实例、原生 Pane min/max、World/Commands 比例设置、通知/拒绝/Pane 按钮计数、被动父级 Resize、resizable/Disabled 差异、局部配色，以及有限最大尺寸与外层 ScrollArea 最小溢出。 |
| ListView | 小型列表、大规模虚拟化列表、Model 数据操作、Selection 及 Disabled 行为，以及文字 / 图标文字 Renderer 的共享 Model 双 View。 |
| Tree | 共享 TreeModel 双 View、基础树、异构 ECS 节点、延迟加载、大规模虚拟化及节点交互。 |
| Table | 共享 Model 双 View 的独立布局、列宽和交互，以及不同 Cell Value、Header、Selection、虚拟化与 Disabled 场景。 |
| TextField | 单行、固定多行、Read-only、Disabled、最多 5 个 Unicode 字素簇的输入、独立及带上限的字数展示、Prefix / Suffix 和 Clear 按钮组合，以及默认 2–6 行和只读 1–3 行的 AutoSize。支持常显警告色 HintText；AutoSize 示例组合全部五项能力，并提供窄/宽切换以观察软换行和独立提示布局。 |
| Tooltip | 基础与富内容 Tooltip、Disabled 控件提示和不同位置的展示行为。 |
| Window | 独立窗口、不同背景、MessageBox 及 FileDialog。 |
| Waveform | 多通道实时波形、数据回放以及高负载波形展示，各 View 的 Δt / ΔV 按钮开启屏幕矩形测量、移动与边角编辑。 |
| Canvas | 共享 Document 的主/辅助视图，矩形选择、移动、绘制、八向尺寸调整、旋转与删除，独立导航及 SVG / PNG 文件导出。 |
| Cursor | System / Kamuri 光标切换、标准与扩展 Cursor 语义、动画及 Disabled 场景。 |

ComboBox 页首次进入时各演示保持未选中，Field 显示本地化 Placeholder；清除或删除当前项后恢复提示，新增选项不会自动首选。状态查询从 ComboBox 本体读取，选择首项按钮按原始索引调用公开 API。Header 的 Theme、Language、Scale Factor 仍按实际设置显式初始化；外部设置变化同步到相应选择器。

本地搜索与数据驱动示例共享 Model，支持中英文水果名匹配，并显示查询、匹配数量、已选 ID 和 Active。查询不提交选择；过滤隐藏选中项时保留 Field。方向键导航，Enter / 鼠标确认，Esc / 外部点击取消，重开后清空查询并恢复全部结果。搜索提示、无结果内容、Placeholder 和状态文字均随语言切换，选择保持有效；禁用搜索示例阻断输入。

ListView 小列表提供启用 / 关闭单项拖拽排序的共享数据对照，大列表默认关闭；禁用 View / 行拒绝拖动。页顶部的两个 View 开启拖拽并共用一个 Model，分别使用纯文字和图标加文字 Renderer。右侧使用独立投影，仅显示偶数标签并倒序排列，不复制原始条目。任一侧的数据按钮修改共享数据；选择集合、范围锚点、Active、滚动和启用 / 禁用各自独立。普通点击单选，Ctrl 切换，Shift 按各自显示顺序选区，Ctrl+A 覆盖整模型未禁用项。拖拽在上 / 下半行给出 Before / After 边框反馈，只重排当前 View 而保持 Model 与另一侧顺序不变，即使多选也只移动起始项且保留集合；小数据状态栏列出 Model 原顺序与 Projection 显示顺序。状态栏通过只读 Selection 显示数量及小集合的原始索引，并统计 SelectionChanged；万项全选仅显示数量，可滚动检查复用行选中样式。按钮可显式重置 View 顺序、关闭/重开拖拽保留顺序，并切换 B 的偶数过滤；隐藏项恢复资格后回到手动顺序中的位置。禁用侧仍显示数据更新。离开页面时两个 View 与共享 Model 一起清理。

Tree 的基础、异构、Lazy 与万项示例默认开启拖拽，共享 A 开启、B 关闭；每侧按钮可独立切换。上/中/下三分之一落点表示之前/内部/之后，修改真实 ChildOf/Children 并只搬运一个节点（子树随动）。整框高亮表示允许拖入，灰框与禁止光标拒绝循环和禁用落点。Lazy 提供本地文件与禁用文件；Unknown/Loading 禁止 Inside，但允许合法 Before/After，加载完成后才允许拖入。状态栏显示拖拽开关、根顺序样本和已选节点的实际父节点。

Tree 页顶部两个不同高度的 View 共用一个 TreeModel，展开、Selection、Active、Focus、滚动和启用 / 禁用各自独立，原始树数据共享。默认支持 Ctrl 切换、Shift 可见范围选择、Ctrl+A 并入可见未禁用节点并保留隐藏选择；各侧提供选择可见节点、展开已加载结构、全部折叠按钮，Lazy 示例的批量展开不请求远端节点。各侧 Select Last API 按本 View 的可见投影选择；状态栏分别显示 Visible Count、Expanded Count 和 Selected IDs；大集合显示数量与两端 ID 样本，避免构造万项状态文本。只选中一个文件后，可编辑、删除或在目录与根之间移动，也可插入新文件；禁用侧继续反映共享数据，程序化选择只改变明确指定的 View。两个 View 使用相同的 App 级 Renderer，语言切换原位更新文字并保留节点身份、选择、展开和滚动。离开页面时清理所有临时 View、Model 与业务节点。

Table 页的 Shared Model 示例让两个 View 共用 64 行、8 列及 App 级 Renderer。A 的初始列宽 / 行高为 180 / 34 px，B 为 120 / 28 px 并采用较紧凑的 Cell Padding 和不同 Border。A 初始显示原始顺序，B 初始提交偶数行的倒序投影；各侧按钮可独立切换默认/受控显示。Gallery 在共享 Model 变化后重新计算并原子提交投影，状态栏显示 View 总行数、显示序号和顺序样本。两侧可拖动调整各自列宽、独立选择、键盘导航和双轴滚动；单侧禁用仍同步共享的编辑、插入、删除和移动。移动保留 Stable ID，删除无效 Selection / Focused Cell 时静默清理。语言和 Theme 切换保持现有 View、Model 与交互状态，离页回收全部临时数据。

Button 页提供 Loading 演示：点击开始按钮只激活一次并显示 Spinner，点击结束按钮恢复；F1 仅在 Button 页面且目标处于 Loading 时执行结束操作，便于保持鼠标原位观察 Wait → Pointer，包括 Kamuri 动画。离页后演示清理，重新进入恢复默认状态；说明和按钮纳入中英文切换。

Button 页的 LinkButton 区域展示正常、Disabled 链接，以及在 `https://www.baidu.com` 与 `https://www.google.com` 间切换的链接。切换按钮通过公开 API 同时更新 URL 文字和跳转目标，只有激活链接才请求系统浏览器。标题、说明及按钮支持中英文，URL 保留原文。Theme Colors 包含 `link_button` 的四态配色，Overrides 提供仅覆盖 Hover 颜色的对比、清除及父级禁用操作；离页清理示例，重新进入恢复初始 URL。

ProgressBar 页只创建一个默认 0% 的实例，不额外摆放静态对照条。加减按钮将页面模拟输入限制在 0%–100%；未知模式下加减只调整页面模拟输入，切回时提交该已知比例并立即显示。重置始终回到确定 0%。状态文字通过公开 API 读取真实状态，确定模式显示百分比，未知模式显示运行提示；页面输入与显示缓存都不作为库内部进度 Authority。四套 Theme、中英文、Scale Factor 原位更新。Theme Colors 展示 Track / Fill / Border，覆盖按钮直接作用于同一实例并可清除，因此此页不创建额外 Overrides 对照区。离页清理全部局部数据，再入恢复初始0%，保留全局设置。

Splitter 页主实例初始 30%，父容器宽 1008px、Handle 8px，Pane A 初始 min/max 为 300/800px、B 为 200/700px，直接 Patch Pane 本体 Node。操作区提供 World 10%/90% Clamp、Commands 60%、同值与非法 NaN 对照、父级 1008/865px、A 最小值 300/100px、resizable、根 Disabled 和颜色覆盖/清除。读数通过公开比例 API 与原生 ComputedNode 区分目标和实际份额，并显示真实 ValueChange、Pane 按钮和拒绝计数；被动 Resize 保留目标且不发数值事件。

Pane B 组合独立的纵向 Splitter（默认 50%、命中区 12px、细线 4px）。固定 resizable=false 与根 Disabled 示例展示 Pane 按钮差异。有限最大值示例在 1000px 父容器内只占 608px；最小溢出示例由调用方在外层组合横向 ScrollArea，360px 视口承载 708px Splitter。主实例同样外包横向滚动容器以适配窄窗口。Theme Colors 展示四态线色，覆盖直接作用于主实例；所有说明、操作和读数支持中英文。离页销毁全部实例和计数，再进入恢复初始状态，全局 Theme/Language/Scale Factor 保持。

### 全局界面设置

Tabs 页的各组操作按钮支持选择首项、末项、重选当前项，切换 Root / 已选 Header 的本地禁用请求，以及紧凑 / 宽敞布局以观察 Auto 滚动槽的出现和消失。状态栏从 Tabs 公开 API 读取当前 Index，独立统计真实 `ValueChange<usize>` 通知及最近通知值；初始化和同值设置不计数。Root 禁用时仍可使用外部按钮合法程序选择，页面内容继承禁用；Header 本地禁用仅约束标签交互。禁用请求与继承后的有效状态区分，祖先禁用期间也可调整单个 Header 的本地请求。

同一实例切换标签时，全部 Content 与输入框常驻，正文和页面自有 ScrollArea 偏移保留。F8 / F9 分别程序选择横向 / 纵向末页，不主动移动输入焦点，可用于观察隐藏旧页面时焦点清除。长标题与 Tooltip 共享本地化绑定；Theme、Language、Scale Factor 的现有设置原位更新两个实例，保留选择和输入。离开整个 Tabs Gallery 页时卸载演示，再进入重新创建。Theme Colors 展开 tabs palette；Override 对照以两个短标签展示无滚动槽布局、整组选中配色及第二个 Header 局部覆盖。

Collapsible 页仅保留两个主要示例：基本设置包含 CheckBox 和 Slider，高级设置包含可编辑 TextField。Header 标题和说明不是折叠热区，固定箭头支持鼠标与 Enter / Space。外部操作区提供展开、折叠、再次设置当前值及根 / Header 禁用；状态文字通过公开 API 读取 Expanded 和 Header 局部请求，独立统计真实展开通知及最近值。依次禁用根、禁用 Header、恢复根，可以观察 Header 请求继续生效而 Content 恢复输入。禁用期间仍可程序化展开，禁用请求和同值展开均不增加通知。F8 / F9 翻转基本 / 高级设置且不提前改变焦点，用于观察编辑时折叠将焦点移到可用箭头、箭头禁用时清除内容焦点，以及重新展开不恢复原编辑焦点。宿主 Window 的焦点作用域可在清除后回退到窗口根。Theme / Language 切换原位更新卡片和文案，保留输入；Theme Colors 提供 Collapsible 五项颜色，不增设额外对照卡片。离开该页卸载示例，再进入恢复初始状态。

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

Replay 进度复用通用 Slider。Gallery 控制器在按下时保存播放状态/倍率并暂停时钟，连续拖动提交 preview，正常或取消结束均按最后位置恢复原状态并请求最终精度。无改值手势保留控制器的精确 Duration，进度投影不反馈成重复 seek。

页面用 Live、Replay、Large local Replay 三个按钮切换场景。隐藏场景不推进其播放时钟，离开当前模式会暂停 Replay 并清理临时手势；Live 生产时钟只在 Live 模式推进，不在返回后补造整个隐藏时段的数据。小型 `.wfrm` Replay 始终保留；本地记录使用独立根实体、真实通道标签和元信息，不再替换小型演示。两种 Replay 根宽度跟随可用区域。

本地记录默认位于 `gallery/data/waveform/replay.widx`（相对编译时 Gallery 目录），可通过 `GALLERY_WAVEFORM_REPLAY` 指定另一文件，推荐绝对路径。启动绝不生成大文件；不存在时显示准备提示，后台打开时显示加载状态，失败保留当前记录。重新加载成功后建立新会话，位置归零暂停，保留显示设置与倍率。大型记录、索引和缓存由 `/gallery/data/waveform/` 的 Git 忽略规则保护，`target/` 也可用于临时验收。

在仓库根目录执行独立生成命令；默认 60 秒、8 kHz、8 通道，避免隐式消耗数 GB。以下参数显式生成 16 通道、1800 秒的数据（原始部分约 1.15 GB，索引另计）：

```powershell
cargo run --release -p bevy_widgetry_waveform_file --example generate_recording --locked -- --output gallery/data/waveform/replay.widx --seconds 1800 --rate 8000 --channels 16 --seed 42
```

`--output/--seconds/--rate/--channels/--seed` 可配置，固定参数生成相同字节。逐通道信号具有不同频率/幅值、单样本尖峰/谷值和通道缺口；流式写入真正的原始记录及独立 MinMax/LTTB 索引，内存不随整条记录增长。已有目标文件不会覆盖。

现有 GUI benchmark 支持独立磁盘场景，保留真实输入、文件读取、状态更新与绘制路径：

```powershell
$env:GALLERY_WAVEFORM_REPLAY = (Resolve-Path gallery/data/waveform/replay.widx).Path
$env:GALLERY_WAVEFORM_BENCH_SCENARIO = 'Replay'
$env:GALLERY_WAVEFORM_BENCH_ALGORITHM = 'MinMax'
cargo bench -p widget_gallery --bench waveform --locked
```

按终端提示将 Gallery 切到前台。它等待真实 OS 焦点，自动选择大文件场景，预热后持续执行远距反向拖动；前 30 秒使用指定算法，后 30 秒通过 ComboBox 切换到另一算法，初始算法也可设为 `Lttb`。`Live` 仍运行原有 64 通道 64 kHz 对照。结果位于 `target/benchmark/`，包括实际窗口配置、硬件、可执行文件和数据集 hash、帧间隔/主更新、render schedule/asset extract、实际算法/preview、读取/丢弃/排队计数、查询与 I/O 耗时、字节和 LOD 桶宽。`file_query_cpu_ms` 为总查询减去 seek/read_exact 时间，仍包括解码、校验与分配，并非纯降采样耗时。随机文件读取可能命中 OS 缓存，不代表冷物理磁盘延迟。

Replay 的 `observable.csv` 从定位意图计时，到完整目标快照被 GPU 读回为止，是包含读回开销的上界，不含 BRP RTT 或 PNG 编码，也不是 OS 实际呈现延迟。只在目标帧已提交时采样，更新目标可合并；`pixels_nonempty` 检查实际 Replay plot 内的彩色像素，不证明逐通道或最终目标正确，这些仍由 BRP 截图与结构化状态核对。benchmark 期间持续重绘用于采样，普通暂停的 Replay 按需更新。

- 100 Hz、4 Channel 的基础实时波形，第 2、3 通道在不同时段出现明确缺口，其他通道继续更新。
- 8 kHz、4 Channel 的回放会话，文件就绪后在同一根实体加载，默认暂停在 0；其内置控制栏提供播放/暂停图标、时间和倍率，Gallery 外部控制器按倍率推进时钟，末尾停止，再次播放从零重启，不自动循环。
- 64 kHz、64 Channel 的高负载实时波形。

这些示例用于观察不同数据密度下的实际显示和持续更新行为。页面独立的降采样 ComboBox 同时对三个主视图选择 Min/Max 或 LTTB，通过 Waveform 公开 API 切换。压力 Live 在每秒各通道注入单样本 1.5 V 尖峰，用于比较 Min/Max 极值包络与 LTTB 代表点折线；LTTB 不保证保留极值。算法切换保持采集与播放状态。

每个主视图顶部显示应用配置的时间范围 ComboBox。基础 Live 与 Replay 提供 1/5/10/30 秒，压力 Live 提供 1/5/10 秒；选择立即更新范围，不重新开始采集。基础源保存 31 秒历史，压力源保存 11 秒历史。状态采集中的高度和输出密度使用实际绘图区，顶部控制栏占用根布局内的独立空间。

时间范围旁提供幅值 ComboBox，当前模拟源以 V 为单位，候选为 0.25/1/2 V 单侧上限。缩小上限可观察波形越过相邻通道行，整体绘图区之外仍裁剪；切换保留会话与运行时间。

三个主视图还提供 8/16/32/∞ 通道每屏候选（∞ 为全部），压力场景默认 16，基础场景默认全部。左侧标签与右侧波形共享连续滚动；选择全部仍遵守 24 逻辑像素的最小行高。切换密度保留顶部通道和行内位置；标签不会被越道波形遮挡。状态采集包含实际行高与滚动偏移。

三个主视图下方显示会话累计秒数标尺，主刻度对应绘图区内的淡色竖向网格。页面的“显示时间网格”CheckBox 同时开关三个主视图的网格，时间标尺继续显示。

### Canvas

Canvas 页面初始创建一个带半透明填充和居中描边的120×80矩形。主视图在上、辅助视图在下，均占满可用宽度，高度分别为320和220逻辑像素。主视图初始Zoom=1、中心(0,0)，辅助视图Zoom=0.5、中心(20,-10)；二者共享页面持有的Document，选择、Pan/Zoom、预览与渲染目标各自独立。点击选中，再次按住移动；八个方点调整局部尺寸，圆点旋转，Shift按15°吸附，Delete删除当前焦点View的选择。矩形工具仅影响主视图，拖动预览、释放提交并返回选择，Esc取消。Space+左键平移锁定至左键释放，滚轮围绕光标缩放。

Canvas外侧工具栏可选择主视图工具、添加示例矩形、独立更新/重置主视图Zoom/中心，以及隐藏/恢复辅助View。Document不限制一个图形；添加示例在原点创建新的120×80矩形。状态文字显示两个View的实际Zoom/中心、主工具、导出区域与倍率。Canvas自身没有专属主题配色树，本页不显示空的Theme Colors/Overrides区域；外部Button/文字使用当前Theme，画布背景与演示Paint由Gallery定义。

导出按钮将文件写入Executable同级`exports/canvas.svg`和`exports/canvas.png`，直接覆盖已有文件；目录/路径在页面显示。区域按钮在内容AABB与Document坐标(-100,-75)至(100,75)之间切换，PNG倍率按1/2/4循环，与窗口Scale Factor及View Zoom独立。SVG同步报告写盘结果，PNG先显示进行中、收到关联完成事件后才显示成功尺寸或错误。为避免并发写入同一PNG路径，已有请求结束前再次点击PNG只提示等待重试；导出进行期间请求重绘，结束后恢复按需更新；离开页面的已发起导出仍完成并记录结果，不更新新页面。空Document导出显示错误且保留已有目标。

离页销毁本页拥有的Document、图形与两个View，重新进入恢复初始状态。隐藏辅助View不删除共享Document，恢复时继续显示当前模型。窗口比例可通过Header在1/1.25/1.5/2之间切换，与CanvasZoom和文件像素尺寸分别生效。

### Cursor

Cursor 页面可以独立选择 System 或 Kamuri Cursor，并通过不同语义的目标区域观察光标变化。

Kamuri 提供 45 种语义：34 种系统光标与 11 种扩展 Hint。43 种普通静态图为 48px，Pointer/Wait 为 64px 动画。ReadOnly 使用蓝白挂锁，并与祖先 Disabled 样例区分。

页面包含官方 System Cursor 语义、Widgetry 扩展 Hint 以及 Disabled 示例，用于比较不同 Cursor Theme 下的表现。Disabled 对照以独立卡片展示，空间不足时换行；Pointer 本地禁用与 ReadOnly 继承祖先禁用均预览禁用图标，悬停时 Disabled 优先于原始声明，显示 NotAllowed。

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

Waveform GUI Benchmark 现有 `pixels_nonempty` 检查仍按包含控制栏和时间标尺的根区域均分通道，不能作为当前布局下的逐通道波形像素验收。数据生成到 GPU 读回的计时包含读回开销，不能当作系统实际呈现延迟。

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

公共文案由 Common、Header、Navigation 与 ColorShowcase 的强类型 Key 分类；页面专属文案由 Button、CheckBox、ComboBox、TextField、Tooltip、ScrollArea、ListView、Tree、Table、Window、Waveform、Canvas 与 Cursor 的 Key 分类。Window 分类包含独立窗口、MessageBox 和 FileDialog 演示。`TextKey` 只组合这些类别，中英文按 Key 穷尽匹配，命名参数只替换一次，原始参数中的花括号不作为新模板求值。

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

Table 的虚拟化示例使用计算字符串列，保持只读；可编辑示例仅对具有真实业务字段写回的列开放 Editor。

Table 的数字、进度、布尔下拉和备注提供强类型原位编辑。CellValue 与 Shared Model 的备注两列分别使用类型默认单行和列覆盖多行 Editor。Enter/双击进入，文本使用 Enter 或多行 Ctrl+Enter 提交，布尔下拉实际切换选项后提交，初始化选中值不提交。Gallery 自动校验请求，通过 row_mut 写回并确认后结束编辑，不提供提交或接受/拒绝回执按钮。数字超过 1000 时自动拒绝，保留错误与草稿，可修改后重试或按 Esc 退出。Shared Model 两侧同步内容；编辑开关与 Disabled 独立，未注册 Editor 的格子只读。Esc/外部点击取消未提交草稿，布尔下拉的内部点击保留事务。

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

## NumberInput 示例

NumberInput 页包含 i32 的 0..10 / step=3、u8 类型边界、f64 两位小数与完整精度焦点显示、ReadOnly、本地及祖先 Disabled、min=max=7、i64 原类型极限，以及 50px 父容器中的 134px 最小宽度示例（外层 180px 裁剪）。各组状态独立展示公开 `value()` 的正式值、根 `ValueChange<T>` 次数及最近事件值；初始化、同值、取消与非法提交不计变化。预设按钮使用公开 Setter，合法同值仍覆盖草稿，只读/禁用也可程序设值。页面离开销毁示例与统计，再进入恢复初始状态。导航、说明和状态文字支持中英文，Theme Colors 展示实际复用的 text_field / button 配色，不提供 NumberInput 专属 Overrides 示例。

预设按钮在按下时执行，先覆盖草稿，再处理由按钮焦点引起的失焦；不会先提交旧草稿再设置预设值。

## 已知限制

- Gallery 当前仅面向项目规定的 Windows x64 / MSVC / DX12 环境。
- 界面语言目前仅提供 English 和简体中文。
- 页面演示状态不跨页面离开与重新进入保留，Gallery 不提供演示 Session 的持久化恢复。
- Gallery 只展示已经实现的代表性功能和交互场景，不保证覆盖各 Widget 的所有公开 API、状态组合和边界条件。完整行为契约以对应 Crate 的功能文档及实际实现为准。
- BRP 无法完整替代涉及原生操作系统行为的真实 Mouse 和 Window 交互验证。

Button 页包含 HoldButton 展示区：64 / 112 / 160px、独立 Disabled、实际 Activate 次数、112px 按钮的 Focus / Blur 入口，以及本地禁用 (F7)、父级禁用 (F8)、隐藏/恢复 (F9)。快捷键可在保持鼠标或键盘充能时中断输入。下层按钮与上方圆形控件组合用于验证透明角落穿透，另显示下层激活次数。所有实例使用默认 3 秒充能 / 1.2 秒衰减。Theme 颜色目录与 Overrides 区域包含 HoldButton。

Table 各示例的表头提供独立排序和筛选按钮。箭头循环无排序、升序、降序、无排序，另一列排序替换当前指标；筛选使用 Gallery 内部谓词，名称筛选偶数序号，数值筛选 3 的倍数，Bool 筛选 true，进度筛选至少 50%，其他列筛选 5 的倍数。多列条件取交集。业务计算及提交均由 Gallery 执行，状态栏显示已确认排序和活跃筛选数；Shared Model 的两侧指标和行结果互不影响。

Table 的普通、Shared Model 和 2000×40 虚拟化示例支持 Cell 矩形拖选、同类型 Shift 连续选中行/列/Cell、跨类型重设锚点，以及 Ctrl+A / Corner 全选当前 View。程序按钮可全选当前投影，状态栏按种类显示当前 1-based 行/列范围，包含离屏选择。Shared Model 两侧范围按各自行投影解析；筛选掉端点时静默清空无效范围。

## Table TSV 复制演示

Table 各示例提供复制完整逻辑选区的按钮和 Ctrl+C。数字列覆盖格式为两位小数，布尔值输出 yes/no，进度与标签各用强类型格式化器；备注包含真实 Tab、换行及引号。复制不包含 Header，跨离屏选区与 Shared 双 View 各自行顺序均有效。下方文本框可粘贴检查 TSV，原位编辑器内 Ctrl+C 仍复制文本选区。布尔格式化器开关可使包含该值的复制整体失败，可见反馈显示稳定 Cell 身份与原因，并保留之前的剪贴板内容。

## Table 冻结列演示

各示例可循环配置冻结 0/1/2/99 列，状态同时显示配置值和仅受真实列数限制的有效值。Shared B 初始冻结一列，与 A 独立；横向滚动只移动剩余列，两侧纵向同步。调整列宽或缩小视口可观察严格 N 与裁剪；冻结区域超过视口时不自动减少配置。Virtualization 的 2000×40 数据可用于冻结侧离屏行验证。

## Table 列拖拽与共享 Schema 演示

列拖拽默认关闭，各示例可程序启停并重置当前 View 顺序。启用后拖动标题主体观察插入线，在标题区外释放取消；右侧 6px Resize 与排序/筛选按钮保持独立操作。状态分别显示 Model 和 View 的前八个实际列标题，列拖拽不会改 Model、列宽或冻结数。

Shared Model 示例额外提供在 Model 第二位置插列、删除首列、将首列移动到末尾，以及替换整个共享 Model 的按钮。手动 View 的新列追加到末尾，Model 移列及行编辑保留偏好，另一 View 独立；显式重置恢复当前 Model 顺序，替换整个 Model 则两侧都清除旧偏好。可结合排序/过滤、连续选择、编辑、TSV和冻结边界观察同一套显示顺序。

Segmented 页提供独立的基础、自然宽度、等宽、三类禁用和竖向实例。点击组内选项获取 Focus，对应轴方向键跳过禁用项并在首尾停止；Tab 前后按钮用于观察组被跳过。各示例状态文字显示实际 Checked 索引、用户事件及程序操作次数，再次点击当前项不增加事件数。外部按钮通过公开 Commands API 循环选择禁用组，并在命令提交后读取 Checked 更新文字，另可切换整组禁用。两种方向及所有实例共享 Header 四套主题；Theme Colors 提供完整 segmented 树，Overrides 展示组级已选背景与中间 Option 局部背景覆盖、清除和父级禁用。所有文本纳入中英本地化；离页清理控件和实体 Observer，再进恢复初始状态。
