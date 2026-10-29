# Asset

## 功能概述

集中管理 Widgetry 内建 Font、SVG Icon 与 Kamuri Cursor 图集，并通过语义标识向使用方提供可加载的 Asset Path。

## 当前设计思路

内建素材由 `WidgetryAssetPlugin` 注册为 Embedded Asset。其他 crate 依靠 `BuiltinFont`、`BuiltinIcon`、`BuiltinCursorImage`、`BuiltinCursorAnimation` 等标识获取路径，不自行依赖素材的实际目录布局。注册与加载是不同阶段。

## 主要功能

- 提供内建 CJK Font 路径，供需要默认字体的应用或基础设施使用。
- 提供 CheckBox、Tree、Window、FileDialog、Table 排序及筛选、Playback 播放/暂停等内建 SVG Icon 的语义路径。
- `BuiltinIcon::DatePickerToday` 提供 DatePicker 固定朱红色透明笔圈 SVG；笔触与颜色由素材本身确定。
- `BuiltinCursorAnimation` 提供 Kamuri 全部 45 种语义的独立动画图集、帧数、时长、热点及帧计算，包括 ReadOnly 扫描文档与蓝白挂锁动画。
- 动画与静态后备帧均为 80×80 RGBA 切片。动画图集为 8 列、行数随帧数确定，Layout 仅包含实际动画帧；静态后备图集为 8×6，Layout 仅包含 45 项。

## 行为与状态契约

`path()` 获取的是可交给 `AssetServer` 的标识，不表示图像、字体已加载。调用方必须先保证 Embedded Source 注册，再依赖正常 Asset 生命周期完成实际加载。内建字体注册不意味着强行替应用设置默认字体。

`BuiltinCursorImage::KamuriStaticAtlas` 的每项均为对应动画的首帧，使用相同的画布和热点。Cursor 播放与 Gallery 预览共用动画目录，不在调用方复制素材或时序表。

## 边界情况与错误处理

缺少 Embedded 注册、Asset Loader 或素材尚未就绪时，应按照使用该素材的上层 Widget 的等待或失败契约处理，不能把取得 Asset Path 等同于成功解码。

## 已知限制

- 只负责素材与 Embedded 注册，不负责上层 SVG 渲染和布局。
- 不管理 Gallery 的应用专属素材。
- 目录提供动画元数据与帧计算，不持有窗口播放原点、Cursor 控制权或重绘生命周期；这些由 Cursor 或 Gallery 消费者管理。


`BuiltinShader::HoldButton` 提供长按控件内建液面 WESL Shader 的语义路径，由 Asset Plugin 注册。

`BuiltinShader::CanvasStraightAlpha` 提供 Canvas 离屏颜色从预乘 Alpha 转为原生 Viewport 所需表示的 WESL 路径。
