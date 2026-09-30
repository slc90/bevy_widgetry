# 01 共享测试基础：让输入、时间与渲染观察点可信

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [后一份](02-icon.md)

## 本份方案

### 目标、范围与预期产出

本方案负责后续控件测试共同使用的环境及基础合同。修改范围是 crates/test_utils，以及 core 中与 font、foreground、UI 构造阶段直接相关的测试；不承担某个控件的 selection、popup 或 window 业务。产出是被现有测试实际复用的少量辅助函数、对应辅助设施自检，以及 core 基础测试的轻量 Coverage Map。Icon 的具体生命周期归下一份方案。

### 现有覆盖与需要保留的部分

scene_app 已装配 MinimalPlugins、AssetPlugin、InputFocusPlugin、Image 和 ScenePatch 等资源；add_ui_plugins 可进一步装配真实 UI、文本、picking、visibility，仍不创建原生窗口或 render device。text_input_app 提供官方文本输入入口。press_key 会写入一个非 repeat 的 Pressed KeyboardInput 并立即 update；它适合逐次按键，但不能表达“同一帧多条消息”。

core/tests/ui_schedule.rs 的 both_build_phases_prepare_replaced_content_in_same_frame 已用提前安排的 visibility/stack 消费阶段验证 Build、Materialize 两种构造时机。default_font.rs 已覆盖显式字体不被覆盖、新组件 fallback、不同文本种类和真实内建字体加载；foreground_color.rs 已覆盖父子传播、TextColor 同步和父颜色变更。这些都保留，不能以“重写更统一的 fixture”为由删掉。

### 共享环境需要怎样调整

现有 ListView、ComboBox、RadioGroup 等测试重复装配键盘消息和 dispatch，ListView/ComboBox 的真实 UI 测试也重复创建计算 camera 和等待字体。提取这些已有重复时，保留轻量 scene_app，不把所有插件默认塞进去。输入辅助需要区分“仅入队”和“推进 update”，允许明确指定 Pressed/Released、repeat、logical key/text、window；保留 press_key 的现有便利语义。用一条辅助设施自检证明批量入队不会暗中推进 App，另一条证明不同 window 的输入没有被 fixture 混为同一对象。

camera 辅助显式接收物理尺寸与 scale factor，不能一律写死 DPI=1。条件等待辅助返回或记录已推进的次数，并在超时报告目标资源/实体；它仅用于前置资源就绪和合法收敛，不能替代同帧断言。TimeUpdateStrategy 的确定性推进优先复用；如果本轮只有 Tooltip 使用其专用 timing helper，就留在 Tooltip，不为一个消费者制造共享抽象。

### core 基础测试的补充

在现有 ui_schedule 测试旁增加或加强一条文字内容场景：在 Build 或 Materialize 内通过 deferred Commands 创建/替换嵌套内容，使用事先加载的字体，检查当帧 fallback、InheritedVisibility、stack 和真实文本 measurement。现有纯 Node 场景继续保护调度基础，不用 glyph 断言取代它。源代码允许的构造阶段才纳入承诺，不能把消费阶段之后创建的文本也要求当帧准备完成。

foreground 测试补“已有传播父节点下新增/替换文字子树”的路径，检查新旧 TextColor 和子树归属，避免只测最初创建。若增加 reparent 场景，只验证 Widgetry 自己的前景色到 TextColor 适配，不复刻 Bevy hierarchy propagation 全套测试。font 测试保留“只处理新 TextFont、已处理组件不持续重写”的语义，不把它改造成全局字体实时主题系统。

### 模块注释与责任边界

core/tests/ui_schedule.rs 可作为基础 Coverage Map 入口，标明 ui_schedule、default_font、foreground_color、icon 的不同职责；icon 的完整子模型留在 icon 测试文件。test_utils 注释说明各 helper 的依赖和推进语义，避免调用者误以为 scene_app 已有完整 UI pipeline。控件特有的 assert_selection_invariants 等不放到 core 通用测试里。

asset/tests/embedded.rs 已检查内建字体和所有当前 BuiltinIcon 的 embedded 路径及内容；log/tests/macros.rs 已检查三级日志、结构化字段、target 和调用位置。本轮保留这些测试，不为了“都有状态机”制造额外模型。所有图标是否真正能 rasterize 的验证由 Icon 方案负责，不给 asset crate 添加 SVG 解析责任。

### 验收与限制

运行 bevy_widgetry_test_utils、bevy_widgetry_core 的测试，并针对实际改用 helper 的控件运行相应 package 测试。新 helper 不能引入全局 subscriber、共享 App、真实窗口或额外隐式 update；原有同帧断言仍在原来的消费边界。此阶段不引入 proptest。仅测试设施调整不要求 BRP；如果发现并获授权修复 shared pipeline 的 GUI 回归，只用一个文字加 icon 的动态替换场景验收，不能据此宣称所有控件都已做过运行时测试。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_test_utils
cargo test -p bevy_widgetry_core
```

### 在执行链中的位置

这是执行链起点，为后续提供可复用且观察点明确的测试环境。下一份 Icon 复用这些设施；不得为尚未实际需要的控件提前建立专用抽象。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/test_utils/src/scene.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/test_utils/src/scene.rs)
- [crates/core/tests/ui_schedule.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/tests/ui_schedule.rs)
- [crates/core/tests/default_font.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/tests/default_font.rs)
- [crates/core/tests/foreground_color.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/tests/foreground_color.rs)
- [crates/asset/tests/embedded.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/asset/tests/embedded.rs)
- [crates/log/tests/macros.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/log/tests/macros.rs)

## 本方案适用的共同约束

### 实施范围与证据

执行的是测试完善，不是新增业务能力。默认修改测试代码、测试模块注释和确有当前消费者的测试辅助设施；需要实际使用 proptest 的阶段才允许添加对应 dev-dependency 和锁文件变化。不修改规则，不顺手升级 Bevy、rstest、BRP，不增加生产 API、公开私有 marker 或新控件。若新增测试暴露失败，先判断是测试环境不完整、期望不符合现有 contract，还是生产回归；保留复现证据，未经当前任务授权不顺手扩大为生产修复。不能为了得到绿色结果删掉原有断言、增加无依据的等待或改写业务语义。

### 按现行规则执行，不依赖历史讨论

Codex 开始实际修改前读取 AGENTS.md、docs/architecture.md、rules/task-scope.md、rules/development.md、rules/code.md、rules/testing.md、rules/documentation.md；涉及 GUI、诊断日志、依赖或提交时继续读取相应规则。本方案作为用户明确提供的任务输入，不要求主动读取仓库 plans/ 中的历史设计。按规则创建并持续更新根目录临时进度记录。测试代码和工程配置变更也需要独立 code-review：全新 reviewer subagent 只审查，原实施者处理 findings，之后由另一个全新 reviewer 复查。Windows 下使用 pwsh。

### 覆盖模型落在测试旁边

复杂测试模块在开头写中文 //! 注释，列出可观察状态维度、stimuli、guards、invariants 和明确的业务耦合；简单测试只写有内容的范围说明，不填空模板。多文件测试由一个现有主测试模块保存 Coverage Map，说明各文件主要负责的 contract；具体转换仍由 Rust 测试表达，不另建完整 Markdown transition table 或强制可执行 Rust 状态机。不做状态笛卡尔积；正常、边界、重复/no-op、拒绝、失效引用仅按实际语义覆盖。每个测试函数前保留或补充中文的场景与目的说明，遵守项目不新增 doctest 的要求。

### 断言证明结果，不重复实现

测试准备阶段可以设置前置状态，但不能直接写入被测结果后宣称输入链路通过。每个转换检查明确的期望结果、相关事件的来源/值/次数，以及本次可能破坏的不变量。assert_*_invariants 辅助函数不能代替场景专属期望：例如 move 完全没执行也可能仍满足“id 唯一”。观察 ECS 状态时使用真实系统执行到合同规定的阶段；model 修改后的待修复瞬间不等于修复后的稳定 invariant。公开组合行为通过公共入口验证；只有依赖私有状态的局部算法、诊断或明确调度约束才留在同模块单元测试。不能仅因为测试使用 App 就机械搬迁。

### 共享设施与测试隔离

复用 test_utils 现有 scene_app、add_ui_plugins、text_input_app、press_key、pointer 与日志辅助设施。重复出现的通用输入、时间、camera、asset 等环境装配收敛到 test_utils；控件特有的 fixture 和 contract 断言可以放在该控件 tests/support 中，不让 test_utils 反向依赖所有控件。所有新辅助设施都必须有本轮实际测试消费者，不预建通用测试框架。fixture 独立持有 App、事件记录和资源；查询在多实例测试中按 root/source/hierarchy 约束，不能全世界找到第一个或假定 single。不使用真实桌面、系统剪贴板或全局时间作为普通 headless 回归的隐含前提。

### 时序、异步资源和测试完成条件

把“等待资源首次就绪”和“资源已就绪后某次操作应当在哪个阶段生效”分开。资源准备可以使用带期限、失败诊断的条件等待；被测操作后的同帧保证应在明确的一次 update、flush 或消费阶段断言，不能用 advance_until 最终成功来证明。需要多轮 layout 的合法收敛按真实合同检查，不强行改成一帧。颜色、文字、icon、visibility、stack、layout 等检查应说明各自观察点。验证刷新请求时按新消息区间读取或清空，避免历史 RequestRedraw 造成假阳性；稳定后也检查不再无意义刷新。私有 cache 不作为业务状态维度，只有其承载明确增量更新/调度合同的部分才做局部回归。

### BRP 只负责本阶段相关的真实场景

各分方案的 BRP 段是条件验收场景：仅补测试和注释、没有修改 GUI 可观察行为时，不为了形式强制启动 Gallery。若本阶段获授权修复了相关 GUI 行为，或用户明确要求运行时验收，再按规则用 BRP 执行列出的少量真实用户路径。不能直接改 selection、focus 或滚动结果冒充用户操作，不重新跑全部历史功能。动态文字、icon、popup 和 subtree 关注操作后立即可见的过程与最终状态，而不是先等稳定再截图。连续 MCP 截图存在采样间隔，不能声称已经排除了所有单帧闪烁或精确测出了输入到显示的延迟；未能观察的时序明确记录，并由可表达的 headless 阶段断言补永久回归。保留 desktop_app 的事件驱动设置，不引入 Win32 自动化或持续高频刷新来掩盖问题。原生窗口状态等 BRP 无法忠实覆盖的路径按规则记录人工验证边界。

### 本轮完成标准

每份方案完成时，Coverage Model/Map 与实际测试对应；已有有价值的测试没有因整理丢失；新增场景具有明确触发输入和独立期望，不仅断言“不崩溃”。局部合同的测试优先用普通 #[test] 或现有 rstest，proptest 不代替已知边界和确定性回归。补已有正确行为的测试可以初次即通过，不人为制造失败；真实 bug 回归按 Red/Green 记录。执行该分方案指定 package 与直接消费者的测试、必要格式和 lint 检查，并记录未运行项与环境限制。最后的 facade 方案完成 workspace 整体验证。只有依赖结构等 architecture 事实确实变化时才同步 docs/architecture.md。

## 来源与整体边界

本方案依据 GitHub main 的固定快照 5d57413cafa65f5ccc47be397e20e917dc2bd00d，以及该快照已经更新的测试、GUI 验证和开发规则编写。目标是整理现有测试：保留已经有价值的回归保护，增强证明不足的断言，补上能够从当前 contract 和代码路径确认的覆盖缺口。不是重新设计控件，也不是再修改一次测试规则。

本次进行了源码和测试的静态盘点，没有执行 cargo test、覆盖率工具、原生窗口或 BRP。因此，文中的“已有覆盖”表示存在对应测试及断言，不表示本次运行通过；“补充”表示本模块缺少足够直接的验证，不声称其他模块完全没有间接覆盖。方案中的测试文件新名称均是拟新增位置，不冒充现有文件。实施时应先核对当前分支与基线的差异。

范围覆盖 Button、CheckBox、RadioGroup、TextField、Tooltip、ScrollArea、ListView、ComboBox、Window、MessageBox 十个控件；二态/三态 CheckBox、普通/只读 TextField 分别留在所属控件方案中。公开的 WidgetryIcon 虽位于 core，也单独形成一份方案。共享测试基础与 facade 各有一份配套方案，共十三份，按编号顺序实施。Gallery 不新增永久自动化测试，asset/log 不因这轮盘点而扩出另一套无关重构。

- [rules/testing.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/testing.md)
- [rules/gui-debugging.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/gui-debugging.md)
- [rules/development.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/development.md)
- [rules/documentation.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/documentation.md)
