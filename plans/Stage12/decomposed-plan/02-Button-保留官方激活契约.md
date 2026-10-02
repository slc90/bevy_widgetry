# Button 保留官方激活契约

## 目标

明确 Activate 是操作通知，保留官方输入与 Component 契约。

## 范围

crates/button 的公共 rustdoc，以及确有误解的 Gallery / consumer 说明。没有自有 selection value，不引入 state setter 或新业务 event。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**Button** 主要接收内容、layout 与 InteractionDisabled，不保存类似 selection 的业务 value。Pressed、Hovered 等交互 state 由官方行为维护；修改这些 Component 不等于执行一次用户激活。背景、border 与 foreground 是 theme/state 的派生结果，直接修改可能被后续 style 同步覆盖。

依据：[Button](../../../crates/button/src/style.rs)、[二态 CheckBox](../../../crates/check_box/src/checkbox.rs)、[三态 CheckBox](../../../crates/check_box/src/tri_state.rs)、[RadioGroup](../../../crates/radio_group/src/group.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

WidgetryButton 复用官方 ButtonPlugin，主要语义输出为 Activate，entity 指向 Button 自身。默认 pointer click 路径要求有效 pressed state 且未 disabled；配置 ActivateOnPress 时改为 Press 激活。focus 下首次 Space / Enter Press 也可激活，repeat 不触发重复 keyboard 激活。

Activate 不携带 value、old value、is_final 或用户来源，不是“Pressed Component 变化”的别名。外部也可以直接 trigger Activate，接收方无法仅从 payload 判断是否来自真实用户；官方输入路径的 disabled 检查不等于所有直接 trigger 都会被统一过滤。

依据：[Button](../../../crates/button/src/style.rs)、[Button 测试](../../../crates/button/tests/widgetry_button.rs)；官方行为见 bevy_ui_widgets 0.19.1 的 src/button.rs。

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

本轮没有已确认的 Button 行为改动。Activate 表达一次激活，不能从 Pressed / Hovered 的变化推导业务操作，也不能套用“相同 value 不通知”的状态变化规则。

保留 ActivateOnPress、focus 下 Space / Enter 首次 Press、keyboard repeat guard 和官方 disabled 路径。程序直接 trigger Activate 继续按官方行为处理，不增加 origin、统一程序过滤或包装请求。内容由应用自己的 Text 等 Component 管理；theme 派生外观不作为新 value authority。

## 未决前提与方案缺口

没有需要新确定的 setter 或 payload。独立确认 / 重选 event 尚未批准，不在本轮补出。

## 验证意图

仅文档澄清时核对公共导出、rustdoc 和 Gallery 说明，不新增镜像官方行为的测试。若确有 composition 修改，才按影响范围验证 pointer、keyboard、disabled 与 ActivateOnPress；本方案本身不授权更改这些行为。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

公共说明准确区分激活、交互 state 和派生外观，原有行为保持。

## 与前后方案的关系

承接规则方案。随后处理二态 CheckBox，二者没有技术硬依赖，仅采用这条整体执行顺序。
