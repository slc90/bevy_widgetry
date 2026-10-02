# Table 统一选择通知与 resize 结束取消语义

## 目标

统一自有 state 更新的已提交变化通知，区分 resize 正常结束与取消。

## 范围

crates/table state / interaction、Column width API、resize lifecycle、event / layout、测试及 facade / Gallery。保留 Model CRUD、renderer、style 与官方滚动。

## 现有输入与 authority

以下是 2026-10-02 盘点时的实现，不是迁移后的承诺。

WidgetryTableProps 提供固定 source，以及初始化 layout / style。runtime 使用独立 WidgetryTableLayout 与 WidgetryTableStyle Component，后续应修改这些 Component，不重写 Props。

独立 WidgetryTableModel<T> 管理 Row / Column 数据，通过 push / insert / remove / move / clear 更新各 Axis；row_mut 更新 Row 内容，set_column / set_header 更新 Column 定义。成功 row_mut 会推进 Row revision，即使业务 value 最终未改；Column 更新有独立 revision。Cell value 是 Row × Column schema 的 projection，不存在独立 set_cell value API。

每个 view root 的 WidgetryTableState 分别保存 selection 与 focused_cell，二者独立；公开 getter 只读。set_selection 接收 &mut World、view root 和 WidgetryTableSelection，支持 None、Row、Column、Cell，返回 Result<bool, BevyError>。同值返回 Ok(false)；无效 root / source 或 stale ID 返回 Error，失败保留原 selection。

程序设置 selection 不改变 cursor / input focus，也不自动 reveal；这与 ListView setter 的附带效果不同。程序滚动应定位 WidgetryTableBody 后修改其 ScrollPosition，Headers 的对应轴位置由 Table 同步。

整体 disabled 通过 root InteractionDisabled 设置，程序 selection、Model 与 layout 更新仍可执行。当前没有 Table 自有的 Row / Column / Cell disabled metadata API。RowId / ColumnId 都是 model-local identity，仍需所属 source 一起解释。

依据：[Table 构造与公开 API](../../../crates/table/src/view.rs)、[interaction](../../../crates/table/src/interaction.rs)、[Model](../../../crates/table/src/model.rs)、[layout](../../../crates/table/src/layout.rs)。

## 现有输出与通知边界

以下保留迁移前的触发、payload、时序及例外；后续以本方案的改动范围更新 public contract。

输出 WidgetryTableEvent，entity 为 Table view root，kind 分为 selection 与 resize 两组。

selection 包括 RowSelected(RowId)、ColumnSelected(ColumnId)、CellSelected { row, column }。有效 primary click 先写入 selection / focused_cell 并更新 root focus，仅在 selection 改变时通知；同值 click 仍可能调整 cursor / focus，但不再通知 selection。

四方向键只更新 focused_cell 和 reveal，不发 selection event。程序 set_selection、Model repair、selection 清空或 focus 丢失也不发 selection event；当前没有独立 cursor changed / selection cleared 通知。

resize 包括：

- ColumnResizeStart(ColumnId)：有效 primary DragStart 建立 gesture 后发出。
- ColumnResized { column, width }：实际 width 改变，写入当前 view 的 Fixed width 和 session 后发出；width 为 logical px。
- ColumnResizeEnd(ColumnId)：gesture 结束时发出，先移除 session；不携带最终 width 或终止原因。

正常 DragEnd 可以先提交最终 distance 对应的 Resized，再发 End。Cancel、disabled、Column 删除、handle / Header lifecycle 失效也可结束 gesture；保留最后 width，不回滚。root 销毁则静默释放，不保证每个 Start 最终都有一个可观察 End。

程序直接改 layout width 不发 Resized；程序修改 Model、disabled 或 UI lifecycle 结束已有用户 gesture 时仍可能发 End。因此“程序设置 selection 与 repair 不通知”不能扩展为“所有程序或 lifecycle 行为完全不发 Table event”。

resize End 不区分成功、取消与失效；Column 已删除时 End 携带的 ID 也可能已无法在 Model 中解析。需要最终 width 时应结合当前 layout / 应用缓存，不能假设 End 总能查到仍然存在的 Column。实现会 flush End observer 的 Commands，允许其修改 Model 或销毁 root，然后再检查后续 projection 的有效性。

依据：[selection 与 event 定义](../../../crates/table/src/interaction.rs)、[resize lifecycle](../../../crates/table/src/resize.rs)、[相关测试](../../../crates/table/tests/interaction.rs)。

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

## resize 官方对照与已确认完整契约

### 6.1 当前官方事实

本次直接检查项目当前使用的 Bevy 0.19.1 本地依赖源码：

- bevy_window 的 src/event.rs：WindowResized 是尺寸变化 Message，包含 native window、logical width 与 height，没有对应的 WindowResizeStart / WindowResizeEnd / WindowResizeCancel。
- bevy_winit 的 src/state.rs：窗口 resize 处理先更新 Window resolution，再构造 WindowResized。
- bevy_picking 的 src/events.rs：DragStart / Drag / DragEnd 表达拖动过程，DragEnd 来自拖动时的 pointer release；Cancel 独立表示 pointer 交互取消。
- bevy_ui_widgets 的 src/slider.rs：拖动用 ValueChange 的 is_final 区分过程和结束；Cancel handler 处理 pressed 清理，不提供统一的 resize 取消结果协议。
- bevy_ui_widgets 的 src/scrollbar.rs：DragEnd 和 Cancel 分别结束 drag state，直接修改官方 ScrollPosition，没有独立的对外滚动完成事件。

因此，官方没有一套通用于窗口 resize 和所有 Widget 的 Start / Changed / End 协议。Table 的三个语义 event 建立在 picking drag 之上；不能把当前全部合并为 End 的行为视为官方要求。

### 6.2 当前项目差异

Table 使用 ColumnResizeStart、ColumnResized、ColumnResizeEnd。正常 DragEnd、Pointer Cancel、disabled、Column 删除或 handle 失效都可发同一种 End，root despawn 则静默释放；当前 End 表达的是 gesture 已停止，不能区分正常释放与取消 / 中断。

Window resize 调用官方 native drag resize，实际尺寸变化使用官方 WindowResized；ScrollArea 的 scrollbar 复用官方 ScrollPosition / drag 行为。按已确认的官方 Component 例外，不额外包装它们的结束通知。

依据：[Table gesture](../../../crates/table/src/resize.rs)、[Table event](../../../crates/table/src/interaction.rs)、[Window resize](../../../crates/window/src/title_bar/resize.rs)、[ScrollArea](../../../crates/scroll_area/src/style.rs)。

### 6.3 已确认规则

Table 的正常释放与取消 / 中断分别表达，保持与官方 picking 的 DragEnd / Cancel 语义对应：

- 正常 DragEnd：提交最终有效 width，再发送 ColumnResizeEnd；没有新 width 时不重复发送 ColumnResized。
- Pointer Cancel 或 gesture 因 disabled、Column / handle 失效而中断：发送独立的 ColumnResizeCancel，保留最后已提交 width，不默认回滚。
- root 销毁：不保证补发结束或取消通知。
- 程序 width 更新：按实际 state 变化通知，不伪造用户 drag 的 Start / End。

开始与实际 width 变化继续分别使用 ColumnResizeStart、ColumnResized。End 表达正常结束，Cancel 表达取消 / 中断；两者不承担 width 变化事件的职责。

当前自有连续 resize gesture 的统一范围主要是 Table。Window 与 ScrollArea 保留官方路径；CheckBox、RadioGroup、ListView、ComboBox、Tree 的离散选择 / 展开，以及 MessageBox 的一次性结果，不统一套用 resize Start / End。以后新增自有连续交互时，再按实际需要复用已确定的结束 / 取消语义。

## 需要修改与保持的行为

WidgetryTableState 已有只读 getter 和 immutable Component，可以沿用。selection 与 focused_cell 独立：程序 set_selection 不改 cursor / input focus、不自动 reveal；UI 同值 click 仍可改 cursor / focus但不重发 selection event；方向键只改 focused_cell / reveal，不新增 cursor changed。

set_selection 实际改变，包括 None 清空，先提交再发可表达结果的 WidgetryTableEvent。已有 Result<bool, BevyError> 保持 changed / legal unchanged / invalid 分工；错误 root / source、stale RowId / ColumnId、非法 cell 保留原 state。默认初始化、Model repair、focus 丢失的通知行为保持。

Column width 是 per-view 自有 runtime state。程序更新需要支持的 Widget API 和实际变化通知，直接修改 layout.columns 不继续被视为享有新保证的正常公开更新路径。只读范围、Fixed / Flexible 与 geometry 的关系尚需明确，不顺势重写整个 layout / style。

正常 DragEnd 提交最终有效 width，实际改变才发 ColumnResized，然后 ColumnResizeEnd。Pointer Cancel、disabled、Column 删除或 handle / Header 失效改发独立 ColumnResizeCancel，保留最后提交 width、不回滚。root despawn 不保证 terminal 通知；程序 width 不伪造 Start / End。

End / Cancel 不替代 width change。取消时 Column 可能已删除，consumer 不能假设 ID 仍可解析或 width 仍可查询。保留 terminal observer Commands 可改 Model / despawn root、后续 projection 重新校验的边界；加入 Cancel 后同样不能遗漏防护。

迁移 event kind、state 查询或 width API 时同步 exhaustive match、Gallery、facade 与测试，旧的中断即 End 断言改成对应 Cancel。Model revisions / source-local identity 不变；官方 ScrollPosition / InteractionDisabled 更新方式不变。

## 未决前提与方案缺口

selection clear event、Cancel payload、End 是否带 width / reason、程序 Column width API 尚未决定。Fixed policy 与实际 logical px、Flexible / viewport layout 引起的尺寸变化的通知边界需明确，不能把所有 layout 求解都升级为 resize event。focused_cell 的公开更新能力与 width 只读封装待设计，不增加独立 cursor event。

## 验证意图

selection observer 内读提交 state，覆盖 Row / Column / Cell / None、同值、stale ID、程序不改 cursor / focus / reveal、UI 同值 cursor 效果与 repair 原样。resize 覆盖真实 drag 最终提交、同 width 不重发 Resized、Cancel / disabled / Column / handle 失效、取消不回滚、root销毁无保证、程序 width 仅实际变化。terminal observer 改 Model / 销毁 root 后无 stale access。Gallery 用真实 drag / pointer / keyboard 验证，直接写 session / layout 不等价于用户路径。

## 实施与交付要求

本文件只定义迁移方案，当前尚未实施或验证。实际代码任务读取届时的 AGENTS、docs/architecture.md 与适用 rules，建立根目录进度记录，按 Type-Driven Development / Test-Driven Development 明确必要 type 与行为。API 改动同步 public rustdoc、facade、Gallery 及其他直接 consumer；消费者不能等到下一份方案才恢复编译。

验证由真实公开输入驱动，不把直接设置最终 Component 当作用户交互证明。根据实际影响选择 targeted tests 与必要 workspace fmt / check / Clippy / test / build，GUI 变化按 rules/gui-debugging.md 使用 BRP 真实输入与截图 / state 验证，性能敏感路径按 rules/benchmark.md 验证。存在代码相关改动时按 AGENTS 进行独立 Review，每轮使用新的 reviewer 并调用 code-review，直到 No review findings.。

如果最终只有文档澄清，采用链接 / 内容 / diff 检查，不为形式跑 Cargo / GUI 或启动代码 Review。保留原始 guard、初始化、repair、程序与 UI 附带行为，不能把本轮有限统一扩展到无关功能。未决前提解决及必要验证完成后才将对应方案标为完成。

## 预期产出

selection / 清空、程序 width、End / Cancel 及消费者迁移完成，独立 state 与生命周期保护保持。

## 与前后方案的关系

在 Tree 之后，无技术硬依赖；下一份 Waveform 不消费 Table，按整体顺序明确数据 runtime 边界。
