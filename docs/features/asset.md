# 内建资源

Asset 集中注册 Widgetry 自身使用的字体、SVG 图标和 Shader，并以语义标识提供资源路径。它主要服务库内模块，不经 Facade 导出。

## 资源注册

### 在独立环境注册内建资源

```rust
use bevy::prelude::*;
use bevy_widgetry_asset::WidgetryAssetPlugin;

app.add_plugins(AssetPlugin::default());
app.add_plugins(WidgetryAssetPlugin);
```

普通应用已有 AssetPlugin 时不重复添加。使用内建资源的 Widgetry 插件会确保自身需要的注册，宿主不需要再为每种控件分别安装 AssetPlugin。

注册只把内嵌资源接入 Bevy Asset 来源，不等于字体、SVG 或 Shader 已完成加载和处理。

## 字体资源

### 加载默认字体

```rust
use bevy_widgetry_asset::BuiltinFont;

let font: Handle<Font> = asset_server.load(BuiltinFont::Default.path());
let emoji: Handle<Font> = asset_server.load(BuiltinFont::Emoji.path());
```

`Default` 是默认 CJK 字体，`Emoji` 提供彩色表情字体。希望直接配置应用字体时，使用 [Core](core.md) 的默认字体能力，避免为每段文字重复建立字体列表。

## 图标资源

### 把语义图标交给 Icon

```rust
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry::icon::WidgetryIcon;

bsn! {
    @WidgetryIcon {
        @path: {BuiltinIcon::WindowClose.path()},
        @max_size: {Some(UVec2::splat(20))}
    }
}
```

`BuiltinIcon` 包括窗口操作、Checkbox 标记、Tree 展开、文件浏览、表格请求和播放控制等图标。调用方使用语义标识，不拼接内部文件目录。

SVG 的解析、栅格化和颜色呈现由 Icon 负责，Asset 本身不把 SVG 当成已经解码的 Image。需要应用自己的图标时，可直接给 Icon 提供应用资源路径，不必加入内建枚举。

## Shader 资源

### 取得专用 Shader 路径

```rust
use bevy_widgetry_asset::BuiltinShader;

let alpha_shader = BuiltinShader::CanvasStraightAlpha.path();
let hold_shader = BuiltinShader::HoldButton.path();
```

这些路径供对应渲染实现使用。取得路径不会自动安装材质、渲染管线或相机后处理。

## 应用资源

应用自己的 Logo、图片、图标与光标图集由应用注册和持有。内建资源的存在不要求应用放弃自己的 Asset Source；两者可以在同一个 AssetServer 中使用。

内建资源属于库实现的组成部分，不把物理目录布局当作业务接口。读取和加载失败应由实际消费者按字体、图标或渲染语境反馈。
