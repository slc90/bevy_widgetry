# 在 Gallery 中展示 Theme、Stretch、Cover 与图片透明度

## 目标

让 Gallery 只作为 Window 背景能力的消费者，使用真实独立 Widgetry Window 展示 Theme、Stretch、Cover 和 opacity 的可见差异，并为人工 resize 验证提供稳定入口。

## 范围

本方案负责：

- Gallery 专用测试图片的 embedded asset 接入；
- 图片格式对应的最小 Bevy feature；
- Gallery 主窗口显式继续使用 Theme 背景；
- Independent Window 页面提供真实独立窗口对照；
- 使用同一张图片验证 Stretch / Cover；
- 增加一个明确的半透明图片窗口用于验证 opacity；
- 保持 Gallery 只是 Demo，不把实现逻辑留在 Gallery。

## 预期产出

用户可以在 Window 页面分别打开 Theme、Stretch、Cover 和半透明 Cover 窗口，通过拖动真实窗口尺寸直观看到背景模式和 opacity 的差别；Gallery 不包含自己的 crop 或 resize 算法。

## 与前后方案的关系

前面的 Window crate 已经完成能力和调用方迁移。本方案只消费公开 API。下一方案会对这些 Demo 和底层行为做统一自动化与手工验收。

---

## Gallery asset 归属

用户会在本地把测试图片预先调整到合适尺寸，再作为 Gallery asset 上传到仓库。

该图片只属于 Demo，不是 Widgetry 内建资源，因此放在 Gallery 自己的 asset 体系中，不进入 `crates/asset`。

在：

```text
gallery/src/assets.rs
```

增加 Gallery 专用图片标识，例如：

```rust
pub(crate) enum GalleryImage {
    WindowBackground,
}
```

提供：

```rust
GalleryImage::WindowBackground.path()
```

并通过：

```rust
embedded_asset!(app, "assets/...");
```

注册最终图片路径。

不增加 `BuiltinImage`，也不把 Gallery 测试图片暴露成 Window crate 的公共资源。

## 图片尺寸

Gallery 使用经过预处理、尺寸适中的图片，不直接嵌入原始超大图。

图片最好明显不是 `640×400` 的同一宽高比，否则 Stretch 和 Cover 在默认窗口尺寸下差异不明显。

实际分辨率由用户准备 asset 时决定，Window crate 不对它做自动 downscale。

## 图片格式 feature

Workspace 的 Bevy 使用：

```toml
default-features = false
```

因此最终 Gallery 图片使用哪种 raster 格式，就只在 Gallery 侧显式启用对应 loader feature。

如果使用 PNG：

```toml
bevy = { workspace = true, features = ["png"] }
```

如果使用其他格式，则启用对应的最小 feature。

不要为了一个 Demo 一次性打开所有图片格式。

## Gallery 主窗口

`gallery/src/main.rs` 的主 Widget Gallery 继续使用 Theme 背景，不把整个 Gallery 自身改成图片背景。

现有：

```rust
widgetry_window(...)
```

只因为 API 改签名而显式增加：

```rust
WidgetryWindowBackground::Theme
```

主 Gallery 的 Dark / Light 主题展示保持不变。

## Independent Window 页面

修改：

```text
gallery/src/pages/window/independent.rs
```

当前只有：

```text
Independent Window

[ Open Window ]
```

改成一组明确对照入口：

```text
Independent Window

[ Theme ]
[ Stretch ]
[ Cover ]
[ Cover 50% ]
```

前三个入口用于原有 Theme 与两种图片 mode 的对照；第四个入口专门让 opacity 有一个不会与 mode 对照混在一起的可视验证场景。

所有窗口都使用真实：

```rust
owned_widgetry_window(...)
```

并保持相同的：

- native window 默认尺寸，例如 `640×400`；
- title bar 内容；
- 正文内容；
- controls 配置。

这样 Stretch / Cover 的视觉差异只来自背景策略，不被其他 layout 差异干扰。

## Theme Demo

传：

```rust
WidgetryWindowBackground::Theme
```

用于证明现有纯色主题背景仍然存在，并继续跟随 Gallery 的 Theme 切换。

## Stretch Demo

`open_window` 或等价入口取得 `AssetServer`，加载：

```rust
GalleryImage::WindowBackground.path()
```

构造：

```rust
WidgetryWindowBackground::Image(
    WidgetryWindowImageBackground {
        image,
        mode: WidgetryWindowImageMode::Stretch,
        opacity: 1.0,
    }
)
```

拖宽或拖高窗口时，图片会填满整个 WindowRoot，但允许宽高比变形。

## Cover Demo

使用同一张图片：

```rust
WidgetryWindowBackground::Image(
    WidgetryWindowImageBackground {
        image,
        mode: WidgetryWindowImageMode::Cover,
        opacity: 1.0,
    }
)
```

拖动窗口时图片保持原图比例，始终铺满，并从中心动态裁剪。

Stretch 和 Cover 必须使用同一个 asset、相同窗口尺寸和相同内容，这样才能直观看出 mode 差异。

## Cover 50% Demo

再使用同一个 asset 与 Cover mode，仅把：

```rust
opacity: 0.5
```

用于人工观察整体透明度：

```text
图片 crop 行为与普通 Cover 完全相同
但整张背景图半透明
透出的是 native window 后方内容
不是 Theme window_background
```

这个 Demo 不需要提供 slider，也不增加运行时 opacity 修改。它只是构造期 opacity 的固定示例。

## 不做静态图片预览

不要另外做一个普通 UI card，里面放 Stretch 和 Cover 的静态图片对照。

本次真正要验证的是：

```text
WindowRoot 尺寸随着 native window resize 变化时
背景行为是否正确
```

因此应该打开真实独立窗口，然后实际横向、纵向拖动窗口边缘。

## Gallery 不拥有业务实现

Gallery 侧不写：

- Cover aspect ratio 计算；
- crop rect 计算；
- Window resize 监听；
- ImageNode.rect 同步；
- opacity shader 或自定义 material；
- Theme/Image fallback 逻辑。

这些全部属于 `crates/window`。Gallery 只通过公开构造 API 选择配置。
