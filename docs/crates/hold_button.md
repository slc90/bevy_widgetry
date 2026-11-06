# HoldButton

`bevy_widgetry_hold_button` 提供独立长按确认控件。安装 `WidgetryHoldButtonPlugin` 后，以 BSN 创建 `WidgetryHoldButton`，通过官方 `Activate` Observer 接收成功确认。

```rust
bsn! {
    @WidgetryHoldButton {
        @diameter: 112.0,
        @children: {Some(Box::new(bsn_list! { Text("Confirm") WidgetryText }) as Box<dyn SceneList>)},
    }
    on(confirm)
}
```

Props 包含直径（默认 112px，必须为有限正数）、`charge_duration`（默认 3 秒）、`decay_duration`（默认 1.2 秒）、`colors`、`content` 容器 Patch 与 `children`。时长为 `Duration`；零充能时长在有效按下后的计时更新立即成功，零衰减时长立即清零。Props 仅负责初始化，运行时输入归属、成功锁存与进度由行为层维护。

外部 Style 安装 `WidgetryHoldButtonHeadlessPlugin`，用 BSN 同根组合 `WidgetryHoldButtonHeadless` 与自有 Node/Children。Headless Props 只有 `charge_duration`、`decay_duration`，默认值与默认控件相同；不创建默认材质、内容或颜色，不要求 Shader Asset 资源。默认 `WidgetryHoldButtonPlugin` 自动安装 HeadlessPlugin，两种 Style 共用同一行为路径。

```rust
bsn! {
    @WidgetryHoldButtonHeadless { @charge_duration: Duration::from_secs(3) }
    Node { width: px(300), height: px(60) }
    on(confirm)
}
```

`WidgetryHoldButtonProgress::value()` 只读返回 0–1 充能值；`WidgetryHoldButtonState` 的 `is_holding()`、`has_succeeded()`、`success_count()` 查询当前输入尝试、成功锁存和累计成功次数。状态字段不公开，应用不直接替换或移除这些托管状态；自有业务计数由应用 Observer 管理。不能同根组合普通 `WidgetryButtonHeadless`，否则普通点击会绕过长按阈值。

Headless 根据真实 Picking 命中与离开取消，不假定圆形尺寸，矩形角落可以正常开始充能。交互 Children 的 Picking 规则由外部 Style 自行安排。行为层在充能、衰减和进度收尾期间请求重绘，Reactive 模式不依赖默认液体动画推进；静止空值/满值停止计时唤醒。外部 Style 可以在 PostUpdate 的 `WidgetryHoldButtonSystems::Update` 之后读取本帧进度。默认 Style 额外负责自身波浪和成功闪光的重绘。

鼠标左键必须从圆形范围内按下；Enter / Space 要求控件获得 Focus，默认 `TabIndex(-1)`。控件不附加原生 Button，短按不产生 Activate。一个输入来源独占一次尝试，按键重复与其他持续按住的来源不能自动接管。

自动计时从实际按下时的单调时钟起算，不计入此前桌面静止或延迟的渲染时间。显式 Bevy 手动时间策略采用 Real Time 的确定性步进。

线性充满时立即发出一次 Activate，持续按住保持满值、不重复激活，成功后松开归零。提前松开或取消时按固定速率衰减，再次有效按下从剩余进度继续。鼠标拖出圆形、指针失效或键盘失焦会取消，重新进入或恢复 Focus 仍需重新按下。本地/祖先 Disabled、Hidden 或 Display::None 会取消并清零，恢复后仍需重新按下；应用窗口失去 OS Focus 的专门处理不在当前范围。

根 Node 使用与最终布局同步的圆角圆形，复用 Bevy 0.20 原生 UI Picking，透明角落可穿透下层控件。内容及后续插入的后代均为无 Picking 的展示内容，内容上不组合其他交互控件。使用直径 Prop 调整大小，外部 Node Patch 用于位置、Margin 等布局；保持根 Node 的正方形尺寸与圆角约束，非均匀 UiTransform 会与 UI 一起变形。

外观使用 Asset crate 管理的嵌入 WESL Shader：浅底深色液体、两层轻微波浪、细圆形边框与前景内容。0% 无液体，100% 完整填充，成功边框提亮约 180ms；液面在充能和衰减时流动。前景跟随 Theme 固定配色，Text 配轻微阴影。动画、进度与反馈变化请求重绘，静止空值/满值不为了波浪持续重绘。

`WidgetryHoldButtonColorOverrides` 的 Normal / Hovered / Pressed / Disabled 分别允许覆写 background、liquid、wave、border、foreground，另可覆写 success_border。`get` 只读查询，`set_in_world` / `clear_in_world` 同步更新，`set` / `clear` 在 Commands 执行时提交；同值返回 false，不发送状态变化事件。未覆写字段跟随当前 Theme，clear 恢复当前 Theme。颜色 World API 返回 `HoldButtonFailure = WidgetryFailure<HoldButtonError>`，区分无效目标与内部颜色状态缺失；非法颜色保留 CoreError、source、原始 origin 与颜色状态/字段，失败保留原有配置。非法直径保留实际 diameter、根和构造操作。Command 与 Scene Template 边界显式转为 `Severity::Error`，保留原 ID 和首次回溯；BSN 构造错误经 Scene 清理路径传播，延迟构造应使用 Widgetry Scene 错误适配。

渲染更新将必要结构损坏和已拥有的材质丢失分别作为领域错误：保留根、阶段、真实 Query source 或材质 ID；多个根的失败完整汇总到宿主。同一根持续相同损坏沿用原诊断，实际修复后记录一次恢复 INFO。正常初次创建材质、布局/可见性等待、输入取消和完成不构造错误；错误记录不重置长按状态。
