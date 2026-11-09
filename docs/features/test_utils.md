# 测试辅助

`bevy_widgetry_test_utils` 提供受控 Bevy App、输入事件、错误捕获与基准辅助。它作为 dev-dependency 使用，不进入生产依赖。

## Scene 测试环境

### 创建最小控件状态测试

```rust
use bevy::prelude::*;
use bevy_widgetry::button::{WidgetryButtonHeadless, WidgetryButtonLoading, WidgetryButtonPlugin};
use bevy_widgetry_test_utils::scene_app;

#[test]
fn loading_is_committed() -> Result {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryButtonHeadless
        WidgetryButtonLoading::default()
    })?.id();

    WidgetryButtonLoading::set_in_world(app.world_mut(), root, true)
        .map_err(|failure| failure.into_bevy_error())?;
    assert!(WidgetryButtonLoading::get(app.world(), root)
        .map_err(|failure| failure.into_bevy_error())?.is_active());
    Ok(())
}
```

`scene_app()` 准备 Scene 所需环境，测试显式决定安装哪些控件插件和何时推进 App。

### 加入布局和相机

```rust
use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};

let mut app = scene_app();
add_ui_plugins(&mut app);
spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
app.update();
```

需要验证 UI 几何时加入真实 UI 系统和确定的视口尺寸。状态 Setter 的即时结果与布局更新后的结果分别断言，不把两者混为同一时刻。

## 指针输入

### 直接验证 Observer 语义

```rust
use bevy_widgetry_test_utils::{press, primary_click, release};

press(&mut app, button);
app.world_mut().trigger(primary_click(button));
app.world_mut().flush();
release(&mut app, button);
```

这些辅助函数向指定实体发送指针事件，适合验证按钮是否接受主键、取消是否清理 Pressed 等行为。它们不执行真实屏幕命中。

### 验证命中与路由

需要经过真实 Picking 时，使用 `picking_app`、`spawn_picking_camera` 建立环境，再通过 `queue_pointer` 向输入消息管线提交带 PointerId、Location 和 PointerAction 的消息，随后推进 App。

同一测试中保持正确的输入身份与 RenderTarget；不能用一条指向别处的取消事件证明原捕获已经释放。Mouse 与 Custom Pointer 可以分别验证路由，但原生系统鼠标呈现属于不同层次。

## 键盘与文本

`add_keyboard_dispatch` 接入键盘派发，`queue_key` 和 `press_key` 提交输入。`text_input_app`、`text_edit_app` 提供文本相关测试环境。

测试先明确设置实际焦点目标，再提交按键和编辑消息。对 Root/Editor 分离控件，焦点放在真正的 Editor，而不是仅把逻辑根当成输入位置。

## 主题变化

### 切换完整主题

```rust
use bevy_widgetry_test_utils::switch_theme;
use bevy_widgetry::theme::WIDGETRY_DARK_THEME;

switch_theme(&mut app, WIDGETRY_DARK_THEME);
app.update();
```

默认仅包含 Dark。测试依赖可启用 `light`、`pink_dream`，或 `all_themes` 集合来覆盖其他预设。外部自定义配色也通过同一入口提交。

## 错误与日志捕获

`ErrorCapture` 接收实际 ECS 错误，`LogCapture` 收集 Tracing 记录。`DiagnosticPhase` 区分 source、handled 等阶段，适合验证错误经过组合后是否保留身份、有没有重复源头报告。

测试应把“诊断被捕获”与“业务已处理”区分开：捕获是观测，不会自动调用回退或恢复。具体场景可同时断言状态保持、失败类别及日志阶段。

## 基准辅助

`benchmark` 提供结果和样本输出能力。基准应先建立可重复场景，再测量目标操作；构建输入数据和写报告不必混入被测区间。CPU/ECS、布局和原生呈现是不同测量对象，应按测试实际覆盖的层次命名结果。
