# Slider

`bevy_widgetry_slider` 提供单值 `f32` 数值模型，经 Facade 默认启用的 `slider` Feature 和模块导出。安装 `WidgetrySliderPlugin`；使用 `@WidgetrySlider` 和一次性 Props 构造。

## 配置与数值

- 默认 `min = 0`、`max = 100`、`step = None`、初始值采用 `min`、方向为 `WidgetrySliderOrientation::Horizontal`。
- `min`、`max`、`step`、方向创建后固定。`WidgetrySlider::orientation` 提供只读方向查询。
- 端点、显式数值必须有限，`min < max` 且 `f32` 跨度有限；步长必须有限且大于零。非法构造明确返回错误。
- 连续模式接受闭区间中的有限值。步进模式接受任一端点以及从任一端点以步长可达的数值。例如 `0..10, step=3` 接受 `4`，拒绝 `4.5`。
- 步进校验容忍少量 `f32` 误差，容差不超过步长的 0.1%；不承诺十进制精确性，也不对非法 API 数值做 Clamp 或吸附。

## Runtime API 与通知

`WidgetrySlider::value(&World, entity)` 查询当前值。内部 Runtime State 不公开，不支持直接替换、移除或以反射改写来修改数值。

`WidgetrySlider::set_value_in_world(&mut World, entity, value)` 返回 `Result<bool, BevyError>`：真实变化 `Ok(true)`，合法同值 `Ok(false)`，无效目标、缺少状态、非法数值返回 `Severity::Error` 并记录诊断。失败不改值、不发事件。

`WidgetrySlider::set_value(&mut Commands, entity, value)` 仅排队，在命令实际执行时校验、提交与通知；失败传给宿主 Error Handler。程序化写入允许修改 Disabled Slider。

每次真实变化先提交状态，再触发以 Slider 根实体为 `source` 的 Bevy `ValueChange<f32>`。初始化和同值不发事件。程序化操作一次完成，`is_final = true`。通知仅保证数值状态已提交，不保证其他 Observer 或未来的视觉同步顺序。

## 当前边界

当前交付 Headless 数值和程序化 API，尚未接入用户输入、Track / Fill / Thumb 或 Gallery 页面。不提供 Range Slider、刻度、控件内数值文字或通用 Binding。
