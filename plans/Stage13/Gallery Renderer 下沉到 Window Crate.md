# Gallery Renderer 下沉到 Window Crate

## 目标

将 `gallery/src/renderer.rs` 中与透明窗口、多窗口渲染相关的实现下沉到 `bevy_widgetry_window`。

Gallery 只负责应用启动与展示，不再持有 Widgetry Window 正常工作的底层渲染修正。

不新增独立 render crate，不引入额外配置对象或抽象层。

## 文件调整

新增：

```text
crates/window/src/render.rs
```

删除：

```text
gallery/src/renderer.rs
```

`render.rs` 基本直接迁移现有实现，包括：

- DX12 `DxgiFromVisual` 的 `RenderCreation`
- 多窗口按 camera flush command
- 无 camera 时延迟 initial present

去掉 Gallery 特有命名。

## Window crate API

将：

```rust
pub(crate) async fn transparent_renderer() -> Result<RenderCreation>
```

改为公开 API：

```rust
pub async fn transparent_render_creation() -> Result<RenderCreation>
```

实现仍放在私有 `render` module：

```rust
mod render;

pub use render::transparent_render_creation;
```

因此外部统一从：

```rust
bevy_widgetry::window::transparent_render_creation
```

访问，不暴露 `window::render` 这个内部模块结构。

手动创建 device 时的：

```rust
label: Some("Widget Gallery")
```

改为 Widgetry 通用名称，例如：

```rust
label: Some("Widgetry")
```

其余 wgpu 初始化逻辑保持不变，不在这次重构里泛化 backend、presentation system 或 device 配置。

## WidgetryWindowPlugin

原来的：

```rust
GalleryRenderPlugin
```

不再作为公开或独立 Plugin 存在。

把它注册的 RenderApp systems 并入 `WidgetryWindowPlugin::build()`。

逻辑保持现状：

```rust
if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
    // 注册 Core2d / Render systems
}
```

注册内容保持：

```text
Core2d
└─ ApplyDeferred
   └─ submit_window_commands
      after(upscaling)

Render / PrepareViews
└─ defer_initial_present_without_camera
   before(prepare_windows)
```

这样：

- 使用 `WidgetryWindowPlugin` 的正常桌面 App 自动获得多窗口渲染修正；
- `MinimalPlugins` / headless 测试没有 `RenderApp` 时直接跳过；
- Gallery 不需要额外记得添加一个 render plugin。

这些 system 和辅助函数保持 crate-private，不成为公共 API。

## Cargo 依赖

`crates/window/Cargo.toml` 增加：

```toml
wgpu.workspace = true
```

`gallery/Cargo.toml` 删除：

```toml
wgpu.workspace = true
```

workspace 根部的 `wgpu` dependency 定义保留。

## Gallery 调整

删除：

```rust
mod renderer;
```

删除：

```rust
renderer::GalleryRenderPlugin
```

`RenderPlugin` 初始化改为：

```rust
use bevy_widgetry::window::transparent_render_creation;

.set(RenderPlugin {
    render_creation: block_on(transparent_render_creation())?,
    ..default()
})
```

Gallery 此后不再直接依赖：

- wgpu
- `RenderApp`
- `FlushCommands`
- `SortedCameras`
- `ExtractedWindows`
- DX12 presentation system 细节

## Window crate 文档

更新 `crates/window/src/lib.rs` 顶部文档。

目前“透明窗口还需要宿主配置支持透明 compositing 的 rendering 环境”的表述改成更具体的 contract，例如说明：

- `prepare_native_window` 配置 native window 属性；
- `WidgetryWindowPlugin` 安装运行期多窗口渲染支持；
- 桌面 App 可将 `transparent_render_creation()` 提供给 Bevy `RenderPlugin`，创建支持透明 compositing 的 DX12 renderer。

不额外设计 builder 或 renderer configuration API。

## 验证

完成迁移后至少执行：

```text
cargo check --workspace
cargo test --workspace
```

然后运行 Gallery，确认：

1. 主窗口正常透明渲染；
2. Window 页面创建额外窗口正常；
3. 多窗口同时存在时无 DX12 swap-chain command submission 问题；
4. 新窗口创建阶段不会因为尚未拥有 camera 而错误执行 initial present；
5. Gallery 的现有 startup / waveform 测量入口没有受到影响。

## 最终结构

```text
gallery
└─ 只负责 Gallery App、页面和 benchmark

crates/window
├─ Window Scene / lifecycle
├─ native window 配置
├─ modal / title bar / background
└─ render.rs
   ├─ transparent_render_creation()   // public
   ├─ submit_window_commands()        // private
   └─ defer_initial_present_without_camera() // private
```

这次重构只迁移已经属于 Window contract 的渲染实现，不顺带抽象 renderer 配置，也不拆新的 crate。
