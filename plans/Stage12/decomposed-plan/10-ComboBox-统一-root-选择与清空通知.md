# ComboBox 统一 root 选择与清空通知

## 目标

使 UI 与程序选择 / 清空都在内部 authority 提交后向 root 通知，并保留各路径 Popup 附带行为。

## 范围

crates/combo_box 的 API、ListView 消费、Popup / Field 与测试，facade / Gallery。共享 ListModel、source identity 与 renderer 不变。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

两者都消费独立 WidgetryListModel<T>；source 与 renderer 必填、构造后固定，source 必须持续持有匹配 Model。业务内容通过 Model 的 push、insert、remove、move_item、clear、get_mut 更新。get_mut 成功开放 mutable value 前推进 revision，即使调用方最终没有改变 value；它不是按 value equality 判断更新。

**ComboBox** 非空 Model 在一次性初始化时默认选择第一项，已有 selection 优先。空 Model 初始化完成后再 push，不自动选择；选中项被删除后不自动改选。selection authority 位于内部 ListView，不在 ComboBox root 或共享 Model。

ComboBox::set_selected 接收 Commands、ComboBox Entity 与 stable WidgetryListItemId，在执行时解析所属 Model 的当前 index，再委托 ListView 更新。无效 root / 当前 Model 中不存在的 ID 为 no-op；内部 shell 缺失则报错。它不关闭 Popup，Field 在后续 PostUpdate 从真实 selection 派生；也没有专门的清空 selection API。

ListView / ComboBox 的整体 disabled 在 root 设置 InteractionDisabled；单项 disabled 通过 ListModel::set_disabled(index, bool) 设置持久 metadata。整体 disabled 不修改 metadata；两类限制都不阻止程序选择或 Model CRUD。共享 Model 的 views 共享内容和 item metadata，各自保留 selection。

WidgetryListItemId 是 model-local ID；不同 Model 可以产生相同数值。调用方必须同时保留所属 source，不能用裸 ID 判断跨 Model ownership。

依据：[ListView](../../../crates/list_view/src/view.rs)、[setter 行为](../../../crates/list_view/src/behavior.rs)、[ListModel](../../../crates/list_view/src/model.rs)、[ComboBox](../../../crates/combo_box/src/combo_box.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

输出与 ListView 相同的 ValueChange<WidgetryListItemId>，但显式将 source 改为 ComboBox root，并原样转发 value / is_final。内部 ListView 的通知也存在；App 级 observer 可能看到内部 source 和外部 source，必须按目标过滤。

正常用户改选时，内部 selection 已由 ListView 写入；ComboBox 转发前先隐藏 Popup，并清除仍停留在内部 ListView 的 focus。Field 内容随后从真实 selection 派生，不保证在 root ValueChange observer 中已完成新的 renderer subtree。

有效 pointer / keyboard 重选当前项会关闭 Popup，但不另发 ValueChange。Escape、outside click、disabled 或空 Model 导致关闭，也没有独立公开 Popup closed / selection confirmed 通知。

程序 set_selected 不通知，也不关闭 Popup；默认初始化、删除选中项造成 selection repair 不转成用户通知。转换 observer 消费的是 event，因此外部人工 trigger 内部 ValueChange 也可能被转发，但并不能替代真实 authority 更新。

依据：[root 转发](../../../crates/combo_box/src/combo_box.rs)、[Popup 与重选](../../../crates/combo_box/src/popup.rs)、[authority 测试](../../../crates/combo_box/tests/selection_field.rs)。

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

内部 ListView state 保持唯一 authority，不增 root 或共享 Model 的 selection 副本。set_selected 继续接受 WidgetryListItemId，在 queue 实际执行时按所属 Model 解析，保留 ListView 的 active / reveal 效果。

程序实际改选也向 ComboBox root 通知；disabled 允许程序更新，现有只对 enabled root 转发的 guard 不能吞掉合法程序通知。同值不发 selection 变化，失效 root / source、缺失内部 shell、Model 不存在的 ID 应反馈错误且不修改 state。

补齐显式清空入口与 None 通知，已空不发变化。非空 Model 一次性默认第一项、已有 selection 优先、空 Model 初始化后 push 不自动选、删除选中项后不自动改选，以及 repair 的通知行为全部保持；Model::clear 不自动成为主动清空通知。

UI 改选与有效重选继续关闭 Popup；程序 set_selected 继续不关闭。handle_value_change 当前同时关闭与转发，不能把新的程序通知直接接进来改变附带行为。内部操作处理与公开已提交变化通知须明确各自职责，具体机制待实施设计，公开 payload 不增加 origin。

Field 在后续阶段从真实 selection 派生，不保证 root observer 中新 renderer subtree 已显示。内部 ListView 与 root 通知仍可能同时存在，consumer 按 type / source 过滤；每次实际 root selection 变化只发布对应通知，人工 trigger 内部 event 不替代 authority 更新。Escape、outside click、disabled / 空 Model 关闭的既有行为不改，不增加 closed / confirmed event。

## 未决前提与方案缺口

清空的 active / focus / Popup 边界、root payload 需与 ListView 设计衔接。程序不关闭、UI 改选 / 重选关闭已确认，不能靠新增公开 origin 解决内部路由。排队错误反馈、old value、独立确认 / 重选尚未决定。

## 验证意图

分别用 UI 和程序 API验证 root observer 读取真实 selection、实际变化与同值、清空 / 重复清空。程序设置不关闭已打开 Popup，UI 有效改选 / 重选关闭；disabled 程序通知可达，内部 / 外部 source正确过滤，Field 后续收敛。错误不破坏 selection / active，初始化 / repair 与共享 Model 独立 selection 保持。Gallery 真实 pointer / keyboard 验证 Popup、focus 和变化显示。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

root 通知覆盖合法程序选择与清空，唯一 authority 和既有 Popup / focus 语义保持。

## 与前后方案的关系

依赖已完成 ListView 的只读 state 与清空表达。下一份 Tree 同样消费 ListView，不依赖 ComboBox；这里仅规定线性顺序。
