# 06 TextField：从输入过滤补到真实编辑消费与恢复

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](05-radio-group.md) | [后一份](07-tooltip.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/text_field 的 src/style.rs 局部测试和 tests/widgetry_text_field.rs、disabled.rs、read_only.rs。产出是三个文件之间明确的 Coverage Map，以及少量连接官方编辑消费阶段的回归。普通/只读是不同公开构造类型，不设计运行期只读模式切换；不增加数字输入、parse/format、SpinBox 或输入验证功能。

### 已有覆盖及证明边界

disabled.rs 已检查清理 pending_edits/pending_paste，并在 EditableTextSystems 设置观察点；read_only.rs 已枚举 Cut/Paste/Insert/Backspace/Delete/IME 等修改操作的过滤，以及保留 navigation/copy/selection、普通编辑器不受影响、focus、程序化修改。无需再次把这份完整过滤表包装成“缺失用例”。

这些测试有不少只证明 queue 被正确保留或清除，没有实际走完文本修改消费。style 测试已经覆盖 focused/hover/disabled 优先级和只读外观；涉及“主题变化不改变内容”的场景需要放入非空内容并真的断言内容，而不是仅凭样式正确推断。

### 集成增量

建立一个使用现有 text_input_app，并按需要补真实 UI/文本消费插件的最小 fixture。用已有公开 EditableText 接口设置内容；检查普通文本框的一个输入确实被消费到文本值，而不是只出现在 pending queue。相同输入到只读文本框时原文不变，但一个允许的 selection/navigation 操作实际改变选区或光标。不要为此复刻官方 EditableText 的整套编辑算法。

禁用恢复用同一个实体形成序列：正常可编辑→在有 pending edit/paste 时禁用→官方编辑阶段不修改文本且队列已清理→重新启用→旧操作不重放，新操作可以执行。程序化 editor 更新继续允许。普通 EditableText 作为对照证明 Widgetry 的过滤范围只覆盖自己的控件。

只读保留现有 copy/selection 过滤断言。真实导航/选区用 headless 消费结果验证；系统剪贴板或跨应用复制属于环境边界，不把开发机器剪贴板的当前内容当成普通测试前提，也不因测试需要引入 mockall。IME 修改过滤继续保留已有确定性枚举，不声称这已完成所有平台原生输入法验收。

加强主题测试：设置非空文本及有效选区/光标，在主题或 interaction 状态变化后检查文本、选区/光标和 entity identity 保留，同时检查 TextColor、边框及选区颜色按当前合同更新。观察位置以官方编辑/样式阶段完成后为准，避免比较还未消费的暂存值。

### 单元与文件责任

style resolver 可用确定性 case 检查完整颜色输出；现有过滤逻辑的语义分支不需要额外抽成生产 API 来写测试。没有当前独立数值算法，本阶段不引入 proptest。

widgetry_text_field.rs 放主 Map，负责 Scene、布局 patch、样式和字体策略；disabled.rs 负责全部用户编辑被阻止以及恢复；read_only.rs 负责修改/非修改操作的分界及对应 focus。模型只写 editing、readonly 类型、enabled、focus、文本/选区等可观察状态，不把 pending queue 长度当成整个控件状态机；queue 断言保留为特定过滤阶段的补充证据。

### 验收与 BRP

运行 bevy_widgetry_text_field 和 facade 的跨控件 focus 测试。fixture 不应使测试因为缺少官方编辑消费插件而空跑通过。条件 BRP 场景是实际点击输入、切到只读框选择文本，再观察主题或禁用变化后文字/光标是否短暂消失；不重新测试整套文本编辑快捷键。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_text_field
cargo test -p bevy_widgetry
```

### 在执行链中的位置

沿用前面已经明确的输入推进规则，不依赖 RadioGroup 的业务实现。下一份 Tooltip 使用可控时间，不能把文本输入延迟与 Tooltip 有意设置的 show delay 混为一类问题。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/text_field/src/style.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/text_field/src/style.rs)
- [crates/text_field/tests/widgetry_text_field.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/text_field/tests/widgetry_text_field.rs)
- [crates/text_field/tests/disabled.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/text_field/tests/disabled.rs)
- [crates/text_field/tests/read_only.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/text_field/tests/read_only.rs)
- [crates/test_utils/src/scene.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/test_utils/src/scene.rs)
- [crates/bevy_widgetry/tests/focus.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/bevy_widgetry/tests/focus.rs)

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
