# Widget API 规则

## 自有 Runtime State

Widgetry 自有的 Runtime State 必须通过明确的 Widget API 进行公开更新，并通过只读接口提供查询。

适用范围包括 Selection、Active、Focused Cell、三态 State、展开状态等由 Widgetry 自行管理的状态。

外部不得通过直接替换或移除 State Component 绕过公开 API。破坏 Widget 内部 Hierarchy、Despawn 等 ECS 结构操作也不属于受支持的 State 更新方式，不承诺产生相同的状态变化通知。

Scene Props 只负责构造阶段的一次性初始化，不作为运行时 State 来源。

Model CRUD、展示配置及外部数据输入不因使用 Component 存储就自动受到上述只读限制，其更新方式由对应 Widget 的公共契约决定。

库内部维护及其他正常 ECS 操作仍按各自契约处理。

## 状态更新与变化通知

### 提交与通知

通过 Widget API 更新自有 Runtime State 时，必须先提交真实状态，再发出对应的变化通知。

用户交互和程序化操作遵守相同的状态提交原则。变化通知用于报告已经发生的状态改变，不作为要求外部再次写入 State 的更新请求。

只有目标 State 实际改变时才发送变化通知：

- 合法同值设置可以成功，但不产生变化通知。
- 显式清空引起状态改变时，先提交清空结果，再发送能够表达该结果的通知。
- 已为空时再次清空不产生变化通知。
- 初始化及 Model Repair / 自动修复保持各 Widget 既有的通知行为，不因本规则额外产生通知。

确认、重选、Activate 等操作语义不等于 State Change，不得借用状态变化通知表达。没有明确需求时，不新增独立的确认或重选 Event。

变化通知不统一增加 User / Programmatic 等 Origin 字段。具体 Event、Target、Source 和业务 Identity 由对应 Widget 定义。

State Authority 的提交不意味着高亮、Renderer Subtree、颜色或 Layout 已经同步，也不保证同一 Event 的多个 Observer 具有固定执行顺序。

## API 设计边界

各 Widget 根据自身语义设计更新、查询及通知接口，不引入统一的 `set_value`、Binding、Callback 或 Event 总线。

保留现有 API 的操作语义及附带行为，包括必要的 Active、Cursor、Focus、Reveal / Scroll 等状态调整，不强制将 Setter 简化为仅修改目标 State。

保留同步 `World` 操作与 `Commands` Queue 两类入口及各自的执行时机：

- 同步入口在执行时完成状态提交。
- 排队入口在 Command 实际执行时完成校验、提交与通知，不承诺入队时状态已经改变。

Disabled、Read-only、Modal、Index、Stable ID 与 Entity 等限制继续遵循对应 Widget 的既有契约，不为统一 API 形式而改变行为。

## Bevy 官方 Component

Bevy 官方 Component 默认保持原生使用方式，不强制套用 Widgetry 自有 Runtime State 的更新及通知规则。

当官方 Component 是复合 Widget 的内部实现，且直接修改它会破坏 Widget 明确承诺的行为约束时，允许该 Widget 将其声明为内部托管组件，通过 Widget 的公开 API 提供受约束的读写操作。托管范围及接口必须由具体 Widget 明确声明，不得维护重复的状态 Authority。未被声明为内部托管的官方 Component 继续保持原有使用方式。

TextField 托管 `EditableText` 的内容写入，以确保 Unicode 字素簇长度约束。通过控件根实体的内容 API 设置正文，不直接修改其编辑器；官方文本、光标、选区与 IME 仍是唯一真实状态。

例如：

- 未被 Widget 声明为托管的 `EditableText`
- `Checked`
- `ScrollPosition`
- `InteractionDisabled`

不得仅为了统一 Widget API 而增加官方 Component Wrapper、额外写入限制、写入监测或 Event 转接。

### Disabled 有效状态与继承

`InteractionDisabled` 的有效状态继承机制属于明确例外。

安装 Widgetry UI 能力后，受管理 UI 的有效 Disabled 状态由以下条件进行 OR 组合：

- 用户对当前 Entity 的本地 Disabled 请求。
- Widget Model 或原生能力施加的内部限制。
- 祖先 Entity 继承的有效 Disabled 状态。

稳定状态下，`InteractionDisabled` 表示实际生效的禁用结果，`WidgetryEffectiveDisabled` 提供对应的只读查询。

原生 BSN、`World` 和 `Commands` 的 Component Insert / Remove 仍然受支持：

- Insert 可以设置或更新本地 Disabled 请求，包括已经继承禁用状态时再次 Insert。
- Remove 撤销本地请求，但不能解除仍然存在的祖先或内部限制。
- 继承投影不得将计算出的有效 Disabled 状态误记为本地请求。

有效状态应在结构写入完成后的 World Flush 与正常输入派发前同步。尚未 Apply 的 Commands 不代表请求已经执行。

Disabled 投影不额外产生 Widgetry 业务状态变化通知，也不改变其他官方 Component 的使用契约。

## 无效请求与错误处理

程序化更新自有 Runtime State 时，以下无法满足 API 前置条件的请求必须明确返回错误：

- 失效的 Node Entity。
- 过期的 Item、Row 或 Column ID。
- 越界 Index。
- 不满足要求的 Widget 或 Source Entity。
- 其他由对应 Widget API 明确定义的无效输入。

无效请求不得静默忽略，也不得伪装成合法的未变化操作。失败时不得执行请求的 State 修改或发送对应变化通知。

对于使用 `Result<bool, BevyError>` 的接口：

- `Ok(true)`：目标 State 实际改变。
- `Ok(false)`：请求合法，但目标 State 未改变。
- `Err(...)`：请求无效或无法执行。

其他返回类型应按照自身 API 契约明确反馈结果。

排队入口在 Command 实际执行时校验，并通过宿主 Error Handler 反馈无法处理的错误，不要求入队时完成校验。

正常 Observer 过滤、`Option` 缺失、Asset 等待、UI Guard、初始化、自动 Repair 和 Bevy 官方 Component 的正常行为，不因此转换为错误。

错误诊断、`Severity::Error` 及传播遵循 `rules/code.md` 和 `rules/logging.md`。
