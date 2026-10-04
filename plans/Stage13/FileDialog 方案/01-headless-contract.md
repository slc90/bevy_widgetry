# 01 · Headless：文件选择业务与公开 contract

## 本阶段目标

建立可独立测试的文件选择业务 state，完整定义文件/文件夹/保存、导航、过滤、selection 与存储语义。

## 范围与输出

**范围：**crates/file_dialog 的初始 Cargo、headless/model/API 与测试，必要的 root Cargo 和 architecture 更新。不构造独立窗口，不接入 Gallery，不实现平台 GUI。

**完成时应有：**可编译的新 crate 与 headless Plugin/公开 contract；mode/result/selection/navigation/filter/storage 的 deterministic unit/integration；初始 Coverage Map。

## 承接关系

**输入：**已核对的仓库规则和本总方案。此前 native picker 性能实验只作为背景，不是待复制实现。

**前序：**[总方案](overall-plan.md)。

**交付给后序：**向 02 交付不可绕过的 session/revision、结果 once-only、路径与 storage contract；异步执行不能重新定义这些语义。

**下一阶段：**[02 · Headless：后台 filesystem 与 storage runtime](02-headless-async-filesystem-storage.md)。

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

## Headless：mode、state 与结果 contract

以 egui-file-dialog 对 DialogMode、DialogState 和 FileDialogStorage 的分离为参考，不复制 egui::Id、egui Context 或它的整个 FileDialog struct。egui 的“选择多个文件和目录”是一个混合 mode；本次明确拆为 PickFiles 与 PickDirectories，避免调用方需要再次猜测返回项的 kind。[R29]

公开命名确定为 WidgetryFileDialog、WidgetryFileDialogProps、WidgetryFileDialogState、WidgetryFileDialogMode、WidgetryFileDialogResult、WidgetryFileDialogResultEvent，以及按语义需要提供的 filter、entry、storage type。具体 module 内部 helper 不机械加长前缀。WidgetryFileDialogState 是业务 authority，公开只读查询；测试可以在没有 native Window / RenderApp 的 headless App 中构造并驱动同一业务 contract，不为测试扩大私有 API。

mode 为 PickFile、PickFiles、PickDirectory、PickDirectories、SaveFile。多选返回有确定顺序、无重复的路径集合；单选恰好返回一条匹配 kind 的路径。结果有独立的 File、Files、Directory、Directories、SavePath 与 Cancelled 语义，不能用空字符串、空路径或空 Vec 同时表达取消、错误和无选择。大量路径的结果允许使用共享 immutable snapshot，避免为了通知多次复制全部字符串。

将独立的 state 维度拆开：session 的 Open/Resolved/Closing；目录的 Loading/Ready/Partial/Failed；selection 的 active、selected set 与 anchor；确认的 Idle/Validating/AwaitingOverwrite；storage 的加载/dirty/保存状态。禁止把这些不同维度塞进一个枚举的完整笛卡尔积。错误目录不等于空目录，读取中也不等于确认中。

每次打开有唯一 SessionId，同一个 root 关闭后再打开也不能复用上一次 session token。每次导航或重新计算目录视图有递增 generation / revision。root Entity、session、query revision 共同关联后台结果；物理 row Entity 和显示 index 都不是文件的业务 identity。

公开 Widget API 至少表达导航、刷新、切换 filter/排序/显示偏好、选择或清空、确认与取消。语义型方法不是另一套 UI 构造 API。立即 World 入口与 deferred Commands 入口按仓库风格各自写明实际生效时机。失败目标、stale EntryId、非法 mode 操作返回错误且不改变 authority；合法同值不发送变化通知。[R03]

selection、目录或偏好变化先提交真实 state 再发对应通知。Confirm 与 Cancel 是独立操作，不依赖 ValueChange 是否发生。一个 session 最多决议一次；第一次接受的结果保留，重复确认不再次发结果。ResultEvent 指向 dialog root，observer 可读取已提交结果，之后才安排回收。多个 observer 没有固定先后顺序。

本阶段实际建立 crate 的 Cargo member、必要的基础依赖与 headless 测试入口。headless 初始 model/state 的创建必须是纯数据构造，不需要先启动 native Window 才能测试；最终 BSN root 复用同一 state 与命令实现。

## Headless：目录、路径、导航与 selection

Entry 保存完整 PathBuf、原始 OsString name、stable EntryId、kind、已获取的 metadata 和属性状态。显示文本可以 lossy 转换，但选中、导航、确认与保存不能把显示文本反向拼回原始路径。两个显示相同的 lossy name 仍是不同条目。未知类型、读取失败、普通文件、目录和 symlink 要区分，不能把“不是目录”直接当成有效文件。

后台处理 symlink 目标类型时保留原始选择路径；dangling link 显示为不可确认或明确错误，不启动递归解析与整棵目录扫描。Windows hidden/system 按对应属性判断，Unix hidden 按文件名规则判断；不要照搬 egui 实现里以非普通文件/目录推导 system file 的简化判断。[R27][R28]

路径输入默认相对当前已确认目录解释；没有当前目录时由初始目录策略在后台确定 base。不要在 UI 中调用 canonicalize、exists、is_dir 或 metadata。不得自行展开 shell 命令、环境变量或把含 symlink 的 .. 简单字符串折叠后当作同一路径。平台 root、盘符、UNC、无权限、目录中途消失均由 backend 返回明确结果。

requested_path 与已显示内容所属路径必须可区分。导航后清空旧 selection，显示目标路径和 Loading，不把旧目录 entries 继续伪装为新目录内容。成功开始枚举后才能提交新的 history 位置；失败保留可返回的历史项。Refresh 不重复压入 history，从历史中进入新目录会截断 forward 分支；root 的 Up 是合法 no-op。

单击选择、Ctrl/Command 点击切换、Shift 连续区间选择、Ctrl/Command+A、方向键/Home/End/PageUp/PageDown 的 active navigation 都以当时的可见数据 snapshot 为准。多选只接受 mode 允许的 kind，目录仍能被双击导航。PickDirectory 无显式 selection 时可确认当前有效目录；PickDirectories 不以空 selection 偷偷表示当前目录。

选中项因 filter 或刷新而不再出现在当前 projection 时，从 selection 中移除，不能把不可见旧文件带进确认结果。排序变更不改变 stable identity。Shift anchor 遇到条目消失则清空；Ctrl+A 只选择发出操作时已经加载且匹配的条目，不自动吸收未来批次。大量 bulk selection 以 snapshot/revision 任务完成，不能在一个 UI frame 逐项复制十万条路径或触发十万次 event。bulk selection 尚未提交时 Confirm 暂不可用，Cancel 仍可用；同一 session 按输入顺序合并 selection intent，不能因后一次 Ctrl 点击取消前一个任务，就把此前 Ctrl+A 的选择意图丢掉。完成回包同时校验 selection revision，旧计算不得覆盖新 selection。

双击与 Enter 的 activation 需要独立入口。重复点击已选项虽然不产生 selection change，仍然可以打开目录或确认文件。物理 row 被复用后，旧 press/release 不能激活新条目；输入绑定至少校验 EntryId、session 与对应 view revision。

## Headless：filter、搜索、保存与目录创建

filter 使用稳定 FilterId 和可显示 label。内建支持 All Files 与一组扩展名/后缀，多个后缀可合成 Images 等组。匹配是 worker 中对 Entry 的纯数据判断：默认扩展名 ASCII case-insensitive，路径本身的比较不因此变为大小写不敏感；无扩展名、点文件、复合后缀如 tar.gz 要有明确测试。目录不被文件扩展名过滤掉。自定义 predicate 如对外提供，也只在后台执行，不能在每帧 row renderer 中调用。

搜索为当前目录文件名的 substring 搜索，不做递归扫描。切换 filter、search 或 sort 产生新的 query revision，基于已有 entry snapshot 在 CPU worker 计算，不重新读取全部 metadata。较早的搜索结果晚到不能覆盖较新的输入。输入中的路径、文件名与搜索文本由各自 EditableText 管理，不让 projection 回包覆写用户尚未提交的编辑内容。

SaveFile 只提供目标选择。用户提交文件名后进入异步 Validating，确认 parent 存在且是目录、文件名符合平台规则、目标不是目录，并取得存在状态。无扩展名时可按当前 filter 的显式 default extension 追加；用户已写不同后缀时不得静默替换，按配置允许或显示明确提示。切 filter 不在用户输入中途擅自重写文件名。

目标已存在时进入 AwaitingOverwrite，确认框的候选路径、edit revision 和 session 必须固定。用户修改文件名、导航、切换会影响候选的配置或关闭 session，旧校验/旧覆盖确认立即失效。覆盖框取消或 No 返回 FileDialog，不能等同于取消整个 session。重复确认不得发两次 SavePath。

检查文件存在与真正创建/覆盖文件之间可能发生 TOCTOU；返回 SavePath 不保证文件仍不存在、权限仍可用，也不保证后续写入成功。调用方负责实际文件写入的原子性、最终权限检查和错误反馈。库绝不为验证“可写”提前创建、截断或删除用户文件。[R37]

New Folder 是本次库唯一主动修改用户浏览目录的功能：只有用户明确提交时才在 worker 执行 create_dir，成功后刷新并定位新目录。名称无效、同名、权限不足均显示可恢复错误。操作进入 OS 后无法靠 generation 撤销副作用；用户关闭窗口只取消结果应用，不能声称新建目录已回滚。

## Headless：DialogStorage 与实例隔离

WidgetryFileDialogStorage 保存最近成功访问目录、最近确认选择的目录、show_hidden/show_system、当前 FilterId、排序偏好和 pinned folders。默认 store 是 App 内存 state。storage scope 由调用方明确区分，例如 OpenImage 与 SaveFile 可独立；未提供 scope 时为当前实例的临时 state，不引入隐式全局“最近目录”。

runtime session 的 selection、root/camera Entity、任务 token、正在输入但未提交的文件名、overwrite pending 均不持久化。可导入/导出纯 storage snapshot，并提供 opt-in 的文件持久化配置；配置文件位置由宿主提供，不由库擅自在用户目录写文件。

初始目录优先级固定：调用方显式初始目录，其次该 scope 已在内存的最近有效目录，再由后台解析宿主配置的 fallback/current directory。第一次读取磁盘偏好不能阻塞窗口首帧。如果偏好回包时用户已经导航或开始编辑，只更新可供后续 session 使用的 store，不让当前窗口突然跳走。

同 scope 的多个 nonmodal session 在 store 中按已提交 mutation 顺序合并偏好，不能在关闭时拿打开时的完整旧副本覆盖新设置。对于“最近确认目录”使用实际确认顺序；对于 pinned 操作合并 add/remove，而不是整表盲写。持久化 revision 单调递增，晚到的旧写入完成通知不能把新 dirty revision 清成 clean。

存储失败与选择失败分离。损坏、不支持版本或权限错误产生 storage warning/error 状态，继续允许用户在当前窗口选择；默认值 fallback 必须可解释，不能静默伪装为已恢复用户偏好。普通取消可以保留已提交的浏览偏好，但不更新 last_picked_dir。

## Headless 的确定性验证边界

Unit test 放在对应 module，验证 mode/kind 约束、过滤规则、后缀处理、history、selection interval/anchor、storage merge 与结果 once-only。使用 deterministic case 覆盖 empty、same-value、invalid target、stale identity、取消确认、覆盖确认回退以及 edit revision 变化。对导航/选择/filter 的随机合法操作序列用 proptest 保持 identity、selection subset、结果唯一等 invariant。[R06]

Integration test 在 crates/file_dialog/tests/ 从公开入口驱动真实 ECS state，不读写私有 field。验证命令真正执行时提交 state、observer 读到新 authority、错误后不产生变化通知、合法 no-op、批量确认结果排序，以及两个 session/state/storage scope 的隔离。

测试文件头部写 state dimensions、stimuli、guards、invariants 与必要 couplings，并由主要测试 module 维护轻量 Coverage Map。参数、路径格式和平台 case 不需要展开全部笛卡尔积；Windows reserved name、Unicode/非 Unicode、symlink 等用平台限定的明确 case。不可建立的 symlink/权限 fixture 记录未覆盖条件，不伪造成功。

复用 scene_app、ErrorCapture、LogCapture 等基础设施。ErrorCapture 与 LogCapture 的捕获范围在当前 thread，涉及 worker 的错误先通过真实回包进入被测 App 的处理边界，不能误以为主线程 capture 自动收集后台事件。[R21]

本阶段不以 sleep 或“update 小于若干毫秒”做普通测试断言，也不把没有 RenderApp 的成功当成 150 ms 通过。确定性测试保护工作量和 state contract，真实耗时留给 benchmark。[R08]

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
- [R21 · 共享测试基础设施](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R27 · egui DirectoryContent 后台加载](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/data/directory_content.rs)
- [R28 · egui FileSystem 抽象与 NativeFileSystem](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_system.rs)
- [R29 · egui DialogMode / State / Storage](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_dialog.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R37 · Rust filesystem 与 TOCTOU](https://doc.rust-lang.org/std/fs/)
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
[R21]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R27]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/data/directory_content.rs
[R28]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_system.rs
[R29]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/src/file_dialog.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R37]: https://doc.rust-lang.org/std/fs/
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
