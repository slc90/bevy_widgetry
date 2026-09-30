# 08 ScrollArea：补公开构造下的真实布局收敛与滚动恢复

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](07-tooltip.md) | [后一份](09-list-view.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/scroll_area/src/headless.rs、layout.rs、style.rs 的现有测试及 tests/bsn_scroll_area.rs。产出是 public Scene 与真实 UI layout 的回归，以及清楚区分算法输入测试和实际布局测试的 Coverage Map。不新增 ScrollArea root disabled、动态 axis setter 或其他源码未承诺的 API。

### 已有覆盖

headless.rs 已测 keyboard_position、align_if_outside、嵌套 viewport 和 focused keyboard。layout.rs 已有 strict overflow、Auto/Always/Hidden、gutter 相互诱发、稳定后重新求解的纯状态测试；real_layout_converges_across_grid_gutter_passes 还跑了真实 UI，但使用手工组装的内部 shell。style.rs 已检查 BSN 配置、thumb 的 hover/drag、主题及内容约束。公开 tests/bsn_scroll_area.rs 则主要覆盖 Scene 与 keyboard_scroll 开关。

这些测试各有价值，不能因“用了手填 ComputedNode”统一否定；手填几何适合局部算法/消费测试，但不能证明公共 Scene 经过 layout 自己产生正确几何。

### 单元测试保留与增强

继续在同模块测试 keyboard_position、align_if_outside 和 advance 等具有独立局部 contract 的逻辑。keyboard 分清各 axis 支持键、边界 clamp 与无关键；scroll-into-view 当前是部分不可见时 top-align 再 clamp，不把它改为另一种最近边缘滚动策略。

对已有 solver 断言补“稳定后一轮没有新 RequestRedraw”，使用新消息区间而非旧消息是否仍存在。内部 convergence state 可用于该局部调度合同，但不加入公开业务状态模型。本轮不要求引入 property 库；若后续数值输入确实需要大空间验证，再另在实际使用时引入，而非为本方案机械套 proptest。

### 公开 Scene 的真实协作

以 WidgetryScrollArea 的公开 BSN 构造真实 viewport/content，使用 shared UI/camera 环境。在 tests/ 中补或迁移一个真实 layout 场景，验证“内容先触发一条 Auto scrollbar，gutter 又触发另一条”的结果来自布局计算，而不是测试先写入预期 ComputedNode。计算边界基于实际 viewport/client extent，不把内部手工 shell 的尺寸常数直接套到带 border 的公开 Scene。

同一控件继续经历内容缩小、恰好贴合、变空、重新增大以及已有滚动位置时可用尺寸改变。观察 scrollbar 展示、reserved gutter、ScrollPosition 合法范围和稳定后的实体关系。需要多轮 layout 的 Auto 收敛按源码算法与测试场景设有依据的上界，不随意 sleep，也不要求一帧结束。窗口 resize 这里只改变测试中 UI 的可用尺寸，不声称覆盖了原生 OS resize。

公开 WidgetryScrollIntoView 场景覆盖已完全可见不动、部分不可见的 top-align、内容分支之外的目标不影响本 viewport、嵌套控件只由最近所属 viewport 处理，以及非 1 scale factor 下的逻辑坐标。已有手工几何测试继续保护数值分支，公开组合只选择必要代表。

keyboard_scroll=false 的场景加强到同一真实 Scene：键盘不被该 ScrollArea 消费，但 wheel 与程序化 scroll-into-view 仍按合同工作。这里不把 keyboard_scroll 解释为整个控件 disabled。thumb style 增加 drag 结束回 hover、hover 离开回 normal，检查颜色恢复，而非只进入拖动。

### 文件组织和验收

bsn_scroll_area.rs 为主 Map；如需要拆分，拟新增 layout.rs、interaction.rs 或 style.rs，只把可通过 public Scene 验证的行为迁出。依赖私有配置和 solver state 的单元测试仍原地保留。通用重复 UI 装配改用共享设施，不复制另一套 plugin 列表。

运行 bevy_widgetry_scroll_area、bevy_widgetry_list_view、bevy_widgetry_combo_box。条件 BRP 使用本轮受影响的一条滚动/内容缩小场景，查看 thumb、内容和 gutter 的过渡；合法 layout 收敛不能被“等稳定再看”的截图掩盖，也不能误称所有合法多轮计算都是闪烁 bug。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_scroll_area
cargo test -p bevy_widgetry_list_view
cargo test -p bevy_widgetry_combo_box
```

### 在执行链中的位置

在 Tooltip 后实施只是链中顺序。ScrollArea 是下一份 ListView 的实际生产组合基础，本份完成后 ListView 可复用其真实布局与滚动测试环境，而不是再次验证全部 scrollbar 策略。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/scroll_area/src/headless.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/scroll_area/src/headless.rs)
- [crates/scroll_area/src/layout.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/scroll_area/src/layout.rs)
- [crates/scroll_area/src/style.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/scroll_area/src/style.rs)
- [crates/scroll_area/tests/bsn_scroll_area.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/scroll_area/tests/bsn_scroll_area.rs)

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
