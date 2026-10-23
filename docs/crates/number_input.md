# NumberInput

NumberInput / SpinBox 共用 `WidgetryNumberInput<T>`，正式值始终以原始 `T` 保存。支持 `i8/i16/i32/i64/i128/isize`、`u8/u16/u32/u64/u128/usize`、`f32/f64`，由 sealed `WidgetryNumber` 描述数值能力。`WidgetryNumberInputPlugin` 注册所有支持类型的运行时反射。

通过 BSN Scene 构造，`@value: Some(value)` 必填；`min/max` 默认无应用范围，`step` 默认为同类型的一。值、端点和步长必须有限，步长严格为正，`min <= max`，初始值必须位于范围内。配置构造后固定；Props 不保留为运行时状态。非法构造通过 Scene 错误通道报告并清理预约根。

`value(&World, root)` 查询正式值；`set_value_in_world(&mut World, root, value)` 返回 `Result<bool, BevyError>`；`set_value(&mut Commands, root, value)` 在执行时校验和提交，失败交宿主 Error Handler。Setter 只校验有限性和范围，不要求步进对齐，也不受 Disabled 限制。

仅真实变化在提交后发送 `ValueChange<T>`，`source` 为 NumberInput 根，`is_final=true`。初始化、合法同值与失败静默。无效目标、类型不匹配、缺失内部状态或非法值均明确报错，失败保持旧值。内部状态通过根 API 管理，不支持替换或移除。

## 文本编辑与提交

安装 `WidgetryNumberInputPlugin` 会自动安装组合所需的 TextField Plugin。内嵌 TextField 使用官方编辑器处理光标、选择、剪贴板与 IME；根实体保持唯一正式数值及公开 API 身份。草稿允许空串、负号或任意临时文字，键入期间不改变正式值。

Enter 仅提交合法草稿；非法草稿保留。键盘焦点离开控件时，合法草稿提交，非法草稿恢复正式值文本。Esc 丢弃草稿、恢复正式文本并保留编辑焦点。鼠标移出不触发提交。合法同值不发数值通知。

只接受普通十进制整数/小数，不接受科学计数法、进制前缀、千位分隔、空白、正号或非有限浮点文本。无符号整数拒绝负号，整数拒绝小数，所有类型按自身范围解析；手动值不要求步进对齐。浮点 `.5`、`-.5`、`1.` 可提交并规范化为 `0.5`、`-0.5`、`1.0`。整值浮点保留 `.0`。

合法程序化 Setter 立即覆盖草稿，合法同值也清理旧 Edit/Paste/IME 请求；无效 Setter 保持数值和草稿。文字替换使用 TextField 公开内容 API，不维护第二份文本数值 Authority。

## 浮点显示精度

浮点构造可选 `decimals: Option<u8>`（0–255），整数配置该项会明确失败。没有运行时 decimals Setter。失焦时按指定小数位数显示，聚焦时使用可往返恢复原类型数值的完整普通十进制文本；未配置时始终使用完整规范文本，浮点整值保留 `.0`。

显示舍入不改写正式值；焦点切换不发变化通知。极小次正规数、最大有限值及负值同样显示不含指数的完整规范文本，文本长度由 TextField 视口处理。输入草稿不会被普通刷新反复覆盖，焦点投影保留同帧待处理编辑。

当前尚未实现步进输入、长按、ReadOnly 模式、统一外观与 Gallery 示例。
