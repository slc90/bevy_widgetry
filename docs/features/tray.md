# 托盘

Tray 提供常驻图标、原生右键菜单和点击消息。开启 Facade 的 `tray` Feature 后使用 `bevy_widgetry::tray`。

## 托盘图标

### 创建图标

先准备调用应用自己的 ICO 文件，然后在 Winit 插件安装后添加 Tray 插件：

```rust
use bevy::prelude::*;
use bevy_widgetry::tray::*;

app.add_plugins(DefaultPlugins);
app.add_plugins(
    WidgetryTrayPlugin::new(include_bytes!("../assets/app.ico"), "我的应用")
        .map_err(|failure| failure.into_bevy_error())?
);
```

图标数据在编译期嵌入，运行时不再读取这个文件。图标和提示文字在创建时固定。托盘实例与某个窗口 Entity 无关；所有窗口隐藏时，它仍然可以存在并接收点击。

### 查询初始化状态

安装 `WidgetryTrayPlugin` 后，宿主可以通过 `Res<WidgetryTrayStatus>` 读取初始化结果。需要在状态更新时采取行动的系统可以使用 Bevy Resource 的变化检测。

| 状态 | 含义 |
|---|---|
| `Initializing` | 插件已安装，原生托盘初始化尚未完成。 |
| `Ready` | 原生图标已创建，Windows Shell 注册已确认，事件桥接已就绪。 |
| `Failed` | Startup 初始化失败，原始诊断交给宿主的 Bevy 错误处理器。 |

```rust
use bevy::prelude::*;
use bevy_widgetry::tray::WidgetryTrayStatus;

fn report_tray_ready(status: Res<WidgetryTrayStatus>) {
    if status.is_changed() && *status == WidgetryTrayStatus::Ready {
        info!("托盘已就绪");
    }
}

app.add_systems(Update, report_tray_ready);
```

`WidgetryTrayPlugin::new` 在安装插件前解码 ICO。无效 ICO 会直接返回 `TrayFailure`，此时尚未创建状态 Resource。插件安装后的失败由 Startup 设置 `Failed`，并通过 Diagnostics 传播。

`Ready` 表示启动时的初始化结果。运行期间的菜单同步、回调桥接等故障由 Diagnostics 报告，初始化结果保持原值。

### 接收左键单击

```rust
fn receive_clicks(mut clicks: MessageReader<WidgetryTrayClick>) {
    for _ in clicks.read() {
        info!("收到托盘单击，可由应用恢复自己的窗口");
    }
}

app.add_systems(Update, receive_clicks);
```

消息对应本实例的有效左键单击，不把按下、右键和悬停重复转成同一种事件。Tray 不自动打开或隐藏窗口，也不替应用决定点击后是否退出。

## 原生菜单

### 定义菜单

```rust
app.insert_resource(
    WidgetryTrayMenu::new(vec![
        TrayMenuEntry::item("show", "显示窗口"),
        TrayMenuEntry::check("always_on_top", "置顶", false),
        TrayMenuEntry::Separator,
        TrayMenuEntry::submenu("工具", vec![
            TrayMenuEntry::item("export", "导出记录"),
            TrayMenuEntry::item("clear", "清理缓存").with_enabled(false),
        ]),
        TrayMenuEntry::Separator,
        TrayMenuEntry::item("quit", "退出"),
    ]).map_err(|failure| failure.into_bevy_error())?
);
```

可操作项目的 ID 在整棵菜单中唯一，包括不同子菜单里的项目。分隔符没有 ID。文字、可用性、勾选状态由宿主 Resource 描述。

### 接收菜单操作

```rust
fn receive_menu(mut clicks: MessageReader<WidgetryTrayMenuClick>) {
    for event in clicks.read() {
        match event.id.as_str() {
            "show" => info!("应用收到显示窗口请求"),
            "quit" => info!("应用收到退出请求"),
            id => info!(id, "菜单操作"),
        }
    }
}

app.add_systems(Update, receive_menu);
```

点击表示操作意图。业务接受后更新真实状态，并重新提交相应菜单描述；不要把原生菜单显示过的勾选符号当成另一份业务状态。

### 更新菜单

重新构造合法 `WidgetryTrayMenu` 并替换 Resource，可以改变项目、文字和状态。移除菜单 Resource 则停止提供对应菜单。新旧原生项目的路由由库管理，宿主继续使用自己的稳定字符串 ID。

## 生命周期与事件接入

托盘在主线程创建和销毁。AppExit 或 App 释放时回收实例。原生回调先排队，再唤醒反应式循环，消息在 ECS 阶段交给应用，回调本身不直接访问 World。

Tray 使用 `tray-icon` 的进程级事件入口。宿主不能另行安装竞争的 tray-icon/MenuEvent 回调，或同时消费其全局 Receiver；应用应使用这里提供的 ECS 消息。

初始化与回调桥接的真实失败通过统一诊断传播，合法菜单关闭、无关原生事件及已结束实例的迟到回调不会变成新的业务操作。
