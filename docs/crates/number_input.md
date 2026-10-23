# NumberInput

NumberInput / SpinBox 共用 `WidgetryNumberInput<T>`，正式值始终以原始 `T` 保存。支持 `i8/i16/i32/i64/i128/isize`、`u8/u16/u32/u64/u128/usize`、`f32/f64`，由 sealed `WidgetryNumber` 描述数值能力。`WidgetryNumberInputPlugin` 注册所有支持类型的运行时反射。

通过 BSN Scene 构造，`@value: Some(value)` 必填；`min/max` 默认无应用范围，`step` 默认为同类型的一。值、端点和步长必须有限，步长严格为正，`min <= max`，初始值必须位于范围内。配置构造后固定；Props 不保留为运行时状态。非法构造通过 Scene 错误通道报告并清理预约根。

`value(&World, root)` 查询正式值；`set_value_in_world(&mut World, root, value)` 返回 `Result<bool, BevyError>`；`set_value(&mut Commands, root, value)` 在执行时校验和提交，失败交宿主 Error Handler。Setter 只校验有限性和范围，不要求步进对齐，也不受 Disabled 限制。

仅真实变化在提交后发送 `ValueChange<T>`，`source` 为 NumberInput 根，`is_final=true`。初始化、合法同值与失败静默。无效目标、类型不匹配、缺失内部状态或非法值均明确报错，失败保持旧值。内部状态通过根 API 管理，不支持替换或移除。

当前仅提供数值核心；尚未实现文本编辑、步进输入、长按、主题外观与 Gallery 示例。
