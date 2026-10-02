# RadioGroup 明确官方 Checked 与 index API 边界

## 目标

保留官方 Checked authority，明确固定 index 输入和多层通知的现有契约。

## 范围

crates/radio_group 公共说明与 facade / Gallery 消费说明。不增 root selection 副本，不包装官方 Component。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**RadioGroup** 的 direct children 必须非空、全部为 RadioOption，并保持固定顺序；初始化后不支持增删或重排。首次 PreUpdate 默认选中 index 0；首帧前调用 set_selected 会先完成初始化，再应用有效的指定 index，因此不会被随后默认初始化覆盖。

RadioGroup::set_selected 接收 Commands、root Entity 与 usize index，queue 执行时更新各 option 的 Checked，不保存另一份 root selection。已初始化后的同值或越界选择为 no-op；无效 root 被忽略，非法构造 hierarchy 则返回 BevyError。首次调用即使 index 越界，也可能已建立默认 selection，不能概括为“无效参数始终不改变任何 state”。disabled 从 root 镜像到 options，程序选择仍允许；单项 disabled 不属于当前契约。

依据：[Button](../../../crates/button/src/style.rs)、[二态 CheckBox](../../../crates/check_box/src/checkbox.rs)、[三态 CheckBox](../../../crates/check_box/src/tri_state.rs)、[RadioGroup](../../../crates/radio_group/src/group.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

公开 Group 级通知为 ValueChange<usize>，source 指向 Group root，value 是固定 direct children 中的 index。Widgetry 将官方 ValueChange<Entity> 的目标 option Entity 转成 index，并原样保留 is_final；正常用户路径输出 true，转换层本身不强制改成 true。

官方 Radio 输入还可发出 option 级 ValueChange<bool> 和 Group 级 ValueChange<Entity>。它们与 Widgetry 的 usize 通知属于同一操作的不同表达，App 级 consumer 如果同时监听这些 type，应避免把一次选择统计多次。

已 Checked option 的重复 click 不发改选通知；Group keyboard navigation 目标不变时也不通知。默认 selection 和 WidgetryRadioGroup::set_selected 静默；root disabled 镜像到 options 后阻止用户改选。

Checked 更新由 radio_self_update 排入 Commands，index 通知由另一个 observer 转发；当前没有在 index 通知之前显式提交全部 Checked 更新的统一边界。因此 consumer 可使用 event.value 识别目标，不应把通知等同于“全部 option state 已完成更新”。

依据：[Group 转换与程序入口](../../../crates/radio_group/src/group.rs)、[observer 装配](../../../crates/radio_group/src/lib.rs)、[相关测试](../../../crates/radio_group/tests/widgetry_radio_group.rs)；官方用户路径见 bevy_ui_widgets 0.19.1 的 src/radio.rs。

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

selection 在 options 的官方 Checked，不因 Widgetry 提供 setter / index 转发就另行处理。默认 index 0、首帧前有效设置优先、固定 direct options、disabled 镜像与程序静默路径保持原样。

consumer 应区分 ValueChange<usize>、ValueChange<Entity> 和 option ValueChange<bool>，按 type / source 避免重复统计。index 通知前不能承诺全部 Checked 已由 Commands 写入；保留转发 is_final，不强制改 target、不增 origin。

现有无效 root、初始化后越界 no-op、非法 hierarchy 错误须如实记录。首次无效 index 可能先建立默认 selection，不能套用自有 state 的“无效请求绝不修改”新行为；初始化和官方 authority 均在保留范围。

## 未决前提与方案缺口

新清空、动态 options、单项 disabled、程序选择通知均未批准。新只读 / 错误规则的作用域须与官方 Component 例外一致。

## 验证意图

核对 index、首帧初始化、no-op / 错误及 notification type / source；无行为变更不新增测试。上游共享代码影响它时才运行现有组合测试，不能用文档声称通知前全部 Checked 已提交。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

输入与通知说明准确，官方路径保留现状。

## 与前后方案的关系

在三态 CheckBox 之后、TextField 之前，无技术硬依赖。
