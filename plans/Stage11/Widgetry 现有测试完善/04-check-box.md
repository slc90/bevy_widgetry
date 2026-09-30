# 04 CheckBox：补三态输入、取消与图标投影的状态转换

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](03-button.md) | [后一份](05-radio-group.md)

## 本份方案

### 目标、范围与预期产出

本方案涵盖二态 WidgetryCheckBox 与 WidgetryTriStateCheckbox，修改 crates/check_box 的现有单元与 integration tests。产出是分清两种控件责任的覆盖模型、三态自有交互的确定性回归，以及 state/a11y/mark 一致性断言。不把 Bevy 二态 Checkbox 重写或扩成新的选中策略。

### 已有测试应保留

checkbox.rs 已有三态程序化循环、完整 click 循环、disabled 与静默程序化操作、ActivateOnPress 一次通知、用户 children、mark entity 稳定、主题及二态 style 变化。binary_uses_official_checkbox 已经验证一次真实 click 更新 Checked。style.rs 的 mark_svg_switches_without_replacing_entity 检查内部 mark cache/identity，不能把它当成 SVG 像素已经生成的证明。

源码 tri_state.rs 还自行实现了 FocusedInput 的 Space/Enter 分支、repeat/Released 拒绝以及 Cancel/DragEnd 清除 Pressed；现有本模块 integration tests 对这些路径没有同等直接的场景。这里应优先补，而不是继续堆二态 click 用例。

### 局部合同

为 next_check_state 的三个输入补清楚的确定性表，验证 Unchecked→Checked→Indeterminate→Unchecked。现有样式优先级测试加强 foreground、border、mark 色的完整输出，不仅背景。非法内部 hierarchy 的诊断测试保持在可访问私有 marker 的同模块测试中，不为它扩大 public visibility。

### 三态输入与程序化序列

通过 shared keyboard dispatch 或明确的 FocusedInput 入口，覆盖 Space/Enter 首次按下产生一次状态变化，Released、repeat 和无关键不产生变化；disabled 下这些用户输入仍静默，重新启用后恢复。断言 ValueChange 的 source、value、is_final 和增量次数。不要直接改 WidgetryCheckState 后再人为发 ValueChange 来代替该路径。

Press→Cancel、Press→DragEnd、Press→Release 各检查 Pressed 清除且不额外循环。ActivateOnPress 下重复 Press 不能重复提交，随后 Click 不再增加通知。不得从“disabled 不能改值”推导所有控件的 focus 一律不能变化；本方案只约束 CheckBox 已有的交互合同。

程序化测试补同一 flush 前连续三个 cycle，以及 set_state 后 cycle 的队列语义，确认动作读取执行时状态，而不是都使用排队前旧值。已有 facade 只覆盖了一个 set+cycle 例子，保留但不拿它代替本模块完整循环。对已销毁 root、只有 state 却不是三态控件的 entity，验证 no-op 与没有用户通知；不生成不存在的非法 enum 值。

### 关联输出与 icon

定义适用范围小的 assert_check_state_projection：在系统合同完成后检查三态 state 对应的 a11y toggled、mark visibility 和实际图标表现。三种 toggled 都要有直接断言，不能仅 Unchecked→Mixed。程序化、用户输入、disabled 恢复各选会破坏该关联的转换调用，场景特定事件期望单独断言。

对 Checked→Indeterminate→Unchecked→Checked，预加载内建 mark 资源后运行真实 Materialize/Propagate，检查已显示 ImageNode/颜色、mark identity 和隐藏/重新出现；不是只检查私有 cache 里保存的路径。二态也保留 Checked 增删的 mark 可见性回归。通用 SVG 等待、尺寸、cache 语义归 Icon，不在本模块再遍历所有图标。

### 组织、验收与 BRP

tests/checkbox.rs 开头用 //! 标明 binary adapter、tri-state behavior、projection/style 的责任；只有文件确实难读时才拆成相应文件并增加主 Map。不新增 proptest。运行 bevy_widgetry_check_box 和 facade 相关测试。

条件 BRP 以三态连续切换为一条场景，同时观察状态、mark 图形和颜色是否同步；需要修 keyboard bug 才加 Space/Enter 路径。没有界面行为修改时，不要求重测所有 CheckBox 示例。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_check_box
cargo test -p bevy_widgetry
```

### 在执行链中的位置

使用共享基础和 Icon 已建立的验证手段。与前一个 Button 阶段主要是执行顺序关系；下一份 RadioGroup 处理自己的互斥选择合同，不复用三态循环语义。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/check_box/src/tri_state.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/check_box/src/tri_state.rs)
- [crates/check_box/src/style.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/check_box/src/style.rs)
- [crates/check_box/tests/checkbox.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/check_box/tests/checkbox.rs)
- [crates/bevy_widgetry/tests/public_api.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/bevy_widgetry/tests/public_api.rs)

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
