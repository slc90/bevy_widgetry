# 13 Facade：补消费者组合测试并完成全仓测试验收

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](12-message-box.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/bevy_widgetry/tests/public_api.rs、focus.rs 以及所有前序测试改动的最终验证。产出是明确区分编译/API 可用性与 runtime behavior 的测试模块说明、必要的消费者组合回归和完整验证记录。不修改 facade 的生产 re-export，不构造第三套全量 Widget 行为测试，也不新增 Gallery 自动化 suite。

### 保留现有 API 测试的用途

public_api.rs 已通过 facade 验证多种 Widget 构造、ListView 的多业务 type 注册与共享 model、公开 ScrollArea viewport、RadioGroup 程序化操作、不同 Widget 的字体策略、Icon 入口和 MessageBox type。部分测试只创建 Scene 值或检查类型可用，本来就是 API/编译 smoke，不应因为没有运行全部 system 被判定为无价值。

例如 window_scene_api_is_usable 和 message_box_scene_api_is_usable 使用 placeholder 构造 Scene，没有把无效绑定当成运行成功。icon_scene_api_is_usable 使用示例路径后只 flush 构造身份，也不是有效图片加载测试；真正的 asset 行为由 Icon 方案承接。保留这些边界，不为了消除示例路径把 smoke 变成异步 I/O。

focus.rs 已覆盖 TextField→Button 的 pointer focus、不同 plugin 添加顺序和 FocusLost 通知，以及普通 Node 点击清除文本 focus。新测试不得重复这两条流程，只扩展当前确有组合空白的路径。

### 需要增强的公开消费者证据

combo_box_scene_api_is_usable_without_installing_font_fallback 调用了 set_selected，但最终主要断言 root 身份和字体未变化。增加通过公开 hierarchy 定位内部 ListViewState 的结果断言，确认所选 stable id 真正生效且 source 归属正确；不直接导入 ComboBox 私有 module，不复制全部 selection 修复测试。

在仅使用 facade 的环境里增加一条有效 parent window + MessageBox 构造和结果交互 smoke。与现有只构造 Scene 的 API 案例并存，验证统一入口能装配真实插件、产生公开结果并释放 owned dialog，parent 保留。Window 内部配置矩阵、MessageBox 所有按钮组合已由专门方案覆盖，这里只选代表。使用现有公开导出；若发现预期导出与仓库事实不符，先报告，不能在测试任务中顺手扩大 facade API。

测试业务 fixture 可以使用 Bevy 和 test_utils，但 Widget 生产入口必须来自 bevy_widgetry，避免表面测试 facade、实际全部从功能 crate 导入。沿用现有字体约定：Button/TextField 不自动覆盖调用方字体政策，而 Window 安装默认 fallback；不能套一个“所有插件都不安装字体”的统一错误断言。

### 跨控件 focus 与关闭

补一个消费者实际组合：同一受控输入环境中的 TextField 与 ComboBox。从 TextField 的真实输入/selection 出发，点击 ComboBox Field 进入 popup/list focus，再点击 TextField 关闭 popup 并把 focus 留在 TextField；接下来的键盘输入由 TextField 消费，不能继续改变隐藏 ListView 的 active/selected 或发出 ComboBox ValueChange。

这个场景不等于 popup_composition 中已有的普通 outside Node 或另一个 ComboBox：它保护的是真实 TextField 接管后续输入。复用共享 enqueue/dispatch 能力，在适用位置安排同帧输入，分别检查 focus、文本、隐藏列表状态和事件，不依赖 helper 自动额外推进一帧掩盖时序。选择两种有意义的 plugin 安装顺序即可，不穷举所有 Widget 插件排列。

### 全仓结果核对

public_api.rs 维护 facade Coverage Map：编译/构造入口、运行时消费、字体与插件组合由本文件负责；focus.rs 负责跨控件输入归属。前序每份控件的状态模型只记录自己的 contract，本份不汇总为一个巨大状态机。

完成全部方案后执行 cargo fmt --all -- --check、cargo check --workspace、cargo clippy --workspace --all-targets、cargo build --workspace、cargo test --workspace；单独记录 cargo check -p widget_gallery 的展示应用编译结果或明确其已被 workspace check 覆盖。必要的 package focused tests 与 property tests 先在各方案留下证据，全仓验证用于捕获共享设施、dev-dependency、并行测试隔离和 facade 组合影响。Windows 上使用 pwsh，依照仓库规则完成独立 code review 和进度记录清理。

核对原有测试没有因迁移、改名、去重而丢失关键语义；只有确属同一边界且较强版本仍保留时才合并，不为降低测试数量删掉局部与真实组合各自有效的证据。Property tests 的失败样本/seed 按所用库机制保存，排查 nondeterminism，不能用重复运行直到偶然通过替代修复。

本份不启动全控件 BRP 巡检。前序存在经授权的 GUI 修复时，只收齐对应任务的代表性场景证据和 temporal/OS-level 限制；只有新增或改变可观察 GUI 才按现行规则决定实际运行验证。未运行的原生最大化、真实 clipboard/IME、不可观察到的单帧间隙要如实保留为证据边界，不把 cargo test 通过写成这些项目也通过。

### 交付范围

最终提交包含实际测试、测试模块注释、必要的共享 helper，以及 ListModel 首次使用 proptest 所需的 dev-dependency/lockfile。rules 已更新，不再改写；没有 workspace 结构或文档明确记录的测试依赖关系变化时，不修改 docs/architecture.md。Gallery 只保留既有展示职责，不因为收尾新建自动化测试或测试专用生产 marker。验证记录中区分通过、失败、未执行和不适用，不能以一份覆盖地图替代实际断言。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry
```

### 在执行链中的位置

本份位于唯一执行链末尾，承接所有 Widget 与共享设施的实际结果。它只补 facade/跨控件边界并做 workspace 级验收，不重新实现前序模块测试，不把先前未验证的原生能力默认为已经覆盖。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/bevy_widgetry/tests/public_api.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/bevy_widgetry/tests/public_api.rs)
- [crates/bevy_widgetry/tests/focus.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/bevy_widgetry/tests/focus.rs)
- [crates/test_utils/src/scene.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/test_utils/src/scene.rs)
- [rules/testing.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/testing.md)
- [rules/gui-debugging.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/gui-debugging.md)
- [rules/development.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/development.md)
- [Cargo.toml](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/Cargo.toml)

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
