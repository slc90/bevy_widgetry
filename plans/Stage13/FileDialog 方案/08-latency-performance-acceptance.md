# 08 · GUI 性能：150 ms 开窗与持续交互验收

## 本阶段目标

证明实际输入到有效弹窗呈现的 latency 满足目标，并保护大目录、慢 I/O、频繁开关的资源与 frame 成本。

## 范围与输出

**范围：**crates/file_dialog/benches、gallery/benches 的正式测量、必要观测工具适配，以及已定位瓶颈的最小修复与回归。不重定义目标或修改全局运行模式。

**完成时应有：**可重复运行的 benchmark、分段与可见帧关联证据、首开/稳态统计和 budget 结论、资源增长检查，以及明确的未验证条件。

## 承接关系

**输入：**07 验收过的真实场景，以及各前序阶段的有界工作量 contract/benchmark 基础。

**前序：**[07 · GUI 测试：BRP 用户场景与 temporal 验收](07-brp-gui-acceptance.md)。

**交付给后序：**输出整项任务的实现/测试/GUI/性能完成状态。未满足 150 ms 或缺少显示证据时不得宣称全部完成。

**下一阶段：**完整任务的实测完成报告。

## 目标、来源与当前事实

本次新增 crates/file_dialog，package 为 bevy_widgetry_file_dialog，提供 Bevy 自绘、可复用、non-blocking 的文件选择 Widget。最终由 bevy_widgetry::file_dialog 导出，并替换 Gallery 的 native FileDialog 示例；不是在 rfd 外再包一层，也不引入 egui runtime。

本方案依据 2026-10-04 读取的 bevy_widgetry commit ea9378a2917672143f68b129343a340e0b27f3d2，以及 egui-file-dialog 0.15.0 commit 42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e。当前 workspace 使用 Bevy =0.19.1，BRP runtime 使用 slc90/bevy_brp 的 v0.2.1。开始实施时核对本地差异，不能用本方案覆盖用户已有修改。[R25][R30]

此前 rfd 与 light-file-dialog 的实验均未解决启动延迟，这是用户提供的实验结论；本方案没有重新运行那些实验，也不把“延迟来自 Show 内部”进一步写成已证明的 Windows 内部根因。当前远端源码仍是 rfd 实现。[R22]

用户提到的 WidgetTryXXX 与现有源码拼写不同。本方案按仓库已建立的 WidgetryXXX 前缀命名，例如 WidgetryFileDialog、WidgetryFileDialogPlugin；不另建 WidgetTryXXX 别名，也不重命名既有 Widget。[R12][R17][R26]

以下章节明确区分“已核对的现有能力”和“本次新增 contract”。新 type、调度策略和数值预算是本方案设计，不是声称仓库已经实现。150 ms 是待验证的交付目标，不是目前已经达到的性能。

## 工程边界与共同约束

所有库实现、默认 style、Plugin、业务 state、测试和 crate benchmark 归 file_dialog；只有明确跨 Widget 的 Window/modal 能力放入 window。facade 只 re-export，不承载行为；库不反向依赖 facade 或 Gallery。新增依赖版本统一进入 root workspace.dependencies，子 crate 使用 workspace 引用。[R01][R02][R09]

UI hierarchy、children、toolbar、rows、弹窗和确认框都通过 BSN 声明。可复用纯结构返回 impl Scene / impl SceneList；需要长期 ECS 身份的 Widget 使用 SceneComponent。Props 只初始化一次，运行时 Component 是唯一 authority。不能用 spawn_file_dialog / builder callback 再隐藏一套 UI 构造语言。deferred 构造使用现有 Scene 错误适配，失败必须清理本次预约的 root 与 owned 资源。[R02][R04]

workspace 自有源码继续 unsafe forbid。非测试库代码不引入 panic、unwrap、expect、todo 或静默丢弃错误；需要处理的失败通过业务 error state 或 Severity::Error 的 BevyError 明确送达相应消费者。库使用 widgetry_* 日志入口，不安装 subscriber，不把正常交互与每帧状态做成永久日志。[R04][R10]

内建 folder/file/navigation icon 统一进入 crates/asset，通过语义标识引用；FileDialog 不提取 Shell icon，不维护自己的 embedded asset 目录。已有 TextField 的 EditableText、ScrollArea 的 ScrollPosition 等官方 Component 保持官方使用方式，不新增同义 wrapper 限制写入。[R02][R03][R20]

实施阶段遵守 Type → Red → Green → Refactor，并按实际变更执行相应验证；代码变更另做独立 Code Review。当前交付只生成方案，不改仓库、不执行编译或 BRP 验证。实际施工的进度记录、review 和清理遵守 AGENTS 与 development 规则。[R05][R44]

某个阶段新增 workspace member、改变内部依赖或公开能力时，当阶段就同步对应 architecture/功能说明，不把文档一致性全部延到 Gallery 阶段。引用 egui 的设计不意味着复制其线程、错误处理或 UI 代码；如实施中复用具体源码，保留适用的原始许可与署名说明。

## 功能范围、存储含义与非目标

完整交付包括：单文件、多文件、单文件夹、多文件夹选择；SaveFile 目标选择；分组文件过滤器与 All Files；文件名搜索；路径输入、上一级、前进/后退、刷新；目录创建；隐藏项与 system 属性显示开关；目录列表排序；常用位置和 pinned folders；modal 与 nonmodal 独立弹窗。

“存储”覆盖两种不同能力。SaveFile 产生用户确认的目标 PathBuf，不替调用方写业务文件。DialogStorage 保存最近访问/确认目录、过滤器、排序、显示偏好和 pinned folders；默认保留在内存，显式配置后才能跨进程持久化。两者有独立错误与生命周期，不能因为偏好写盘失败而撤销已经确认的文件选择。

Windows 是这次 GUI/performance 验收的目标环境。filesystem 层按 Rust Path/OsString 和平台边界组织，其他桌面平台不保留 rfd fallback；只能声明实际编译、测试过的平台。WASM browser picker、系统 sandbox 授权桥接不在本次范围。

本次不复刻 Explorer：不做 Shell namespace、Libraries、MTP、系统 Quick Access、网络发现、Shell overlay/thumbnail、递归全文搜索、文件内容预览与 filesystem watcher。手动输入的 UNC/已挂载网络路径可以经普通 filesystem backend 访问，但不承诺网络访问完成时限。云盘项只按平台返回的普通路径与错误处理。

常用位置来自后台枚举或调用方配置，不把 Home/Documents 一律假定为本地快速目录。侧栏加载失败不能阻止用户输入其他路径、Cancel 或关闭窗口。

## 交互预算与“出现”的统一定义

150 ms 的目标是：在记录的目标 Windows/GPU 环境、Gallery 已启动且可交互时，从打开按钮对应的 input 被 App 接收，到包含 FileDialog 有效内容的第一帧实际呈现，不超过 150 ms。原生 HWND 出现、root Entity 创建、layout 完成、GPU command 提交与屏幕呈现不是同一个时刻。

有效首帧至少包含 title、路径输入区、目录内容区域或明确的 Loading state，以及可操作的 Cancel。只有空白/透明 Window 或裸背景不能算通过。目录读取可以仍在进行；同时单独报告首批条目和最终排序列表的 latency，不能用永远 Loading 掩盖不可用的浏览器。

所有 filesystem 调用、系统目录/磁盘枚举、路径验证、批量 metadata、存储序列化与写盘都不在交互关键路径同步执行。目录越大，允许后台工作越多，不允许每一帧对全部数据重新处理。持续交互按仓库默认 60 FPS / 约 16.7 ms frame budget 验证，静止时保持 reactive idle。[R08]

保留 Gallery 的 WinitSettings::desktop_app() 和真实 rendering 配置。不得用 Continuous update、提前偷偷打开 native Window、跳过字体/布局、改变功能输出或用 BRP 请求耗时替代真实测量来通过预算。[R07][R24]

首开与后续新建弹窗分别计量，不能把首开排除后只宣称中位数达标。计时边界、采样与 display 关联在性能方案中落实；没有对应证据的阶段只说明行为完成，不能先标记 150 ms 已达成。

## Reactive 唤醒、调度预算与关闭

worker 只返回拥有独立 lifetime 的数据，不持有 World、Query、Commands 或 UI Entity 的可变引用。OS/window API 的 main-thread 要求与普通 ECS system 调度是两回事；本方案要求的是 filesystem 不阻塞驱动 App 的线程，而不是把所有 ECS 工作都手工钉在 OS main thread。

桌面集成通过可用的 EventLoopProxyWrapper/WinitUserEvent::WakeUp 唤醒 event loop；headless 测试不需要 winit，显式推进 App update 即可。适配器在 file_dialog 内部有明确平台边界，不强迫消费者自己轮询，也不引入另一个 runtime。[R38]

合并多个 worker 通知，避免一个 batch 产生大量 redraw。必须测试“生产者恰好在消费者清除 wake 标记时发数据”的 race：清除后重新检查 pending；消费者因预算退出且仍有数据时安排下一次 wake。只在已有待消费结果或真正需要推进 UI materialization 时要求后续 update，不能在等待慢盘的两秒内持续空转刷新。

建议初始 merge/projection 应用预算为每 frame 不超过 1–2 ms，并同时设置最大 batch/entry 数；这个值在真实 benchmark 中校准，不作为 sleep-based unit test。单条操作本身也必须有界，不能外面检查 timer、里面一次 sort/drop 十万项。

Cancel、关闭 root、parent 消失、页面结束都会使 session token 失效。UI 立即完成对应 state/lifecycle，不等待 filesystem 返回。正在执行的读操作可以稍后结束，完成时不再接触已销毁 Entity。普通接收方已取消属于正常生命周期；当前活跃接收方异常消失则按错误 contract 处理。

主线程不使用 recv、join、block_on 等待任务。JoinHandle::join 会等待线程终止，drop 则 detach，所以不能在 Component/Resource Drop 中隐式 join 尚在 OS I/O 内的线程。[R36] App 结束时关闭 admission、发出 cooperative shutdown；空闲 worker 退出，仍卡在 OS 的 worker 不阻塞 UI 退出，诊断记录必须如实说明尚未结束的工作。正常测试释放受控阻塞后在测试收尾等待线程，避免测试互相污染。

这个限制不代表线程可以无限遗留：同一 App 只保留固定服务，不随 session 重建。资源 benchmark 统计反复开关 dialog 的线程数、队列和 retained snapshots；对无限期阻塞 OS 调用不承诺可靠超时中止，但必须证明不会无限加线程。

## 性能：起点、首帧与分段计时

旧 benchmark 起点是 Gallery 业务日志，终点是 native EVENT_OBJECT_SHOW，且明确不是首个像素。新 self-drawn Window 不使用 #32770，也不能继续把 WindowCreated/SHOW 当作内容已显示。旧约 267 ms 只作为历史背景，新结论必须注明新旧边界差异。[R34]

新 benchmark 至少记录：t_input（目标按钮对应的 release/input 被 App 接收）、t_activate（Activate handler）、t_scene（BSN 展开）、t_camera_ready、t_first_content_frame、t_presented，以及 first_batch 和 final_projection。主指标为 t_presented - t_input；另报 Activate→presented，避免把输入派发成本藏掉。

first_content_frame 要关联当前 sample/session、dialog native Window、camera 与 render frame。shell 的 title、路径输入区、列表占位/Loading、Cancel 必须已经布局并绘制，不能把只有 backdrop 的早期帧算进去。实际提交的 frame 可以通过明确的 readiness 与内容证据识别，但 CPU 标记本身不是 display timestamp。

BRP 只负责触发真实输入，MCP/HTTP 往返与截图编码/返回耗时不进入主指标。起点应在被测 App 的输入边界采集，不能从测试脚本发 HTTP 的时刻开始后又随意减掉一段估计网络时间。真实人工鼠标输入可以做补充交叉检查，并说明它与 BRP 注入的边界差别。

本次不把目标夸大为测量鼠标硬件到屏幕光子的全链路。要验证的是被测 App 收到打开动作到对应内容真正进入可见呈现路径的工程 latency，必须包含 scene、layout、rendering 与首次显示成本。

## 性能：可见帧证据与 Windows 测量工具

计时必须有两类相互对应的证据：App 内部 sample/session/window/camera/frame 的阶段记录，以及正确 native Window 的内容/呈现证据。不是在同一进程里随便找到一个 Present 就算弹窗出现；PrimaryWindow 与多个 dialog 的 swap chain 必须分清。

推荐用 PresentMon/ETW 观察目标进程的 display/present 时间，并将特定 swap chain 的 frame 与 App 标记的 first_content_frame 关联；结合正确窗口的帧捕获或截图确认该帧包含完整 shell。工具和 schema 在执行机器上核对后固定版本，不能把不同版本 CSV 字段按位置硬解析。[R40]

PresentMon 的 DisplayedTime 是该帧显示了多久，不是首次显示的绝对时间戳；未显示帧的 NA 不能变成 0 ms。使用该版本公开的 CPUStart/QPC、DisplayLatency 等定义还原正确的显示时间，并报告丢弃、无法关联或未显示样本。不能把 MsClickToPhotonLatency 自动认作这一次 BRP 注入的按钮动作。[R40]

如使用 Windows Graphics Capture，SystemRelativeTime 是 compositor 渲染该 frame 的 QPC 时间，可用于内容帧对齐，不是捕获回调收到的时间，更不是屏幕光子时间。[R41] 只取得 compositor/content timestamp 而无法确认 presentation 关联时，单独报告 input→compositor 的结果，标明严格的 input→presented 验收仍未完成，不能用更早的边界悄悄通过 150 ms。

跨进程时钟使用经验证的 QPC/同源时间映射，记录校准与不确定度。std::time::Instant 的任意进程局部起点不能直接和 ETW 或另一个进程的时间相减；wall clock 回拨、无法关联 camera/swapchain、截图只拍到主窗口等均使该样本无效。

观测工具不驱动鼠标、不修改 ECS state、不改变 App update mode；它是性能观测，不是用 Win32 自动化替代 BRP GUI 验收。任何辅助实现仍遵守 workspace 自有 unsafe forbid，可使用已有外部记录工具或经过核对的安全依赖。工具缺失或 frame mapping 无法建立时，保留 crate/internal benchmark 结果，但最终 150 ms 标为未验证。

计时数据先缓存在内存，测量关键路径不逐帧写日志/文件。测量完成后输出 JSON/CSV、时钟说明和少量匹配内容的截图；比较开启/关闭 instrumentation 的额外成本，排除观测本身造成的显著扰动。

## 性能：场景、样本、预算与失败判定

首开定义为新 Gallery 进程已经进入可交互页面后的第一次新建 FileDialog，不包括编译和整个 App 启动。重复场景每次真正关闭并新建 dialog，不复用隐藏的 native Window。正常 Plugin/共享 asset 初始化可以保留，但必须记录；不得把为 benchmark 偷跑的预开窗当作正常初始化。

建议首开至少 20 次独立进程样本，稳态至少 100 次新建样本，分别统计 median、P95、max、超预算数量与无效样本。首开样本量较小，不宣称精确 P99。这里“新进程”不等于清空 OS/Shell/disk cache；cache 条件如实记录。

在固定 viewport 和相同 renderer 下覆盖空目录、1k、10k、100k 条目、受控慢 I/O，以及 Modal/NonModal 的代表场景；文件、文件夹、保存 mode 均要有真实开窗样本。大目录和慢 I/O 不允许拖延 shell 出现，但 first_batch/final_projection 另报，不要求磁盘工作在 150 ms 内完成。

150 ms 不是只比较平均数的目标。已定义的目标场景中，首开与稳态每个有效样本连同测量不确定度均应满足 <=150 ms；出现超预算先定位，不能自动删成 outlier。无效样本必须给出独立于“结果慢”的原因。硬件/运行条件超出声明范围时另列结果，不泛化为所有机器保证。

同时记录活跃交互 frame 时间和长帧，按 60 FPS / 约 16.7 ms 目标检查；FileDialog 自己的 merge/view 工作初始控制在每帧 1–2 ms 范围并有工作量上限。超过 frame budget 的场景需说明具体 stage 并修复或取得明确 trade-off 决策，不能用平均 FPS 掩盖冻结。[R08]

反复 200 次开关/导航后检查 thread 数、owned Window/camera、row Entity、response backlog 与 retained snapshot 是否回到规定基线。受控挂起任务在测试结束时释放后再检查服务回收；不把 OS 无限期阻塞误判成 UI 必须等待，也不允许为此无限扩张线程。

使用 release/bench 等优化构建，不与构建或其他压力任务并行。记录 commit/working tree、CPU/GPU、Windows、Rust、features、profile、viewport、DPI、update mode、fixture 规模、工具版本、时钟和采样命令。JSON 至少包含 sample/session/window/frame 关联、各 latency、有效性原因和 budget 判定；截图只作为内容证据，不是 latency 数值。

## 性能回归代码与最终完成条件

crates/file_dialog/benches 通过生产 headless/view 路径测量 metadata 获取策略、batch 应用、过滤/排序、bulk selection、visible-row materialization、无变化 update 与大 snapshot cleanup。共享 fixture 先复用 test_utils，只有真实跨 crate 共用时再下沉，不创建通用 benchmark framework。[R08][R21]

gallery/benches/file_dialog 的新 harness 负责完整输入到显示场景、window/camera/frame 对齐、采集、汇总与 budget 判定。旧 rfd phases/native class 逻辑已删除。工具输出改变时 parser 明确报错，不默默读错字段。长期保留脚本和 fixture 构造方式，不只保留一张手工测量截图。

如果超出 150 ms，依据分段定位到 input dispatch、BSN/scene、camera/Window、layout/text/icon、render submission 或 presentation，而不是回头优化已经不在路径上的 COM。针对实际瓶颈完成局部修复，再重复同一负载。不要改小 workload、去掉真实内容、改成隐藏窗口复用或减少功能来制造通过结果。

最终交付必须同时满足：新 crate 的公开功能和 state contract、所有对应 unit/integration、适用 BRP/人工 GUI 验收、rfd 生产与诊断路径清理、architecture/facade/Gallery 一致，以及有证据的性能预算。某项因工具/平台依赖阻塞，就明确保留未完成，不把总体工作标记为全部通过。

最终报告分别列出实现内容、实际执行的测试、GUI evidence、性能场景/样本/统计/预算、未覆盖条件和独立 Code Review 结果。当前这份方案不是上述结果报告，不声称已经运行任何测试或达到 150 ms。

## 核对依据

- [R01 · 当前 workspace architecture](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/docs/architecture.md)
- [R02 · BSN、依赖方向与 asset 规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/architecture.md)
- [R03 · Widget runtime state 与公开 API 规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/widget-api.md)
- [R04 · 错误、unsafe 与 Scene 错误适配规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/code.md)
- [R05 · Type / TDD 与进度、验证流程](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/development.md)
- [R07 · BRP GUI 与 temporal 验证规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md)
- [R08 · Benchmark 与性能完成条件](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/benchmark.md)
- [R09 · Package 命名与依赖管理](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/dependencies.md)
- [R10 · Widgetry / Gallery 日志规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/logging.md)
- [R12 · 现有 Window 能力](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs)
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R21 · 共享测试基础设施](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R34 · 旧 native FileDialog benchmark](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/benches/file_dialog.py)
- [R36 · Rust JoinHandle 的 join / detach](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
- [R38 · Bevy 0.19.1 EventLoopProxyWrapper](https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html)
- [R40 · PresentMon Console 的时间与 display 指标](https://github.com/GameTechDev/PresentMon/blob/main/README-ConsoleApplication.md)
- [R41 · Windows Graphics Capture 的 compositor QPC 时间](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframe.systemrelativetime?view=winrt-26100)
- [R44 · 项目开发入口与独立 Code Review](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md)

[R01]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/docs/architecture.md
[R02]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/architecture.md
[R03]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/widget-api.md
[R04]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/code.md
[R05]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/development.md
[R07]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md
[R08]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/benchmark.md
[R09]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/dependencies.md
[R10]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/logging.md
[R12]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R21]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R34]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/benches/file_dialog.py
[R36]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html
[R38]: https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html
[R40]: https://github.com/GameTechDev/PresentMon/blob/main/README-ConsoleApplication.md
[R41]: https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframe.systemrelativetime?view=winrt-26100
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
