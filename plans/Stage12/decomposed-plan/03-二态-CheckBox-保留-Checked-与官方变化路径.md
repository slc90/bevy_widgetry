# 二态 CheckBox 保留 Checked 与官方变化路径

## 目标

明确 Checked 的官方例外范围，保留现有程序输入与通知。

## 范围

crates/check_box 中二态 CheckBox 的 rustdoc / 使用说明。不改 CheckboxPlugin、Checked、官方请求或 checkbox_self_update。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**二态 CheckBox** 使用官方 Checked 作为唯一选中 state，默认未选中。程序通过在 root insert / remove Checked 修改选中状态；目前没有与三态 set_state 对应的 Widgetry 二态 setter。

依据：[Button](../../../crates/button/src/style.rs)、[二态 CheckBox](../../../crates/check_box/src/checkbox.rs)、[三态 CheckBox](../../../crates/check_box/src/tri_state.rs)、[RadioGroup](../../../crates/radio_group/src/group.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

**二态 CheckBox** 使用 ValueChange<bool>，source 为 root，value 表示目标 Checked state；真实用户切换输出 is_final = true。Widget 的 checkbox_self_update observer 根据 event value 排队 insert / remove Checked。

因此 consumer 应把 event.value 作为本次目标值，不能假设在同一个 ValueChange observer 中查询 Checked 就一定得到新值。程序直接修改 Checked 不自动发出 ValueChange。

官方 CheckboxPlugin 还接收 SetChecked / ToggleChecked 请求：未 disabled 时，SetChecked 在目标与当前 state 不同时发 ValueChange，ToggleChecked 发出下一值。程序通过这些请求进入行为路径也会产生相同 event，payload 没有 origin。这与直接 Component 更新的静默路径不同。

依据：[二态构造与 self-update 安装](../../../crates/check_box/src/checkbox.rs)、[三态行为](../../../crates/check_box/src/tri_state.rs)、[相关测试](../../../crates/check_box/tests/checkbox.rs)；官方请求与 self-update 见 bevy_ui_widgets 0.19.1 的 src/checkbox.rs。

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

二态 authority 是官方 Checked。root insert / remove Checked 继续合法且静默；SetChecked / ToggleChecked 的 disabled 与 ValueChange<bool> 行为也保持原样。

不为与三态 API 形式相同而增加二态 setter、wrapper、写入监测或事件转接。ValueChange<bool> 表示目标值，不能承诺同一 observer 中 Checked 已由 self-update 的 Commands 更新。consumer 使用 event.value 的必要说明直接附着公共使用位置。三态迁移不得改变同 crate 内的二态行为。

## 未决前提与方案缺口

没有批准二态新 API 或 event，不将自有 state 的错误规则扩展到官方 Component 更新。

## 验证意图

仅说明改动核对直接 Checked 写入与官方请求差异。三态修改共享 crate 后，现有二态组合测试应继续通过；不重复证明官方内部行为。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

公共说明准确区分两条程序路径，行为维持现状。

## 与前后方案的关系

在 Button 之后、三态 CheckBox 之前。与 Button 无技术依赖；先明确同 crate 的二态例外，约束下个方案的范围。
