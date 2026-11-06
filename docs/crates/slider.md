# Slider

`bevy_widgetry_slider` 提供单值 `f32` 数值模型，经 Facade 默认启用的 `slider` Feature 和模块导出。安装 `WidgetrySliderPlugin`；使用 `@WidgetrySlider` 和一次性 Props 构造。

`@WidgetrySliderHeadless` 与 `WidgetrySliderHeadlessProps` 独立构造数值能力，不创建 Node、Track、Fill、Thumb、默认颜色或交互。BSN 需要 Bevy Asset/Scene 基础设施，不需要 UI、布局或渲染 Plugin。独立 `WidgetrySliderHeadlessPlugin` 仅登记状态反射。默认控件在原根组合数值能力与既有线性输入/Style，默认 Plugin 自动确保独立 Plugin。

## 配置与数值

- 默认 `min = 0`、`max = 100`、`step = None`、初始值采用 `min`、方向为 `WidgetrySliderOrientation::Horizontal`。
- `min`、`max`、`step` 创建后固定。方向不属于数值模型，保留在默认线性输入配置中；`WidgetrySlider::orientation` 提供默认控件的只读方向查询。
- 端点、显式数值必须有限，`min < max` 且 `f32` 跨度有限；步长必须有限且大于零。非法构造明确返回错误。
- 连续模式接受闭区间中的有限值。步进模式接受任一端点以及从任一端点以步长可达的数值。例如 `0..10, step=3` 接受 `4`，拒绝 `4.5`。
- 步进校验先比较可达值的 `f32` 舍入投影，再允许不超过步长 0.1% 的额外误差；不承诺十进制精确性，也不对非法 API 数值做 Clamp 或吸附。
- 尺寸 Props：`track_thickness = 6px`、`thumb_diameter = 20px`，均要求有限正数；一次性构造，不作为运行期 Props 保留。

## Runtime API 与通知

`WidgetrySliderHeadless::value(&World, entity)` 查询当前值；`state` 返回同一份 `WidgetrySliderState` 的只读引用。该不可变 Component 的私有字段通过 `value()`、`min()`、`max()`、`step()` 读取，外部 Style 可使用 `Ref` / `Changed` 检测真实提交。数值仍只有一份权威，不支持通过替换、移除或反射改写状态修改数值。数值查询和提交只要求 Headless 身份及数值状态，不查询默认 Parts 或布局。

查询、程序化设置与颜色 API 返回 `SliderFailure`（`WidgetryFailure<SliderError>`）。`WidgetrySliderHeadless::set_value_in_world(&mut World, entity, value)` 返回 `Result<bool, SliderFailure>`：真实变化 `Ok(true)`，合法同值 `Ok(false)`，无效目标、缺少状态、非法数值返回领域失败。失败不改值、不发事件。

数值失败保存 `SliderNumericConfig { min, max, step, value }` 原始 `f32` 配置，`SliderValueRejection` 区分非有限范围、逆序范围、非有限跨度、非法步长、越界数值与不可达数值。尺寸错误保存轨道厚度、Thumb 直径及无效字段；目标和结构错误保存 Root、部件与缺失类型。颜色失败保留 Core 原因与状态 / 字段上下文。源头一次 ERROR 保存原始回溯，传播不重复记录，Template / Command / System 边界显式转为 `Severity::Error` 并保留原始 ID。样式按结构业务键保存持续诊断，聚合多个 Root，恢复时记录 INFO。正常布局等待、Pointer 取消、Disabled、Clamp 与合法同值不产生错误；输入位置投影中缺失自有 State / 线性绑定 属于真实结构失败。

`WidgetrySliderHeadless::set_value(&mut Commands, entity, value)` 仅排队，在命令实际执行时校验、提交与通知；失败传给宿主 Error Handler。程序化写入允许修改 Disabled Slider。

每次真实变化先提交状态，再触发以 Slider 根实体为 `source` 的 Bevy `ValueChange<f32>`。初始化和同值不发事件。程序化操作一次完成，`is_final = true`。通知仅保证数值状态已提交，不保证其他 Observer 或未来的视觉同步顺序。

## 可替换交互策略

独立 `WidgetrySliderInteractionPlugin` 安装数值、Core UI 和输入生命周期，不安装默认样式。以 BSN 在 `@WidgetrySliderHeadless` 同根插入 `WidgetrySliderInteraction::new(strategy)` 即可使用库外策略；该组件确保 Node、Hovered 和内部手势状态，键盘导航可按需设置原生 TabIndex。应用可同时组合不同策略实例，不需要默认 Slider 身份或 Track / Thumb。

`SliderInteractionStrategy::begin(&World, SliderPointerInput)` 读取布局、实例配置及当前只读数值，返回 `None` 拒绝/等待开始，或 `SliderInteractionStart { position, drag }`。`position` 为连续数值坐标，库负责 Clamp、方向锚点、步进和真实提交。每次手势的 `Box<dyn SliderDragStrategy>` 独立保存抓取偏移和轨迹历史；其 `project` 返回 `SliderProjection::Position`、`Wait`（保留捕获且不改值）或 `Cancel`（正常取消）。策略不可写 World，不直接提交 value 或控制 Capture。

`SliderInteractionStrategy::key` 返回 `SliderDirection::Increase / Decrease`，由库按一步或跨度 1% 提交；默认 `None` 保留冒泡。只读 World 支持读取应用自己的实例组件；跨 Move 的可变轨迹放在手势 session。策略失败返回 `SliderFailure`，可用 `SliderError::Strategy { root, source }` 保留库外原因。非有限投影保存原始 f64 的 `InvalidPosition`，失败先清理捕获、Pressed 和 session，再传播同一诊断；状态不存在时不伪造结束值。

默认 `WidgetrySliderLinear { track, thumb, orientation }` 是公开线性绑定，其 required 交互组件选择内置线性策略。绑定可指向应用自有节点，不依赖私有 Style Parts。布局尚未产生时等待，绑定实体或 Node 丢失明确报错。默认控件自动组合该绑定；默认样式仍只写自己的 Parts、颜色和焦点环。

## 用户输入

水平向右增大，垂直向上增大。轨道点击立即更新并开始手势；按 Thumb 仅开始手势，保留抓取偏移。拖出控件仍由 Bevy Pointer Capture 继续输入，超出行程限制到端点。

步进按移动方向从当前值递进，反转时以当前值建立新锚点。例如 `0 → 3 → 6 → 9 → 10`，反向 `10 → 7 → 4 → 1 → 0`。相同指针行程不依赖 Move 事件数量。未走满一步保留当前值，端点始终可到达。

仅获得焦点且有效启用的 Slider 响应对应方向键（水平左右、垂直上下）；支持重复按键。步进模式一次移动一步，连续模式一次增减跨度的 1%。未处理的方向键及 Disabled 下的按键保留冒泡，允许父容器处理。不提供 Home / End。

有效 Disabled（包括祖先继承）、Release、Cancel、DragEnd、窗口失焦、有效 Pointer / Capture 丢失或隐藏会结束手势。Disabled 不接受用户输入，程序化更新仍可执行。拖动变化 `is_final=false`，键盘变化 `is_final=true`；结束手势不补发同值通知。

`SliderDragStarted { entity, value }` 在成功捕获 Pointer 后、轨道初次改值前发送，`entity` 是 Slider 根，`value` 是开始时已提交值。按住 Thumb 不移动也会发送；Started Observer 的 Commands 先于初次轨道提交执行，可先暂停消费方时钟或取消控件。无效 Press 不发送。

`SliderDragEnded { entity, value, outcome }` 在清除活动手势、释放自身捕获与移除 Pressed 后发送，提供终止时已提交值。Primary Release / 正常 DragEnd 为 `SliderDragOutcome::Completed`；Cancel、不可用、失焦、Pointer / Capture 丢失及键盘打断为 `Cancelled`。多条终止入口只能结算一次，无移动也有完整 Start/End。取消保留当前值，不自动回滚。键盘及程序化操作本身不合成手势通知，`ValueChange::is_final` 也不表示手势结束。实体树销毁后不能保证向已销毁根发送事件，消费方须在销毁/换会话时清除自身暂存状态。

拖动期间策略绑定 / State 损坏也先清理手势和捕获，再传播原始诊断。绑定缺失时仍能发送真实值的 Cancelled；State 已丢失则不伪造 Ended 的数值，错误上下文记录已执行的清理。

## 样式与尺寸

控件绘制胶囊形 Track、主题强调色 Fill 和圆形实心 Thumb。水平 Fill 从左端延伸到 Thumb 中心；垂直 Fill 从底端向上延伸。

根 Node 默认主轴长度 `200px`，交互区域厚度至少 `28px`，并容纳自定义的轨道与 Thumb。通过 BSN Patch 根 Node 的主轴长度即可改变行程；Track 厚度和 Thumb 直径通过 Props 设置。运行期根布局变长不拉伸 Thumb，输入读取实际 Thumb `ComputedNode`，并使用当前根布局、DPI、UiScale 和 Camera viewport 计算行程。

Normal / Hovered / Pressed / Disabled 使用当前 Theme 的 `slider` 配色。键盘可见焦点在 Thumb 外侧显示环；状态切换不改变几何尺寸。Disabled 保留当前值对应的位置。

`colors` Props 接受 `WidgetrySliderColorOverrides`，按状态独立覆盖 `track`、`fill`、`thumb`、`focus_ring`。通过 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 查询或更新局部覆盖；无效目标或颜色明确报错，合法同值返回 `Ok(false)`。未覆盖字段随当前 Theme 更新，清除后恢复当前主题。

不提供 Range Slider、刻度、控件内数值文字、内部 Track / Thumb 的公开 Patch 承诺或通用 Binding。

## Gallery

[Gallery Slider 页面](../../gallery/src/pages/slider.rs) 展示连续、步进、垂直、Disabled 及自定义尺寸。数值 Text 由 Gallery 在控件外组合并监听 `ValueChange<f32>` 更新，不属于 Slider 内部结构。离开页面销毁临时状态，再次进入采用各示例的初始值。页面支持现有 Theme、语言、Scale Factor 和配色对照机制。

页面另有只组合 Headless 与应用 Text 的独立数值读数，显示当前值、范围、步长及提交次数，提供合法预设和禁用期间程序设置。它没有默认 Slider 标记或 Track/Thumb，不提供点击或拖动输入。

页面还提供独立弧形实例：只组合公开数值和交互策略，用应用自有圆弧点、手柄及读数绘制。会话保存角度切口连续性和抓取偏移；W / S 为应用自定义键位，可聚焦、禁用和在禁用期间设为 50。它没有默认线性绑定或隐藏的默认部件。
