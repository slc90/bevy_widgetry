# 04 · Style：独立窗口、modal 与 lifecycle

## 本阶段目标

使同一 FileDialog BSN 在独立 Window 中正确运行，并提供真正区分 modal/nonmodal 的输入与关闭语义。

## 范围与输出

**范围：**file_dialog 的 Window 组合，以及 window crate 中本目标必需的 modal focus/input 与关闭调度改动；相关 MessageBox/Window regression。不替换既有透明 rendering 架构。

**完成时应有：**@WidgetryFileDialog 独立弹窗入口、Modal/NonModal、嵌套覆盖确认、once-only 决议和 owned cleanup；组合 integration。

## 承接关系

**输入：**03 的完整 BSN 内容、01/02 的业务与后台生命周期；已有 owned_widgetry_window/MessageBox。

**前序：**[03 · Style：BSN 内容与多选虚拟化列表](03-bsn-style-virtualized-view.md)。

**交付给后序：**向 05 交付应用可直接消费的公共 Widget/Plugin/result API，不要求 Gallery 自己管理 worker、COM 或 raw handle。

**下一阶段：**[05 · Gallery：接入新 crate 并移除 rfd](05-gallery-migration-rfd-removal.md)。

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

## 独立窗口：沿用 owned Window 与现有 rendering

最终公开 BSN 入口为 @WidgetryFileDialog。dialog root 同时承载 FileDialog 业务身份与 owned window 组合，结果 event 指向这个 root；内部 content、rows 与 controls 不是独立的业务 session。使用已有 owned_widgetry_window 将 native Window、camera、title bar 和文件选择内容组合起来，不重新实现 event loop 或 OS 窗口。[R42]

Props 以明确的 window/display options 表达初始尺寸和 Modal/NonModal。Modal 必须提供有效 parent native Window Entity；NonModal 可绑定 owner lifecycle，也可显式独立存在。这里的 parent 不是任意 UI Node 或 camera Entity。构造检查错误通过 Scene Result 传播，不能构造一半后静默变成 nonmodal。

独立窗口的 UI root 不放进主窗口的 UI hierarchy。沿用 UiTargetCamera 与对应 RenderTarget::Window 绑定、透明 native Window 配置、owned cleanup、DX12 多窗口提交以及初次缺少 camera 时的 present guard。不能为了速度绕开已有 rendering 修复，也不要再把 Gallery 的 renderer 实现复制进新 crate。[R12][R14][R43]

需要确定 dialog 的 native Window 时，优先使用已有公开的 UiTargetCamera / Camera render target 关系或增加确有业务意义的窄接口；不得把整个 private WindowRoot/OwnedWindow 类型公开给 FileDialog。库内部保留 root→native Window→camera 的清晰关联，供 lifecycle 和正确窗口的输入路由使用。

window/camera 的创建与销毁仍经过 Bevy/winit 支持的线程与 schedule。后台 worker 不创建 Window、不调用 GUI event loop、不操作 World。首帧是否达到 150 ms 要实测；自绘只让业务 I/O 与窗口内容的出现解耦，并不保证 OS/GPU 创建成本自动为零。

## Modal：pointer blocker 之外的 focus 与 keyboard contract

当前 WidgetryModalWindow 已处理 parent pointer blocker、多个 modal child 共享 blocker 与 parent 消失时的清理，但源码没有完整的 keyboard/focus 隔离。这次不能把它直接当成完整 modal 支持。[R13]

将必要的通用 modal scope 补在 window crate：由 parent/native Window 与 modal child 关系确定当前可交互的 scope，pointer blocker 与 focus dispatch 使用同一关系。FileDialog 复用该关系，不再维护一套独立的 parent-disabled 全局状态。

Modal 打开后捕获可恢复的前一 focus，将输入定位到 dialog 合理的初始控件。Tab/Shift+Tab 在有效 modal scope 中循环；父界面已有 TextField、按钮的 keyboard activation 和 IME 编辑不能继续接收本应属于弹窗的输入。嵌套覆盖确认出现时，只让最内层 modal 接受交互；关闭后恢复到 FileDialog 的 filename/confirm 等有效目标。

focus 恢复要校验原目标仍存活、仍属于正确 window 且用户没有主动转去另一个有效 scope。多个 modal child 和嵌套 MessageBox 不能提前移除最后仍需保留的 blocker。不能通过批量覆盖父控件的 InteractionDisabled 再盲目恢复，破坏应用本来动态维护的 enabled 状态。

本库保证 Widgetry 管理的窗口 UI scope，不宣称将 OS 的整个父 HWND disable，也不宣称能阻止宿主自己实现的全局快捷键。宿主绕过 Widgetry input dispatch 的全局操作要显式尊重 modal 状态。系统 Alt+Tab、跨应用 focus、native resize 等仍有人工验证边界，不为模仿 native dialog 引入 unsafe Win32 操作。

NonModal 不创建 parent blocker，parent 与其他 nonmodal dialog 均能交互。每个 dialog 有自己的 session、selection、filename 和结果 route；切换 focus 不会把某窗口的输入、Clipboard/IME 或结果送给另一个窗口。Modal 与 NonModal 是公开行为选择，不只是背景遮罩颜色不同。

## 关闭、取消、覆盖确认与 owned cleanup

Cancel 按钮、当前 dialog 的 Escape 和 title bar X 都是明确的用户取消，可产生一次 Cancelled 结果。Escape 在 editor/IME 或嵌套确认中优先处理该局部交互，不能一次按键同时关闭两个层级。业务调用方也可通过公开 cancel API 取消，语义明确，不把取消写成错误。

已有 title bar X 会写 WindowCloseRequested，默认 WindowPlugin 在 Last 中处理关闭。FileDialog 在同一 schedule 内显式排在 close_when_requested 之前，把匹配自己 native Window 的请求转换为决议，并让结果 observer/deferred 操作完成后才清理 owned root。不依赖两个任意 observer 恰巧先后执行，也不等 WindowClosed 才试图读取已经销毁的业务 state。[R15][R39]

保持其他 Window 的默认 close 行为，不全局禁用 close_when_requested 只为了接管一个 dialog。FileDialog 自身主动完成时同样走统一 once-only finalize；必要的 ApplyDeferred/cleanup 顺序纳入 integration test。关闭 native Window 与 root 两条来源最终都不能重复发结果或遗留 camera。

parent 被销毁、外部直接 despawn root、App 退出属于 lifecycle 结束，不伪造用户按过 Cancel。未决议任务全部失效，已决议结果不撤销。直接破坏 ECS hierarchy 不属于支持的 state 更新 API，但已知 owned cleanup 与后台 token 失效仍须可靠。消费者 entity 已消失时只结束对应 route，不能误写新实体或重新打开窗口。[R03][R14][R16]

覆盖确认复用 WidgetryMessageBox 的现有结果机制，以 FileDialog 的 native Window 为 parent。只接受与当前 candidate/edit revision 匹配的确认；Yes 发出 SavePath，No/Cancel 回到仍打开的 FileDialog。FileDialog 或它的 parent 关闭时同步结束覆盖子窗口，不出现单独滞留的 MessageBox。

完成/关闭后及时释放 modal blocker、focus scope、owned native Window、camera、viewport rows 与 session admission。大 snapshot 和在途 I/O 按后台回收 contract 处理；不能为了清理干净而在 main thread join worker。

## 窗口组合的 integration 与回归保护

在 Window + FileDialog 的真实组合 fixture 中验证：owned Window/camera 绑定唯一、root 关闭回收完整；modal parent 无效构造失败；两个 modal child 关闭一个时 blocker 保留；最后一个关闭后解除阻挡；parent 销毁清理 child；两个 nonmodal 状态互不串扰。

结果 contract 覆盖 Cancel、Escape、X、Confirm、重复 Confirm、Confirm 后立刻关闭，以及覆盖框 Yes/No/Cancel。observer 读取已提交的 result 后还能排队消费，之后 root 再回收。用捕获 handler 验证 Scene/runtime 错误的 severity 与 cleanup，不使用 panic 作为通过标准。

keyboard/focus integration 验证父 TextField 在 modal 中不继续编辑、Tab scope、嵌套 modal 恢复、失效 focus target，以及 NonModal 切换后的 window identity。改动 window 的通用 modal/close 行为时，同时覆盖已存在的 MessageBox 和独立 Window，不要求重测所有不相关控件。

headless Window/camera fixture 只能证明 ECS/lifecycle，不证明 OS owner、真实 IME 或首帧显示。真实窗口的视觉与输入由后续 BRP GUI 场景和必要人工检查完成；性能由实际 display benchmark 完成。[R06][R07][R08]

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
- [R12 · 现有 Window 能力](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs)
- [R13 · 现有 ModalWindow pointer blocker](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/modal.rs)
- [R14 · Window / camera lifecycle](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/window_root.rs)
- [R15 · 现有 title bar 关闭路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/title_bar/close.rs)
- [R16 · MessageBox 的结果与回收语义](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/message_box/src/lib.rs)
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R36 · Rust JoinHandle 的 join / detach](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
- [R38 · Bevy 0.19.1 EventLoopProxyWrapper](https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html)
- [R39 · Bevy 0.19.1 WindowPlugin 关闭 schedule](https://docs.rs/bevy_window/0.19.1/bevy_window/struct.WindowPlugin.html)
- [R42 · Window 的 owned Scene 构造接口](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/scene.rs)
- [R43 · Window 已有多窗口 render 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/title_bar.rs)
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
[R12]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs
[R13]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/modal.rs
[R14]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/window_root.rs
[R15]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/title_bar/close.rs
[R16]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/message_box/src/lib.rs
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R36]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html
[R38]: https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html
[R39]: https://docs.rs/bevy_window/0.19.1/bevy_window/struct.WindowPlugin.html
[R42]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/scene.rs
[R43]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/title_bar.rs
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
