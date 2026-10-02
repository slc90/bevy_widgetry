# ReadOnlyTextField 明确只读输入过滤边界

## 目标

明确 read-only、disabled 和程序修改的边界，保留官方通知。

## 范围

crates/text_field 中 ReadOnlyTextField 的 public rustdoc / 使用说明。不增 runtime read-only 切换。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

两者都使用同 root 的官方 EditableText，文本、allow_newlines、visible_lines 等配置由调用方 patch，不另建 Widgetry value 副本。初始化内容可通过 EditableText::new 提供，程序修改文本使用 EditableText::set_text。

TextField 的 InteractionDisabled 会在官方编辑阶段前清除 pending_edits 和 pending_paste，保留程序直接设置的文本。ReadOnlyTextField 保留用户 navigation、selection 和复制能力，只过滤文本 mutation；同时 disabled 时则清除全部待处理编辑操作。

read-only 身份在构造时选择，不提供 runtime 切换 API。read-only 和 disabled 都不禁止程序修改 EditableText；它们也不是相同含义的输入限制。

依据：[TextField](../../../crates/text_field/src/style.rs)、[disabled 测试](../../../crates/text_field/tests/disabled.rs)、[read-only 测试](../../../crates/text_field/tests/read_only.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

Widgetry 未新增文本变化 event，复用官方 TextEditChange。target 为持有 EditableText 的同一个 root，event 不携带新文本、旧文本、is_final 或 origin；consumer 通过 event target 查询 EditableText::value。

官方 apply_text_edits 先处理 pending edits，再比较 EditableTextGeneration 与 editor generation，存在差异时排队触发 TextEditChange。官方 rustdoc 明确 cursor motions 也可能产生通知，即使文本 value 未改变；这个输出不是严格的“文本 value changed”。

generation 检查不要求本帧存在用户 pending edit；程序 set_text、其他 editor mutation，以及初始 generation 差异也可能进入同一通知路径。当前不能把这个 type 当作“仅用户编辑”的来源标识，也不能承诺每次文本 mutation 都有一条独立快照 event。

普通 TextField disabled 时清除待处理用户 edit / paste；ReadOnlyTextField 保留 navigation / selection / copy，所以仍可能出现 generation 通知。programmatic text 更新仍允许，不能从“用户文本 mutation 被阻止”推导“没有任何 TextEditChange”。

当前没有 Widgetry 自有的 submit、commit、cancel、文本 selection changed 或严格 value changed event；若应用需要这些语义，应先在目标 3 明确需求与边界。

依据：[Widgetry 过滤与构造](../../../crates/text_field/src/style.rs)、[disabled 测试](../../../crates/text_field/tests/disabled.rs)、[read-only 测试](../../../crates/text_field/tests/read_only.rs)；官方声明与触发路径见 bevy_text 0.19.1 的 src/editing.rs。

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

read-only 构造时确定，保留 navigation、selection、copy，只过滤用户文本 mutation。disabled 不与 read-only 等价，同时 disabled 仍清除现有全部 pending edit。

程序 EditableText 更新不受 read-only 禁止；navigation / selection 仍可能产生 TextEditChange，不能写成“没有变化通知”。沿用官方例外，不加文本副本、统一 setter、strict value event 或 origin。共用的 generation 背景完整保留，保证单独阅读不会误解。

## 未决前提与方案缺口

runtime read-only 切换、submit / cancel 和新通知未批准。

## 验证意图

纯说明核对 read-only 与 disabled 区别，无变更不新增测试。若共用过滤实现确有必要修改，验证 navigation / selection / copy、用户 mutation 被拒绝、程序 set_text 允许和 disabled 组合。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

公共契约与现有过滤 / 官方输出一致。

## 与前后方案的关系

承接 TextField，随后 ScrollArea 无技术硬依赖，仅为整体执行顺序。
