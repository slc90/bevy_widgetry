# Slider

`bevy_widgetry_slider` 提供单值 `f32` 数值模型，经 Facade 默认启用的 `slider` Feature 和模块导出。安装 `WidgetrySliderPlugin`；使用 `@WidgetrySlider` 和一次性 Props 构造。

## 配置与数值

- 默认 `min = 0`、`max = 100`、`step = None`、初始值采用 `min`、方向为 `WidgetrySliderOrientation::Horizontal`。
- `min`、`max`、`step`、方向创建后固定。`WidgetrySlider::orientation` 提供只读方向查询。
- 端点、显式数值必须有限，`min < max` 且 `f32` 跨度有限；步长必须有限且大于零。非法构造明确返回错误。
- 连续模式接受闭区间中的有限值。步进模式接受任一端点以及从任一端点以步长可达的数值。例如 `0..10, step=3` 接受 `4`，拒绝 `4.5`。
- 步进校验先比较可达值的 `f32` 舍入投影，再允许不超过步长 0.1% 的额外误差；不承诺十进制精确性，也不对非法 API 数值做 Clamp 或吸附。
- 尺寸 Props：`track_thickness = 6px`、`thumb_diameter = 20px`，均要求有限正数；一次性构造，不作为运行期 Props 保留。

## Runtime API 与通知

`WidgetrySlider::value(&World, entity)` 查询当前值。内部 Runtime State 不公开，不支持直接替换、移除或以反射改写来修改数值。

`WidgetrySlider::set_value_in_world(&mut World, entity, value)` 返回 `Result<bool, BevyError>`：真实变化 `Ok(true)`，合法同值 `Ok(false)`，无效目标、缺少状态、非法数值返回 `Severity::Error` 并记录诊断。失败不改值、不发事件。

`WidgetrySlider::set_value(&mut Commands, entity, value)` 仅排队，在命令实际执行时校验、提交与通知；失败传给宿主 Error Handler。程序化写入允许修改 Disabled Slider。

每次真实变化先提交状态，再触发以 Slider 根实体为 `source` 的 Bevy `ValueChange<f32>`。初始化和同值不发事件。程序化操作一次完成，`is_final = true`。通知仅保证数值状态已提交，不保证其他 Observer 或未来的视觉同步顺序。

## 用户输入

水平向右增大，垂直向上增大。轨道点击立即更新并开始手势；按 Thumb 仅开始手势，保留抓取偏移。拖出控件仍由 Bevy Pointer Capture 继续输入，超出行程限制到端点。

步进按移动方向从当前值递进，反转时以当前值建立新锚点。例如 `0 → 3 → 6 → 9 → 10`，反向 `10 → 7 → 4 → 1 → 0`。相同指针行程不依赖 Move 事件数量。未走满一步保留当前值，端点始终可到达。

仅获得焦点且有效启用的 Slider 响应对应方向键（水平左右、垂直上下）；支持重复按键。步进模式一次移动一步，连续模式一次增减跨度的 1%。未处理的方向键及 Disabled 下的按键保留冒泡，允许父容器处理。不提供 Home / End。

有效 Disabled（包括祖先继承）、Release、Cancel、DragEnd、窗口失焦、有效 Pointer / Capture 丢失或隐藏会结束手势。Disabled 不接受用户输入，程序化更新仍可执行。拖动变化 `is_final=false`，键盘变化 `is_final=true`；结束手势不补发同值通知。

`SliderDragStarted { entity, value }` 在成功捕获 Pointer 后、轨道初次改值前发送，`entity` 是 Slider 根，`value` 是开始时已提交值。按住 Thumb 不移动也会发送；Started Observer 的 Commands 先于初次轨道提交执行，可先暂停消费方时钟或取消控件。无效 Press 不发送。

`SliderDragEnded { entity, value, outcome }` 在清除活动手势、释放自身捕获与移除 Pressed 后发送，提供终止时已提交值。Primary Release / 正常 DragEnd 为 `SliderDragOutcome::Completed`；Cancel、不可用、失焦、Pointer / Capture 丢失及键盘打断为 `Cancelled`。多条终止入口只能结算一次，无移动也有完整 Start/End。取消保留当前值，不自动回滚。键盘及程序化操作本身不合成手势通知，`ValueChange::is_final` 也不表示手势结束。实体树销毁后不能保证向已销毁根发送事件，消费方须在销毁/换会话时清除自身暂存状态。

## 样式与尺寸

控件绘制胶囊形 Track、主题强调色 Fill 和圆形实心 Thumb。水平 Fill 从左端延伸到 Thumb 中心；垂直 Fill 从底端向上延伸。

根 Node 默认主轴长度 `200px`，交互区域厚度至少 `28px`，并容纳自定义的轨道与 Thumb。通过 BSN Patch 根 Node 的主轴长度即可改变行程；Track 厚度和 Thumb 直径通过 Props 设置。运行期根布局变长不拉伸 Thumb，输入读取实际 Thumb `ComputedNode`，并使用当前根布局、DPI、UiScale 和 Camera viewport 计算行程。

Normal / Hovered / Pressed / Disabled 使用当前 Theme 的 `slider` 配色。键盘可见焦点在 Thumb 外侧显示环；状态切换不改变几何尺寸。Disabled 保留当前值对应的位置。

`colors` Props 接受 `WidgetrySliderColorOverrides`，按状态独立覆盖 `track`、`fill`、`thumb`、`focus_ring`。通过 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 查询或更新局部覆盖；无效目标或颜色明确报错，合法同值返回 `Ok(false)`。未覆盖字段随当前 Theme 更新，清除后恢复当前主题。

不提供 Range Slider、刻度、控件内数值文字、内部 Track / Thumb 的公开 Patch 承诺或通用 Binding。

## Gallery

[Slider 页面](../gallery.md) 展示连续、步进、垂直、Disabled 及自定义尺寸。数值 Text 由 Gallery 在控件外组合并监听 `ValueChange<f32>` 更新，不属于 Slider 内部结构。离开页面销毁临时状态，再次进入采用各示例的初始值。页面支持现有 Theme、语言、Scale Factor 和配色对照机制。
