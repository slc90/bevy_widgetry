# 建立 Window 背景 API 与渲染语义

## 目标

把“窗口背景”变成 `crates/window` 的正式构造期能力，而不是 Gallery 的临时效果。完成后，主窗口、borrowed 独立窗口和 owned 独立窗口都通过同一套公开配置选择 Theme 背景或图片背景；图片背景同时携带显示模式与整体透明度。

## 范围

本方案负责：

- 公开背景配置类型及两个 Window 构造函数的新签名；
- Theme 与 Image 两种背景的互斥语义；
- `Stretch` 与 `Cover` 的公开模式定义，但不在本方案展开 Cover 的裁剪同步算法；
- 图片 `opacity` 的构造期语义；
- `WindowRoot` 直接承担背景绘制，不增加 background child；
- ThemeChanged 对背景色、Window border 与 TitleBar border 的职责拆分；
- 图片未加载时保持透明；
- 现有圆角和最大化圆角切换继续作用于图片背景。

本方案不负责 Gallery 接入、调用方迁移和 Cover 的运行时同步细节。

## 预期产出

`crates/window` 获得一套明确、强制、无兼容包装的背景构造契约。Theme 背景继续沿用现有主题颜色；Image 背景完全由 `ImageNode` 绘制，并可在构造时指定 `Stretch`/`Cover` 和 `opacity`。

## 与前后方案的关系

这是整条执行链的第一步。后续 Cover 同步、MessageBox 迁移、Gallery Demo 和测试都依赖这里确定的公开类型和运行语义。

---

## 公开背景模型

新增：

```rust
pub enum WidgetryWindowBackground {
    Theme,
    Image(WidgetryWindowImageBackground),
}

pub struct WidgetryWindowImageBackground {
    pub image: Handle<Image>,
    pub mode: WidgetryWindowImageMode,
    pub opacity: f32,
}

pub enum WidgetryWindowImageMode {
    Stretch,
    Cover,
}
```

`WidgetryWindowBackground` 不提供 `Default`。既然窗口构造函数已经要求调用方明确选择背景，就不再提供一个隐式默认入口。

`opacity` 是图片背景整体透明度，语义区间为 `0.0..=1.0`：

```text
1.0 → 完全不透明
0.7 → 70% 不透明度
0.3 → 30% 不透明度
0.0 → 完全透明
```

透明度只属于 `Image` 背景，Theme 背景仍完全由 `theme.window_background` 决定。本轮只支持构造时给定 opacity，不提供运行时修改 API。

## 构造函数签名

两个现有入口统一增加强制 `background` 参数：

```rust
pub fn widgetry_window(
    target_window: Entity,
    target_camera: Entity,
    controls: WidgetryWindowControlsConfig,
    background: WidgetryWindowBackground,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene
```

```rust
pub fn owned_widgetry_window(
    native_window: Window,
    controls: WidgetryWindowControlsConfig,
    background: WidgetryWindowBackground,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene
```

参数顺序保持为：

```text
window / camera
→ controls
→ background
→ title bar content
→ content
```

不保留旧签名，不增加 wrapper，也不为了兼容现有代码设计第二套入口。

## Theme 背景

`WidgetryWindowBackground::Theme` 保持当前窗口纯色背景语义：

```text
WindowRoot
+ BackgroundColor(theme.colors().window_background)
+ ThemeWindowBackground
```

增加私有 marker：

```rust
#[derive(Component)]
struct ThemeWindowBackground;
```

这个组件只表示：当前 `WindowRoot` 的背景颜色由 Widgetry Theme 管理。

Theme 切换时，只有拥有该 marker 的 `BackgroundColor` 需要更新。图片背景窗口不会拥有 `BackgroundColor`，也不会拥有 `ThemeWindowBackground`。

## Image 背景

图片直接作为 `ImageNode` 挂在 `WindowRoot` 本身，不额外创建绝对定位 background child。

窗口树仍然保持：

```text
WindowRoot
├── TitleBar
├── WindowContent
└── ResizeArea
```

区别只在于 `WindowRoot` 自己使用哪一种视觉背景：

```text
Theme
→ BackgroundColor

Image
→ ImageNode
```

因此图片覆盖的是整个 WindowRoot，而不是只覆盖正文；title bar 和 content 都绘制在同一张窗口背景图之上。

Theme 与 Image 是互斥背景模式。图片背景不叠加 Theme 纯色，也不在图片下面保留 `BackgroundColor`。

### 图片未加载

当调用方已经提供 `Handle<Image>`，但 `Assets<Image>` 中尚未得到实际图片资源时：

```text
无 BackgroundColor
+ ImageNode 暂时没有可绘制纹理
→ WindowRoot 背景透明
```

不使用 `theme.window_background` 作为 fallback，也不增加任何临时占位色。

由于 Widgetry Window 的 native window 与 Camera 本来就是透明链路，图片背景透明区域以及加载前的空背景会直接显示窗口后方内容。

### 图片透明度

图片整体透明度通过 `ImageNode` 的颜色乘法表达，只改变 alpha，不对 RGB 做 tint：

```rust
ImageNode {
    image,
    color: Color::srgba(1.0, 1.0, 1.0, opacity),
    ..
}
```

`opacity` 对 `Stretch` 和 `Cover` 使用同一语义，不改变 Cover 的裁剪计算，也不引入新的背景层。

当 `opacity < 1.0` 时，透出的仍然是 native transparent window 后方内容，而不是 Theme 背景色，因为 Image 模式从一开始就不挂 `BackgroundColor`。

本轮不支持运行时修改 opacity；它和 image handle、mode 一样属于窗口构造期配置。

## Stretch

`WidgetryWindowImageMode::Stretch` 使用：

```rust
NodeImageMode::Stretch
```

并保持：

```rust
ImageNode.rect = None
```

图片直接映射到整个 `WindowRoot`。窗口 resize 后由 Bevy UI 按新的 node 尺寸重新绘制，不需要 Widgetry 自己做尺寸同步。

Stretch 不保持原图宽高比。例如：

```text
原图：1000 × 800
窗口：1920 × 1080

Stretch：
1000 × 800
→ 1920 × 1080
```

因此横向或纵向 resize 都可能让图片变形，但整张图片始终完整填满窗口。

## ThemeChanged 的职责拆分

当前 `refresh_window_theme()` 同时查询 `BackgroundColor` 和 `BorderColor`：

```rust
Query<(&mut BackgroundColor, &mut BorderColor), With<WindowRoot>>
```

引入 Image 背景后不能继续这样绑定，因为 Image WindowRoot 不再拥有 `BackgroundColor`。如果保持原 query，图片窗口会连 Window border 都一起被排除。

因此 Theme 刷新应拆成三类职责：

```text
所有 WindowRoot
→ 更新 window border

所有 TitleBar
→ 更新 title bar border

With<ThemeWindowBackground>
→ 更新 BackgroundColor
```

这样 Image 背景本身不响应 ThemeChanged，但窗口边框和 title bar 边框仍保持现有主题行为。

图片模式下以下内容都不随 Theme 改变：

- image handle；
- `Stretch` / `Cover` mode；
- crop rect 的语义；
- opacity；
- 图片 RGB。

## WindowRoot、圆角与层级

图片继续使用现有 `WindowRoot`：

```rust
width: percent(100),
height: percent(100),
border: UiRect::all(px(1)),
border_radius: BorderRadius::all(px(8)),
```

不额外增加用于背景绘制或 clipping 的 child。

普通窗口状态下，图片跟随现有 WindowRoot 的 `8px` 圆角；最大化后现有逻辑把圆角切成 `0px`，图片背景跟随同一个 root 的最终外形。

如果实际运行验证发现 Bevy 的 `ImageNode + BorderRadius` 在当前结构上存在视觉异常，再单独处理；本轮不提前引入 mask 或专门 background child。

## 本方案明确不引入的设计

这里不做：

- 固定自定义纯色 `Color(Color)` 背景；
- Theme 色与图片叠加；
- 图片加载前 Theme fallback；
- 运行时切换 Theme / Image；
- 运行时替换 image；
- 运行时修改 mode；
- 运行时修改 opacity；
- 通用 `ImageBackground` core 组件。

这些都不是本轮 Window 图片背景能力成立所必需的内容。
