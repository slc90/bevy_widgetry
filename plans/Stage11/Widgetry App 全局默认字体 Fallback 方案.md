# Widgetry App 全局默认字体 Fallback 方案

## 目标

为 Widgetry 增加一套 **App 级默认字体 fallback 机制**：

- Widgetry 自带一个默认字体。
- 默认字体使用 **Smiley Sans / 得意黑 `SmileySans-Oblique.ttf`**。
- 用户可以在 App 初始化阶段通过公开 API 替换默认字体。
- 该默认字体作用于整个 App，而不仅限于 Widgetry 控件。
- 用户显式为某个 `TextFont` 指定字体时，必须以用户字体为准。
- 不支持运行时统一切换既有文本字体，本功能只定义 App 初始化后的默认 fallback 行为。

Bevy 0.19.1 中 `TextFont::default()` 的 `font` 为 `FontSource::default()`，因此可直接把这个值作为“未显式指定字体”的判定标志。

---

## 对外 API

在 `bevy_widgetry_core` 中定义 App 扩展 trait，并由 facade 的 `style` 模块导出：

```rust
pub trait WidgetryAppExt {
    fn set_default_font(&mut self, font: FontSource) -> &mut Self;
}
```

典型用法：

```rust
use bevy::prelude::*;
use bevy::text::FontSource;
use bevy_widgetry::style::WidgetryAppExt;

App::new()
    .add_plugins(DefaultPlugins)
    .set_default_font(FontSource::Handle(my_font))
    .add_plugins(/* Widgetry plugins */)
    .run();
```

如果不调用：

```rust
set_default_font(...)
```

则使用 Widgetry 内建的得意黑。

`set_default_font()` 设计为 **App 初始化 API**，应在第一次 `App::update()` / `run()` 之前调用。

它接受完整的 `FontSource`，不额外设计 Widgetry 自有字体类型。

`FontSource::Family`、`SystemUi`、`SansSerif` 等系统/通用字体的实际解析能力继续遵循 Bevy 自身配置；用户加载自己的字体资产时，最直接的方式是：

```rust
FontSource::Handle(font_handle)
```

---

## 字体 fallback 规则

新增内部资源：

```rust
#[derive(Resource)]
struct DefaultFont(FontSource);
```

所有新加入世界的 `TextFont` 都执行一次判断：

```rust
if text_font.font == FontSource::default() {
    text_font.font = default_font.0.clone();
}
```

因此：

```rust
Text::new("Hello")
```

使用 Widgetry/App 默认字体。

只设置字号等其他属性：

```rust
TextFont::from_font_size(16.0)
```

由于 `font` 仍为 `FontSource::default()`，也使用 App 默认字体。

用户显式指定：

```rust
TextFont::from(FontSource::Handle(my_font))
```

或：

```rust
TextFont::from(FontSource::Monospace)
```

则 Widgetry 不修改。

需要接受一个明确边界：

```rust
FontSource::default()
```

本身就是“使用 App fallback”的哨兵值。因此即使用户手工显式写入该值，也仍然会被 fallback 替换。

---

## `WidgetryFontPlugin`

在 `core` 中增加内部共享插件：

```rust
pub struct WidgetryFontPlugin;
```

它负责两件事：

1. 初始化 `DefaultFont`。
2. 注册新 `TextFont` 的 fallback system。

该插件属于 Widgetry 内部基础设施，不作为推荐公共 API 暴露给最终用户。

### 默认字体初始化

不能简单：

```rust
init_resource::<DefaultFont>()
```

因为内建字体需要通过 `AssetServer` 获取 `Handle<Font>`。

正确逻辑是：

```text
如果 DefaultFont 已存在
    → 保留用户配置

否则
    → 确保 WidgetryAssetPlugin 已注册
    → 从内建字体路径加载 SmileySans-Oblique.ttf
    → 插入 DefaultFont(FontSource::Handle(handle))
```

这样同时支持两种插件顺序：

```rust
.set_default_font(...)
.add_plugins(StyledButtonPlugin)
```

以及：

```rust
.add_plugins(StyledButtonPlugin)
.set_default_font(...)
```

只要都发生在 App 真正开始运行之前，最终用户配置都会覆盖内建默认值。

---

## `set_default_font()` 同时确保插件存在

`set_default_font()` 不应只写资源，否则在没有注册任何 Widgetry styled plugin 的情况下，fallback system 不会运行。

因此实现语义应为：

```rust
fn set_default_font(&mut self, font: FontSource) -> &mut Self {
    self.insert_resource(DefaultFont(font));

    if !self.is_plugin_added::<WidgetryFontPlugin>() {
        self.add_plugins(WidgetryFontPlugin);
    }

    self
}
```

因此：

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .set_default_font(...)
```

即使后面完全没有使用 Widgetry 控件，对整个 App 的默认字体 fallback 仍然生效。

---

## Widgetry 插件自动接入

所有会创建、承载或组合文字内容的 Widgetry 样式/窗口插件，都应像现有 `ThemePlugin` 等共享插件一样：

```rust
if !app.is_plugin_added::<WidgetryFontPlugin>() {
    app.add_plugins(WidgetryFontPlugin);
}
```

至少应覆盖：

- Styled Button
- Styled ComboBox
- Styled TextField
- Window

这样用户即使不主动调用 `set_default_font()`，只要正常使用 Widgetry UI，就自动获得内建默认字体。

纯 headless、与文本无关的基础逻辑不需要强行依赖字体插件。

当前工程已经采用共享插件由上层控件自动确保注册的模式，例如 Window 会检查并自动添加 `WidgetryAssetPlugin`、`IconPlugin` 和 `ThemePlugin`，因此字体插件沿用相同模式即可。

---

## fallback system

核心 system：

```rust
fn apply_default_font(
    default_font: Res<DefaultFont>,
    mut fonts: Query<&mut TextFont, Added<TextFont>>,
) {
    for mut text_font in &mut fonts {
        if text_font.font == FontSource::default() {
            text_font.font = default_font.0.clone();
        }
    }
}
```

使用：

```rust
Added<TextFont>
```

而不是每帧扫描所有文本。

效果是每个 `TextFont` 第一次进入 ECS 时只判断一次。

这同时覆盖：

- UI `Text`
- `TextSpan`
- EditableText / TextField 使用的 `TextFont`
- `Text2d`
- 用户自己创建的普通 Bevy 文本
- Widgetry 内部创建的文本

Bevy UI 的 `Text` 本身会 require `TextFont`，因此普通：

```rust
commands.spawn(Text::new("Hello"));
```

也会进入这套机制。

---

## 调度

fallback system 放在：

```rust
PostUpdate
```

并明确：

```rust
.before(bevy::text::detect_text_needs_rerender)
```

原因是修改 `TextFont` 后，要让 Bevy 本帧的文字变化检测、测量和布局看到最终字体。

Bevy 0.19.1 的 UI text measurement 位于 `PostUpdate / UiSystems::Content`，并排在 `detect_text_needs_rerender` 之后，因此只要 fallback system 明确早于该检测系统即可进入正常文字 pipeline。

不需要引入字体层级传播，也不需要类似 `ForegroundColor` 的 `HierarchyPropagatePlugin`。

---

## Asset crate

字体实体仍属于 `asset` crate。

建议目录：

```text
crates/asset/
├── src/
│   └── assets/
│       ├── icons/
│       └── fonts/
│           └── SmileySans-Oblique.ttf
└── LICENSES/
    └── SmileySans-OFL-1.1.txt
```

当前 `asset` crate 已经集中维护内建图标资源，因此字体继续放在这里符合现有资源边界。

新增语义标识：

```rust
pub enum BuiltinFont {
    Default,
}
```

类似现有 `BuiltinIcon`：

```rust
impl BuiltinFont {
    pub fn path(self) -> AssetPath<'static> {
        // embedded SmileySans path
    }
}
```

`WidgetryAssetPlugin` 中增加：

```rust
embedded_asset!(app, "assets/fonts/SmileySans-Oblique.ttf");
```

`core` 增加对 `asset` crate 的内部依赖，用于：

- `BuiltinFont`
- `WidgetryAssetPlugin`

依赖方向保持：

```text
core → asset → bevy
```

不会形成循环。

---

## 字体与许可证

使用官方 **Smiley Sans / 得意黑 v2.0.1**：

```text
SmileySans-Oblique.ttf
```

官方来源：

- [Smiley Sans 官方 GitHub 仓库](https://github.com/atelier-anchor/smiley-sans)
- [Smiley Sans v2.0.1 Release 页面](https://github.com/atelier-anchor/smiley-sans/releases/tag/v2.0.1)
- [v2.0.1 官方字体发布包 smiley-sans-v2.0.1.zip](https://github.com/atelier-anchor/smiley-sans/releases/download/v2.0.1/smiley-sans-v2.0.1.zip)
- [v2.0.1 对应官方 LICENSE](https://github.com/atelier-anchor/smiley-sans/blob/v2.0.1/LICENSE)

官方 latest release 当前为 **v2.0.1**，其中提供官方字体发布包。
