# Stage11 内建 Asset 方案

## 目标

为 Widgetry 增加独立的内部资源 crate，集中管理库自身运行所需的内建资源，并在编译期嵌入 binary。

完成后：

- Widgetry 各控件 crate 不再自行保存内建资源文件。
- 控件代码不再书写 `assets/...`、`embedded://...` 等具体资源路径。
- 内建资源通过语义化枚举访问。
- 内建资源系统属于库内部实现，不进入 `bevy_widgetry` facade 的公共 API。
- Gallery 使用独立的资源域，不与 Widgetry 库内资源混合。
- 资源归属原则写入 `rules`，成为后续开发必须遵守的架构约束。

---

## 1. 新增 `bevy_widgetry_asset`

新增 workspace crate：

```text
crates/asset/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── assets/
        └── icons/
            ├── window_close.svg
            ├── window_maximize.svg
            ├── window_minimize.svg
            └── window_restore.svg
```

Package 名：

```toml
name = "bevy_widgetry_asset"
```

该 crate：

- 只依赖 workspace 中的 `bevy`。
- 不依赖 `core` 或任何控件 crate。
- 只承担 Widgetry 内建资源的存储、嵌入注册与资源标识职责。
- 不负责解析 SVG，不知道 `Icon`、`SvgAsset` 等上层类型。

---

## 2. 内建资源 API

定义：

```rust
pub struct WidgetryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinIcon {
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
}
```

`BuiltinIcon` 提供：

```rust
impl BuiltinIcon {
    pub fn path(self) -> AssetPath<'static>;
}
```

`path()` 内部使用 Bevy 的 `embedded_path!` 构造与嵌入注册一致的路径，并指定 `embedded` AssetSource。

调用方只处理：

```rust
BuiltinIcon::WindowClose
```

不得知道：

```text
assets/icons/window_close.svg
embedded://bevy_widgetry_asset/...
```

等物理或虚拟路径细节。

`BuiltinIcon` 和 `path()` 使用 `pub`，因为需要跨 crate 调用；它们仍属于 workspace 内部 API，不从顶层 `bevy_widgetry` facade re-export。

---

## 3. 资源注册

`WidgetryAssetPlugin` 统一注册全部 Widgetry 内建资源：

```rust
impl Plugin for WidgetryAssetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/icons/window_close.svg");
        embedded_asset!(app, "assets/icons/window_maximize.svg");
        embedded_asset!(app, "assets/icons/window_minimize.svg");
        embedded_asset!(app, "assets/icons/window_restore.svg");
    }
}
```

资源位于 `src/assets/`，因此宏路径无需使用 `../`。

当前只有四个资源，不引入额外宏或代码生成来消除注册与枚举映射中的少量路径重复；资源数量明显增长后再考虑抽取唯一资源表。

---

## 4. 自动注册语义

`WidgetryAssetPlugin` 是纯内部插件，用户不负责注册，也不需要知道它存在。

任何实际使用 Widgetry 内建资源的库插件负责确保它已注册：

```rust
if !app.is_plugin_added::<WidgetryAssetPlugin>() {
    app.add_plugins(WidgetryAssetPlugin);
}
```

当前首先由 `WindowPlugin` 执行该检查。

以后若其他控件也使用内建资源，采用相同方式自行确保插件存在。

不要依赖“目前只有 Window 使用资源”这一假设，也不要允许同一插件无条件重复添加。

---

## 5. Window 迁移

`bevy_widgetry_window` 新增内部依赖：

```toml
bevy_widgetry_asset = { path = "../asset" }
```

`WindowPlugin`：

- 自动确保 `WidgetryAssetPlugin` 已注册。
- 删除原本由 Window 自己执行的四次 `embedded_asset!`。
- 继续按现有逻辑确保 `IconPlugin`、`ThemePlugin` 等依赖插件存在。

窗口标题栏不再接收路径字符串：

```rust
system_icon(BuiltinIcon::WindowMinimize)
system_icon(BuiltinIcon::WindowMaximize)
system_icon(BuiltinIcon::WindowClose)
```

还原图标使用：

```rust
BuiltinIcon::WindowRestore
```

`system_icon` 内部直接调用：

```rust
Icon::new(asset_server, icon.path())
```

删除当前自行通过 `module_path!()`、crate 名和相对路径构造 embedded `AssetPath` 的逻辑。

Window crate 最终不再知道这些 SVG 文件存放在哪里。

---

## 6. 清理旧资源

删除：

```text
crates/window/assets/
```

其中四个 Window SVG 已迁入：

```text
crates/asset/src/assets/icons/
```

同时删除：

```text
crates/core/assets/icons/check.svg
crates/core/assets/icons/wide.svg
```

这两个文件属于早期 SVG 学习 example 的遗留资源，当前库实现不再使用，不迁入新的 asset crate。

删除后若 `crates/core/assets/` 为空，则整个目录删除。

---

## 7. Gallery 独立资源域

Gallery 资源不进入 `bevy_widgetry_asset`。

Gallery 自身使用独立资源模块：

```text
gallery/src/
├── assets.rs
└── assets/
    └── icons/
        └── logo.svg
```

将当前：

```text
gallery/assets/gallery.svg
```

迁移到：

```text
gallery/src/assets/icons/logo.svg
```

`assets.rs` 内定义：

```rust
pub(crate) struct GalleryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GalleryIcon {
    Logo,
}
```

并提供：

```rust
impl GalleryIcon {
    pub(crate) fn path(self) -> AssetPath<'static>;
}
```

`GalleryAssetPlugin` 使用 `embedded_asset!` 注册 Gallery 自有资源。

Gallery 是最终应用，因此由 `main` 显式装配：

```rust
app.add_plugins(GalleryAssetPlugin);
```

不需要自动注册机制。

Gallery 标题栏加载 Logo 时改为：

```rust
GalleryIcon::Logo.path()
```

Gallery 其他代码不再书写 Logo 的真实路径。

Gallery 不直接使用 `BuiltinIcon` 中的 Window 内部资源；Window 图标只通过 Window 控件自身间接使用。

---

## 8. 资源归属规则

在 `rules/architecture.md` 中新增独立的“资源管理”或“Asset 归属”章节，将本次设计提升为长期架构约束。

建议规则内容如下：

### Widgetry 库资源

Widgetry 库自身运行所需的静态资源，包括但不限于：

- SVG；
- 图片；
- 字体；
- 其他需要随库发布或嵌入程序的静态文件；

统一归属于：

```text
crates/asset
```

由 `bevy_widgetry_asset` 集中管理。

其他库 crate，包括：

```text
core
button
combo_box
text_field
window
以及后续新增控件 crate
```

不得自行建立和维护运行时资源目录。

控件专属资源仍然可以在语义上属于某个控件，但其物理资源文件和嵌入注册统一由 `bevy_widgetry_asset` 管理。

### 资源访问

库内其他 crate 不应直接依赖具体资源路径。

不得在 Asset 模块之外散布：

```text
assets/...
../assets/...
embedded://...
```

等资源路径字符串。

资源应通过 `bevy_widgetry_asset` 提供的语义标识访问，例如：

```rust
BuiltinIcon::WindowClose
```

后续若引入其他资源类型，应按资源语义建立对应类型，例如：

```rust
BuiltinFont
BuiltinImage
```

而不是建立一个混杂所有资源种类的大型通用枚举。

### 资源注册

使用 Widgetry 内建资源的库插件负责自动确保 `WidgetryAssetPlugin` 已注册。

注册属于库内部实现责任，不得要求 Widgetry 用户手动配置内部资源插件。

### 应用资源

应用自身资源不得因为使用了 Widgetry 而下沉到 `bevy_widgetry_asset`。

Gallery 的：

- Logo；
- 展示页面图片；
- Demo 专属图标；
- 未来其他仅用于 Gallery 展示的资源；

全部继续属于 Gallery 自身。

Gallery 使用：

```text
gallery/src/assets.rs
gallery/src/assets/
```

管理自己的资源，并由 `GalleryAssetPlugin` 统一完成嵌入注册。

Gallery 资源只有在其本身已经明确成为 Widgetry 库功能的一部分时，才允许迁移到 `bevy_widgetry_asset`。

不得因为“未来可能复用”提前把应用资源下沉到库资源层。

---

## 9. 依赖关系

新增后的相关生产依赖：

```text
bevy
 ↑
bevy_widgetry_asset

bevy_widgetry_window
 ├──> bevy_widgetry_core
 └──> bevy_widgetry_asset
```

`core` 不依赖 `asset`。

`asset` 不依赖 `core`。

顶层 `bevy_widgetry` 不直接依赖或 re-export `bevy_widgetry_asset`；现有对 `window` 的依赖自然将其带入构建图。

Gallery 的 `GalleryAssetPlugin` 只是 Gallery 内部 module，不额外创建 Cargo crate。

---

## 10. Workspace 与架构文档

根 `Cargo.toml` 的 workspace members 增加：

```text
crates/asset
```

更新 `docs/architecture.md`：

- workspace 结构中加入 `crates/asset`；
- 将其描述为 Widgetry 内建资源基础设施；
- dependency graph 增加 `window --> asset`；
- 将 `asset` 放入 Infrastructure；
- Gallery 的 `assets.rs` 属于应用内部实现，不作为独立 crate 绘入依赖图。

更新 `rules/architecture.md`：

- 增加 Widgetry 库资源必须集中进入 `bevy_widgetry_asset` 的规则；
- 禁止其他库 crate 自行维护运行时资源目录；
- 规定库资源必须通过语义资源标识访问；
- 规定应用资源与库资源保持边界；
- 规定 Gallery 使用自身 `assets` module 与 `GalleryAssetPlugin` 管理应用资源。

无需扩大顶层 facade 公共 API。

---

## 11. 测试

### `bevy_widgetry_asset`

增加轻量测试，验证：

- `WidgetryAssetPlugin` 能完成四个内建资源的 embedded 注册；
- 四个 `BuiltinIcon::path()` 均可指向已注册的 embedded asset；
- 测试不得依赖真实磁盘工作目录；
- production `asset` crate 不因测试需要反向依赖 `core`。

### `bevy_widgetry_window`

保留并调整现有 Window 图标 materialize 集成测试。

测试中的资源访问全部改用：

```rust
BuiltinIcon::WindowRestore.path()
```

等枚举入口。

不得再出现硬编码：

```text
embedded://...
../assets/...
assets/icons/...
```

Window 测试继续验证：

```text
WindowPlugin
→ WidgetryAssetPlugin
→ BuiltinIcon
→ AssetServer
→ Icon / SVG loader
→ Image
```

完整链路能够成功 materialize。

### Gallery

运行 Gallery 验证：

- Logo 正常显示；
- Window 三个常驻系统图标正常显示；
- 最大化/还原切换时 Restore 图标正常显示；
- 从 Gallery binary 启动时所有 embedded asset 均不依赖当前工作目录。

---

## 完成标准

完成后：

```text
Widgetry 库资源
    → 统一进入 crates/asset

Gallery 应用资源
    → 统一进入 gallery/src/assets

资源文件在哪里
    → 只有对应 Asset 模块知道

资源如何注册
    → 只有对应 AssetPlugin 知道

业务代码需要哪个资源
    → 通过语义资源标识表达

Widgetry 用户如何注册内部资源
    → 不需要处理

Gallery 资源是否进入库
    → 默认绝不进入
```

除负责资源管理的 Asset 模块自身外，不应再出现 Widgetry 或 Gallery 内建资源的具体文件路径字符串。
