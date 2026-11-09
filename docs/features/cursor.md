# 光标

Cursor 为每个原生窗口协调光标语义，并将其映射为系统光标、静态图片或图集动画。它与颜色 Theme 独立。

```rust
use bevy::prelude::*;
use bevy_widgetry::cursor::*;

app.add_plugins(WidgetryCursorPlugin);
```

## 控件光标声明

### 声明系统语义

```rust
use bevy::picking::cursor::EntityCursor;
use bevy::window::SystemCursorIcon;

bsn! {
    Node { width: px(120), height: px(36) }
    EntityCursor::System(SystemCursorIcon::Pointer)
}
```

声明放在命中实体或可被继承的控件祖先上。内部更具体的声明可以覆盖祖先语义，最终由 Cursor 协调，而不是各控件直接写操作系统光标。

### 声明等待反馈

```rust
commands.entity(button).insert(WidgetryCursorLoading);
// 等待展示结束后：
commands.entity(button).remove::<WidgetryCursorLoading>();
```

这个组件只是展示声明，不是 Loading 状态权威，也不会自行禁用输入。默认 Button Style 会根据自己的 Loading 状态维护它；自定义 Style 可以自行选择反馈方式。

## 静态主题

### 用图片替换一种语义

准备 32×32 的应用图片 `assets/cursors/pointer.png`：

```rust
let theme = WidgetryCursorTheme::new([
    (
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(CursorImage {
            source: CursorImageSource::Path("cursors/pointer.png".into()),
            rect: None,
            hotspot: (4, 2),
        }),
    ),
]).map_err(|failure| failure.into_bevy_error())?;

WidgetryCursorTheme::set_in_world(world, theme);
```

热点以选定图像区域左上角为原点，不是整张图集原点。没有映射的语义继续回退到对应系统光标，主题不必覆盖所有状态。

也可以把 `CursorImageSource::Handle(handle)` 交给主题，复用应用已经加载的 Image。路径支持 Bevy Asset 的来源语法。

### 恢复系统光标

```rust
WidgetryCursorTheme::set(&mut commands, WidgetryCursorTheme::system());
```

同步 `set_in_world` 返回是否改变，Commands 入口延迟提交。同值不标记资源变化。显式 `EntityCursor::Custom` 不会被普通系统语义的主题映射替换。

## 图集动画

### 定义帧、时间与热点

下例使用横向排列的两个 32×32 帧，图集总尺寸为 64×32。

```rust
use std::time::Duration;

let animation = CursorAnimation {
    source: CursorImageSource::Path("cursors/wait-atlas.png".into()),
    size: UVec2::new(64, 32),
    frames: vec![
        CursorAnimationFrame {
            rect: URect::from_corners(UVec2::ZERO, UVec2::new(32, 32)),
            duration: Duration::from_millis(120),
        },
        CursorAnimationFrame {
            rect: URect::from_corners(UVec2::new(32, 0), UVec2::new(64, 32)),
            duration: Duration::from_millis(180),
        },
    ],
    hotspot: (15, 15),
    fallback: None,
};

let theme = WidgetryCursorTheme::new([
    (CursorSemantic::System(SystemCursorIcon::Wait), CursorStyle::Animation(animation)),
]).map_err(|failure| failure.into_bevy_error())?;
WidgetryCursorTheme::set_in_world(world, theme);
```

所有帧的画布大小一致，时长严格大于零，区域不能越界，热点必须在帧内。帧顺序可以重复同一区域。`fallback` 可提供动画不可用时的静态图像，继续不可用则使用系统语义回退。

### 复用动画定义制作预览

`frame_at(elapsed)` 返回当前帧索引，`atlas_layout()` 返回同一帧序列的图集布局。应用可用它们制作主题预览，不需要另写一份时间表。

## 运行时行为

真实 OS 光标只由有效鼠标身份驱动，Touch 或程序化 Custom Pointer 不冒充系统鼠标。拖拽中的 `CursorLease` 以拥有者固定语义；原生窗口移动、Resize 期间让出光标控制权。

图片按实际需要加载。只有系统光标时不需要图片环境；图片不可用时保留回退反馈。主题定义成功不等于图像文件已经加载成功。

当前同一个 Image Handle 的像素热修改可能仍命中原生缓存。需要换位图时使用新的 Handle，而不是反复修改原 Handle 的像素。热点、区域和窗口身份也参与正确的呈现选择。
