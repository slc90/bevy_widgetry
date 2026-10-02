# ListView 统一只读 state 与程序选择通知

## 目标

收敛自有 selection / active 的公开更新入口，使程序选择和显式清空在 state 提交后通知。

## 范围

crates/list_view 的公开 state / behavior、必要 style / navigation 消费，facade、Gallery 与 ComboBox / Tree 必要适配。ListModel CRUD、renderer、官方 ScrollPosition / InteractionDisabled 保留。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

两者都消费独立 WidgetryListModel<T>；source 与 renderer 必填、构造后固定，source 必须持续持有匹配 Model。业务内容通过 Model 的 push、insert、remove、move_item、clear、get_mut 更新。get_mut 成功开放 mutable value 前推进 revision，即使调用方最终没有改变 value；它不是按 value equality 判断更新。

**ListView** 默认 selection / active 为 None。root 的 WidgetryListViewState 是 authority，可见 row 的 Selected 只是 projection。set_selected 接收 Commands、view Entity 与当前 index，在 queue 执行时把 index 解析为 stable ID，同时设置 selected、active 并请求 reveal。

因此 ListView 的“设置 selection”还包含 active 与 scroll 效果；同一 selection 再设置也不是严格 no-op。公开 state 的 selected / active field 可以直接修改，但这种写入不能视为与 setter 完全等价，尤其不应假设会执行相同的 reveal。当前没有专门的 clear_selection method。

ListView / ComboBox 的整体 disabled 在 root 设置 InteractionDisabled；单项 disabled 通过 ListModel::set_disabled(index, bool) 设置持久 metadata。整体 disabled 不修改 metadata；两类限制都不阻止程序选择或 Model CRUD。共享 Model 的 views 共享内容和 item metadata，各自保留 selection。

WidgetryListItemId 是 model-local ID；不同 Model 可以产生相同数值。调用方必须同时保留所属 source，不能用裸 ID 判断跨 Model ownership。

依据：[ListView](../../../crates/list_view/src/view.rs)、[setter 行为](../../../crates/list_view/src/behavior.rs)、[ListModel](../../../crates/list_view/src/model.rs)、[ComboBox](../../../crates/combo_box/src/combo_box.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

输出 ValueChange<WidgetryListItemId>，source 为 ListView root，value 为所属 source-local stable item ID，正常用户路径 is_final = true。

primary click 命中有效未 disabled item，且 selection 与该 ID 不同时通知。Space / Enter 在 active item 有效、未 disabled 且 selected != active 时提交 selection 并通知。Arrow / Home / End 更新 active，PageUp / PageDown 滚动，不输出 selection ValueChange。keyboard handler 不排除 repeat；连续确认同一 active item 通常因 selection 已相同而不再通知，不能把它概括为统一禁止 repeat。

实现先写入 WidgetryListViewState、navigation 及相关 focus / reveal state，再 trigger ValueChange。consumer 可以查询 logical selection，但 row 的 Selected 和 ActiveDescendant 等 projection 仍由后续同步维护。

同一 selection 重选不通知；item disabled 仍可成为 active，但不能由用户确认成 selection。set_selected、直接 state 更新、初始化以及删除失效 item 后的 state repair 都不会自动发用户 ValueChange。

当前没有独立 active changed、reselected、selection cleared 或 item removed event。需要观察这些 state 时，调用方读取 WidgetryListViewState / Model，而不是依靠 ValueChange 覆盖所有变化。

依据：[用户与程序行为](../../../crates/list_view/src/behavior.rs)、[state 定义](../../../crates/list_view/src/view.rs)、[相关测试](../../../crates/list_view/tests/behavior.rs)。

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

WidgetryListViewState.selected / active 当前公开可写，需要提供只读查询和正常 runtime 更新所需的 Widget API。pointer、keyboard 与程序 setter 在同一真实 state 提交边界产生 selection 通知，不再只表达用户选择。active 更新也须有合法入口，但不借此增加尚未确认的 active changed event。

set_selected 继续接收 index，在 Commands 实际执行时解析 source-local stable ID；不统一 identity。保留 selected / active 联动和 reveal，同一 selection 再设置仍可改变 active / reveal，但不重复 selection 通知。root / item disabled 仍允许程序设置，用户 guard 不改。

补齐适用的显式清空能力，selected 从某个 ID 到 None 时通知，已空再次清空不通知。清空对 active / reveal 的影响尚未确定，不默认重置全部 navigation。初始化与 Model 删除后 repair 保留，Model::clear 不冒充主动 Widget selection 清空。

无效 view、source / Model type、执行时越界等应反馈错误；失败不能先修改 selected / active 或请求 reveal。ID 必须结合所属 source 解释，不把裸数值认作跨 Model identity。Model get_mut / revision、metadata 与 CRUD 契约不变。

public state / event 改形状时，在本方案完成 ComboBox、Tree、facade、Gallery 和测试的必要编译及组合适配，不能等后续方案才修断开的消费者。已发现普通消费者直接读 field、测试 get_mut / insert state，普通操作迁移到合法 API；专门验证结构破坏的场景仍可明确保留绕过操作。

组合适配需保留 ComboBox 一次性默认选择及 Tree 内部 projection 同步的既有静默行为，不能通过开始通知的公开 setter 引出额外业务通知。内部 projection 不是另一个 authority，不增公开 silent setter 供外部绕过新契约。

## 未决前提与方案缺口

只读实现、active 更新 API、清空 API 及 active / reveal 行为、None 的通知形态、排队错误反馈尚未决定。ValueChange<WidgetryListItemId> 不能携带 None，实施 type 设计需明确并迁移消费者。old value、发起 view、独立 reselected / confirmed 仍未决定，不增加 origin。

## 验证意图

observer 内查询已提交 selected / active；覆盖 UI click / Space / Enter、queue 程序选择、同值无 selection event但保留 active / reveal、非空清空 / 重复清空、无效 source / 越界及入队后 Model 变化。保留 Arrow / Home / End active、Page scrolling、用户 disabled guard、允许 disabled 程序设置、初始化 / repair 和 shared Model 每 view 独立。Gallery 真实输入验证 selection、focus / reveal、清空 projection，验证 ComboBox 初始化与 Tree projection 不新增业务通知。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

只读查询、Widget 更新、程序选择 / 清空通知和错误契约一致；直接消费者可编译且组合边界保持。

## 与前后方案的关系

承接 ScrollArea 官方例外。下一份 ComboBox 依赖本方案；Tree 也消费其 state / event。必要跨 crate 适配随本方案完成，后续方案再分别迁移各自公开契约。
