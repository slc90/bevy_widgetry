# Stage 6：SVG Icon 支持总结

## 目标

Stage 6 的目标是为 `bevy_widgetry` 增加一套独立于具体 SVG 渲染库的 `Icon` 控件能力，最终让控件库可以使用 SVG 图标，并支持：

- SVG 作为 Bevy 自定义 Asset 加载；
- 保持 SVG 原始画布和宽高比；
- 按指定尺寸重新栅格化，避免 UI 放大低分辨率图片导致模糊；
- 栅格结果缓存；
- 运行时切换 SVG；
- 默认跟随 `ForegroundColor`；
- 显式指定 Icon 颜色时覆盖继承颜色；
- 运行时 `set_color()` / `clear_color()`；
- Theme / ForegroundColor 改变时只更新 tint，不重新 rasterize。

---

## 一、最终整体链路

```text
.svg
 ↓
AssetServer
 ↓
SvgAssetLoader
 ↓
usvg::Tree
 ↓
resvg
 ↓
tiny-skia::Pixmap
 ↓
Bevy Image
 ↓
Handle<Image>
 ↓
IconImageCache
 ↓
ImageNode
 ↓
ImageNode.color tint
```

职责划分：

```text
svg.rs
├─ SVG Asset
├─ SVG Loader
├─ usvg::Tree
└─ Tree -> Pixmap -> Image

icon.rs
├─ public Icon API
├─ IconPlugin
├─ raster spec
├─ raster cache
├─ Icon UI materialize
├─ SVG runtime update
└─ ForegroundColor / explicit color sync
```

---

## 二、依赖选择

最终没有使用 `bevy_resvg`。

原因是 `bevy_resvg` 内部的 `SvgVectorAsset` 对外不可直接使用，而我们的需求又需要自己保存解析后的 SVG Tree，并控制任意尺寸 raster。

最终直接依赖：

```toml
resvg = "0.48.1"
```

该依赖放在：

```text
crates/core/Cargo.toml
```

而不是 workspace 公共依赖中，因为这是 `core` 的内部实现细节。

---

## 三、SvgAsset

SVG 文件加载后，不直接保存 PNG / Image，而是保存解析完成的：

```rust
usvg::Tree
```

大致结构：

```rust
#[derive(Asset, TypePath)]
pub(crate) struct SvgAsset {
    tree: Tree,
}
```

这样同一个 SVG：

```text
文件读取
XML/CSS/SVG parse
```

只需要做一次。

之后可以从同一棵 `Tree` 生成：

```text
16x16
24x24
32x32
64x64
96x96
...
```

不同尺寸的 raster image。

---

## 四、SvgAssetLoader

Loader 完成：

```text
Reader
 ↓
Vec<u8>
 ↓
usvg::Tree::from_data()
 ↓
SvgAsset
```

并注册：

```rust
app.init_asset::<svg::SvgAsset>()
   .init_asset_loader::<svg::SvgAssetLoader>();
```

同一路径重复：

```rust
asset_server.load::<SvgAsset>("icons/check.svg")
```

Bevy 会复用同一个底层 Asset，因此 Loader 不会重复解析同一路径。

---

## 五、SVG Raster 规则

### 1. 保留 SVG 声明画布

使用：

```rust
tree.size()
```

作为 SVG 原始尺寸。

不使用内容 bbox，不裁剪内部留白，也不对 SVG 内部内容重新居中。

例如 SVG：

```text
canvas = 24 x 12
```

即使内容本身只占其中一部分，也仍然认为源尺寸是：

```text
24 x 12
```

---

### 2. contain 缩放

指定最大尺寸：

```text
max_width × max_height
```

缩放比例：

```text
scale = min(
    max_width / source_width,
    max_height / source_height
)
```

例如：

```text
source = 24 x 12
max    = 80 x 80
```

得到：

```text
scale = min(80/24, 80/12)
      = 3.333...

raster = 80 x 40
```

不会生成一个人为填充的 `80x80 Pixmap`。

---

### 3. raster 尺寸向上取整

理论像素尺寸：

```rust
width  = (source_width  * scale).ceil() as u32;
height = (source_height * scale).ceil() as u32;
```

但 render transform 仍然使用原来的统一 `scale`：

```rust
Transform::from_scale(scale, scale)
```

不会根据 ceil 后的整数宽高重新计算 X/Y scale。

因此始终保持统一缩放。

---

### 4. intrinsic raster

未指定 Icon 尺寸时：

```text
scale = 1.0
```

直接按 SVG intrinsic size raster。

---

## 六、Pixmap -> Bevy Image

Raster 后的：

```text
tiny_skia::Pixmap
```

直接转换为：

```text
Bevy Image
```

使用：

```rust
TextureFormat::Rgba8Unorm
```

无需先编码为 PNG 再由 Bevy 解码。

---

## 七、Icon 公共 API

最终 `Icon` 的核心状态：

```rust
#[derive(Component)]
#[require(Node)]
pub struct Icon {
    svg: Handle<svg::SvgAsset>,
    max_size: Option<UVec2>,
    color: Option<Color>,
}
```

### 创建

```rust
Icon::new(
    &asset_server,
    "icons/check.svg",
)
```

默认：

```text
intrinsic size
color = None
```

---

### 固定逻辑尺寸

```rust
Icon::new(
    &asset_server,
    "icons/check.svg",
)
.with_size(24, 24)
```

语义：

```text
Icon 外层 Node = 24 x 24
SVG raster      = contain 到 24 x 24
ImageNode       = raster 的真实尺寸
ImageNode       = 在 Icon Node 内居中
```

---

### 创建时指定颜色

```rust
Icon::new(
    &asset_server,
    "icons/check.svg",
)
.with_color(Color::srgb(1.0, 0.0, 0.0))
```

显式颜色优先于传播来的 `ForegroundColor`。

---

### 运行时切换 SVG

```rust
icon.set_svg(
    &asset_server,
    "icons/up.svg",
);
```

支持例如：

```text
ComboBox closed
→ down arrow

ComboBox open
→ up arrow
```

---

### 运行时设置颜色

```rust
icon.set_color(
    Color::srgb(1.0, 0.0, 0.0),
);
```

此后 Icon 使用显式颜色，不再受传播颜色影响。

---

### 恢复继承颜色

```rust
icon.clear_color();
```

恢复：

```text
Icon.color = None
```

然后立即重新使用当前传播到 Icon 上的 `ForegroundColor`。

---

## 八、Icon UI 结构

最终结构：

```text
Icon Entity
├─ Icon
├─ Node
├─ ForegroundColor（可能由父级传播）
└─ child
   ├─ IconImage
   └─ ImageNode
```

`Icon` 是一个 UI 容器。

外层：

```rust
Node {
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    ...
}
```

如果 `.with_size(w, h)`：

```text
Node.width  = w
Node.height = h
```

如果没有 `.with_size()`：

```text
Node.width  = Auto
Node.height = Auto
```

由内部 raster image 的 intrinsic size 撑开。

---

## 九、IconImage marker

内部生成的图片节点带：

```rust
#[derive(Component)]
struct IconImage;
```

用于区分：

```text
Icon 自己生成的 ImageNode
```

和应用里普通的：

```text
ImageNode
```

因此颜色同步 system 不会误修改其他图片。

---

## 十、Raster Cache

Cache key：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic,
    MaxSize {
        width: u32,
        height: u32,
    },
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconImageCacheKey {
    svg_asset_id: AssetId<svg::SvgAsset>,
    raster_spec: IconRasterSpec,
}
```

Cache：

```rust
#[derive(Resource, Default)]
struct IconImageCache {
    images: HashMap<
        IconImageCacheKey,
        Handle<Image>,
    >,
}
```

使用 Bevy：

```rust
bevy::platform::collections::HashMap
```

---

## 十一、Cache 生命周期

Cache 中保存：

```rust
Handle<Image>
```

这是 strong handle。

因此已经 rasterize 的 Icon Image 会随着：

```text
IconImageCache
```

一直保留。

当前设计明确选择：

```text
整个 App 生命周期缓存
```

不做：

```text
LRU
eviction
自动释放
```

原因是常规桌面控件库中：

```text
SVG 数量有限
常用尺寸有限
组合基本稳定
```

没有必要为了极少见的 SVG 编辑器 / 大量动态资源场景增加复杂度。

---

## 十二、resolve_icon_image_handle

`materialize_icons` 和 `update_pending_icons` 原本存在重复逻辑：

```text
构造 cache key
→ cache lookup
→ rasterize
→ Assets<Image>::add
→ cache insert
```

最终抽为内部 helper：

```rust
resolve_icon_image_handle(...)
```

职责统一为：

```text
Icon + SvgAsset + raster spec
↓
已有 cache
    → clone Handle<Image>

没有 cache
    → rasterize
    → add Image
    → cache
    → Handle<Image>
```

这样：

```text
materialize_icons
```

只负责创建 UI child，

而：

```text
update_pending_icons
```

只负责替换现有图片。

---

## 十三、首次 Materialize

首次发现：

```text
Icon
但还没有 IconMaterialized
```

时：

1. 等待 `SvgAsset` ready；
2. resolve raster image；
3. 设置 Icon Node 布局；
4. 创建 child ImageNode；
5. 记录 child Entity；
6. 记录当前实际显示的 SVG AssetId。

状态：

```rust
#[derive(Component)]
struct IconMaterialized {
    image_entity: Entity,
    svg_asset_id: AssetId<svg::SvgAsset>,
}
```

`image_entity` 用于以后直接定位内部 `ImageNode`。

---

## 十四、运行时 SVG 更新

单纯使用：

```rust
Changed<Icon>
```

不足以跨越 Asset 的异步加载过程。

因为：

```text
set_svg()
↓
Icon Changed
↓
新 SvgAsset 这一帧可能尚未 ready
↓
下一帧 Changed<Icon> 已经结束
```

因此增加：

```rust
#[derive(Component)]
struct IconPendingUpdate;
```

---

### mark_changed_icons

当 `Icon` Changed 时，不再直接认为 SVG 一定变了。

而是比较：

```text
icon.svg.id()
```

和：

```text
materialized.svg_asset_id
```

只有真的不同才：

```text
insert IconPendingUpdate
```

这样：

```text
set_color()
clear_color()
```

虽然也会产生 `Changed<Icon>`，但不会被错误地当成 SVG 更新。

---

### update_pending_icons

只处理：

```text
With<IconPendingUpdate>
```

如果新 `SvgAsset` 还没 ready：

```text
continue
```

并保留 `IconPendingUpdate`。

下一帧继续尝试。

加载成功后：

```text
resolve image handle
→ 替换 ImageNode.image
→ 更新 IconMaterialized.svg_asset_id
→ remove IconPendingUpdate
```

因此切换 SVG 时旧图片会一直保留，直到新图片已经可以显示。

---

## 十五、ForegroundColor tint

SVG raster image 不因为 Theme / ForegroundColor 改变而重新生成。

颜色直接通过：

```rust
ImageNode.color
```

进行 tint。

基本关系：

```text
texture color × tint color
```

因此单色 SVG 推荐使用白色图形：

```text
white × target color
= target color
```

透明区域继续保持透明，抗锯齿也由现有 raster alpha / sampling 保留。

---

## 十六、颜色优先级

最终颜色规则：

```text
Icon explicit color
        ↓
ForegroundColor
        ↓
Color::WHITE
```

即：

```rust
icon.color
    .or_else(|| {
        foreground_color
            .map(|foreground| foreground.0)
    })
    .unwrap_or(Color::WHITE)
```

---

### 情况 1：无显式颜色

```text
Icon.color = None
ForegroundColor = Blue
```

结果：

```text
Blue
```

---

### 情况 2：运行时 set_color

```text
Icon.color = Red
ForegroundColor = Blue
```

结果：

```text
Red
```

传播并没有停止。

只是：

```text
explicit Icon color
```

优先级更高，因此忽略传播颜色。

---

### 情况 3：父级颜色继续变化

```text
Icon.color = Red
ForegroundColor:
Blue -> Green
```

结果仍然：

```text
Red
```

---

### 情况 4：clear_color

```rust
icon.clear_color();
```

变成：

```text
Icon.color = None
ForegroundColor = Green
```

结果立即恢复：

```text
Green
```

因此不需要真的阻止 `ForegroundColor` 传播。

---

## 十七、颜色同步 System

颜色同步监听：

```text
Changed<Icon>
或
Changed<ForegroundColor>
```

原因：

```text
set_color()
clear_color()
```

会改变 `Icon`；

Theme / 父控件变化则会改变传播来的：

```text
ForegroundColor
```

最终都只更新：

```rust
ImageNode.color
```

不会重新 rasterize。

---

## 十八、已经实际验证的行为

### SVG Loader 去重

同一路径多次 `AssetServer::load()`：

```text
Loader 只执行一次
```

---

### Raster Cache

例如：

```text
check.svg @ 96x96
check.svg @ 96x96
check.svg @ 64x64
```

结果：

```text
miss
hit
miss
```

---

### 高分辨率 raster

原来直接使用低分辨率 raster 后由 UI 放大，会明显模糊。

现在：

```text
目标 96x96
→ SVG Tree 直接 raster 到对应尺寸
```

显示清晰。

---

### 非正方形 SVG

使用：

```text
wide.svg
source = 24 x 12
Icon box = 80 x 80
```

实际：

```text
raster image = 80 x 40
outer Node   = 80 x 80
```

图片在外层 Node 中垂直居中。

说明：

```text
aspect ratio
contain
UI centering
```

全部正确。

---

### 动态 SVG 切换

使用：

```text
check.svg
↔
wide.svg
```

每 2 秒自动切换。

验证：

```text
set_svg()
pending update
async asset ready
image replacement
cache reuse
```

全部正常。

---

### ForegroundColor 传播

父 Node：

```text
Propagate<ForegroundColor>
```

Icon 不指定颜色。

测试：

```text
Red ↔ Blue
```

Icon 可以正确跟随变化。

---

### 显式颜色覆盖传播

测试流程：

```text
父 ForegroundColor = Blue
Icon.color = None
→ Blue

set_color(Red)
→ Red

父 ForegroundColor = Green
→ 仍然 Red

clear_color()
→ Green
```

全部验证通过。

---

## 十九、当前明确不做的内容

### 动态尺寸修改

目前：

```rust
with_size(...)
```

只负责初始化 Icon 固定逻辑尺寸。

暂不增加：

```text
set_size()
动态尺寸变化
```

等整个控件库未来统一做 UI Scale / DPI 相关设计时再一起处理。

---

### Cache eviction

暂不实现：

```text
LRU
超时回收
弱引用 cache
```

---

### SVG 内容 bbox 裁剪

不裁剪：

```text
SVG 内部留白
```

始终尊重 SVG 自己声明的画布。

---

### Theme 直接依赖

`Icon` 本身不直接依赖：

```text
ThemeMode
ColorTheme
ThemeChanged
```

它只关心：

```text
ForegroundColor
```

因此：

```text
Theme / Button / Label / 其他父控件
↓
ForegroundColor
↓
Icon
```

Icon 与 Theme 系统保持解耦。

---

## 二十、Stage 6 最终能力

Stage 6 完成后，`Icon` 已具备：

```text
✓ SVG Asset 加载
✓ usvg Tree 复用
✓ 任意目标尺寸 raster
✓ intrinsic size
✓ contain 缩放
✓ aspect ratio 保持
✓ SVG 画布留白保留
✓ Pixmap -> Bevy Image
✓ Image raster cache
✓ Icon UI Node
✓ Image child 居中
✓ 动态 set_svg
✓ 异步 pending update
✓ ForegroundColor 传播
✓ ImageNode tint
✓ with_color
✓ set_color
✓ clear_color
✓ explicit color > ForegroundColor
✓ Theme 改色不重新 raster
```

---

# Stage 6 结论

Stage 6 已经不只是“让 Bevy 能显示 SVG”。

最终形成的是一层真正属于 `bevy_widgetry` 的 Icon 抽象：

```text
业务控件
↓
Icon API
↓
SVG Asset / Raster Cache / ImageNode
↓
resvg
```

上层控件无需知道：

```text
usvg
resvg
tiny-skia
Pixmap
```

未来无论 SVG 后端如何变化，Button、ComboBox、Menu 等控件依然只依赖：

```rust
Icon
```

这正好满足整个控件库的边界：

> 控件库负责 Icon 自身的行为、渲染、尺寸、颜色和资源管理；
> 业务层只负责选择“用哪个图标”。

至此，Stage 6 可以视为完成。
