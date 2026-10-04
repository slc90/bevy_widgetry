# 07 · GUI 测试：BRP 用户场景与 temporal 验收

## 本阶段目标

通过真实输入、截图和业务状态验证新 FileDialog，而不是只证明代码能编译或 ECS 最终值正确。

## 范围与输出

**范围：**Gallery 运行验收、FileDialog case/evidence 文档、必要的库 regression。OS/IME 边界单列人工验证，不建立平行 exhaustive GUI 行为框架。

**完成时应有：**FD-G01–FD-G18 的可复现实测记录，早期/中间/稳定状态证据，以及 passed/failed/blocked/manual-unverified 分项。

## 承接关系

**输入：**05 的真实 Gallery 与 06 的 BRP secondary keyboard 支持，01–04 的业务与组合测试结果。

**前序：**[06 · GUI 前置：BRP 独立窗口 keyboard 支持](06-brp-secondary-window-keyboard.md)。

**交付给后序：**向 08 交付已确认功能正确、窗口 target 正确的场景和 fixture，性能测量不得悄悄换成缩水 UI。

**下一阶段：**[08 · GUI 性能：150 ms 开窗与持续交互验收](08-latency-performance-acceptance.md)。

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

## Modal：pointer blocker 之外的 focus 与 keyboard contract

当前 WidgetryModalWindow 已处理 parent pointer blocker、多个 modal child 共享 blocker 与 parent 消失时的清理，但源码没有完整的 keyboard/focus 隔离。这次不能把它直接当成完整 modal 支持。[R13]

将必要的通用 modal scope 补在 window crate：由 parent/native Window 与 modal child 关系确定当前可交互的 scope，pointer blocker 与 focus dispatch 使用同一关系。FileDialog 复用该关系，不再维护一套独立的 parent-disabled 全局状态。

Modal 打开后捕获可恢复的前一 focus，将输入定位到 dialog 合理的初始控件。Tab/Shift+Tab 在有效 modal scope 中循环；父界面已有 TextField、按钮的 keyboard activation 和 IME 编辑不能继续接收本应属于弹窗的输入。嵌套覆盖确认出现时，只让最内层 modal 接受交互；关闭后恢复到 FileDialog 的 filename/confirm 等有效目标。

focus 恢复要校验原目标仍存活、仍属于正确 window 且用户没有主动转去另一个有效 scope。多个 modal child 和嵌套 MessageBox 不能提前移除最后仍需保留的 blocker。不能通过批量覆盖父控件的 InteractionDisabled 再盲目恢复，破坏应用本来动态维护的 enabled 状态。

本库保证 Widgetry 管理的窗口 UI scope，不宣称将 OS 的整个父 HWND disable，也不宣称能阻止宿主自己实现的全局快捷键。宿主绕过 Widgetry input dispatch 的全局操作要显式尊重 modal 状态。系统 Alt+Tab、跨应用 focus、native resize 等仍有人工验证边界，不为模仿 native dialog 引入 unsafe Win32 操作。

NonModal 不创建 parent blocker，parent 与其他 nonmodal dialog 均能交互。每个 dialog 有自己的 session、selection、filename 和结果 route；切换 focus 不会把某窗口的输入、Clipboard/IME 或结果送给另一个窗口。Modal 与 NonModal 是公开行为选择，不只是背景遮罩颜色不同。

## BRP 前置条件：补齐独立窗口 keyboard target

已核对当前 slc90/bevy_brp v0.2.1：mouse API 有可选 window Entity，screenshot 可以指定 camera/entity；send_keys 与 type_text 的实现及测试则把 KeyboardInput.window 固定为 PrimaryWindow。仅把 FileDialog UI focus 移到新窗口不足以修复这个输入 target。[R31][R32]

因此完整的独立弹窗 keyboard BRP 验收有一个明确前置工作：在 bevy_brp fork 增加可选 window 参数，并贯通 MCP schema/参数、运行时 handler、按键 press/release 以及逐字符 typing。省略 window 保持当前 PrimaryWindow 行为，显式无效或已销毁的 window 返回清晰错误，不默默转发给其他窗口。

一次输入序列确定 target 后，延迟 release 与后续字符沿用同一个 window。中途 focus 或主窗口身份变化不能把释放事件发去别处。销毁目标时正确终止 pending 输入和 modifier 状态，不能留下 Ctrl/Shift 卡住。为默认兼容、secondary target、无效 target、销毁中的 timed key/typing 增加 fork 自己的 unit/integration regression，并检查 MCP 到 runtime 的真实参数透传。

runtime 与 MCP 发布到同一个经过验证的兼容 revision/tag，Widgetry 再更新实际 dependency/tool 版本。这里不预写一个尚未存在的版本号，也不为了这项能力升级整套无关 BRP 功能。

这部分是另一个仓库的明确依赖任务，不属于本次只读规划已经修改的内容。实施需要用户在该 fork 中单独执行或授权；在它完成之前，允许继续完成 Widgetry 的代码与 mouse GUI 检查，但独立窗口 keyboard BRP 验收必须标为阻塞，不能将其删掉或假装已通过。

## BRP GUI：执行环境、fixture 与证据

使用已通过多窗口 keyboard 能力验证的 BRP runtime/MCP。通过 brp_list_bevy 发现 widget_gallery，brp_launch 启动，读取 baseline screenshot，再执行对应场景。保持正常 desktop_app update mode，验证完成后通过 BRP 正常 shutdown。[R07][R33]

fixture 分为真实临时目录与受控慢/失败 backend。真实目录包含普通文件、多个扩展名、中文/空格/点文件、嵌套目录、同名不同目录、已有保存目标与可选 symlink。慢/失败 backend 只在明确的诊断 fixture 中替换 filesystem，不替换 UI，不直接注入“选择成功”结果；它必须经过和生产相同的调度、state、BSN 与 rendering 路径。

通过少量 Gallery Name 找到 launcher、dialog root、native Window 与对应 camera。mouse 调用显式使用目标 window；secondary screenshot 指定对应 camera 或带正确 bounds 的 entity。不能只截图 PrimaryWindow 后推断新弹窗正常。[R31]

keyboard case 先实际点击取得目标 editor/list 的 focus，再使用与其一致的 window target。selection、filter、路径输入、confirm 都通过真实 input，不通过 world mutation 替代。业务状态可结合只读查询、result Text、日志与公开 state 检查，禁止为 BRP 强行公开 worker/channel 等实现。

每个 case 记录 fixture、输入、可观察期望、结构化状态、截图/日志路径、实际结果及未覆盖项。动态场景观察操作后的早期、中间和稳定状态，特别是首帧空白、旧目录残留、晚到 reply、focus 跳转与行复用闪烁。BRP 请求成功只表示输入/请求处理成功，不等于 Widget 响应正确。[R07][R33]

用例文档可以长期保存以便复现，但 BRP 是本次运行时验收，不建设一套替代 unit/integration 的第三类 exhaustive regression suite。发现可表达的稳定 bug 必须补回所属库的 deterministic regression。必要 BRP 能力/图形环境缺失时该项标为未完成，不静默改用另一套桌面自动化。[R06][R07]

## BRP GUI：必须覆盖的真实用户场景

下表是验收 case 的最低语义覆盖，不要求对 mode/theme/window 的所有组合做笛卡尔积。测试执行人需保存对应输入与证据，不能仅勾选“没有崩溃”。

| Case   | fixture 与真实输入                               | 期望与证据                                                                               |
| ------ | ------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| FD-G01 | 单文件，点击并 Confirm；另开一次 Cancel          | 返回准确原始路径；取消独立表达；每个 session 一次结果；关闭后无 owned Window/camera 残留 |
| FD-G02 | 多文件，Ctrl/Shift/方向键、滚动后再选择          | selected set、active、anchor 正确；row 复用不点错文件；重复确认无第二份结果              |
| FD-G03 | 单/多文件夹，双击导航与确认                      | 文件不被当作文件夹接受；单选当前目录语义成立；多选空集合不能偷换成当前目录               |
| FD-G04 | Images/All filter、搜索、快速改输入              | 目录仍可导航；只接受最新 query projection；不可见旧 selection 被移除                     |
| FD-G05 | Save 输入无扩展名、带扩展名与非法名称            | 默认扩展名规则明确；错误不关闭 dialog；未确认前不创建/截断文件                           |
| FD-G06 | 已有保存目标，覆盖 Yes/No/Cancel；确认期间改名   | Yes 返回当前候选；No/Cancel 回到 FileDialog；旧确认不会确认新输入                        |
| FD-G07 | New Folder 成功、重复与无权限 fixture            | 仅明确操作修改目录；错误可恢复；成功刷新并定位；取消不伪称回滚已完成操作                 |
| FD-G08 | 路径输入、Back/Forward/Up、Refresh、失败目录     | history 不被失败/刷新污染；错误与空目录不同；失败后仍可导航与取消                        |
| FD-G09 | hidden/system/中文/长名称/不同目录同名           | 显示开关和原始路径返回正确；布局不挤坏 controls；不可创建的平台 fixture 标明缺口         |
| FD-G10 | 同 scope 重开、不同 scope、启用持久化后重启      | 最近目录/过滤偏好恢复；scope 隔离；存储失败不撤销文件选择                                |
| FD-G11 | 受控延迟 2 秒，加载时拖动列表区域/点击 Cancel    | shell 已出现；App 和 Cancel 响应；不等待任务结束；后台释放后不重开旧窗口                 |
| FD-G12 | A 目录慢，快速切换 B/C 或关闭后重开              | 只显示当前 session/generation；旧结果不覆盖新目录与新实例                                |
| FD-G13 | parent 原先 focus 在 TextField，再开 modal       | parent pointer/keyboard 编辑被正确隔离；Tab 留在有效 scope；关闭后合理恢复               |
| FD-G14 | 两个 nonmodal，再与一个 modal 组合               | window target、selection、filename 和结果彼此独立；非相关窗口不被错误禁用                |
| FD-G15 | 读取/校验/覆盖中按 X、Escape；关闭 parent/切页面 | 用户取消与生命周期结束语义正确；无 orphan dialog、blocker、camera 和 route               |
| FD-G16 | 开窗/加载/滚动中切换 Dark/Light；改变窗口尺寸    | 文字/icon/背景同期更新，无空白或旧高亮闪烁，snapshot 不重扫磁盘                          |
| FD-G17 | 键盘输入、复制粘贴、IME 编辑与确认               | BRP 可表达部分实际执行；OS 输入法与跨应用 focus 另附人工结果，不用 type_text 冒充 IME    |
| FD-G18 | 受影响的已有独立 Window 与 MessageBox            | 新 modal/关闭支持不破坏既有 show、结果、parent cleanup 与窗口 controls                   |

最终 evidence 区分 passed、failed、blocked、manual-unverified。keyboard target 尚未补齐时，FD-G02/G04/G05/G08/G13/G17 等相关分支仍是未完成；不能只完成 mouse 部分后声称全部 GUI 用例通过。

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
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R20 · TextField 与 EditableText](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R24 · Gallery App / rendering / BRP 装配](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R31 · BRP mouse window 与 screenshot camera 参数](https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/lib.rs)
- [R32 · BRP v0.2.1 keyboard 主窗口绑定](https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/keyboard.rs)
- [R33 · BRP MCP 调用、发现与结果 contract](https://github.com/slc90/bevy_brp/blob/v0.2.1/docs/mcp.md)
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
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R20]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/text_field/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R24]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/main.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R31]: https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/lib.rs
[R32]: https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/keyboard.rs
[R33]: https://github.com/slc90/bevy_brp/blob/v0.2.1/docs/mcp.md
[R44]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/AGENTS.md
