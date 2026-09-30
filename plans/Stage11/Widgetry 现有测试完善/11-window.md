# 11 Window：补 resize、modal 隔离与原生能力的验证边界

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](10-combo-box.md) | [后一份](12-message-box.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/window/tests/window.rs、owned_window.rs 以及 src/modal.rs、title_bar.rs、title_bar/resize.rs、title_bar/maximize.rs 中的相关测试边界。产出是 public lifecycle 与局部 native-request 语义的覆盖，外加明确记录的原生窗口验证限制。不为了测试引入 OS mock 层，不改变 Window 的 ownership、modal 或 maximize 设计。

### 已有覆盖

window.rs 已验证 native 属性归一化、独立内容 slot、两个 window 各自的 UI/camera 绑定、重复 window/camera 拒绝、同帧及跨帧重复绑定、非法属性和实体的完整 tree 清理、camera target/viewport/clear color 配置以及 CameraUpdateSystems 前生效。owned_window.rs 已覆盖 root 先销毁和 native window 先结束两条资源释放路径。保留这些案例，不重新生成一份所有参数组合矩阵。

modal.rs 的局部测试已经验证后加 modal relationship、child/root/native parent 结束后的立即清理、stray modal entity 不计数、两个 child 共享一个 blocker、最后引用退出后解除 blocker、无效 parent 清理 owned child。title_bar.rs 已测控制按钮可见配置与 native enabled_buttons、禁用同步前拒绝部分请求、resize handles 的启用状态，以及四种内建图标经真实 asset pipeline materialize。

### resize 的局部 contract

resize_handle_node 与 resize_cursor 是具有独立语义的有限映射，在 resize.rs 内补八个 CompassOctant 的 table-driven 单元测试：每个方向对应正确边/角几何与 cursor，边区避开角区，不占主体 layout。这里检查 Widgetry 自己的映射，不测试操作系统是否真的执行 resize，也不需要 proptest。

增强 resize_press_respects_native_resizable_and_window_binding。现有案例选择第一个 handle，主要断言 take_resize_request 是否有值；新增断言要精确到目标 window 与方向。用两个 root/window，操作 A 的方向 handle 只能写 A 的对应请求，B 不变；非 primary press 不产生请求；禁用时拒绝，恢复后才接受。不能因为某个 request 为 Some 就认为绑定正确。

围绕当前 cursor/Resizing 合同补有意义的序列：Over 显示该方向 cursor，Press 后 Out 不提前清除，左键释放解除 resizing 后 Out 恢复默认；已有 hover 或 resize 时，native resizable 变为 false，hit area、cursor 与 resizing state 应共同退回合法状态。私有 marker 可在同模块检查，不把它们公开成调试 API。正常/拒绝/恢复路径即可，不把八方向与所有状态做笛卡尔积。

### modal 和 owned 资源的公开组合

在 tests/ 中增加以公开 WidgetryModalWindow 与 owned_widgetry_window/widgetry_window 构造的代表性协作。parent 指 native Window entity，不是 UI root。两个 parent 中，仅 A 有 modal child 时，blocker 只附着 A；A 的 child 数量经历 0→1→2→1→0，唯一 blocker 的创建、保留、销毁和 B 的完整性要被观察。通过公开 hierarchy、Pickable 和 z-index 等可观察输出定位，不依赖私有 ModalState。

已有局部测试已完整覆盖单 parent 的计数算法，因此公开组合不逐条复制，只保护公共构造入口、跨窗口归属与真实 tree。modal 目前仅遮挡 parent 的 pointer，不建立 OS modal relationship，也不接管 keyboard focus；不得新增“阻止父窗口一切键盘输入”的断言。

owned cleanup 增加重复生命周期信号、排队清理与另一个仍存活 root 并存的情况，检查属于被关闭 root 的资源消失、借用和其他 root 的 window/camera/content 保留。记录具体 entity 集合，不只比较全局数量相同。源 root 已销毁后的合法重复信号不得使另一个资源被回收；涉及超出当前支持的外部资源重新绑定需求，应停在合同缺口，不顺手扩接口。

### 必须如实保留的原生边界

on_maximize_restore 与 sync_maximize_state 读取 WINIT_WINDOWS 中的真实 window。没有原生窗口时会提前返回。因此 headless 测试中没有 maximize request，不能单独证明 enabled_buttons.maximize 的 guard 有效；当前图标测试额外创建 WindowRestore icon，也不能证明真实最大化后 icon/radius 同步。

本份将这部分明确列为原生运行时待验证，而不是通过注入私有 maximized state 冒充 OS 确认。允许同模块测试已有 private state 对下游 resize guard 的影响，但必须标明只覆盖下游逻辑。真实 maximize/restore、边角、标题栏和内容圆角、对应 icon、原生移动/resize，需要本地真实窗口；BRP 能忠实触发并观察的部分按规则执行，其余人工验证并记录限制。不得为关闭这一缺口改造成另一套 native adapter 或桌面自动化工具。

### 文件组织与验收

window.rs 维护绑定/资源/controls/modal 的 Coverage Map；owned_window.rs 继续承接 owned 生命周期。public 可表达的组合进入 tests/，私有 mapping、调度依赖和 handler 局部边界留在源码测试模块，不能按“用了 App”一刀切迁移。

运行 bevy_widgetry_window、bevy_widgetry_message_box 和 facade。只补测试时无需为了形式启动原生窗口；无法通过 headless 建立证据的原生场景仍应明确列为未执行/需人工，不能标记为通过。如果另行授权修复 native GUI，验收要包含本次受影响的实际动作与 icon/圆角过渡，不重复全部 Window 功能。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_window
cargo test -p bevy_widgetry_message_box
cargo test -p bevy_widgetry
```

### 在执行链中的位置

前一份 ComboBox 与本份没有新增依赖，先后只是执行链安排。Window 的 owned lifecycle 和 parent pointer blocker 是下一份 MessageBox 的实际组合基础，MessageBox 不再复制全部 native window 配置测试。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/window/tests/window.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/tests/window.rs)
- [crates/window/tests/owned_window.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/tests/owned_window.rs)
- [crates/window/src/modal.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/src/modal.rs)
- [crates/window/src/title_bar.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/src/title_bar.rs)
- [crates/window/src/title_bar/resize.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/src/title_bar/resize.rs)
- [crates/window/src/title_bar/maximize.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/window/src/title_bar/maximize.rs)

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
