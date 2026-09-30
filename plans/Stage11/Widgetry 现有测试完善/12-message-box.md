# 12 MessageBox：补结果重入、关闭顺序与真实内容投影

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](11-window.md) | [后一份](13-facade-and-final-validation.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/message_box/tests/message_box.rs、src/lifecycle.rs 和 src/scene.rs 的测试。产出是公开结果 contract、结果 callback 与关闭之间的边界测试，以及实际正文/标题输出的断言。Dialog 仍是固定尺寸、non-blocking parent-window pointer modal；不新增关闭返回 Cancel、自动 resize 或 keyboard focus trap。

### 已有覆盖

real_button_clicks_return_all_results_and_release_last_blocker 已通过真实 Button press/click 验证 OK、Yes、No、Cancel 的结果、通知 root、两个 dialog 共享 parent blocker 的最后释放，以及 owned window/camera 回收。native_close_has_no_result 明确原生关闭不返回 Cancel。plugin 依赖装配、disabled action 和主题也有测试。

scene.rs 已测三种结果按钮组合的固定顺序、正文普通 Button 不携带私有 action、parent relationship 与不可 resize。lifecycle.rs 的 result_precedes_closing_and_cleans_owned_resources 已测结果 observer 期间 root/state 可读、Closing 尚未开始、同帧重复同一个 Activate 只决议一次，以及正文普通按钮被忽略。这些不是空白覆盖，不重新写一个仅更换名字的版本。

### 一次性结果与 observer 生命周期

现有重复同一按钮案例不等于 callback 重入。新增一个通过公开 WidgetryMessageBoxResultEvent observer 排队激活另一个结果按钮的场景：只发布第一次有效结果，source 始终是 dialog UI root，后续动作不能覆盖它或再发布。另一个代表场景将两个不同结果动作按已知顺序排队，核对第一个被接受的结果；不要把独立 observer 的未定义执行次序硬编码为产品合同。

根据公开注释“observer command 应用后关闭”，在 result observer 中排队一个读取 dialog root/正文并记录结果的 command。它应能在声明的生命周期边界内看到该 root，最终 owned resources 正常清理。单独保留同步 observer 时 root 存在的已有断言，不能把同步可读误当作 command 阶段也可读的证明。

补 callback 自行销毁 dialog root 或 parent 的清理代表：不能重复发布结果，不能因后续关闭再次处理而误删其他 dialog，相关 blocker 按最后有效 child 收敛。这些测试只能保护源码已经声明的 callback/cleanup 合同；若更复杂的 callback ownership 语义没有定义，记录缺口，不临时为测试设计新业务规则。

### 拒绝、恢复和无结果结束

disabled_action_does_not_resolve 当前主要断言 root 还活着。给它安装结果记录，确认无事件、无提前关闭，再解除 disabled 并通过真实结果按钮输入确认仅发布一次。不能以“没有销毁”代替“没有错误发布结果”。正文普通按钮无决议的既有案例继续保留。

parent 无效或 parent lifecycle 结束导致 dialog 清理时，补 MessageBox 自己的结果流断言：没有产生任何结果，尤其不是合成 Cancel。具体资源与 blocker 清理复用 Window 已保护的 contract，MessageBox 只验证组合后没有多出业务通知。

### 从传播配置推进到实际 UI 输出

message_box_text_tracks_theme 目前检查 root 上的 Propagate<ForegroundColor>。增强为真实标题和至少一层嵌套正文的 TextColor；使用准备好的字体和真实 UI 消费阶段，检查主题转换后文本没有保留旧色。正文中有显式独立样式时按其已有公开语义处理，不强行让所有内容一律覆盖。

选一个有较长正文、固定结果按钮行的真实 layout 场景，核对固定 dialog 尺寸下正文受约束、结果区不被内容挤没、首次有效内容帧具备测量与可见性。这里只测试当前固定布局的组合，不增加自动调整 dialog 大小或全套排版压力用例。需要 text+icon 内容时复用 Icon 的真实 image 检查，不能只看 WidgetryIcon 身份，也不把普通纯文本 dialog 强行加 icon 作为新产品要求。

### 文件组织和验收

message_box.rs 为主 Coverage Map，区分构造/输入到决议/observer 与关闭/主题内容。需要拆分时才增加 lifecycle.rs 或 rendering.rs；private action 构造和 Closing marker 的局部断言继续放同模块。公开状态模型写未决议、已发出结果、已关闭，不把 resolved 私有 bool 当成必须公开的业务接口。

运行 bevy_widgetry_message_box 与 facade；改到 Window 共用 helper 时一并验证其消费者。条件 BRP 仅验证这次实际 GUI 修复对应的一条打开→读取正文→结果按钮→parent 恢复路径，观察标题/正文/icon 的中间状态。没有原生窗口的测试结果不能证明跨窗口 OS focus 或原生 modal。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_message_box
cargo test -p bevy_widgetry
```

### 在执行链中的位置

在 Window 的资源与 modal 测试完善后实施；只补 MessageBox 增加的结果/关闭语义。下一份 facade 验证消费者通过统一入口能够组合这些能力，不再重复本份全部按钮与生命周期矩阵。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/message_box/tests/message_box.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/message_box/tests/message_box.rs)
- [crates/message_box/src/lifecycle.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/message_box/src/lifecycle.rs)
- [crates/message_box/src/scene.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/message_box/src/scene.rs)
- [crates/window/src/modal.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/src/modal.rs)

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
