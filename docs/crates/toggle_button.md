# Toggle Button

`bevy_widgetry_toggle_button` 提供通用二态按钮；Facade 通过默认可选 Feature `toggle_button` 导出同名模块。安装 `WidgetryToggleButtonPlugin` 自动装配 Core UI、Theme 与官方 Checkbox 输入能力。

## 状态与输入

BSN 使用 `@WidgetryToggleButton`，Props `selected` 负责初始选中；`content` 和 `children` 接受外部 Scene 内容，也可直接追加 Children。没有固定的 CheckBox 方框或勾选图标。根采用 AccessKit `Button` 角色及 `toggled` 属性，子文本提供可访问内容；支持常规焦点、Space / Enter、Pointer 和本地 / 继承 Disabled。

官方 `Checked` 是唯一二态 Authority，保持原生 ECS 使用方式。`WidgetryToggleButton::selected` 查询，`set_selected_in_world` 同步设置，`set_selected` 排队设置。API 与用户交互实际改变状态时，先提交 Checked，再发出根实体上的 `WidgetryToggleButtonChanged { entity, selected }`；同值不通知。直接增删原生 Checked 不额外发业务通知。官方 `ValueChange<bool>` 是输入请求，消费已提交状态请监听专用 Changed 事件。

无效 / 已销毁 / 类型不符目标返回 Error，排队请求在执行时校验并交给宿主错误处理。Disabled 阻止用户输入，程序化设置仍可用。初始化不通知。通知保证状态提交，不保证颜色或布局已经刷新；这些在正常 UI 阶段更新。

## 外观

未选中复用当前 Theme 的 Button 色，选中复用 CheckBox checked 的背景、边框与 mark 前景色，保持激活外观。交互优先级 Disabled > Pressed > Hovered > Normal。

`WidgetryToggleButtonColorOverrides` 提供 normal / hovered / pressed / disabled 与 selected / selected_hovered / selected_pressed / selected_disabled 八态，每态可覆盖背景、边框、前景。支持 get、同步 / 排队 set 与 clear，未覆盖字段跟随当前主题；非法颜色拒绝且不改变原覆盖。托管 Style Owner 拒绝外部覆盖。受管理 Text / Icon 继承根前景。

不提供群组互斥、三态、播放或测量业务。Gallery Button 页展示普通、禁用选中和程序化切换。
