# Asset

## 功能概述

集中管理 Widgetry 内建 Font、SVG Icon 与 Kamuri Cursor 图集，并通过语义标识向使用方提供可加载的 Asset Path。

## 当前设计思路

内建素材由 `WidgetryAssetPlugin` 注册为 Embedded Asset。其他 crate 依靠 `BuiltinFont`、`BuiltinIcon`、`BuiltinCursorImage` 等标识获取路径，不自行依赖素材的实际目录布局。注册与加载是不同阶段。

## 主要功能

- 提供内建 CJK Font 路径，供需要默认字体的应用或基础设施使用。
- 提供 CheckBox、Tree、Window、FileDialog 等内建 SVG Icon 的语义路径。
- 提供 Kamuri 44 种 Cursor 样式所需的静态图集，以及 Pointer、Wait 动画图集。
- 提供相应 Atlas Layout：普通静态光标使用 48px 切片；Pointer、Wait 使用 64px；静态图集使用 96px Slot 排列。

## 行为与状态契约

`path()` 获取的是可交给 `AssetServer` 的标识，不表示图像、字体已加载。调用方必须先保证 Embedded Source 注册，再依赖正常 Asset 生命周期完成实际加载。内建字体注册不意味着强行替应用设置默认字体。

## 边界情况与错误处理

缺少 Embedded 注册、Asset Loader 或素材尚未就绪时，应按照使用该素材的上层 Widget 的等待或失败契约处理，不能把取得 Asset Path 等同于成功解码。

## 已知限制

- 只负责素材与 Embedded 注册，不负责上层 SVG 渲染和布局。
- 不管理 Gallery 的应用专属素材。
- 当前 Kamuri 静态图与动画图采用不同的实际提取尺寸；图集 Slot 大小不等于每个 Cursor 的实际尺寸。


`BuiltinShader::HoldButton` 提供长按控件内建液面 WESL Shader 的语义路径，由 Asset Plugin 注册。
