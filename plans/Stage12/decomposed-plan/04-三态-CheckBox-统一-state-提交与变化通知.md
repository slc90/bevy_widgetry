# 三态 CheckBox 统一 state 提交与变化通知

## 目标

使 UI 与程序更新都在 WidgetryCheckState 提交之后通知，并让公开 Component 查询只读。

## 范围

crates/check_box 的三态行为、Component、style / accessibility 与测试，facade 使用和 gallery/src/pages/check_box.rs。二态官方路径不改。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**三态 CheckBox** 使用 WidgetryCheckState 表达 Unchecked、Checked、Indeterminate，默认 Unchecked。set_state 和 cycle_state 接收 Commands 与 root Entity，在 queue 执行时读取真实 state。相同 state 不写入；无效 entity 或非三态 Widget 为 no-op；disabled 不阻止调用。cycle_state 按执行时 state 连续循环，顺序为 Unchecked → Checked → Indeterminate → Unchecked。

依据：[Button](../../../crates/button/src/style.rs)、[二态 CheckBox](../../../crates/check_box/src/checkbox.rs)、[三态 CheckBox](../../../crates/check_box/src/tri_state.rs)、[RadioGroup](../../../crates/radio_group/src/group.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

**三态 CheckBox** 输出 ValueChange<WidgetryCheckState>，source 为 root，真实 pointer / keyboard 切换输出 is_final = true。self-update observer 排队写入 WidgetryCheckState，所以同样不能保证本轮外部 observer 立即读到目标 state。

set_state / cycle_state 是静默程序入口，即使 disabled 时更新也不发 ValueChange。disabled 会拦截 Widget 的用户切换路径；程序直接 trigger ValueChange 则不等同于经过用户 guard 的一次操作。

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

### 自有 state 与提交顺序

确认日期：2026-10-02。

> **公开支持的 state 更新必须通过 Widget API；state Component 提供只读查询。API 保证先提交 state，再发事件。**

此规则适用于 Widgetry 自己定义和管理的 runtime state，例如自有 selection、active、focused_cell、三态 CheckBox state，以及 Tree 的展开 state。构造 Props 仍按现有规则负责一次性初始化，不作为 runtime 更新入口。

用户 UI 操作与程序化设置采用相同的 state 提交与通知顺序：

```text
UI 操作或程序化设置
        ↓
通过 Widget 的 state 更新路径处理
        ↓
提交真实 state
        ↓
发出对应的状态变化事件
```

程序化设置引起的 state 变化也通过上述路径通知，不再默认采用静默 setter。状态变化事件表达已经提交的变化；不能依靠外部收到该事件后再回写 state，才完成本次更新。初始化与 Model repair 的行为按本方案的初始化与 Model repair 保留约束保持现状，不纳入本轮调整。

调用方可以查询自有 state Component，并通过公开只读接口读取当前 state；公开支持的 runtime 修改入口由 Widget API 提供。外部主动替换或移除自有 state Component、破坏内部 hierarchy、销毁 entity 等 ECS 结构操作，不能视为正常 Widget state 更新 API，不承诺具有相同的变化通知行为。

“先提交 state”描述真实 state 的写入顺序，不要求对应的高亮、renderer subtree 或 layout 已在通知前完成同步，也不建立同一事件的多个 observer 之间的固定执行顺序。

### 官方 Component 例外

直接复用的 Bevy 官方 Component 不纳入本次自有 state 只读化与更新入口治理，保持其原有使用方式和行为。

例如 EditableText、Checked、ScrollPosition、InteractionDisabled 等官方 Component，不为了本规则新增 wrapper、修改入口限制、写入监测或事件转接等特殊处理。

因此，本规则不要求每个 Widget 的所有官方 Component 更新都改为先提交再通知，也不为统一接口而单独改造这些官方 Component 的契约。

### 同值、操作语义与来源

状态变化事件只在 state 实际改变时发出；重复设置相同 state 不发变化事件。

确认、重选等操作如需对外表达，使用独立的操作语义，不借用状态变化事件。此规则不要求为每个 Widget 新增确认或重选 event，具体需要哪些通知仍按各 Widget 的实际行为决定。

统一后的状态变化通知不增加 User / Programmatic 等 origin 区分。UI 操作与程序化设置都按已确认契约提交并通知，consumer 不依赖统一的来源字段。

各 Widget 的事件含义、target / source 与业务 identity 继续按各自契约表达，本轮不强制归并为同一种事件语义或 target。具体 type 形态尚未决定。

### 显式清空、保留行为与错误

通过 Widget API 显式清空 state，导致 state 实际改变时，先提交再发送对应的变化事件。例如 selection 从某个 item 变为 None，事件需要能表达清空后的未选中 state；已经为空时再次清空，不发变化事件。

初始化与 Model repair / 自动修复保持现有行为，不修改其通知规则，不为它们新增通知。程序删除 Model 数据后触发的自动 selection repair，仍属于保留现状的自动修复，不等同于调用 Widget API 显式清空 selection。

此规则针对适用 Widget 的程序显式清空，不要求原本必须保持选中的 Widget 新增空 state。

保留各 Widget setter 当前对 active、cursor、focus 和 reveal / scroll 的联动行为，不在本轮统一为“只修改目标 state”，也不为此拆分现有 API。

保留现有立即更新 World 与 Commands queue 更新两类入口及其执行时机，不在本轮统一调用形式或重新安排执行阶段。已经确认的“先提交 state，再发事件”在各入口实际执行更新时落实，不据此要求排队 API 在调用瞬间提交。

Widgetry 自有 state 更新 API 接收到无效目标时，作为错误返回，不再将其静默忽略或与合法同值合并为“未改变”。例如失效的 node Entity、stale item / Row / Column ID、越界 index，或不满足 API 要求的 Widget / source entity。

合法同值设置属于成功但 state 未改变，不作为错误，也不发状态变化事件。对于已有 Result<bool, BevyError> 入口，语义分别为：

| 结果 | 含义 |
| --- | --- |
| Ok(true) | 合法输入使目标 state 实际改变 |
| Ok(false) | 合法输入未改变目标 state，例如设置相同值 |
| Err(...) | 目标无效，无法执行请求 |

无效目标不执行请求的 state 修改，不发对应变化事件。此规则约束程序化 state 更新 API，不据此改变原始 UI 输入过滤、初始化、自动 repair 或直接复用官方 Component 的行为。

立即与排队入口的执行时机继续保持现状；排队入口在实际执行时反馈错误，不把函数调用瞬间视为最终目标校验完成。具体错误反馈 API 按各入口形态在实施设计中明确，本轮不要求所有入口改成相同 signature。

## 需要修改与保持的行为

当前 UI 先 trigger ValueChange，再由 self-update observer 排队写 state；set_state / cycle_state 则静默。两条路径须收敛到真实 state 提交边界，通知不再承担请求外部回写的职责。保留 Unchecked → Checked → Indeterminate → Unchecked；同值 set_state 不发变化，连续 cycle 按各 command 实际执行时的 state 计算。

公开保留三个 enum 值及只读查询，正常 runtime 修改经 Widget API。不保证阻止主动替换 / 移除 Component；immutable 或其他承载 type 的选择尚未确定。保留 Commands queue 和 disabled 时程序仍可更新的行为。

无效 root、非三态 Widget 或必需 state 缺失从静默忽略改为实际执行时反馈错误，失败不修改 state、不发变化事件。默认 Unchecked 初始化和后续 style / accessibility projection 时序保持，不要求通知时外观已经同步。

Gallery 的 disabled Indeterminate 演示当前通过 Query<&mut WidgetryCheckState> 赋值，须改为合法 Widget API。facade、测试的读取 / 构造及消费者同步适配。人工 trigger 变化 event 不再作为公开 state 更新方法；不扩大为过滤所有官方 event。

## 未决前提与方案缺口

只读承载方式、变化 event 是否继续 ValueChange<WidgetryCheckState>、程序路径 is_final 及排队错误反馈形态尚未决定。实施前通过 Type-Driven Development 明确，不由拆分文档确定 signature，不增 origin、old value 或重选 event。

## 验证意图

以真实 pointer / focused keyboard 和程序 setter 作为 stimulus，在 consumer observer 内读取已提交 authority。覆盖同值、连续 cycle、disabled 程序更新、无效 root / 缺失 state及入队后 entity 被删除；错误路径无修改无通知。保留默认初始化和二态组合。Gallery 验证真实切换及 disabled Indeterminate 演示。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

三态 API、只读查询、提交后通知和错误契约一致，消费者 / rustdoc 与必要验证完成。

## 与前后方案的关系

承接二态例外。下一份 RadioGroup 使用官方 Checked，不依赖三态新实现，仅遵循整体顺序。
