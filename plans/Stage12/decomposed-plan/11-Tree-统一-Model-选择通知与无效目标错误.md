# Tree 统一 Model 选择通知与无效目标错误

## 目标

使 Model UI / 程序 selection 和清空提交后通知，区分非法目标与合法无变化。

## 范围

crates/tree Model / behavior / view、public event、测试与 facade / Gallery。保留业务 hierarchy、shared source 和 lazy loading 模型。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

业务 node identity 是 ECS Entity，业务内容和 Children / ChildOf hierarchy 由调用方持有；WidgetryTreeModel::new(root) 关联不显示的 hierarchy 容器。TreeView Props 提供固定 source、行高、缩进与 icons。

selection 和 expanded authority 在 TreeModel；多个 TreeView 共享同一 source 时也共享这些 state。内部 ListModel、ListView state 与可见 rows 都属于 projection，调用方不应把内部 ListModel 当作独立业务输入修改。

公开程序入口为 WidgetryTreeModel::select、expand、collapse、toggle_expand，接收 &mut World 和 Model source Entity，返回 Result<bool, BevyError>。select 接收 Option<Entity>，None 清空；允许选择隐藏但仍可达的 node，不自动展开 ancestors。source / node 失效或同值返回 Ok(false)，不读取 view disabled。

expand / collapse 操作也不受 view disabled 限制，但与 select 不同，成功时会触发对应 transition；lazy Unknown 展开还会进入 Loading 并请求 children。程序输入并不全部静默，这一事实保留给目标 2 / 3 继续分析。

lazy loading 的外部输入是创建业务 children，并把 node 的 WidgetryTreeChildrenState 更新为 Loaded。普通 hierarchy mutation 由 plugin 重新生成 projection；不是直接手工改 visible rows。

TreeView root 的 InteractionDisabled 限制该 view 用户输入，并镜像到内部 ListView 和 expander，不冻结 Model 或其他共享 views。

依据：[TreeModel](../../../crates/tree/src/model.rs)、[程序行为 API](../../../crates/tree/src/behavior.rs)、[TreeView](../../../crates/tree/src/view.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

输出 WidgetryTreeEvent，entity 是持有 WidgetryTreeModel 的 source，而非 TreeView root。所有 kind 的 payload 都是业务 node Entity：Selected、Expanded、Collapsed、ChildrenRequested。

**Selected**：有效用户 ListView 改选经当前 Model 解析为 node，WidgetryTreeModel::select 成功改变 selection 后触发。程序 select、清空或删除 node 后 repair 不发 Selected；同一 selection 重选不通知。

**Expanded / Collapsed**：展开 / 收起意图改变并成功同步 Model projection 后触发；既可由用户 expander，也可由程序 expand / collapse / toggle_expand 产生。同值、无效 node 和无需展开的普通 leaf 不产生 transition；失败路径不把失败操作发成成功通知。

**ChildrenRequested**：lazy Unknown 首次有效展开时，node 已置为 Loading，先 trigger Expanded，再 trigger ChildrenRequested。collapse / re-expand Loading node 不重复请求；外部 loader 完成后创建 children 并写入 Loaded，当前没有对应公开加载完成 event。

event 没有 origin、TreeView identity 或 old state。多个 views 共享同一 Model 时，consumer 知道 source 和 node，但不能仅从 event 判断是哪一个 view 或程序调用发起。TreeView disabled 只限制该 view 用户路径，不阻止程序 Model transition 通知。

这些通知前已更新各自 Model authority；Selected 后其他 view 的 selection projection 仍可能等待同步。Expanded 与 ChildrenRequested 是两次独立 trigger，不应当作带全部结果快照的单一事务输出。

依据：[event 与 Model 行为](../../../crates/tree/src/behavior.rs)、[用户选择转换](../../../crates/tree/src/view.rs)、[Model 行为测试](../../../crates/tree/tests/model_behavior.rs)。

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

TreeModel 管 selection / expanded，多个 views 共享同一 source；内部 ListModel / ListView 与 visible rows 是 projection。select 与 UI handler 应在同一 Model 提交边界通知，避免 select 开始通知后 UI 再重复发 Selected。

select(None) 从已选 node 清空后发布可表达无 selection 的通知；同值或已空为 Ok(false)。继续允许隐藏但可达的 node，不自动展开 ancestors / 移动 view focus；保持 &mut World 立即执行。

source 不存在 / 不是 TreeModel、node 已删除或不属于真实 hierarchy，现有 Ok(false) 改为 Err。expand / collapse / toggle_expand 的非法目标同样区分；有效重复展开 / 收起、无需展开的普通 leaf 仍合法无变化。失败不发成功 transition，保留现有成功同步与失败回退边界。

Expanded / Collapsed 已涵盖 UI 与程序，保留提交后通知；lazy Unknown 有效 expand 后进入 Loading，先 Expanded 再 ChildrenRequested，collapse / re-expand Loading 不重复请求。不增加加载完成通知。

Model 删除后的 selection repair 和内部同步仍保持现状。Model 反向同步内部 ListView 不能又转成用户请求或重复 Model 通知，保留 ListView 迁移时的组合保护。target 继续 Model source，不强制改 view root、不增加 origin。

WidgetryTreeChildrenState 是自有 Component，现有 loader 直接写 Loaded 和外部创建 hierarchy 的输入边界需明确。不能把它写为官方例外，也不未经决定重写 lazy lifecycle。

## 未决前提与方案缺口

Selected 只能携带 Entity，清空的 kind / payload 未决定，不选择 Option 或独立 cleared kind。TreeModel 的只读访问和内部更新封装需核对；WidgetryTreeChildrenState 如何通过 loader API 更新并提供只读查询，是必要的实施前提，尚未确定。不得借此添加 Loaded 完成 event、old value 或 view 字段。

## 验证意图

source observer 内读已提交 selection / expanded / Loading。覆盖 UI / 程序只发对应 Model 通知、隐藏可达 node、同值、None / 重复清空、合法 leaf no-op、stale node / source / 非法 hierarchy 返回 Err 且不通知。保护 shared views 反向 projection 无循环 / 重复、lazy 顺序与幂等、失败回退、disabled view 不冻结 Model、初始化 / repair 原样。Gallery 真实展开 / 收起、keyboard / pointer 选择与 lazy loading。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

Model selection / 清空、transition、错误契约一致，projection 与 lazy lifecycle 边界保持。

## 与前后方案的关系

在 ComboBox 之后，技术依赖来自 ListView 而非 ComboBox。后续 Table 使用独立 Model / state，不依赖 Tree，只沿用规则与顺序。
