# ScrollArea 保留官方滚动 state 与请求机制

## 目标

明确 ScrollPosition 查询与 ScrollIntoView 请求，保留官方滚动行为。

## 范围

crates/scroll_area 的公共说明。不增加 root disabled、滚动完成 event 或 runtime 配置 setter。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

构造 Props 提供 axis、scrollbar_visibility、scrollbar_thickness、keyboard_scroll、content 与 children。配置被一次性写入内部 Component，当前没有公开 runtime 配置 setter；axis 与 keyboard_scroll 明确为构造后固定。

滚动 state 保存于带 WidgetryScrollAreaViewport 的内部 entity 的官方 ScrollPosition，不在 root 上。调用方可以定位该 Viewport 后直接修改 ScrollPosition，也可以向 descendant 发出 WidgetryScrollIntoView，由最近的有效 Widgetry Viewport 根据真实 layout 处理。

keyboard_scroll = false 只关闭 Widgetry keyboard scroll，不关闭 wheel、trackpad 或程序滚动。当前实现没有将 root InteractionDisabled 统一镜像到 Viewport / scrollbar 的契约，keyboard handler 也没有检查它；不能把其他 Widget 的 root disabled 规则直接套到独立 ScrollArea。

依据：[构造](../../../crates/scroll_area/src/style.rs)、[输入与公开 marker](../../../crates/scroll_area/src/headless.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

当前没有 Widgetry 对外 scroll changed / scroll completed event。调用方读取带 WidgetryScrollAreaViewport 的 entity 上的官方 ScrollPosition，结合真实 layout 理解有效位置。

WidgetryScrollIntoView 是外部发给 descendant 的滚动请求，不是滚动完成通知。一次请求是否移动位置、何时有有效 geometry、最终位置是多少，不能只凭发出请求判断。

wheel、trackpad、scrollbar、keyboard 与程序直接修改共享同一 ScrollPosition，不携带 origin。Changed<ScrollPosition> 是 ECS change detection，不自动等价于用户滚动或数值确实改变，也不表达 interaction 结束。

依据：[公开 marker 与请求处理](../../../crates/scroll_area/src/headless.rs)、[组合与原生 Scrollbar](../../../crates/scroll_area/src/style.rs)。

## 共用 event 消费背景

Activate 和 ValueChange<T> 是官方 EntityEvent，WidgetryTreeEvent、WidgetryTableEvent、WidgetryMessageBoxResultEvent 也是 EntityEvent。通常由 entity 上的 observer 或 App 级 observer 通过 On<Event> 消费。

上述 event 的声明没有启用沿 ChildOf 自动 propagation；不能假设把 observer 放在父 entity 上就能收到所有后代的语义通知。App 级 observer 则需要按 source / entity 和 Widget type 过滤。ComboBox、RadioGroup 等会显式转换或重新定向 event。

官方 ValueChange<T> 的 payload 只有 source、value、is_final，没有旧 value、origin、pointer / keyboard 类型，也没有所属 Model source。is_final 描述一次 interaction 是否结束，不表示程序 / 用户来源，更不表示所有 UI projection 已更新。

Bevy 对同一 event 的多个 observer 不提供固定执行顺序，observer 中排入 Commands 的写入也不在其他同一轮 observer 运行前自动完成。因此需要区分两种路径：

- CheckBox / RadioGroup：ValueChange 先驱动 self-update observer 排队写入 state。
- ListView / Table / Tree 的部分语义 event：实现先写入 authority，再 trigger 通知。

authority 已写入不等于物理 row、Field 内容、颜色或真实 layout 已更新；其他 observer 也可能继续修改 state。本文中的先后关系描述生产者发出通知时的实现顺序，不承诺多个消费者之间的执行顺序。

官方声明与调度语义依据当前本地依赖：bevy_ui_widgets 0.19.1 的 src/lib.rs，以及 bevy_ecs 0.19.1 的 src/observer/distributed_storage.rs；依赖版本见 [Cargo.toml](../../../Cargo.toml) 与 [Cargo.lock](../../../Cargo.lock)。

上述 self-update 与静默描述属于迁移前背景；自有 state 按后文迁移，直接复用官方 Component 按例外保留。

## 本方案适用的共同契约

直接复用的 Bevy 官方 Component 不纳入本次自有 state 只读化与更新入口治理，保持其原有使用方式和行为。

例如 EditableText、Checked、ScrollPosition、InteractionDisabled 等官方 Component，不为了本规则新增 wrapper、修改入口限制、写入监测或事件转接等特殊处理。

因此，本规则不要求每个 Widget 的所有官方 Component 更新都改为先提交再通知，也不为统一接口而单独改造这些官方 Component 的契约。

自有 state 的统一要求不覆盖本方案直接复用的官方 Component；不增加 wrapper、监测、事件转接或写入限制。UI 与程序的现有 event 路径如实保留，不改成严格变化通知。

统一后的状态变化通知不增加 User / Programmatic 等 origin 区分。UI 操作与程序化设置都按已确认契约提交并通知，consumer 不依赖统一的来源字段。

各 Widget 的事件含义、target / source 与业务 identity 继续按各自契约表达，本轮不强制归并为同一种事件语义或 target。具体 type 形态尚未决定。

保留各 Widget setter 当前对 active、cursor、focus 和 reveal / scroll 的联动行为，不在本轮统一为“只修改目标 state”，也不为此拆分现有 API。

保留现有立即更新 World 与 Commands queue 更新两类入口及其执行时机，不在本轮统一调用形式或重新安排执行阶段。已经确认的“先提交 state，再发事件”在各入口实际执行更新时落实，不据此要求排队 API 在调用瞬间提交。

构造 Props 仍只负责一次性初始化。初始化与 Model repair 保持原样，不统一 disabled、read-only、modal、identity、调用形式或 setter 附带行为。确认 / 重选的新 event 未决定，不为统一而新增。

## 需要修改与保持的行为

authority 位于内部 WidgetryScrollAreaViewport 的官方 ScrollPosition，程序直接更新继续合法。WidgetryScrollIntoView 是请求，依赖真实 layout，发出请求不等于滚动完成或位置必变。

keyboard_scroll 只关闭 keyboard 路径，不影响 wheel、trackpad、程序滚动；axis 和 keyboard_scroll 构造后固定。既有 root InteractionDisabled 无通用镜像契约，不为统一而增加。

官方 scrollbar 的 DragEnd / Cancel 清理不套 Table Start / Resized / End / Cancel。Changed<ScrollPosition> 是 ECS change detection，不自动表达业务 changed / completed 或来源。

## 未决前提与方案缺口

公开 scroll changed / completed、统一 disabled、origin 均未批准。

## 验证意图

核对 marker、Viewport、请求 / 结果区别。纯说明不新增官方内部测试；有实际 composition 修改再验证 wheel / keyboard / scrollbar 及 layout。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

查询 / 请求契约明确，官方 runtime 保持原样。

## 与前后方案的关系

在 ReadOnlyTextField 之后、ListView 之前。ListView 组合 ScrollArea，先明确其官方例外；不要求改 ScrollArea 才能实现新 selection 通知。
