# Asset

## 功能概述

集中管理 Widgetry 内建 Font、SVG Icon 与 Shader，并通过语义标识向使用方提供可加载的 Asset Path。

## 当前设计思路

内建素材由 `WidgetryAssetPlugin` 注册为 Embedded Asset。其他 crate 依靠 `BuiltinFont`、`BuiltinIcon`、`BuiltinShader` 等标识获取路径，不自行依赖素材的实际目录布局。注册与加载是不同阶段。

Kamuri Cursor 素材和元数据属于 Gallery，由应用 Asset Plugin 注册；库的 Asset crate 不包含角色图像、动画目录或对应 Feature。

## 主要功能

- `BuiltinFont::Default` 提供得意黑 CJK 字体，`BuiltinFont::Emoji` 提供 Noto Color Emoji；两者均由同一 Asset Plugin 注册，来源版本和许可证见 `crates/asset/LICENSES/README.md`。
- 提供 CheckBox、Tree、Window、FileDialog、Table 排序及筛选、Playback 播放/暂停等内建 SVG Icon 的语义路径。
- FileDialog 的后退、前进、上一级、刷新和搜索使用 Fluent 20 Regular 图形，按控件前景色着色；素材来源版本及随附许可证见 `crates/asset/LICENSES/README.md`。
- `BuiltinIcon::DatePickerToday` 提供 DatePicker 固定朱红色透明笔圈 SVG；笔触与颜色由素材本身确定。



## 行为与状态契约

`path()` 获取的是可交给 `AssetServer` 的标识，不表示图像、字体已加载。调用方必须先保证 Embedded Source 注册，再依赖正常 Asset 生命周期完成实际加载。内建字体注册不意味着强行替应用设置默认字体。



## 边界情况与错误处理

缺少 Embedded 注册、Asset Loader 或素材尚未就绪时，应按照使用该素材的上层 Widget 的等待或失败契约处理，不能把取得 Asset Path 等同于成功解码。

## 已知限制

- 只负责素材与 Embedded 注册，不负责上层 SVG 渲染和布局。
- 不管理 Gallery 的应用专属素材。



`BuiltinShader::HoldButton` 提供长按控件内建液面 WESL Shader 的语义路径，由 Asset Plugin 注册。

`BuiltinShader::CanvasStraightAlpha` 提供 Canvas 离屏颜色从预乘 Alpha 转为原生 Viewport 所需表示的 WESL 路径。
