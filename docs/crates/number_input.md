# NumberInput

NumberInput / SpinBox 共用 `WidgetryNumberInput<T>`，正式值始终以原始 `T` 保存。支持 `i8/i16/i32/i64/i128/isize`、`u8/u16/u32/u64/u128/usize`、`f32/f64`，由 sealed `WidgetryNumber` 描述数值能力。`WidgetryNumberInputPlugin` 注册所有支持类型的运行时反射。

`WidgetryNumberInputHeadless<T>` 提供相同数值与编辑行为，不创建默认外框或加减按钮。默认控件组合该能力，值与提交仅有一条路径。两者共用 `WidgetryNumberInputProps<T>`；Headless 上公开 `value`、同步/排队 `set_value`、`field_entity` 和 `editor_entity`，以 Headless 根作为数值身份。

通过 BSN 在数值根挂载不可变的 `WidgetryNumberInputEditor { field }`，指向其后代的公开 TextField Headless 根。TextField 可以用任意层数 Node 包含官方 Editor，不要求默认 Frame。绑定必须处于该数值根内，TextField 的构造期只读模式必须与 NumberInput 相同；失效绑定通过领域错误反馈。外部绑定在首次输入派发前初始化，运行中新增的实例在 Materialize 初始化；之后普通刷新不覆盖草稿。外部 TextField 的布局、边框和颜色由调用方拥有，默认三段样式不处理这些实例。

绑定 Editor 必须使用 `max_length: None`、`multiline: false`。数值的完整精度与固定小数位投影不能被文字长度限制截断，Enter 必须交给数值根提交。初始化、查询 Editor 及 Setter 均验证这一前提，`IncompatibleEditor` 保留根、TextField、Editor、长度上限与多行配置；拒绝发生在写入之前，不修改正式值或草稿。

通过 BSN Scene 构造，`@value: Some(value)` 必填；`min/max` 默认无应用范围，`step` 默认为同类型的一。值、端点和步长必须有限，步长严格为正，`min <= max`，初始值必须位于范围内。配置构造后固定；Props 不保留为运行时状态。非法构造通过 Scene 错误通道报告并清理预约根。

`value(&World, root)` 查询正式值，返回领域结果；`set_value_in_world(&mut World, root, value)` 返回 `Result<bool, NumberInputFailure>`；`set_value(&mut Commands, root, value)` 在执行时校验和提交，失败交宿主 Error Handler。Setter 只校验有限性和范围，不要求步进对齐，也不受 Disabled 限制。

`NumberInputFailure` 为 `WidgetryFailure<NumberInputError>`。数值失败保存 `NumberInputConfigSnapshot` 中的类型名、请求值、min/max、step 和 decimals；缺少显式初值、非有限范围、逆序边界、无效步长、整数误配小数位和越界值分别可匹配。`NumberInputValue` 分为 Signed(i128)、Unsigned(u128)、Float32(f32)、Float64(f64)，保留所有支持类型的整数精度及原始浮点位，不统一转为 f64；sealed `WidgetryNumber::diagnostic_value()` 提供该精确快照。

结构错误保存 Root、field / editor 或缺失部件及操作，TextField 失败继承原始领域 source、origin / ID 与回溯，仅增加 NumberInput 上下文。源头一次 ERROR，不在传播时重复记录；只在 Template / Command / System 执行边界显式转为 `Severity::Error`。同步投影若在文字写入后发现展示状态损坏，会在诊断中标明已写入；普通半成品输入、用户提交拒绝、失焦恢复、ReadOnly / Disabled Guard、端点 Clamp 和合法同值不产生错误或虚假处理 DEBUG。

仅真实变化在提交后发送 `ValueChange<T>`，`source` 为 NumberInput 根，`is_final=true`。初始化、合法同值与失败静默。无效目标、类型不匹配、缺失内部状态或非法值均明确报错，失败保持旧值。内部状态通过根 API 管理，不支持替换或移除。

## 文本编辑与提交

安装 `WidgetryNumberInputPlugin` 会自动安装组合所需的 TextField Plugin。内嵌 TextField 使用官方编辑器处理光标、选择、剪贴板与 IME；根实体保持唯一正式数值及公开 API 身份。草稿允许空串、负号或任意临时文字，键入期间不改变正式值。

仅复用行为时安装 `WidgetryNumberInputHeadlessPlugin`，它注册全部支持类型的编辑与步进，并安装 TextField Headless 和原生 Button 输入，不安装 NumberInput 或 TextField 的默认 Style。默认 Plugin 自动组合它与原有视觉层及长按，不要求应用重复注册。

Enter 仅提交合法草稿；非法草稿保留。键盘焦点离开控件时，合法草稿提交，非法草稿恢复正式值文本。Esc 丢弃草稿、恢复正式文本并保留编辑焦点。鼠标移出不触发提交。合法同值不发数值通知。

只接受普通十进制整数/小数，不接受科学计数法、进制前缀、千位分隔、空白、正号或非有限浮点文本。无符号整数拒绝负号，整数拒绝小数，所有类型按自身范围解析；手动值不要求步进对齐。浮点 `.5`、`-.5`、`1.` 可提交并规范化为 `0.5`、`-0.5`、`1.0`。整值浮点保留 `.0`。

合法程序化 Setter 立即覆盖草稿，合法同值也清理旧 Edit/Paste/IME 请求；无效 Setter 保持数值和草稿。文字替换使用 TextField 公开内容 API，不维护第二份文本数值 Authority。

## 浮点显示精度

浮点构造可选 `decimals: Option<u8>`（0–255），整数配置该项会明确失败。没有运行时 decimals Setter。失焦时按指定小数位数显示，聚焦时使用可往返恢复原类型数值的完整普通十进制文本；未配置时始终使用完整规范文本，浮点整值保留 `.0`。

显示舍入不改写正式值；焦点切换不发变化通知。极小次正规数、最大有限值及负值同样显示不含指数的完整规范文本，文本长度由 TextField 视口处理。输入草稿不会被普通刷新反复覆盖，焦点投影保留同帧待处理编辑。

## 单步输入

`WidgetryNumberInputHeadless::<T>::step_in_world(world, root, increase)` 与排队 `step(commands, root, increase)` 提供用户单步操作；`increase=true` 增加，`false` 减少。同步结果为 `Result<bool, NumberInputFailure>`，真实变化返回 true，端点同值或 ReadOnly/Disabled/IME 守卫返回 false，无效根或绑定返回错误。排队请求在执行时校验。它与合法程序 `set_value` 的权限不同，不能绕过只读、根/编辑区/Editor 有效禁用以及输入法组合态。

在任意触发器上通过 BSN 挂载不可变的 `WidgetryNumberInputStep { root, increase }`，将其 `Activate` 关联到唯一数值根。关联不创建按钮或默认样式；可组合原生 `Button + ActivateOnPress + WidgetryPointerPressed + Hovered` 保留按下单步，也可使用触发器自己的公开输入能力。触发器可以位于数值根外、只有一个，或多个方向指向同一根；获取焦点时会转到绑定 Editor，避免先失焦提交草稿。不同原类型根共用同一调度入口，仍按各自 T 运算。失效关联在实际执行时通过宿主错误通道报告。

没有按钮的 Headless 编辑器也支持上下方向键。外部触发器在本阶段只执行单步；默认按钮继续组合既有长按。

可聚焦的外部触发器会将 `AcquireFocus.focused_entity` 一并重定向到 Editor，避免原生 TabIndex Observer 覆盖焦点。IME 守卫区分实际组词与普通清理：活动组合态或非空待处理预编辑阻止步进，空的清理请求照常处理，不丢弃用户动作。

数值根、TextField 或 Editor 被禁用、或根为 ReadOnly 时，触发器仍保留绑定 Editor 的焦点归属，避免拒绝的步进经失焦间接提交草稿；这不会解除文本的禁用/只读限制，实际步进仍返回合法无变化。

左右 `−/+` 输入区域使用原生 Button，在按下时各执行一次；聚焦编辑器时 `↑/↓`（含键盘重复）调用相同的步进路径。IME 组合输入期间方向键仍归输入法，不执行数值步进。内部按钮获取并保留文本焦点，不提前提交草稿。

合法草稿作为步进起点，非法草稿回退至旧正式值，然后按原类型 `step` 安全增减，截断到显式范围和类型边界。整数不回绕，浮点不溢出为无穷。最终值仅一次提交/通知，不暴露草稿中间值；最终同值静默。端点仍可操作，不自动 Disabled。

例如 `0..10, step=3` 可达 `0→3→6→9→10`，反向从 `10→7→4→1→0`。鼠标滚轮不改变数值，也不截断父 ScrollArea 滚动。Disabled 阻止用户步进，合法程序化 Setter 仍可执行。

按住按钮立即步进一次，持续约 400ms 后开始每约 100ms 重复一次。计时使用实时时间，不受游戏时间暂停影响；掉帧后每帧最多尝试一次，不追补积压周期。按住期间请求后续帧，支持 Reactive 桌面调度。每次复用单步提交路径，端点同值不通知也不禁用按钮。释放、Pointer 取消或有效 Disabled 会停止计时，恢复可用后需重新按下；同帧释放后重按同样重新等待 400ms。

连续真实变化可能每秒通知约十次；昂贵的消费方工作可自行合并或延迟处理，再用公开 `value()` 查询最新值。

原生 Mouse 按住期间所属窗口失焦会终止重复，重新聚焦不续接旧计时；Custom Pointer 继续遵守独立的 Pointer 取消/释放语义，不以 OS 窗口焦点作为有效性条件。

## ReadOnly 与有效 Disabled

构造时可配置 `read_only: true`（默认 `false`），创建后不提供模式切换入口。只读实例组合 `WidgetryReadOnlyTextField`，保留选择、复制及完整精度焦点显示；不接受文字/Paste/IME 修改、方向键步进、按钮或长按。两侧按钮保持显示并禁用，文本仍可聚焦；整个控件的根身份、泛型值与公开 API 不变。

根或祖先的 `InteractionDisabled` 通过共享有效 Disabled 继承阻止内部文本及按钮交互。进入禁用时立即放弃未提交草稿、待处理 Edit/Paste/IME，并显示当前正式值的规范文本；长按取消，重新启用不恢复旧动作，也不在之后失焦时补交旧草稿。禁用时不保留用户选择/复制能力。

ReadOnly 和 Disabled 均不限制程序化 Setter：合法变化仍先提交后通知，同值不通知，无效值保持数值与草稿。模式与焦点显示变化不额外产生 `ValueChange<T>`。

## 三段布局

控件固定为水平 `− / 数值 / +`，根提供一圈 1px 外框与 4px 圆角，按钮各有一条内部分隔线，内部 TextField 不重复绘制边框。两侧按钮始终显示，宽度各 36px；中间编辑区占剩余宽度，数字在普通显示和编辑时均水平居中。

按钮仅在外侧使用贴合外框内沿的 3px 圆角，避免 Normal / Hover / Pressed / Disabled 背景盖住共同圆角；靠中间分隔线的角保持直角。

默认整体宽度 200px、最小高度 38px、最小宽度 134px（含外框、两个按钮及至少 60px 的中间区域）。通过根的 `Node.width` 调宽只扩展中间区域；极窄父容器下仍保留控件最小宽度，超出部分由调用方决定裁剪或滚动。长数值沿用 TextField 的官方视口与光标滚动，两侧按钮不随文本伸长或收缩。

## 主题与交互反馈

整体外框、输入文字和选择/光标复用当前 Theme 的 `text_field` 配色，ReadOnly 使用其只读调色板，内部文本编辑器聚焦时整个外框显示焦点边框。两侧按钮复用 `button` 的 Normal / Hovered / Pressed / Disabled 配色；Hover 和 Pressed 只改变本侧背景与前景，不改变几何。两条分隔线随整体边框更新。

NumberInput 统一拥有内部 TextField 的样式，按钮文字通过共享前景继承刷新。四套 Theme 切换、焦点、Hover、Pressed 和有效 Disabled 变化只更新颜色，不修改草稿、光标或正式值，不发数值通知。ReadOnly 的按钮使用 Disabled 配色；数值到达端点仍保留普通交互反馈，不转为 Disabled。当前没有 NumberInput 专属颜色 Overrides 协议。

Gallery 的 NumberInput 页展示不同类型、范围、步长、精度、只读/禁用及窄容器，并通过公开 API 查询正式值、统计根 `ValueChange<T>`、提供合法预设 Setter。另有只含多层 Editor 的自定义输入区，分别显示正式值与草稿。进入页面重新创建示例和统计，离开后销毁。
