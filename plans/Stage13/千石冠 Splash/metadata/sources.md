# Bevy 0.20 / bevy_widgetry：实现依据

固定版本原则：只采用 GitHub `bevyengine/bevy` 的 **`v0.20.0`** tag 作为 API 证据，避免网络搜索中混入 0.19.1 或 master 的示例。目标仓库是 `slc90/bevy_widgetry` 的本次调研 `main` 版本，Codex 本地开始时仍需对照最新源代码。

| 需求 | 已核验的源码与理由 |
|---|---|
| 离屏 UI RenderTarget | [`examples/ui/render_ui_to_texture.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/render_ui_to_texture.rs) 显示 `Image::new_target_texture`、`RenderTarget::Image`、`Camera.order=-1`、`UiTargetCamera` 的共用模式 |
| ImageNode/旋转平移缩放 | [`examples/ui/ui_transform.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_transform.rs)；[`crates/bevy_ui/src/ui_transform.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/ui_transform.rs) 定义 `UiTransform`，旋转使用 `Rot2`、注释说明顺时针 |
| MaterialNode + Custom UI Material | [`examples/ui/ui_material.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_material.rs)，`UiMaterialPlugin`、`AsBindGroup`、贴图 sampler、可变 material 属性 |
| UI fragment shader | [`assets/shaders/custom_ui_material.wesl`](https://github.com/bevyengine/bevy/blob/v0.20.0/assets/shaders/custom_ui_material.wesl)；注意是 `import bevy_ui_render::ui_vertex_output::UiVertexOutput;` |
| UI 全局 Z 顺序 | [`crates/bevy_ui/src/ui_node.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/ui_node.rs)，`GlobalZIndex` 可跨 UI 树排序 |
| UI picking/input 阻挡 | [`crates/bevy_ui/src/picking_backend.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/picking_backend.rs) 指出新的 backend 忽略 `FocusPolicy`，未设置 `Pickable` 的 Node 默认也可 block |
| 嵌入式素材 | [`examples/asset/embedded_asset.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/asset/embedded_asset.rs)；当前仓库的 `GalleryAssetPlugin` 更可直接沿用 |
| Asset 加载与状态 | [`crates/bevy_asset/src/server/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_asset/src/server/mod.rs)，加载阶段与 `AssetEvent` 等体系；注意 CPU 加载状态≠GPU present |
| Bevy feature 控制 | [`docs/cargo_features.md`](https://github.com/bevyengine/bevy/blob/v0.20.0/docs/cargo_features.md)，Bevy 的 `bevy_ui`/`bevy_ui_render` 与 `bevy_sprite_render` 是不同 feature |

宿主仓库实际落点：

- [`Cargo.toml`](https://github.com/slc90/bevy_widgetry/blob/main/Cargo.toml)：`bevy = "=0.20.0"`，配置 feature 和 Clippy deny。
- [`gallery/src/main.rs`](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)：`DefaultPlugins`、渲染初始化、`setup`、Camera/Window shell、benchmark 安装。
- [`gallery/src/gallery.rs`](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/gallery.rs)、[`gallery/src/pages/button.rs`](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/button.rs)：`GalleryPage` 状态与 Button 页挂载。
- [`gallery/src/assets.rs`](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/assets.rs)：现行 EmbeddedAssetRegistry 注册。
- [`gallery/src/startup_cursor.rs`](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/startup_cursor.rs)：已有首帧 present、管线与 `DwmFlush` 判断。
- [`crates/window/src/scene.rs`](https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/scene.rs)、[`crates/window/src/render.rs`](https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/render.rs)：透明原生窗口、DX12 `DxgiFromVisual`。

以上均为来源核对，不表示本包已验证各组合在目标机器上成功渲染。正式执行应编译并观察系统的实际行为。
