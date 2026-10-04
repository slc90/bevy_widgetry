# 05 · Gallery：接入新 crate 并移除 rfd

## 本阶段目标

把新库变成真实 Gallery 的唯一文件选择路径，完成 facade、依赖和旧 native 实现的迁移。

## 范围与输出

**范围：**facade、Gallery Window 页/App 装配、旧 worker/插桩/benchmark 迁移、Cargo 和当前文档。不删除其他 Widget 使用的共享能力，不清理无关本地修改。

**完成时应有：**原有五项加多目录的 BSN 演示、modal 选择与多实例 route；无旧 native picker 的生产依赖图；可定位的 GUI 场景和新计时协议入口。

## 承接关系

**输入：**04 已可消费的完整 FileDialog，以及 01–03 已通过的库行为与 view 测试。

**前序：**[04 · Style：独立窗口、modal 与 lifecycle](04-independent-window-modality-lifecycle.md)。

**交付给后序：**向 06/07 交付真实、稳定的独立弹窗验证场景；向 08 交付实际生产路径上的输入与 frame 关联点。

**下一阶段：**[06 · GUI 前置：BRP 独立窗口 keyboard 支持](06-brp-secondary-window-keyboard.md)。

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

## Gallery 接入与 facade 装配

核对 headless 阶段已加入的 crates/file_dialog workspace member，package 保持 bevy_widgetry_file_dialog。facade 增加 path dependency 和 file_dialog module re-export。新 crate 的外部依赖统一在 workspace 管理，默认 Plugin 自动装配实际用到的 sibling Plugin，避免 Gallery 用一串内部 Plugin 才能打开窗口。[R09][R26]

保留 Window 页当前 Open File、Open Files、Select Folder、Open Image、Save File 的演示语义，全部改为自己的 BSN FileDialog；补充 Select Folders 和清楚可见的 Modal/NonModal 选择。Open Image 使用新 filter contract，Save 演示初始 filename、默认扩展名和覆盖确认。

Gallery 的 Activate handler 只组成 BSN 并提交 scene，不访问 WINIT_WINDOWS 去获取 native FileDialog owner，不建立 future 或 filesystem worker。通过 FileDialog result event 更新对应演示结果。大批结果显示 count 和有限路径摘要，不能为了显示或日志把十万条路径在主线程拼成一个巨大字符串。

取消旧的“全页面任意一个 future pending 就拒绝全部按钮”的 guard。每个 launcher 可以避免重复打开自己的同一个演示实例，但多个不同 nonmodal 演示必须能够同时存在。route 保存实际 dialog root/session 与对应 result entity；页面销毁时按显式 owner 生命周期结束 route，迟到结果不能写入新页面。

为演示 root、launcher、result 提供少量稳定 Name，供 BRP 查找。不要仅为测试给所有库内部 row 添加永久调试标签。首次显示、Loading、Partial/error、状态存储和 reopening 都应能在 Gallery 实际体验，不建立仅 benchmark 才有的另一套文件选择 UI。

Gallery 正常业务操作使用 info/warn/error，库内部错误按既有规则传播。补全新功能的 lib.rs/main.rs 功能概览与 docs/architecture.md 的成员/依赖图；不把历史 plans 当作当前架构事实。[R01][R10][R11]

## 移除 rfd：实现、依赖与旧测量一起迁移

删除 Gallery 旧 file_dialog/worker.rs 的 native dialog worker，以及 file_dialog.rs 中 rfd/AsyncFileDialog、FileDialogFuture、SyncCell、first poll、poll_results、native set_parent 和只为旧路径存在的 cfg 分支。Windows 与其他桌面平台都不能悄悄保留 native fallback。[R22]

移除 root 和 gallery 的 rfd dependency，让 Cargo 更新 lockfile 的实际依赖图。实验性 light-file-dialog 或临时 patch 若仍留在本地，也只清理本次已确认的残留，不覆盖用户其他未提交修改。EventLoopProxy/WinitUserEvent 等在新库有合法用途，不能按名字把整个 workspace 中的相关功能删掉。

旧 gallery/benches/prepare_file_dialog_trace.py 只用于 rfd 源码插桩，应移除。旧 file_dialog.py 的 #32770、native thread、rfd phase markers 和 EVENT_OBJECT_SHOW endpoint 不能继续当作新弹窗验收；该 benchmark 入口应重写为新测量 contract，而不是彻底删除性能测试。[R34]

新 GUI benchmark 放 gallery/benches，crate 的后台/viewport benchmark 放 crates/file_dialog/benches。保留可重复执行的 fixture、采集与分析代码，结果 artifact 留独立输出目录，不提交临时 ETL、截图洪流、用户真实路径或 registry 副本。

对现行源码、Cargo、测试、bench scripts 和维护文档做 rfd / AsyncFileDialog / native FileDialog 检查，解释任何仍有业务用途的命中。历史实验结论可保留为明确标注的历史记录或 Git 历史，不再让旧方案作为当前运行指引。目标依赖图中不应残留 rfd；如果出现第三方 transitive 引用，定位来源后再决定必要处理，不为了清字符串破坏无关依赖。

## Gallery 阶段的验证与交付边界

按影响范围执行新 crate 的 unit/integration、facade 消费测试，以及 workspace fmt/check/clippy/build/test。facade integration 验证消费者只通过 bevy_widgetry::file_dialog 与 BSN 就能构造所需场景，且不需要额外直接依赖 rfd 或 file_dialog 的内部 module。

Gallery 按现有规则不新增普通 unit/integration 行为 suite；可重复的 GUI case 文档、运行证据和性能 harness 不受这一例外免除。把真实跨 Widget contract 的 regression 放库 tests，而不是在 Gallery 建一套平行测试业务逻辑。[R06][R08]

Windows 命令使用 pwsh。更新的 architecture、依赖图与功能说明必须对应实际落地内容，不能在仅编译通过时就写“首帧小于 150 ms”。本阶段输出可交互的新 Gallery、无旧 native picker 的生产路径和可供下一阶段操作的稳定实体定位。

## 性能：起点、首帧与分段计时

旧 benchmark 起点是 Gallery 业务日志，终点是 native EVENT_OBJECT_SHOW，且明确不是首个像素。新 self-drawn Window 不使用 #32770，也不能继续把 WindowCreated/SHOW 当作内容已显示。旧约 267 ms 只作为历史背景，新结论必须注明新旧边界差异。[R34]

新 benchmark 至少记录：t_input（目标按钮对应的 release/input 被 App 接收）、t_activate（Activate handler）、t_scene（BSN 展开）、t_camera_ready、t_first_content_frame、t_presented，以及 first_batch 和 final_projection。主指标为 t_presented - t_input；另报 Activate→presented，避免把输入派发成本藏掉。

first_content_frame 要关联当前 sample/session、dialog native Window、camera 与 render frame。shell 的 title、路径输入区、列表占位/Loading、Cancel 必须已经布局并绘制，不能把只有 backdrop 的早期帧算进去。实际提交的 frame 可以通过明确的 readiness 与内容证据识别，但 CPU 标记本身不是 display timestamp。

BRP 只负责触发真实输入，MCP/HTTP 往返与截图编码/返回耗时不进入主指标。起点应在被测 App 的输入边界采集，不能从测试脚本发 HTTP 的时刻开始后又随意减掉一段估计网络时间。真实人工鼠标输入可以做补充交叉检查，并说明它与 BRP 注入的边界差别。

本次不把目标夸大为测量鼠标硬件到屏幕光子的全链路。要验证的是被测 App 收到打开动作到对应内容真正进入可见呈现路径的工程 latency，必须包含 scene、layout、rendering 与首次显示成本。

## 核对依据

- [R01 · 当前 workspace architecture](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/docs/architecture.md)
- [R02 · BSN、依赖方向与 asset 规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/architecture.md)
- [R03 · Widget runtime state 与公开 API 规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/widget-api.md)
- [R04 · 错误、unsafe 与 Scene 错误适配规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/code.md)
- [R05 · Type / TDD 与进度、验证流程](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/development.md)
- [R06 · Unit / integration / Gallery 测试边界](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/testing.md)
- [R07 · BRP GUI 与 temporal 验证规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md)
- [R08 · Benchmark 与性能完成条件](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/benchmark.md)
- [R09 · Package 命名与依赖管理](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/dependencies.md)
- [R10 · Widgetry / Gallery 日志规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/logging.md)
- [R11 · 中文文档、注释与禁止 doctest](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/documentation.md)
- [R12 · 现有 Window 能力](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs)
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R34 · 旧 native FileDialog benchmark](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/benches/file_dialog.py)
- [R44 · 项目开发入口与独立 Code Review](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md)

[R01]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/docs/architecture.md
[R02]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/architecture.md
[R03]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/widget-api.md
[R04]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/code.md
[R05]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/development.md
[R06]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/testing.md
[R07]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md
[R08]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/benchmark.md
[R09]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/dependencies.md
[R10]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/logging.md
[R11]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/documentation.md
[R12]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R34]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/benches/file_dialog.py
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
