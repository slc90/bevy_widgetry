# 10 ComboBox：保留已有交互回归，补内容首帧与恢复路径

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](09-list-view.md) | [后一份](11-window.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/combo_box/tests/data_contract.rs、selection_field.rs、popup_composition.rs、bsn_combo_box.rs 及对应源码合同。产出是明确各文件 owner 的覆盖地图、加强后的时序断言和必要的组合场景。ComboBox 不拥有另一套 model/id/renderer，不能在本份建立第二个 ListModel property suite。

### 已有覆盖和必须保留的语义

现有 data_contract 已保护非 Clone/Default 的业务 type、独立 model、共享 source、排队选择与 model move、非法构造和 source 诊断。selection_field 已验证 selection authority、identity/index/revision 驱动的内容更新、无关 revision 不重建、旧 subtree 清理、程序化静默、同值和无效 id、真实 UI 中新 Text 当帧的 glyph/layout/visibility/stack。

popup_composition 已覆盖真实 row click、重选不重复通知、Escape、outside click 不抢 focus、两个 ComboBox 的切换、disabled item/root、嵌套 ListView 归属、10k model 的 bounded viewport，以及多条同帧关闭回归。closed_popup_cannot_select_hidden_items_from_keyboard、closing_popup_stops_remaining_keyboard_inputs_in_same_frame、keyboard_reselection_closes_popup_without_notification、pointer_close_stops_keyboard_selection_in_same_frame 必须保留，不把它们列为新需求重新实现。

非空 model 的初始默认选择只做一次；第一次为空、随后 push，不会自动补第一项。内部 ListView.selected 是唯一 selection authority。程序化 set_selected 使用 source 内的 stable id，允许 disabled 状态并保持静默，本身不关闭 popup。root 新增 disabled、model 清空等关闭条件是另外的路径。不同 source 可以分配相同 id 数值，不能要求跨 source provenance 校验。

### 加强现有测试，而不是增加重复案例

empty_and_deleted_selection_preserve_field_shell 在删除 selected item 后连续调用了两次 app.update 才断言。把属于 state repair → Field projection 合同的断言放到第一轮规定的消费阶段：selection 已清空、旧 wrapper/text 已销毁、content 已空，Field/Button/箭头仍完整。第二轮可以检查稳定性，但不能遮住第一轮旧内容残留。若第一轮失败，按源码调度合同核对原因，不能直接把等待恢复为两帧来通过。

保留 programmatic_selection_prepares_field_text_in_same_frame 的真实 UI 和提前消费阶段约束；它已经是有效时序测试，不替换为简单 Text 字符串判断。现有测试中人为把 disabled popup 设为 visible 是为了隔离“程序化选择不关闭”的局部合同，不能把这种构造直接复制成真实用户 workflow。

### text + icon 与真实布局

arbitrary_renderer_builds_independent_field_and_row_subtrees 已验证 Field 与 row 的嵌套 Text/Icon 是独立 subtree，但主要检查 WidgetryIcon 身份。增强到资产已就绪后的真实 ImageNode 和 UI 消费结果：selected revision 重建后，Field 和已实例化 row 各自有正确内容，旧子树清理，图像有效且可见；主题或 foreground 改变后颜色与文本一致，不等下一次选择才恢复。通用 SVG 替换正确性由 Icon 方案负责，ComboBox 只保护动态 composition。

现有 popup geometry fixture 会手填 ComputedNode，适合测试高度派生与行为消费。另用公开 ComboBox Scene 加真实 UI/camera，选一个超过 max_visible_items 的 model，验证 popup wrapper、内部 ListView viewport 与实际可见 rows 的有界关系确由布局产生，且开关不销毁持久 ListView authority。对初始布局准备和打开后的有效消费阶段分别断言，不把可配置高度 Node 值等同于最终 viewport 几何，也不重复 Bevy Popover 的全部 placement 算法。

### 必要的恢复路径

在已有 root disabled 自动关闭的测试上补恢复：打开后禁用，field/list 都禁用且 popup 关闭；移除 root disabled 后，通过真实 Field 输入再次打开，focus 进入内部列表；model 中原本 disabled 的 item 仍 disabled，普通 item 可以选择且只通知一次。现有测试已覆盖禁用方向，不再为每个按钮/状态分别复制一套；本场景保护的是“解除禁用后还能用”。

无效 id、同值选择、直接改 authority 与孤立 ValueChange 的差异已有测试，主要核对断言完整和归属。正常交互测试必须走真实 Button/ListView 输入桥接；直接 trigger ValueChange 只保留在明确验证事件桥接或不以事件为 authority 的场景，不能把它当作实际选择成功的证据。

### 文件组织和验收

以 data_contract.rs 为主 Coverage Map，selection_field 负责选择到 Field projection，popup_composition 负责 popup/focus/真实 ListView 组合，bsn_combo_box 保留其构造范围。各文件开头给适用的状态维度、stimuli、guards、invariants、couplings，不再维护另一本完整转换表。

运行 bevy_widgetry_combo_box 和 facade。条件 BRP 选择一次真实打开、改选 text+icon 项并关闭的 workflow，或本份实际修复的禁用恢复场景；关注箭头、正文和 row 是否出现短暂空白/旧内容/迟滞。没有实际 GUI 行为变化的测试整理不强制重跑全部 popup 历史场景。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_combo_box
cargo test -p bevy_widgetry
```

### 在执行链中的位置

在 ListView 后进行，直接依赖其 selection/active、virtualization 与 model 契约；Icon 的公共行为由 02 保护。本份之后进入 Window，只是唯一执行链的安排，不存在 ComboBox 对 Window 的新增技术依赖。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/combo_box/tests/data_contract.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/tests/data_contract.rs)
- [crates/combo_box/tests/selection_field.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/tests/selection_field.rs)
- [crates/combo_box/tests/popup_composition.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/tests/popup_composition.rs)
- [crates/combo_box/tests/bsn_combo_box.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/tests/bsn_combo_box.rs)
- [crates/combo_box/src/field.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/src/field.rs)
- [crates/combo_box/src/popup.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/src/popup.rs)
- [crates/combo_box/src/combo_box.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/combo_box/src/combo_box.rs)

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
