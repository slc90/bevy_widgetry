# MessageBox 保留一次性结果与关闭顺序

## 目标

明确既有先决议再通知，保留一次性结果及资源关闭顺序。

## 范围

crates/message_box 的 factory / result / lifecycle 文档。不增程序 result setter 或 runtime button 重配。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**MessageBox** 通过 widgetry_message_box(parent, title, buttons, content) 一次性指定 parent native Window、标题、固定结果 button 组与任意 SceneList 内容。没有公开程序赋值结果、切换 button 组或完成 dialog 的 API；自定义内容的 Component 仍由应用按其自身契约更新。

依据：[Tooltip](../../../crates/tooltip/src/style.rs)、[Tooltip disabled 行为](../../../crates/tooltip/src/headless.rs)、[Window](../../../crates/window/src/scene.rs)、[modal](../../../crates/window/src/modal.rs)、[MessageBox](../../../crates/message_box/src/scene.rs)、[Icon](../../../crates/core/src/icon.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

输出 WidgetryMessageBoxResultEvent，entity 指向 dialog UI root，不是结果 button 或 native Window；result 为 Ok、Yes、No、Cancel 中与固定 button 组对应的结果。

结果 button 的 Activate 通过私有 MessageBoxClick 桥接到 root。首次有效决议先把内部 resolved 写为 true，再排队触发公开结果 event；同帧重复激活和 reentrant 激活不重复发布。

结果 observer 及其 Commands 应用后才进入独立关闭阶段，因此 consumer 可在通知中读取仍存在的 dialog，也可自行结束 root。不要把 result event 当作“资源已经全部销毁”的完成通知。

正文内普通 button 不发布 dialog result；disabled 结果 button 不能经直接 Activate 绕过检查。操作系统关闭或程序销毁 native Window / root 不转换为 Cancel，也不产生结果。外部主动 trigger 一个有效结果 button 的 Activate 仍可进入同一路径，公开结果没有来源区分。

依据：[公开结果声明](../../../crates/message_box/src/scene.rs)、[一次性决议与关闭顺序](../../../crates/message_box/src/lifecycle.rs)、[相关测试](../../../crates/message_box/tests/message_box.rs)。

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

直接复用的 Bevy 官方 Component 不纳入本次自有 state 只读化与更新入口治理，保持其原有使用方式和行为。

例如 EditableText、Checked、ScrollPosition、InteractionDisabled 等官方 Component，不为了本规则新增 wrapper、修改入口限制、写入监测或事件转接等特殊处理。

因此，本规则不要求每个 Widget 的所有官方 Component 更新都改为先提交再通知，也不为统一接口而单独改造这些官方 Component 的契约。

状态变化事件只在 state 实际改变时发出；重复设置相同 state 不发变化事件。

确认、重选等操作如需对外表达，使用独立的操作语义，不借用状态变化事件。此规则不要求为每个 Widget 新增确认或重选 event，具体需要哪些通知仍按各 Widget 的实际行为决定。

统一后的状态变化通知不增加 User / Programmatic 等 origin 区分。UI 操作与程序化设置都按已确认契约提交并通知，consumer 不依赖统一的来源字段。

各 Widget 的事件含义、target / source 与业务 identity 继续按各自契约表达，本轮不强制归并为同一种事件语义或 target。具体 type 形态尚未决定。

通过 Widget API 显式清空 state，导致 state 实际改变时，先提交再发送对应的变化事件。例如 selection 从某个 item 变为 None，事件需要能表达清空后的未选中 state；已经为空时再次清空，不发变化事件。

初始化与 Model repair / 自动修复保持现有行为，不修改其通知规则，不为它们新增通知。程序删除 Model 数据后触发的自动 selection repair，仍属于保留现状的自动修复，不等同于调用 Widget API 显式清空 selection。

此规则针对适用 Widget 的程序显式清空，不要求原本必须保持选中的 Widget 新增空 state。

保留各 Widget setter 当前对 active、cursor、focus 和 reveal / scroll 的联动行为，不在本轮统一为“只修改目标 state”，也不为此拆分现有 API。

保留现有立即更新 World 与 Commands queue 更新两类入口及其执行时机，不在本轮统一调用形式或重新安排执行阶段。已经确认的“先提交 state，再发事件”在各入口实际执行更新时落实，不据此要求排队 API 在调用瞬间提交。

内部 lifecycle 不必新增公共 state 或 setter；一次性操作通知与状态变化通知保留各自含义。自有 state 的提交顺序不保证 layout、图像或其他 projection 在通知时已收敛，也不保证 observer 执行顺序。

## 需要修改与保持的行为

首次有效结果激活先写 resolved 再发 event，已满足相应顺序。resolved 为内部 state，无需为只读查询暴露新 Component / getter。

target 为 dialog UI root，固定 Ok / Yes / No / Cancel；结果 observer 与 Commands 后才进入关闭阶段，通知不是资源销毁完成。consumer 可读取 dialog 或自行结束 root。

同帧重复与 reentrant 激活不重复决议，正文普通 button 不发布结果，disabled 结果 guard 保留。OS close / 程序 despawn 不转成 Cancel，显式清空 state 的规则不套到 dialog 销毁。无需 origin 或统一结果 event。

## 未决前提与方案缺口

程序决议、button 重配和 OS close 转 Cancel 均未批准。

## 验证意图

纯说明核对一次性、target、关闭时序。共享代码确有影响时验证 observer 读 dialog / despawn、重复与 reentrant、disabled guard，不为无变更增加形式测试。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

result 与 lifecycle contract 清楚，已有提交顺序和关闭行为保持。

## 与前后方案的关系

承接 Window 的 native close 边界及早先 Button Activate 说明。下一份 Icon 独立，仅作为执行链末项。
