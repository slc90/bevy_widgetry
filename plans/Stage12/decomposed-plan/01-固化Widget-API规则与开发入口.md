# 固化 Widget API 规则与开发入口

## 目标

将已确认的输入 / 输出契约写入正式工程规则，让后续 Widget 开发能读取正确的约束。

## 范围

新增 rules/widget-api.md，修改 AGENTS.md 的条件读取入口、rules/architecture.md 的 ECS 更新边界，并仅澄清 rules/testing.md 的旧通知示例。本方案只规划这些修改；不实施控件迁移或在 docs 中提前宣称新行为。

## 规则落地范围

本方案把讨论中已经确认的行为契约转成可被后续开发读取的工程规则。plans 继续保存规划和历史讨论，不成为开发的默认上下文。这里只规划规则修改，不在本次拆分中实施。

新增 rules/widget-api.md，明确 Widgetry 自有 runtime state 的支持更新入口、只读查询、提交后通知、同值、显式清空、无效目标与例外。它不重新定义每个控件的 event type / target；Table 具体 resize payload 和其他未决 API 留在各控件设计中。下文完整保留已确认规则、理由、官方对照、例子和未决事项，正式规则只把其中已确认的契约写成要求。

AGENTS.md 的条件读取列表新增：“涉及 Widget 的公开 API、state 更新或输入输出语义：rules/widget-api.md”。入口不重复详细契约，保留 plans 的上下文边界，不将拆分方案列为开发默认事实。

rules/architecture.md 的“ECS 运行时操作”保留 Commands / World 正常使用，但补充：Widgetry 自有 runtime state 的公开更新遵守 rules/widget-api.md；库内部维护、直接复用官方 Component 及其他正常 ECS 结构操作仍按各自契约。避免现有“可直接修改 state”被理解为允许外部绕过 Widget API。

rules/testing.md 的 Invariant 示例目前为“programmatic selection 不产生用户通知”。需仅澄清这个示例，避免新 state-change 语义被读成程序必须静默；建议替换为“Widgetry 自有 selection 实际变化时，先提交 state 再发对应变化通知；合法同值不发变化通知。”附近明确直接复用官方 Component 不纳入该例，不扩张自动化覆盖规则。

rules/code.md 已有错误上抛 / 宿主处理、合法 no-op 不伪造错误的要求，无需重写。新规则限定无效自有 state 更新目标不再属于合法 no-op，正常 observer 过滤、Option 缺失、asset 等待保留。rules/development.md、rules/documentation.md 等现有流程无需另造。

docs/architecture.md 描述当前事实，不在规则方案中把尚未实施的通知写成现状。控件实施时同步相关 rustdoc / docs / consumer；仅当实际 architecture 事实改变才同步 architecture，不能用规则变更掩盖现存迁移缺口。后续新 Widget 按该规则定义 state / API / event，各控件继续保留适合其语义的 identity、输出和副作用。

## 已确认规则及完整讨论边界

## 2. 已确认：state 更新入口与通知顺序

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

程序化设置引起的 state 变化也通过上述路径通知，不再默认采用静默 setter。状态变化事件表达已经提交的变化；不能依靠外部收到该事件后再回写 state，才完成本次更新。初始化与 Model repair 的行为按第 7 节保持现状，不纳入本轮调整。

调用方可以查询自有 state Component，并通过公开只读接口读取当前 state；公开支持的 runtime 修改入口由 Widget API 提供。外部主动替换或移除自有 state Component、破坏内部 hierarchy、销毁 entity 等 ECS 结构操作，不能视为正常 Widget state 更新 API，不承诺具有相同的变化通知行为。

“先提交 state”描述真实 state 的写入顺序，不要求对应的高亮、renderer subtree 或 layout 已在通知前完成同步，也不建立同一事件的多个 observer 之间的固定执行顺序。

## 3. 已确认：直接复用的官方 Component 保持原样

直接复用的 Bevy 官方 Component 不纳入本次自有 state 只读化与更新入口治理，保持其原有使用方式和行为。

例如 EditableText、Checked、ScrollPosition、InteractionDisabled 等官方 Component，不为了本规则新增 wrapper、修改入口限制、写入监测或事件转接等特殊处理。

因此，本规则不要求每个 Widget 的所有官方 Component 更新都改为先提交再通知，也不为统一接口而单独改造这些官方 Component 的契约。

## 4. 已确认：变化通知与重复操作

状态变化事件只在 state 实际改变时发出；重复设置相同 state 不发变化事件。

确认、重选等操作如需对外表达，使用独立的操作语义，不借用状态变化事件。此规则不要求为每个 Widget 新增确认或重选 event，具体需要哪些通知仍按各 Widget 的实际行为决定。

## 5. 已确认：不增加用户 / 程序来源字段

统一后的状态变化通知不增加 User / Programmatic 等 origin 区分。UI 操作与程序化设置都按已确认契约提交并通知，consumer 不依赖统一的来源字段。

各 Widget 的事件含义、target / source 与业务 identity 继续按各自契约表达，本轮不强制归并为同一种事件语义或 target。具体 type 形态尚未决定。

## 6. 已确认：Table resize 正常结束与取消分别表达

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

## 7. 已确认：程序清空通知，初始化与自动修复保持现状

通过 Widget API 显式清空 state，导致 state 实际改变时，先提交再发送对应的变化事件。例如 selection 从某个 item 变为 None，事件需要能表达清空后的未选中 state；已经为空时再次清空，不发变化事件。

初始化与 Model repair / 自动修复保持现有行为，不修改其通知规则，不为它们新增通知。程序删除 Model 数据后触发的自动 selection repair，仍属于保留现状的自动修复，不等同于调用 Widget API 显式清空 selection。

此规则针对适用 Widget 的程序显式清空，不要求原本必须保持选中的 Widget 新增空 state。

## 8. 已确认：setter 附带行为与执行时机保持现状

保留各 Widget setter 当前对 active、cursor、focus 和 reveal / scroll 的联动行为，不在本轮统一为“只修改目标 state”，也不为此拆分现有 API。

保留现有立即更新 World 与 Commands queue 更新两类入口及其执行时机，不在本轮统一调用形式或重新安排执行阶段。已经确认的“先提交 state，再发事件”在各入口实际执行更新时落实，不据此要求排队 API 在调用瞬间提交。

## 9. 已确认：无效程序更新目标返回错误

Widgetry 自有 state 更新 API 接收到无效目标时，作为错误返回，不再将其静默忽略或与合法同值合并为“未改变”。例如失效的 node Entity、stale item / Row / Column ID、越界 index，或不满足 API 要求的 Widget / source entity。

合法同值设置属于成功但 state 未改变，不作为错误，也不发状态变化事件。对于已有 Result<bool, BevyError> 入口，语义分别为：

| 结果 | 含义 |
| --- | --- |
| Ok(true) | 合法输入使目标 state 实际改变 |
| Ok(false) | 合法输入未改变目标 state，例如设置相同值 |
| Err(...) | 目标无效，无法执行请求 |

无效目标不执行请求的 state 修改，不发对应变化事件。此规则约束程序化 state 更新 API，不据此改变原始 UI 输入过滤、初始化、自动 repair 或直接复用官方 Component 的行为。

立即与排队入口的执行时机继续保持现状；排队入口在实际执行时反馈错误，不把函数调用瞬间视为最终目标校验完成。具体错误反馈 API 按各入口形态在实施设计中明确，本轮不要求所有入口改成相同 signature。

## 10. 后续尚未决定

以下内容继续讨论，本轮不提前确定：

- 哪些 Widget 需要独立确认 / 重选 event，以及具体命名与 payload。
- 事件是否携带 old value 或发起 view 等信息；不增加统一 origin 字段已经确认。
- 适用 Widget 的清空 API 与通知 payload；显式清空通知、初始化与 Model repair 保持现状已经确认。
- Table resize event 的具体 payload；正常 End 与独立 Cancel 已经确认。
- 具体 event type、Widget API 形态，以及实现只读查询的方式。
- 排队 state 更新入口具体如何反馈错误；无效目标作为错误、合法同值不作为错误已经确认。

后续实现应以届时已确定的完整契约为依据，同步正式规则、公共 rustdoc 与消费者，并完成必要验证。本文先保留讨论共识，不在本轮实施代码迁移。


## 规则与实现的过渡边界

规则落地不代表所有现有 Widget 已迁移。按下游方案逐个完成迁移、消费者同步、行为验证与必要 Review；不能在第一份方案完成时将 Stage12 或全部控件标为完成。读到现有实现差异时应明确它属于尚未完成的迁移，不能以历史 plans 自动放宽新规则。

初始化、Model repair、setter 的 active / cursor / focus / reveal / scroll、World 与 Commands 执行时机全部按已确认边界保留。不统一 disabled / read-only / modal，不统一 index / stable ID / Entity，不添加 binding、callback、通用 set_value 或统一 event 总线。自有业务数据 Model CRUD 与展示配置是否属于需迁移的 interaction state，应在受影响方案明确；不能只因其使用 Component 就冻结所有数据 / style 输入。

## 未决前提

具体只读 type、event payload、清空 API、排队错误反馈仍按原讨论保留。其实现设计不是本规则方案提前规定的公共 signature。Tree loader state、Waveform 外部 cursor 和 Icon mutable method 的适用边界在各自方案明确；不能无依据认作官方例外，也不能借拆分增加业务通知。

## 验证边界

规则方案是 Markdown 修改，检查 AGENTS 的入口是否能命中新规则、新旧规则无真实矛盾、官方例外和保留决定是否完整、链接 / 术语 / Markdown 格式是否正确。无需 Cargo、GUI 或独立 Code Review。

后续代码方案按项目 Type-Driven Development / Test-Driven Development 实施，使用真实 public API / UI stimulus，consumer observer 中验证已提交 authority。仅验证最终 state 不足以证明提交顺序。按实际影响选择 crate 与消费者检查；GUI 变化按 BRP 真实用户路径验收，性能敏感路径按 benchmark 规则验证；代码相关改动按 AGENTS 使用全新 reviewer 反复 Review 至 No review findings.。

## 盘点背景与差异的完整保留

以下表格和讨论项来自目标 1 / 2 的现状盘点，保留参数、ownership、生命周期、消费方式与原始问题的细节。它们是迁移前背景，不能覆盖上文已确认决定；例如 origin、统一 identity / disabled、初始化与 repair 新通知均不在本轮批准范围。未决项只在受影响方案明确前提，不自动成为新需求。

每个 Widget 按以下维度检查：

| 维度 | 需要明确的内容 |
| --- | --- |
| 构造输入 | 使用 BSN Props、官方 Component patch，还是 Scene factory；哪些参数必填 |
| 初始 state | 默认值是什么；在 Scene 展开还是后续 system 中建立；能否提前程序设置 |
| state authority | 当前值保存在哪个 Component / Model、哪个 entity；哪些只是 UI projection |
| 更新入口 | 直接修改 Component、调用 method、发出请求 event，还是修改外部 Model |
| 参数语义 | 当前 index、stable ID、业务 Entity、数值或 enum；ID 是否依赖 source |
| 执行时机 | 立即更新 World，还是排入 Commands；UI 何时同步 |
| 更新边界 | 同值、清空、越界、失效 entity/source、初始化前调用和连续调用 |
| 附带行为 | 是否改变 active、cursor、focus、scroll、Popup 或触发业务动作 |
| 输入限制 | disabled / read-only 是否阻止程序更新；整体与单项限制是否独立 |

现有 [SceneComponent 规则](../../../rules/architecture.md) 已明确：Props 只负责一次性初始化，Scene 展开后由对应 Component 承担 runtime state。本文沿用这项现有约束。

### 输入现状总览

| Widget | 构造输入 | runtime authority / programmatic update | disabled / 限制入口 |
| --- | --- | --- | --- |
| Button | @WidgetryButton；children 与 Node patch | 无业务 value；内容由调用方持有的 Text 等 Component 管理 | root 的 InteractionDisabled |
| 二态 CheckBox | @WidgetryCheckBox；默认未选中 | root 的官方 Checked；insert / remove Checked | root 的 InteractionDisabled |
| 三态 CheckBox | @WidgetryTriStateCheckbox；默认 Unchecked | root 的 WidgetryCheckState；set_state / cycle_state | root 的 InteractionDisabled；程序 API 仍可用 |
| RadioGroup | @WidgetryRadioGroup；固定 direct options | option 的 Checked；set_selected 接收当前 direct child index | root 的 InteractionDisabled；不支持单项 disabled |
| TextField | @WidgetryTextField；patch EditableText | root 的 EditableText；set_text 等官方入口 | root 的 InteractionDisabled；程序文本更新仍可用 |
| ReadOnlyTextField | @WidgetryReadOnlyTextField；patch EditableText | 与 TextField 相同 | read-only 身份构造时确定；程序更新仍可用 |
| ScrollArea | Props 配置 axis、scrollbar 与内容 | Viewport 的 ScrollPosition；直接更新或 WidgetryScrollIntoView 请求 | 无统一的 root disabled API；keyboard_scroll 是构造配置 |
| ListView | Props 提供 ListModel source、renderer 与 item_height | 独立 Model 管内容；root 的 WidgetryListViewState 管 selection / active；set_selected 接收 index | root 的 InteractionDisabled；Model 的 set_disabled 管单项 |
| ComboBox | Props 提供 ListModel source、renderer 与 Popup 配置 | 内部 ListView state 管 selection；set_selected 接收 WidgetryListItemId | root 的 InteractionDisabled；单项限制复用 ListModel |
| Tree | TreeView Props 提供 TreeModel source；业务 hierarchy 在外部 | TreeModel 管 selection / expanded；select / expand / collapse / toggle_expand | TreeView root 的 InteractionDisabled；不限制 Model API |
| Table | Props 提供 TableModel source、layout 与 style | Model 管数据；root state 管 selection / cursor；set_selection、layout / style Component 更新 | root 的 InteractionDisabled；程序更新仍可用 |
| Waveform | Props 提供 config、source 与 style | WaveformRuntime 管已提交数据；WaveformCursor 驱动读取；WaveformStyle 管外观 | 无通用 disabled 数据暂停入口 |
| Tooltip | Props 提供重复构造内容的 factory | 内部 hover state machine 控制显示；无公开 show / hide / set_content API | disabled anchor 仍可显示 Tooltip |
| Window | widgetry_window / owned_widgetry_window Scene factory | native Window 与平台窗口 API；控件配置在构造时决定 | native 能力限制与 modal relationship；无统一 root disabled API |
| MessageBox | widgetry_message_box Scene factory | 无公开结果赋值或 runtime button 组替换 API | parent pointer modal；不是通用 disabled |
| Icon | WidgetryIconProps 提供 path、max_size 与 color | WidgetryIcon Component 的 set_svg / set_color / clear_color | 展示内容，无独立 disabled 输入语义 |

以上三类 runtime 输入方式都已存在：直接修改官方 / Widget Component、调用 Widget 行为 API、更新独立 Model。采用不同入口不自动构成问题，需要继续比较它们表达的语义。

### 原盘点的输入差异

以下是已确认的差异，不在本目标中自动认定为 bug 或决定修改：

| 议题 | 当前差异 | 后续需要决定什么 |
| --- | --- | --- |
| selection 参数 | RadioGroup / ListView 用 index；ComboBox 用 stable ID；Tree 用 Entity；Table 用 RowId / ColumnId | 各种 identity 是否符合各自数据模型；同类操作是否需要补充明确入口 |
| 调用形式 | 三态 CheckBox / RadioGroup / ListView / ComboBox 接收 Commands；Tree / Table 接收 &mut World | 如何说明 queue 执行时机、成功反馈与错误；是否需要补入口 |
| selection 清空 | Tree 接收 None，Table 有 None；ListView / ComboBox 没有专门 clear method；RadioGroup 要求单选默认值 | 哪些 Widget 需要显式清空；是否保持必须选中等各自约束 |
| 同值设置 | 三态 CheckBox / 已初始化 RadioGroup / Tree select / Table selection 不改变 state；ListView 仍设置 active 并 reveal，ComboBox 委托此行为 | “设置值”与“设置 active / reveal”是否应明确区分 |
| 无效输入 | 部分 setter 静默 no-op；Tree select 返回 Ok(false)；Table selection 返回 Error | 区分普通参数失效、source 前置条件和内部 invariant 错误 |
| 初始化 | RadioGroup 默认第一项；ComboBox 仅一次性默认第一项；ListView / Tree / Table 默认无 selection | 何时建立默认值，如何保证首帧前显式输入不被覆盖 |
| authority 所属 | ListView / Table 每 view 独立；ComboBox 在内部 ListView；Tree 在共享 Model | 从公开接口能否清楚知道修改哪个 state、影响哪些 view |
| disabled 层级 | 常见控件用 root InteractionDisabled；ListModel 另有单项 metadata；Tooltip 保留 hover；ScrollArea 无统一 root disabled 契约 | 为适用的 Widget 明确用户输入限制与程序更新边界，而非要求所有类型行为相同 |
| 输入限制类型 | TextField read-only 保留 selection / copy；disabled 清除全部待处理编辑；modal 主要限制 parent pointer | 分别定义 read-only、disabled、modal 的含义 |
| 输入附带行为 | ListView selection 会 reveal；Table selection 不移动 cursor；Tree expand 可能启动 lazy loading | 在 API / rustdoc 中明确操作效果，避免程序输入被误认为简单赋值 |

在目标 2 完成之前，不根据“静默”一项就决定统一 event 机制。也不预先引入 binding、callback、总线或通用 set_value abstraction。

| 维度 | 需要明确的内容 |
| --- | --- |
| 输出语义 | 激活、value change、state transition、业务请求，还是操作完成 |
| type 与消费方式 | On<Event> observer、MessageReader，或查询公开 Component / Model |
| target / source | Widget root、内部子 Widget、共享 Model source，还是 native Window |
| payload | 新 value、index、stable ID、业务 Entity、宽度或结果 enum；是否带旧值 |
| 触发条件 | pointer、keyboard、程序 API、初始化、数据修复和 lifecycle |
| 重复操作 | 相同 value 是否通知；重选与确认是否独立表达 |
| 时序 | 发出时 authority 是否已更新；projection 是否已经同步 |
| 结束语义 | is_final、Start / End、取消或中断是否能区分 |
| 来源识别 | 能否区分用户与程序、不同 view、pointer 与 keyboard |
| lifecycle | disabled、失效 source、删除数据和 root 销毁是否通知 |

### 输出现状总览

| Widget | 主要对外输出 | target / source 与 payload | 当前主要触发边界 |
| --- | --- | --- | --- |
| Button | 官方 Activate | entity = Button root；无业务 value | 有效 pointer 激活或 focus 下 Space / Enter |
| 二态 CheckBox | 官方 ValueChange<bool> | source = root；value 为目标 Checked 状态；is_final | 用户切换；官方 SetChecked / ToggleChecked 请求也可通知 |
| 三态 CheckBox | ValueChange<WidgetryCheckState> | source = root；目标三态；is_final | 用户切换；set_state / cycle_state 静默 |
| RadioGroup | ValueChange<usize> | source = Group root；direct child index；is_final | 用户改选；程序选择和默认初始化静默 |
| TextField | 官方 TextEditChange | target = 同 root EditableText entity；不携带文本快照 | editor generation 与 layout generation 不一致时；不限于文本 mutation |
| ReadOnlyTextField | 同 TextField | 同上 | navigation / selection 等仍可能通知；用户文本 mutation 被过滤 |
| ScrollArea | 无独立公开滚动结果 event | 查询内部 Viewport 的 ScrollPosition | wheel、keyboard、scrollbar 与程序滚动改变 state |
| ListView | ValueChange<WidgetryListItemId> | source = view root；stable item ID；is_final | 有效 click 或 Space / Enter 确认且 selection 改变 |
| ComboBox | 转发 ValueChange<WidgetryListItemId> | source = ComboBox root；stable item ID；is_final | 内部 ListView 用户改选；重选关闭 Popup 但不再通知 |
| Tree | WidgetryTreeEvent | entity = Model source；Selected / Expanded / Collapsed / ChildrenRequested 携带业务 node Entity | 用户改选；用户或程序展开 / 收起；首次 lazy 请求 |
| Table | WidgetryTableEvent | entity = view root；Row / Column / Cell ID 或 resize 信息 | 用户 selection 改变；resize Start / 实际 width 变化 / End |
| Waveform | 无独立公开数据提交 event | 查询 WaveformRuntime 的 ranges、revision、reduced data 和 stats | 外部 cursor 驱动数据提交；失败通过 Result / 宿主错误处理 |
| Tooltip | 无公开显示结果 event | ShowTooltip / HideTooltip 为 crate 内部 type | 内部 hover timing 驱动 Popup lifecycle |
| Window | 官方 WindowCloseRequested、WindowClosed 等 Message | window = native Window Entity | 关闭请求与实际关闭；其他操作通过 native state / 平台能力体现 |
| MessageBox | WidgetryMessageBoxResultEvent | entity = dialog UI root；result 为 Ok / Yes / No / Cancel | 首次有效结果 button 激活；之后关闭 dialog |
| Icon | 无公开加载完成 / 内容变化 event | 通过配置输入与实际显示内容体现结果 | asset 加载及内部 materialization；无业务交互输出 |

### 原盘点的输出差异

| 议题 | 当前差异 | 后续需要决定什么 |
| --- | --- | --- |
| 用户与程序来源 | 大部分 Widgetry setter 静默；二态 Checkbox 官方请求、Tree transition 和 TextEditChange 可以含程序路径 | 每种输出是否表达用户操作、所有 state transition 或请求；是否需要 origin |
| state 更新顺序 | CheckBox self-update 排队；RadioGroup 另行转发；ListView / Table / Tree 先写 authority | value 通知到底是修改提议还是已提交通知；是否明确统一读 state 边界 |
| value change 与确认 | ListView / ComboBox 重选不发变化；ComboBox 重选仍关闭；Button Activate 不代表 value | 哪些控件需要独立确认或重选语义 |
| 文本变化 | TextEditChange 含 cursor generation，且无文本快照 | 是否需要严格 value changed / commit；如何保留官方编辑能力 |
| target identity | 常见输出指向 root，Tree 指向 Model，Window 指向 native Window | 如何明确 event ownership；共享 Model 是否需要携带 view |
| 组合转发 | Radio 与 ComboBox 同时存在内部和外部通知 | 公共 consumer 应监听哪个 type / source，如何避免重复统计 |
| payload | 通常只有新值 / ID，没有 old value、origin 或来源 view | 哪些信息确有业务用途，是否由 event 或 state 查询提供 |
| interaction 结束 | ValueChange 用 is_final；Table 用 Start / Resized / End；其他无结束通知 | 结束、提交、取消和中断是否需要分别表达 |
| 自动修复 | selection 删除 / 清空多为静默；Table resize lifecycle 中断可能发 End | repair 与用户变化是否应区分；静默变化如何被外部观察 |
| 销毁顺序 | MessageBox 通知后关闭；Table root 销毁不发 End | 对 lifecycle event 做什么承诺，observer 能读到哪些资源 |
| 无 event 的 state | scroll / active / cursor / runtime 数据主要靠查询 | 哪些需要新增通知，哪些保持 ECS state 读取就足够 |

这些差异不自动构成 bug，也不要求所有 Widget 采用同一 event type。目标 3 应把相同语义说清楚，再决定具体 type 与实现方式。

### 盘点验证的实际限度

目标 1 / 2 依据 2026-10-02 静态源码、rustdoc、既有测试和锁定 Bevy 0.19.1 依赖源码整理，未在本次重新运行 Cargo / Gallery。现有测试链接不等于重新验收通过，尤其不证明 observer 内读取时序。后续代码方案在实施时重新核对工作区与 public contract，不能把盘点当作长期运行时证据。

## 预期产出

正式规则准确承载已确认契约，入口指引可发现，现有 architecture / testing 示例不再造成误解；未决 API、官方例外和逐控件迁移边界完整保留。

## 与前后方案的关系

这是执行链第一份。后续每份控件方案独立包含其适用契约；规则完成后进入 Button。当前只是规划交付，不代表规则或任何控件已经完成实施。
