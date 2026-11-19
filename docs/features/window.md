# Window

Window 为原生窗口提供自绘标题栏、内容区域、边缘缩放、模态输入和窗口生命周期协调。可以绑定应用已有的 Window / Camera，也可以由控件创建并管理自己的资源。

```rust
use bevy::prelude::*;
use bevy_widgetry::window::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryWindowPlugin);
```

## 透明窗口与渲染初始化

### 准备渲染设备

自绘窗口需要在原生窗口创建前确定透明呈现方式。`transparent_render_creation` 是异步函数，放在应用启动流程中执行，再用于 Bevy RenderPlugin 配置。

```rust
use bevy::render::RenderPlugin;

let render_creation = transparent_render_creation().await
    .map_err(WindowFailure::into_bevy_error)?;
app.add_plugins(DefaultPlugins.set(RenderPlugin {
    render_creation,
    ..default()
}));
```

这段位于异步初始化函数中。不要在 App 已经创建渲染设备后再次安装 DefaultPlugins。`prepare_native_window` 将 Window 配置为透明、无原生装饰和预乘 Alpha；原生配置与渲染设备两边都需要匹配。

## 独立窗口

### 创建由控件管理的窗口

```rust
let root = world.spawn_scene(bsn! {
    @owned_widgetry_window(
        Window { title: "数据面板".into(), resolution: (800, 560).into(), ..default() },
        WidgetryWindowControlsConfig::default(),
        WidgetryWindowBackground::Theme,
        WidgetryWindowColorOverrides::default(),
        bsn_list! { WidgetryWindowTitle WidgetryText },
        bsn_list! { Text("窗口正文") WidgetryText },
    )
})?.id();
```

返回的 `root` 是窗口 UI 根。`widgetry_window_target(world, root)` 返回关联的原生 Window Entity。Owned 方式创建专属 Window 和 Camera，关闭回收时由控件清理它们。

### 绑定已有资源

```rust
let scene = widgetry_window(
    native_window,
    camera,
    WidgetryWindowControlsConfig::default(),
    WidgetryWindowBackground::Theme,
    WidgetryWindowColorOverrides::default(),
    bsn_list! { WidgetryWindowTitle WidgetryText },
    bsn_list! { Text("复用应用已有窗口") WidgetryText },
);
```

`native_window` 与 `camera` 已由应用创建，并满足透明窗口及目标绑定条件。Borrowed 根回收不会销毁这些资源。一个原生窗口和相机不能同时被多个窗口 UI 根重复绑定。

## 标题与窗口控制

### 更新标题

在自定义标题栏的一个 Text 节点上组合 `WidgetryWindowTitle`，再使用根实体更新标题：

```rust
WidgetryWindowTitle::set(&mut commands, root, "数据面板 — 已保存");
```

同步入口为 `set_in_world`。原生 Window.title 是标题来源；没有可见标题标记时仍能更新原生标题。每个 Window 最多一个标题标记，正文中的普通 Text 不参与同步。

### 选择控制按钮

```rust
let controls = WidgetryWindowControlsConfig {
    minimize_visible: false,
    maximize_visible: false,
    close_visible: true,
    resizable: false,
    double_click_maximize: false,
};
```

`minimize_visible`、`maximize_visible` 和 `close_visible` 控制默认三按钮的显示。`resizable` 控制外壳的边缘缩放区域。系统操作权限由原生 `Window.enabled_buttons` 决定。

标题栏空白始终支持原生鼠标拖动。`double_click_maximize` 默认启用双击切换最大化／还原，设为 `false` 时只保留拖动。双击使用 Windows 的时间和位置容差，并遵循原生 `enabled_buttons.maximize`。标题栏内的交互控件接收自己的输入。

### 从应用发起窗口操作

```rust
WidgetryWindowAction::Maximize.queue(&mut commands, root);
```

`WidgetryWindowAction` 提供 `Minimize`、`Maximize`、`RestoreFromMaximized` 和 `RequestClose`。把 Owned 或 Borrowed 的 UI 根传给 `queue`，即可在 Observer 中发起操作。持有 `&mut World` 时可以调用 `apply`。动作通过现有 Window / Camera 绑定找到原生窗口，隐藏默认按钮时也能调用。

`apply` 返回 `Ok(true)` 表示请求已提交，`Ok(false)` 表示 `Window.enabled_buttons` 拒绝操作。无效绑定返回 `WindowFailure`。恢复操作请求退出最大化状态；关闭操作发送 `WindowCloseRequested`，由宿主决定是否关闭。

### 替换标题栏右侧控制区

`owned_widgetry_window_with_controls` 和 `widgetry_window_with_controls` 在标题内容与正文参数之间接收右侧控制区的 `SceneList`。它可以为空，也可以容纳调用方定义的布局和控件。`WidgetryWindowBehavior` 负责边缘缩放与双击设置，标题栏空白仍由外壳处理原生拖动。

```rust
use bevy::ui_widgets::{Activate, Button};

#[derive(Component)]
struct PanelWindow;

let scene = bsn! {
    @owned_widgetry_window_with_controls(
        Window { title: "数据面板".into(), ..default() },
        WidgetryWindowBehavior { double_click_maximize: false, ..default() },
        WidgetryWindowBackground::Theme,
        WidgetryWindowColorOverrides::default(),
        bsn_list! { WidgetryWindowTitle WidgetryText },
        bsn_list! {
            Button Node { width: px(92), justify_content: JustifyContent::Center }
            on(|event: On<Activate>, parents: Query<&ChildOf>, windows: Query<(), With<PanelWindow>>, mut commands: Commands| {
                if let Some(root) = parents.iter_ancestors(event.entity).find(|&entity| windows.contains(entity)) {
                    WidgetryWindowAction::RequestClose.queue(&mut commands, root);
                }
            })
            Children [Text("关闭") WidgetryText template(|_| Ok(Pickable::IGNORE))]
        },
        bsn_list! { Text("窗口正文") WidgetryText },
    )
    template(|_| Ok(PanelWindow))
};
```

自定义控件管理自己的视觉、命中与禁用反馈，Window 主题和配色覆盖继续管理外壳。查询 UI 根的 `WidgetryWindowMaximized` 可读取最近一次同步的原生最大化状态。自定义控件可以使用 `Changed<WidgetryWindowMaximized>` 更新还原提示。

在 Gallery 的 Window 页选择「自定义标题栏」可打开演示。右侧文字按钮提供选项、最小化、最大化／还原和关闭。演示关闭了双击切换，标题栏空白仍支持原生拖动。可在同一页打开 Theme 独立窗口对照默认样式。

## 模态窗口

### 将新窗口设为 Modal Child

```rust
let scene = bsn! {
    @owned_widgetry_window(
        Window { title: "设置".into(), ..default() },
        WidgetryWindowControlsConfig::default(),
        WidgetryWindowBackground::Theme,
        WidgetryWindowColorOverrides::default(),
        bsn_list! { WidgetryWindowTitle WidgetryText },
        bsn_list! { Text("设置内容") WidgetryText },
    )
    WidgetryModalWindow { parent: parent_native_window }
};
```

`parent` 是有效原生父窗口 Entity。同父多个 Modal Child 共享阻挡，最后一个结束才释放；同层最新打开的子窗口优先接收输入，嵌套时最内层有效 Modal 优先。

父窗口自身的 Disabled 请求不会因为 Modal 关闭而被清除。宿主的全局快捷键不自动服从所有模态策略，业务仍需判断当前操作作用域。

## 初始焦点与输入作用域

### 指定进入窗口后的焦点目标

```rust
use bevy_widgetry::text_field::WidgetryTextField;

bsn! {
    @WidgetryTextField { @value: {String::new()} }
    WidgetryWindowInitialFocus(0)
    Node { width: px(240) }
}
```

优先选取较小数值的有效、可见、启用候选。组合控件会解析到真实输入实体；没有有效候选时回退 Window Root，不猜测正文里哪个输入框最合适。

Keyboard / IME 按原始 Native Window 身份路由。Modal 关闭可以恢复之前仍有效的焦点；应用显式清空焦点后，普通按键不会擅自重新聚焦旧 Editor。

## 背景与颜色

背景支持 Theme 或指定 Image。图片未就绪时保留透明，不把主题色冒充图片结果；Cover 保持比例并裁剪。标题栏、内容和系统按钮的配色可以通过 `WidgetryWindowColorOverrides` 设置。

```rust
WidgetryWindowColorOverrides::clear(&mut commands, root);
```

样式刷新保留现有正文实体。窗口组合不会递归接管正文中的所有按钮或改写其业务状态。
