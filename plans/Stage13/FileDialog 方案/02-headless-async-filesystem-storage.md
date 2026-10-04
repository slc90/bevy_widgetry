# 02 · Headless：后台 filesystem 与 storage runtime

## 本阶段目标

让所有可能阻塞的 filesystem/存储工作在有界服务中执行，提供 streaming、取消、reactive 唤醒与可靠回收。

## 范围与输出

**范围：**file_dialog 的 filesystem、worker、storage 与对应 tests/benches，必要外部依赖和 headless/desktop wake 适配。不做真实 GUI layout，不修改 Window 外观。

**完成时应有：**固定并发服务、分批与 snapshot 协议、Native/Fake backend、内存及 opt-in 文件存储；非阻塞/取消/容量/错误测试和后台 benchmark。

## 承接关系

**输入：**01 的 mode、session、generation、路径、selection、结果与 storage contract。

**前序：**[01 · Headless：文件选择业务与公开 contract](01-headless-contract.md)。

**交付给后序：**向 03 交付只读 entry/projection snapshots、明确的 Loading/Partial/error，以及不阻塞 UI 的结果消费与回收边界。

**下一阶段：**[03 · Style：BSN 内容与多选虚拟化列表](03-bsn-style-virtualized-view.md)。

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

## Headless：DialogStorage 与实例隔离

WidgetryFileDialogStorage 保存最近成功访问目录、最近确认选择的目录、show_hidden/show_system、当前 FilterId、排序偏好和 pinned folders。默认 store 是 App 内存 state。storage scope 由调用方明确区分，例如 OpenImage 与 SaveFile 可独立；未提供 scope 时为当前实例的临时 state，不引入隐式全局“最近目录”。

runtime session 的 selection、root/camera Entity、任务 token、正在输入但未提交的文件名、overwrite pending 均不持久化。可导入/导出纯 storage snapshot，并提供 opt-in 的文件持久化配置；配置文件位置由宿主提供，不由库擅自在用户目录写文件。

初始目录优先级固定：调用方显式初始目录，其次该 scope 已在内存的最近有效目录，再由后台解析宿主配置的 fallback/current directory。第一次读取磁盘偏好不能阻塞窗口首帧。如果偏好回包时用户已经导航或开始编辑，只更新可供后续 session 使用的 store，不让当前窗口突然跳走。

同 scope 的多个 nonmodal session 在 store 中按已提交 mutation 顺序合并偏好，不能在关闭时拿打开时的完整旧副本覆盖新设置。对于“最近确认目录”使用实际确认顺序；对于 pinned 操作合并 add/remove，而不是整表盲写。持久化 revision 单调递增，晚到的旧写入完成通知不能把新 dirty revision 清成 clean。

存储失败与选择失败分离。损坏、不支持版本或权限错误产生 storage warning/error 状态，继续允许用户在当前窗口选择；默认值 fallback 必须可解释，不能静默伪装为已恢复用户偏好。普通取消可以保留已提交的浏览偏好，但不更新 last_picked_dir。

## 后台 runtime：有界并发，不使用单队列阻塞全部导航

借鉴 egui 的 FileSystem 抽象、Entry 缓存和 UI try_recv 思路，但不照搬它的每次加载新建线程，以及全部 read_dir/metadata/filter/sort 完成后才返回一个 Vec。它当前 read_dir 还会丢弃单项迭代错误，本库必须显式处理 Partial/Failed。[R27][R28][R35]

FileSystem backend 接口提供可流式枚举目录、按需补充 metadata、验证候选、创建目录、枚举位置等语义。同步 backend 接口可以保留普通 Rust io::Result，但所有调用只能由专用服务执行。UI 和 renderer 不持有可随手调用 std::fs 的路径处理 closure。

采用 App 级、数量固定的服务：初始设计为 2 个 blocking I/O worker 和 1 个 CPU/projection worker。I/O 执行目录枚举、系统查询、校验和持久化；CPU worker 执行大数据过滤/排序/选择集构造与 snapshot 回收。不是每次导航或每个 dialog 创建线程，也不把 blocking I/O 直接塞进 async executor 的 poll 中。线程数是明确配置并纳入 benchmark 的边界，不能为绕开慢盘无限加线程。

一个串行 worker 被慢网络 read_dir 卡住后，后续本地导航也会排队。generation 只能阻止旧结果应用，不能解决这个 head-of-line blocking。因此服务必须保留少量并行 I/O 槽，并对每个 session 只保存最新的尚未执行导航请求，跳过已过期队列项。

当两个 I/O 槽都被 OS 调用占住时，新请求允许显示 Queued/Loading，UI、Cancel 与其他纯内存交互继续工作。标准阻塞 filesystem 调用不具备本方案可保证的强制终止机制；不把“立即取消 UI 任务”写成“已终止 kernel I/O”。这是明确限制，不以 spawn 第三个、第四十个临时线程掩盖。

queue、同时活跃 session 数、每个 snapshot 和待回收数据都要有容量策略。超过预算返回明确的服务繁忙或容量错误，而不是截断文件列表后装成加载成功。主线程提交使用 non-blocking admission，不能等待工作槽、bounded channel 空位或 worker 持有的长时间锁。

## 后台 runtime：streaming、排序与数据 ownership

请求和回复都携带 root/session/generation/query revision。reply 分为 Started、EntriesBatch、MetadataPatch、ProjectionReady、Finished、Failed 等按需要成立的消息，不用一个 Option<Vec<_>> 同时表达所有状态。已知旧 session 的回复按取消 contract 丢弃；当前有效任务的 channel 异常中断必须变成明确失败。

初始 batch 大小可设 128 项，并在已有数据且达到短时间预算时提前 flush；这只保证已有数据不被人为扣住，不能保证 OS 在几毫秒内产出第一项。每个条目的 file_type/metadata 尽量一次获取并缓存，基础 name/kind 先可用，额外 size/modified 可稍后 patch，UI 不补查。

枚举期间按已到达次序显示，明确处于 Loading/部分结果状态。最终由 CPU worker 生成“目录优先、名称排序”的完整 projection，再按 stable EntryId 保留 selection 与 active。不要把每批独立排序谎称全局有序，不要每来一批在 UI 上重排全部 entries。用户明确选择其他排序规则时，仍由后台生成对应 revision 的 projection。

目录在枚举中发生变化时，分别处理可识别的重复 entry、单项消失与整个枚举失败。Partial 包含已获得的有效条目、错误计数和有界错误摘要，可让用户 Refresh；不能吞掉错误后显示 Ready。只拿到部分数据时，bulk selection 的范围仍是当时已经到达的数据。

后台 snapshot 是 immutable 的共享数据，UI 每帧只取得便宜 handle 和当前 visible range。投递队列满时实行 backpressure；允许丢弃过期 generation，不允许丢弃当前 generation 的条目来伪造快速完成。worker 在队列可消费后唤醒 App，消费者每帧按消息数/条目数和时间双预算应用，剩余结果留待后续 update。

不能忽略销毁成本：把大 Vec 装进 Arc 只让 clone 便宜，最后一个 Arc drop 仍可能在 UI 上逐项析构。服务对大 snapshot 保留回收 ownership，替换目录、关闭 session 与销毁 view 时只释放轻量引用，最后的大块回收在后台进行。回收队列也必须有界，必要时对后续大任务 backpressure，不能把无限历史 snapshot 当成缓存。

不对“任意十亿项目录”承诺恒定内存。本次固定 viewport 的目标负载为 1k/10k，压力负载为 100k 项；记录实际峰值内存并有显式上限/报错策略。无变化 frame 不应重新遍历全部 entries；切目录的清理也属于需要验证的性能路径。

## Reactive 唤醒、调度预算与关闭

worker 只返回拥有独立 lifetime 的数据，不持有 World、Query、Commands 或 UI Entity 的可变引用。OS/window API 的 main-thread 要求与普通 ECS system 调度是两回事；本方案要求的是 filesystem 不阻塞驱动 App 的线程，而不是把所有 ECS 工作都手工钉在 OS main thread。

桌面集成通过可用的 EventLoopProxyWrapper/WinitUserEvent::WakeUp 唤醒 event loop；headless 测试不需要 winit，显式推进 App update 即可。适配器在 file_dialog 内部有明确平台边界，不强迫消费者自己轮询，也不引入另一个 runtime。[R38]

合并多个 worker 通知，避免一个 batch 产生大量 redraw。必须测试“生产者恰好在消费者清除 wake 标记时发数据”的 race：清除后重新检查 pending；消费者因预算退出且仍有数据时安排下一次 wake。只在已有待消费结果或真正需要推进 UI materialization 时要求后续 update，不能在等待慢盘的两秒内持续空转刷新。

建议初始 merge/projection 应用预算为每 frame 不超过 1–2 ms，并同时设置最大 batch/entry 数；这个值在真实 benchmark 中校准，不作为 sleep-based unit test。单条操作本身也必须有界，不能外面检查 timer、里面一次 sort/drop 十万项。

Cancel、关闭 root、parent 消失、页面结束都会使 session token 失效。UI 立即完成对应 state/lifecycle，不等待 filesystem 返回。正在执行的读操作可以稍后结束，完成时不再接触已销毁 Entity。普通接收方已取消属于正常生命周期；当前活跃接收方异常消失则按错误 contract 处理。

主线程不使用 recv、join、block_on 等待任务。JoinHandle::join 会等待线程终止，drop 则 detach，所以不能在 Component/Resource Drop 中隐式 join 尚在 OS I/O 内的线程。[R36] App 结束时关闭 admission、发出 cooperative shutdown；空闲 worker 退出，仍卡在 OS 的 worker 不阻塞 UI 退出，诊断记录必须如实说明尚未结束的工作。正常测试释放受控阻塞后在测试收尾等待线程，避免测试互相污染。

这个限制不代表线程可以无限遗留：同一 App 只保留固定服务，不随 session 重建。资源 benchmark 统计反复开关 dialog 的线程数、队列和 retained snapshots；对无限期阻塞 OS 调用不承诺可靠超时中止，但必须证明不会无限加线程。

## 持久化 backend 与最小依赖

内存 storage 是默认实现；显式 file persistence 的读取、解析、编码、写入、flush/replace 全部经过 I/O 服务。格式有 schema version、scope 与受限大小，限制单文件和 pinned 条目数量，越界返回可解释错误，不在主线程反序列化巨大外部文件。

Path 的持久化必须 lossless。不要直接把 PathBuf.to_string_lossy() 放进 JSON 再声称能还原所有路径。采用带平台标识的路径 DTO：Windows 保存原始 UTF-16 units，Unix 保存原始 bytes，或经评估使用有相同 round-trip contract 的安全编码。跨平台导入不能解释的路径标记为不适用，普通显示偏好仍可恢复。

同 scope 同时最多一个持久化写入在途，期间只保留最新待保存 revision。写入同目录临时文件，再使用经验证的安全 replace 路径提交，失败保留原文件；不得先删除旧文件再 rename。具体安全 crate/标准库能力须在目标平台验证，不以一个 rename 名字就承诺所有平台的原子覆盖和断电耐久性。文件配置本身不包含业务文档数据。

成功确认文件立即发 result，不必等偏好写盘。正常 App 关闭可以提供明确的异步 flush 完成状态；不能在 main-thread Drop 中等待，也不能把尚未持久化的数据标记为已保存。突然终止进程时最后一笔 preference 可能未落盘，UI 与文档要区分 memory committed 和 disk committed。

filesystem 层优先使用 std。系统常用目录/磁盘枚举如需 directories 或专门的安全 Rust dependency，只引入当前确实需要的能力并集中版本管理，不照抄 egui Cargo.toml 的全部依赖。serde/serde_json 或其他 codec 也只按实际持久化实现引入。不得为这些查询引入 workspace 自有 unsafe 或 PowerShell 子进程。

## 后台与存储的测试、工作量 invariant

提供 FileDialog 专属 FakeFileSystem / controlled backend，先放本 crate 的测试支持 module。通过 barrier、可控 channel 和显式完成 token 决定何时返回，不靠随意 sleep。真 filesystem integration case 使用临时目录；cleanup 无论成功失败都不触碰用户文件。

Unit test 验证分批边界、单项错误汇总、Partial、过滤/排序确定性、metadata 缓存、generation 替换、backpressure 与 storage revision 合并。Integration test 验证首次 update 即有 Loading state，在 read_dir 尚未释放时 App 仍能处理另一个按钮/Cancel；取消后释放旧任务不产生 stale state、结果或 Entity 访问。

固定两个 I/O 槽的关键 case：一个任务阻塞时另一个本地任务可以前进；两个槽都阻塞时 UI 仍可取消且不会生成第三个线程；快速 A→B→C 导航只应用 C 的回复；关闭再开同一个 UI 位置不能接收上一个 session 的结果。

设置一个会记录调用 thread ID 的 backend，断言 read_dir、metadata、exists/校验、位置枚举、storage load/save 从不发生在测试 App 驱动 thread。同时验证空结果、当前有效任务异常断开、取消中的 send 失败和 worker 启动失败有不同处理。

存储测试覆盖默认不写盘、重复开关恢复、scope 隔离、并发 session merge、乱序完成、损坏文件、未知版本、路径 round-trip、旧文件保留与受控写失败。SaveFile 结果成功但 preference 保存失败时，结果不可被回滚或重复发送。

长期 regression 保护可确定的上限：固定 worker 数、bounded pending 队列、单次 update 消费量、旧 revision 不应用、未变化时零 filesystem 查询、一次批次不重复查询全部 metadata、大 snapshot 不在 UI 最后释放。耗时、峰值内存与吞吐用 crates/file_dialog/benches/ 测量，不用单次 CI wall-clock 断言代替。[R06][R08]

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
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R27 · egui DirectoryContent 后台加载](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/data/directory_content.rs)
- [R28 · egui FileSystem 抽象与 NativeFileSystem](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_system.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R35 · Rust read_dir 的迭代、错误与顺序](https://doc.rust-lang.org/std/fs/fn.read_dir.html)
- [R36 · Rust JoinHandle 的 join / detach](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
- [R38 · Bevy 0.19.1 EventLoopProxyWrapper](https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html)
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
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R27]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/data/directory_content.rs
[R28]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_system.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R35]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
[R36]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html
[R38]: https://docs.rs/bevy_winit/0.19.1/bevy_winit/struct.EventLoopProxyWrapper.html
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
