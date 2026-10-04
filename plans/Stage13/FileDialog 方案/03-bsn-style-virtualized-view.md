# 03 · Style：BSN 内容与多选虚拟化列表

## 本阶段目标

把 headless state 投影为完整可操作的 FileDialog 内容，保持大目录与频繁更新下的 UI 工作量有界。

## 范围与输出

**范围：**file_dialog 的 style、BSN scene、私有 entry viewport、输入适配、style tests/benches 与必要内建 assets。不扩展通用 ListView 的单选 API，不新建 native event loop。

**完成时应有：**完整工具栏/路径/搜索/列表/filter/保存/footer；ScrollArea 组合的 virtualized multi-select view；theme/输入/temporal integration 与 viewport benchmark。

## 承接关系

**输入：**01 的业务 contract 与 02 的纯数据 snapshots、reply 状态、后台预算。

**前序：**[02 · Headless：后台 filesystem 与 storage runtime](02-headless-async-filesystem-storage.md)。

**交付给后序：**向 04 交付无需等目录数据就能显示的 BSN 内容、统一业务 root 身份与实际 keyboard/focus 需求。

**下一阶段：**[04 · Style：独立窗口、modal 与 lifecycle](04-independent-window-modality-lifecycle.md)。

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

## 后台 runtime：streaming、排序与数据 ownership

请求和回复都携带 root/session/generation/query revision。reply 分为 Started、EntriesBatch、MetadataPatch、ProjectionReady、Finished、Failed 等按需要成立的消息，不用一个 Option<Vec<_>> 同时表达所有状态。已知旧 session 的回复按取消 contract 丢弃；当前有效任务的 channel 异常中断必须变成明确失败。

初始 batch 大小可设 128 项，并在已有数据且达到短时间预算时提前 flush；这只保证已有数据不被人为扣住，不能保证 OS 在几毫秒内产出第一项。每个条目的 file_type/metadata 尽量一次获取并缓存，基础 name/kind 先可用，额外 size/modified 可稍后 patch，UI 不补查。

枚举期间按已到达次序显示，明确处于 Loading/部分结果状态。最终由 CPU worker 生成“目录优先、名称排序”的完整 projection，再按 stable EntryId 保留 selection 与 active。不要把每批独立排序谎称全局有序，不要每来一批在 UI 上重排全部 entries。用户明确选择其他排序规则时，仍由后台生成对应 revision 的 projection。

目录在枚举中发生变化时，分别处理可识别的重复 entry、单项消失与整个枚举失败。Partial 包含已获得的有效条目、错误计数和有界错误摘要，可让用户 Refresh；不能吞掉错误后显示 Ready。只拿到部分数据时，bulk selection 的范围仍是当时已经到达的数据。

后台 snapshot 是 immutable 的共享数据，UI 每帧只取得便宜 handle 和当前 visible range。投递队列满时实行 backpressure；允许丢弃过期 generation，不允许丢弃当前 generation 的条目来伪造快速完成。worker 在队列可消费后唤醒 App，消费者每帧按消息数/条目数和时间双预算应用，剩余结果留待后续 update。

不能忽略销毁成本：把大 Vec 装进 Arc 只让 clone 便宜，最后一个 Arc drop 仍可能在 UI 上逐项析构。服务对大 snapshot 保留回收 ownership，替换目录、关闭 session 与销毁 view 时只释放轻量引用，最后的大块回收在后台进行。回收队列也必须有界，必要时对后续大任务 backpressure，不能把无限历史 snapshot 当成缓存。

不对“任意十亿项目录”承诺恒定内存。本次固定 viewport 的目标负载为 1k/10k，压力负载为 100k 项；记录实际峰值内存并有显式上限/报错策略。无变化 frame 不应重新遍历全部 entries；切目录的清理也属于需要验证的性能路径。

## BSN 与 crate 组织：自绘内容，不增加平行 UI API

WidgetryFileDialog 作为长期存在的 SceneComponent 身份，位于整个 dialog root，Props 接收 mode、initial directory、filter、storage scope 等一次性配置。state 与 worker session 初始化只做纯数据工作。布局初建不等待 backend、磁盘或字体以外的业务数据。

提供 WidgetryFileDialogHeadlessPlugin 承载业务 state、backend service 和命令/回复处理；WidgetryFileDialogPlugin 组合 headless、默认 style 与实际需要的 sibling Plugin，按已有 is_plugin_added 模式避免重复注册。headless 测试不要求 RenderApp 或 native Window。不要为了分阶段实施永久保留一套平行业务 state。

内部 module 按职责组织，例如 lib.rs、headless.rs 及其 model/navigation/selection/confirm 子 module、filesystem.rs、worker.rs、storage.rs、style.rs、window.rs。名称与拆分以实际职责为准，不机械做到一种 type 一个文件，不使用源码 mod.rs。lib.rs 保留 module/re-export/必要装配，功能说明按仓库注释规则写。[R02][R11]

UI 组合复用 WidgetryButton、WidgetryTextField、WidgetryComboBox、WidgetryCheckBox、WidgetryScrollArea、WidgetryWindow 及必要的 MessageBox。FileDialog 对这些 sibling 的依赖是实际组合依赖，不反向依赖 facade。内部 `scene()` 返回 Scene/SceneList，消费者可以把 FileDialog BSN 与自己的内容组合，不提供闭包式 spawn tree builder。

常规内容包括 navigation toolbar、路径编辑、搜索、位置/pinned 区、目录 viewport、filter 选择、status，以及 Open/Select/Save/Cancel footer。Save mode 有 filename editor，其他 mode 不展示无效保存 controls。Loading、空目录、没有匹配项、Partial、不可访问与校验中都有不同表现，不能全部画成空白。

布局与默认配色由 WidgetryFileDialogStyle 表达，随 ThemeChanged 刷新；公开 style 仅包含真实可配置的尺寸/间距/配色，不存业务 state。重建 style 不重建 session，不丢 history、selection 或编辑光标。path/filename 使用现有 EditableText，不另存每帧双向同步的同义文本。[R20]

## 文件列表：复用 ScrollArea，私有 virtualized view 承担多选

现有 WidgetryListView 的正式 contract 是单选，没有可直接用于多选的外部 selection authority 模式。本方案不强行把文件多选塞进它的 selected 字段，也不同时维护 ListView selected 与 FileDialog selected 两套互相回写的 authority。[R17]

本次明确选择：在 file_dialog 内实现私有 FileDialogEntryViewport，组合现有 WidgetryScrollArea，参考 ListView 的 fixed-height visible-range、上下 spacer 和 BSN row materialization。这样复用现有滚轮、scrollbar、ScrollPosition 与 layout 基础能力，但文件多选、activation 和 stable EntryId 全部由 FileDialog headless contract 管理。不扩展通用 ListView 公共 API，不新增另一个对外通用列表库。[R18][R19]

文件条目 model 保存后台 snapshot handle，不把每个目录 entry 展开为 ECS Entity。按 viewport 高度、统一 row height 和少量 overscan 只构造可见 rows。固定 viewport 从 1k 增长到 100k 项时，row Entity 数应保持有界；不能仅证明 Entity 数有界，却每帧遍历全部文件生成中间 Vec。

row renderer 只读取已准备的 entry name/kind/metadata/selection，使用 BSN 创建文字与内建图标。物理 row 的 identity 可复用，业务 EntryId 不变；press 时捕获业务 identity 与 view revision，release/activation 时复核，防止快速 scroll、sort 或新 batch 造成点错文件。

滚动位置按新长度 clamp；用户导航新目录通常回顶，刷新/排序则尽可能按 active EntryId 定位。数据 snapshot 变更时只重建受影响的 visible rows，不逐次调用 Vec::move_item 做全量重排。大数据 ownership 由服务保持，viewport 只移动共享 handle，关闭不会同步析构整个目录。

entry selection 与 keyboard active/focus 分开。可见高亮是 authority 的 projection，不持有另一份业务 selected bool。大的 Ctrl+A/Shift selection 通过已冻结的 snapshot 与 CPU 任务完成，确认结果不会包含后续自动到达的 entries。提供正确的 ListBox/Option accessibility 信息与 multi-selectable/selected state；不要靠背景色作为唯一选择提示。

## Style 的交互、IME 与 temporal 验证

path/filename/search editor 的 Enter、Escape、Tab 与列表的 Enter/Space 不混用。先依据真实 focus 和 IME composition 判断事件含义：路径 editor Enter 导航，filename Enter 发起保存校验，列表 Enter activation；IME 正在确认候选时不能顺便关闭或提交 FileDialog。

列表自己处理 keyboard navigation，关闭其所组合 ScrollArea 的同义 keyboard scrolling，避免一个方向键移动 active 又额外滚动一屏。Tab 顺序覆盖 toolbar、路径、搜索、viewport、filter 与 footer；disabled 的 Confirm 不接受 pointer/keyboard，Cancel 在加载与校验时仍可用。双击文件与目录分别遵循 mode，不以 selection changed 代替双击。

GUI 行为的真值由 headless API 提交，再投影到 rows/status/button enabled。Data batch 不应抢走用户正在编辑的 focus，切换 theme 不应清空路径，滚动复用不能造成旧高亮闪在新文件上。新 Window 首帧先保证完整 shell 和 Loading 可见，业务内容晚到只能更新目录区域。

Style integration test 复用 scene_app、add_ui_plugins、输入 helper 和真实 BSN。覆盖构造当帧结构、props 一次性初始化、两个独立 view、theme 切换、TextField 光标保留、visible-range 边界、scroll clamp、recycled-row click、multiselection projection、filter/search 变化后的 repair，以及 Cancel 的可用性。[R21]

需要 keyboard 的 integration case 通过 focus dispatch 与 KeyboardInput 驱动；pointer helper 只能证明对应目标收到 event，不代替实际 picking 命中测试。真实 picking、IME、独立 window 的 focus 与首帧视觉留到 BRP/人工边界验收。[R06][R07]

本阶段新增 viewport 的 benchmark 测首次 shell、首批 row materialization、无变化 update、滚动、新 batch、filter projection 应用、关闭。按真实文字/icon renderer 测，不用空行 renderer 宣称全部 UI 成本。确定性测试保护“可见 rows 与每帧处理量有界”，耗时由性能场景验证。

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
- [R18 · ListView 的 visible-range 与 BSN materialization](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/virtualization.rs)
- [R19 · 可组合 ScrollArea 的公开能力](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/scroll_area/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R21 · 共享测试基础设施](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
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
[R18]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/virtualization.rs
[R19]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/scroll_area/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R21]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/test_utils/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
