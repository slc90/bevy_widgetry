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

Props 包含直径（默认 112px，必须为有限正数）、`charge_duration`（默认 3 秒）、`decay_duration`（默认 1.2 秒）、`colors`、`content` 容器 Patch 与 `children`。时长为 `Duration`；零充能时长在有效按下后的计时更新立即成功，零衰减时长立即清零。Props 仅负责初始化，运行时输入归属、成功锁存与进度由内部 ECS Component 保存；进度不作为公共 API 导出。

鼠标左键必须从圆形范围内按下；Enter / Space 要求控件获得 Focus，默认 `TabIndex(-1)`。控件不附加原生 Button，短按不产生 Activate。一个输入来源独占一次尝试，按键重复与其他持续按住的来源不能自动接管。

自动计时从实际按下时的单调时钟起算，不计入此前桌面静止或延迟的渲染时间。显式 Bevy 手动时间策略采用 Real Time 的确定性步进。

线性充满时立即发出一次 Activate，持续按住保持满值、不重复激活，成功后松开归零。提前松开或取消时按固定速率衰减，再次有效按下从剩余进度继续。鼠标拖出圆形、指针失效或键盘失焦会取消，重新进入或恢复 Focus 仍需重新按下。本地/祖先 Disabled、Hidden 或 Display::None 会取消并清零，恢复后仍需重新按下；应用窗口失去 OS Focus 的专门处理不在当前范围。

根 Node 使用与最终布局同步的圆角圆形，复用 Bevy 0.20 原生 UI Picking，透明角落可穿透下层控件。内容及后续插入的后代均为无 Picking 的展示内容，内容上不组合其他交互控件。使用直径 Prop 调整大小，外部 Node Patch 用于位置、Margin 等布局；保持根 Node 的正方形尺寸与圆角约束，非均匀 UiTransform 会与 UI 一起变形。

外观使用 Asset crate 管理的嵌入 WESL Shader：浅底深色液体、两层轻微波浪、细圆形边框与前景内容。0% 无液体，100% 完整填充，成功边框提亮约 180ms；液面在充能和衰减时流动。前景跟随 Theme 固定配色，Text 配轻微阴影。动画、进度与反馈变化请求重绘，静止空值/满值不为了波浪持续重绘。

`WidgetryHoldButtonColorOverrides` 的 Normal / Hovered / Pressed / Disabled 分别允许覆写 background、liquid、wave、border、foreground，另可覆写 success_border。`get` 只读查询，`set_in_world` / `clear_in_world` 同步更新，`set` / `clear` 在 Commands 执行时提交；同值返回 false，不发送状态变化事件。未覆写字段跟随当前 Theme，clear 恢复当前 Theme。无效目标、非法颜色及无效直径记录 ERROR 并返回 Severity::Error；失败保留原有颜色。BSN 构造错误经 Scene 错误路径传播，延迟构造应使用 Widgetry Scene 错误适配。
