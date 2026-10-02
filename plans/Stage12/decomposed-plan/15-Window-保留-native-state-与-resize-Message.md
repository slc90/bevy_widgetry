# Window 保留 native state 与 resize Message

## 目标

保持 native Window 官方输入输出，明确它与 Table gesture 的不同契约。

## 范围

crates/window 的 Scene factory、identity、controls / modal 与 resize 说明，不新建 Window / ready event。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

**Window** 通过 widgetry_window 显式绑定外部 native Window / camera，或通过 owned_widgetry_window 创建并拥有两者。WidgetryWindowControlsConfig 决定构造时的按钮显隐和 resize 区域，没有 runtime 重构 controls 的公开 setter。native runtime 配置走官方 Window / 平台窗口能力；创建期 transparent 等属性不能概括为可随时更新。

WidgetryModalWindow.parent 指向 parent native Window Entity，在 child UI root 上表达 pointer modal relationship。它不是 child 的普通 disabled，也不建立 keyboard focus 或 OS modal 契约。

依据：[Tooltip](../../../crates/tooltip/src/style.rs)、[Tooltip disabled 行为](../../../crates/tooltip/src/headless.rs)、[Window](../../../crates/window/src/scene.rs)、[modal](../../../crates/window/src/modal.rs)、[MessageBox](../../../crates/message_box/src/scene.rs)、[Icon](../../../crates/core/src/icon.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

**Window** 的 close button 写入官方 WindowCloseRequested Message，window 指向 native Window Entity，是关闭请求而非实际完成。实际关闭由官方 WindowClosed 等 Message 表达，Widgetry 据此清理 UI；调用方应区分 native identity 与 UI root。minimize / maximize / drag / resize 走 Window / winit 能力，当前没有统一 WidgetryWindowEvent 或公开控件级 Start / End 契约。私有 WindowInitialized 也不是公开 ready event。

依据：[Window close](../../../crates/window/src/title_bar/close.rs)、[Window lifecycle](../../../crates/window/src/window_root.rs)、[Waveform runtime](../../../crates/waveform/src/runtime.rs)、[Tooltip 内部 event](../../../crates/tooltip/src/headless.rs)、[Tooltip 公共导出](../../../crates/tooltip/src/lib.rs)、[Icon](../../../crates/core/src/icon.rs)。

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

widgetry_window 绑定外部 native Window / camera，owned_widgetry_window 创建并拥有二者，ownership / lifecycle 保留。controls 与 resize 区域为构造配置，不增 runtime 重建 setter，创建期 transparent 等不描述为任意 runtime 可改。

native state 复用官方 Window / winit，保留官方例外。WindowCloseRequested 是请求，WindowClosed 是实际关闭；target 为 native Entity而非 UI root。modal parent 也是 native Entity，pointer modal 不增 keyboard / OS modal 契约。

WindowResized 无统一 Start / End / Cancel，native drag resize 不套 Table Column event；私有 WindowInitialized 不升级为 public ready。

## 未决前提与方案缺口

统一 Window setter、控件级 resize / ready / close result event 未批准。

## 验证意图

仅说明核对 ownership、native / UI identity、请求 / 完成、modal 和官方 resize。无代码变化不跑窗口测试；有 lifecycle composition 改动才验证真实场景。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

native 输入输出与 lifecycle 说明一致，官方 Component / Message 保留。

## 与前后方案的关系

在 Tooltip 之后、MessageBox 之前。MessageBox 依赖 Window lifecycle，先明确关闭和结果的边界。
