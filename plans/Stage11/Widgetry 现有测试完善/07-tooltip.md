# 07 Tooltip：连接悬停计时与公开 popup 生命周期

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](06-text-field.md) | [后一份](08-scroll-area.md)

## 本份方案

### 目标、范围与预期产出

当前 tooltip crate 没有 tests/ 目录，测试集中在 src/headless.rs 和 src/style.rs。这里不是“没有测试”，而是 headless 的 Show/Hide 事件与 styled popup 多由分离测试验证。产出是保留精确局部计时测试，再用公开 WidgetryTooltip 建立 integration test 的完整行为入口；不公开 TooltipPopup、ShowTooltip 或内部计时 state。

### 现有覆盖

headless.rs 已验证 cold 200ms、warm 50ms、cooldown 300ms、pointer 移动重置、同 anchor descendant 切换保持显示、warm delay 在 cooldown 尾部锁定、Virtual time 暂停不影响 Real time、等待时 RequestRedraw 与显示后停止。style.rs 已验证必填 factory、popup 壳层、Pickable、theme、重复 show/hide 重建和 anchor despawn。以上全部保留，尤其不能用真实 sleep 代替精确 199ms+1ms、49ms+1ms 边界。

### 状态模型与局部补充

公开模型是无候选、等待显示、显示、离开后的 warm 窗口；另有 anchor identity、内容 factory 和前景色。固定 delay 是产品行为，不属于要消除的“卡顿”。局部测试继续精确检查时间，补候选尚未显示便销毁的清理；HoverMap 有多个深度命中时只取最前方 target，再找最近 Tooltip ancestor。前方普通 target 没有 Tooltip ancestor 时，不应穿透去取更深层的 Tooltip。

补非 mouse pointer movement 不重置 mouse candidate，以及同一 update 中 mouse 发生移动的已有重置规则保持。重复内部 Show 只能有一个 popup/factory 调用的 guard 可留在 style.rs 的局部测试，不把内部事件当公开 API。

### 公开 integration tests

拟新增 tests/tooltip.rs，作为主 Coverage Map；只通过 WidgetryTooltip Scene、TooltipContentFactory、受控 HoverMap/PointerInput、手动 Real time 推进触发。不要直接 trigger 私有 Show/Hide。使用 public hierarchy、Popover 和测试内容 marker 找到 popup，不暴露生产私有标记。

代表性 cold 场景：hover 调用方 label descendant，门限前无新 popup，达到门限后出现一次 factory 内容；pointer 留在同 anchor 内容内不重建。离开后 popup 及嵌套内容销毁，另一个 anchor 在 warm 条件下显示，任何观察点最多存在一个实际可见的 Tooltip。完整数值边界继续由局部测试负责，不在 integration 中重复所有计时排列。

使用有变化的 factory 内容验证重新显示确实调用 factory 并生成新子树，而非沿用过期内容；候选和可见两种状态下分别销毁 anchor，之后推进时间不产生孤儿 popup。祖先是 disabled 控件时仍按现有 Tooltip 合同显示，不套用其他控件“disabled 不响应”的规则。

真实 UI 场景检查 popup 和所有新 descendant 在 picking 消费前为 IGNORE；显示中的 popup 再追加嵌套内容也检查，不只最初的 children。预加载字体/icon 后，popup 创建当帧应完成适用的 visibility、stack、文字/图像准备，并保留 OverrideClip 与 Tooltip layer。窗口边缘 placement 验证 Widgetry 选择的 Popover 参数和一条组合效果即可，不复制官方 Popover 全部几何算法。

### 整理方式与验收

可从 style.rs 迁移那些完全能用公开 Scene 表达的外观/生命周期测试到 tests/；需要私有标记或日志恢复状态的局部测试保留。迁移保留原断言和场景，不因为已有新的端到端场景删除精确 timing 回归。无需 proptest。运行 bevy_widgetry_tooltip 与 facade。

条件 BRP 用相邻两个 anchor 悬停、离开、返回，关注 popup 出现位置、文字/icon 首次显示和短暂闪烁；尊重 200/50/300ms 的有意延迟，截图请求时间不是可靠的精确计时器。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_tooltip
cargo test -p bevy_widgetry
```

### 在执行链中的位置

复用共享时间/输入设施及 Icon 的图像准备证明。与前一份 TextField 是顺序关系；下一份 ScrollArea 处理真实 layout 的合法多轮收敛，不能直接套用 Tooltip 固定时钟。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/tooltip/src/headless.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/tooltip/src/headless.rs)
- [crates/tooltip/src/style.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/tooltip/src/style.rs)
- [crates/tooltip/src/lib.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/tooltip/src/lib.rs)
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
