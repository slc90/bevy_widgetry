# Splash

Splash 提供独立启动窗口、首帧就绪握手与主窗口交接。内容、相机和播放逻辑由应用提供；插件不自带 Logo，也不会仅因安装就隐藏主窗口。

Facade 启用 `splash` Feature。

```rust
use bevy::prelude::*;
use bevy_widgetry::splash::*;

app.add_plugins(SplashPlugin);
```

## 启动窗口

### 创建会话

主窗口需要从原生创建时就设为隐藏。准备好主 Window Entity 后创建 Splash：

```rust
let splash = Splash::start(
    world,
    main_window,
    Window { resolution: (480, 270).into(), ..default() },
).map_err(SplashFailure::into_bevy_error)?;
```

返回值是 Splash Window Entity。它使用固定物理尺寸、无原生边框并排除普通应用缩放。位置以主窗口中心所在显示器为依据，而不是固定选主显示器。

### 为窗口提供内容

```rust
use bevy::camera::RenderTarget;
use bevy::window::WindowRef;

let camera = world.spawn((
    Camera2d,
    RenderTarget::Window(WindowRef::Entity(splash)),
)).id();
world.spawn_scene(bsn! {
    UiTargetCamera(camera)
    Node { width: percent(100), height: percent(100) }
    BackgroundColor(Color::srgb(0.08, 0.10, 0.15))
})?;
```

Camera 与内容归应用。会话完成或取消后，应用负责清理这些资源，不能认为 Splash 会遍历 RenderTarget 并删除所有相关实体。

## 首次显示

### 声明内容已准备

```rust
Splash::ready(world, splash).map_err(SplashFailure::into_bevy_error)?;
```

排队方式为 `SplashReady { splash }`。Ready 表示应用内容语义已经满足，但窗口仍需完成真实渲染及呈现检查。不要直接把 `Window.visible` 设为 true 跳过这一步。

```rust
fn shown(event: On<SplashShown>) {
    info!(splash = ?event.splash, main = ?event.main, "启动窗口已显示");
}
```

`Splash::phase(world, splash)` 查询当前阶段；缺少显示器元信息或尚无有效渲染目标时可以继续等待，不会用猜测的位置提前显示。

## 主窗口交接

### 分别提交两个条件

```rust
Splash::finished(world, splash).map_err(SplashFailure::into_bevy_error)?;
Splash::main_ready(world, splash).map_err(SplashFailure::into_bevy_error)?;
```

Finished 表示启动播放结束，Main Ready 表示主业务内容就绪。两者独立，可以任意先后到达，重复信号合法。主内容先就绪不会强行截断启动播放。

条件满足后，库等待主窗口隐藏状态下的有效帧，显示主窗口并确认显示后的新帧，再销毁 Splash。应用应保持 Splash 末帧可渲染，直到收到 `SplashCompleted { splash, main }`。

## 取消与异常处理

### 主动终止启动会话

```rust
Splash::cancel(world, splash).map_err(SplashFailure::into_bevy_error)?;
```

Cancel 只结束 Splash 会话，不打开、聚焦或销毁主窗口，也不发送 Completed。应用自行决定退出、降级启动或显示其他内容。

真实呈现失败保留当前阶段与已经发生的原生状态。Show 后失败时主窗口可能已经可见，不能把取消理解为原子回滚整个桌面。

## 尺寸与定位

Splash 保持请求的物理宽高，不随普通应用缩放改变。超出目标显示器尺寸会明确失败，不自动缩小图片。主窗口最小化时保留已确认显示器身份；尚无法确定位置时等待恢复，不拿屏幕外的最小化窗口坐标计算居中。
