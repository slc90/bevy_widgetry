# 09 ListView：补强状态转换、跨实例隔离与 ListModel 性质测试

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](08-scroll-area.md) | [后一份](10-combo-box.md)

## 本份方案

### 目标、范围与预期产出

范围为 crates/list_view 的 model、behavior、virtualization 局部测试，以及 tests/contract.rs、behavior.rs、virtualization.rs、style.rs。产出是按真实 contract 组织的 Coverage Map、少量缺失的协作场景、强断言的 ListModel property tests。只有本份实际写入 property tests 时，才在 workspace 声明兼容的 proptest 版本，由 bevy_widgetry_list_view 作为 dev-dependency 使用并更新 Cargo.lock；不修改其他依赖版本，不让 test_utils 或生产依赖链承担它。

### 必须保留的当前语义和覆盖

ListModel 的 id 是 model-local stable identity，不同 model 可以有相同数值。ListView 的公开 set_selected 接收 index，而 ComboBox 接收 item id；测试不能混用。方向键、Home、End 只改变 active，Space/Enter 才确认 selection；disabled item 仍可成为 active，但用户不能通过它改变 selection。PageUp/PageDown 只改变滚动位置。root disabled 阻止用户输入和 wheel，程序化选择仍允许。

model.rs 已有 id 唯一且不复用、mutable access 推进局部 revision、disabled metadata 独立、move 保留 identity/revision/disabled、clear 与非法 index 等确定性测试。behavior 的单元测试已覆盖起点/wrap、reveal 数学和删除后 repair。集成测试已有 descendant click、重选、disabled item、取消 press、逻辑状态修复、键盘、wheel、无效 source 配置、Accessibility 和主题样式。

virtualization.rs 已有 10k 列表、index overlap 复用、visible/offscreen revision、resize、shrink、clear，以及真实 UI 中首帧字体、stack、visibility 和非 1 DPI 的回归。保留这些测试，不把它们重新写成一份通用清单。首次 viewport 尚无有效 layout 时允许暂时没有 rows；已有真实 layout 测试在后续有效布局后生成 rows，不能把“所有内容必须在 App 第一次 update 出现”写成新 contract。

### ListModel 的 property tests

在 model.rs 的测试模块内生成有界的 push、insert、remove、move、clear、get_mut、set_disabled 操作序列。操作输入同时安排合法位置、边界、同位置和越界情况；根据当前长度解释合法位置的选择，不能因为固定生成大 index，导致多数操作只在空列表上无效返回。重要的空列表、删除后再插入同值、clear 后新增、move 同位置等场景继续保留命名清楚的 deterministic tests，不依赖随机命中。

性质断言使用测试侧独立维护的预期顺序和历史账本。每个 entry 记录预期 value、id、revision、disabled；另外保存所有曾分配的 id 与已经失效的 id。每一步核对长度、顺序、按 index/id 的查询、返回值、当前 id 唯一性、旧 id 不复活、内容和 metadata 是否仍跟随原 identity。无效操作要检查状态未改变，get_mut 即使没有实际写入也按约定推进 revision，set_disabled 和 move 不推进内容 revision。

不能只调用一个检查 id 唯一、index/id round trip 的 helper。那样 move 完全不执行、clear 后重新复用旧 id，也可能在每一步看起来内部一致。move 必须验证目标最终 index 与其余项的相对顺序；remove/clear 必须核对 retired-id 账本；新 id 必须与历史已分配集合不重复。预期计算表达独立业务规则，不照抄被测函数的 control flow，也不再调用被测算法来生成 expected。

不引入 proptest-state-machine，不建立通用 executable state-machine 框架。测试侧的小型 Op enum 和预期账本只服务 ListModel。生成器限制列表长度、数值与序列长度，避免把性质测试变成压力测试；初始可用每个 property 64 个 case、每个序列至多 128 步，再按实际运行成本调整。失败必须能定位操作位置并保留可复现输入；按所采用 proptest 版本的机制保留失败回归数据，同时把有代表性的产品 bug 提炼为普通 regression test。不得把随机通过解释为穷举证明。

### 局部边界与算法

为 id/revision checked_add 的耗尽分支补小规模同模块测试：直接设置私有计数到边界，检查 ERROR 后终止、没有回绕或返回可复用 identity，不分配海量 item。保留非法 index 不分配 id、不改变内容的既有约定。

visible_range 继续保留当前精确数值案例，并补非有限/负 offset 等实际处理分支。已有 proptest 后可增加一个有界合法输入性质：height 为正且有限、总高度有限，输出 offset 合法、start <= end <= len；同时保留部分可见行与精确行边界的确定性 expected。这个函数的前置条件由构造层保证，不能为了随机输入把零行高等不支持调用变成新功能。

### 需要补强的公开协作

已有局部 repair 测试覆盖中间 active 删除后的 successor，现有集成案例也覆盖结构变化；仍需在完整 plugin 路径中明确检查“active 位于中间时删除它，转到原位置 successor”与“active 位于末尾时删除它，退到 predecessor”的区别。当 selected 与 active 指向不同 item 时，删除其中一个不能把另一个有效 identity 一并清空。检查 repair 完成阶段的状态及公开 Selected/ActiveDescendant projection，并验证没有伪造用户 ValueChange。

已有 facade 对两个 view 的构造和程序化选择隔离测试，不再重复。这里补同 source 的两个真实 ListView：用户操作其中一个只改变它的 active/selected 与通知，随后 model mutation 让两者各自修复和重建 rows。辅助查询必须限定 root/hierarchy，不能继续用全 App 的单一 row 列表证明跨实例隔离。

在现有真实 Text 渲染准备场景上增加一个具有 text + icon 的业务 renderer 代表。区分外部 asset 尚未加载与已就绪后新 row/subtree 的 materialization：前者允许有依据地等待，后者在指定生成帧检查真实 ImageNode、有效 image、继承可见性和 stack/layout。滚入或 visible revision 重建不能只检查 WidgetryIcon component 存在。SVG 的通用颜色、缓存和等待行为由 Icon 方案负责，这里只验证 ListView 动态构造与它的真实组合。

### 文件组织和验收

以 contract.rs 为集成 Coverage Map，明确 behavior、virtualization、style 的主要 owner；model.rs、behavior.rs、virtualization.rs 中有独立局部语义的测试仍就近保留。只对可能破坏的约束调用 assert_selection_invariants、assert_active_invariants、assert_row_projection 等小 helper，不能以它们代替本次操作的精确结果断言。

运行 bevy_widgetry_list_view、bevy_widgetry_combo_box 和 bevy_widgetry 的测试。BRP 仅在本份涉及经授权的 GUI 行为修复时执行对应场景，例如真实键盘导航/确认或滚入 text+icon；不重新跑全量 ListView 历史功能，不把 headless 渲染前置状态当成 GPU 最终画面的证明。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_list_view
cargo test -p bevy_widgetry_combo_box
cargo test -p bevy_widgetry
```

### 在执行链中的位置

前一份 ScrollArea 提供已核对的布局/滚动组合基础；本份独立保护 ListModel 与 ListView 的业务 contract。下一份 ComboBox 复用这些保证，不再复制 model 的全部 property tests 或 ListView 的完整键盘矩阵。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/list_view/src/model.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/src/model.rs)
- [crates/list_view/src/behavior.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/src/behavior.rs)
- [crates/list_view/src/virtualization.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/src/virtualization.rs)
- [crates/list_view/tests/contract.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/tests/contract.rs)
- [crates/list_view/tests/behavior.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/tests/behavior.rs)
- [crates/list_view/tests/virtualization.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/tests/virtualization.rs)
- [crates/list_view/tests/style.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/list_view/tests/style.rs)
- [crates/bevy_widgetry/tests/public_api.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/bevy_widgetry/tests/public_api.rs)
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
