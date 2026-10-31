# WidgetryIcon 初始化路径校验

`WidgetryIcon` 使用 SVG 路径确定初始图标，并在资源可用后将 SVG 栅格化为 UI 图片。

当前 `WidgetryIconProps::default()` 的 `path` 为空，而 `WidgetryIcon::scene()` 没有明确拒绝这一配置，可能构造出没有初始图标的实例。

本次修改要求：**SVG 路径在 Scene 构造时必须提供，空路径直接报错；非空路径继续异步加载。**

## 初始化行为

通过 `@WidgetryIcon` 创建 Scene 时，检查 `WidgetryIconProps.path`：

- 未指定 `path`：记录错误，返回 `BevyError`。
- 显式传入空路径：同样返回 `BevyError`。
- 传入非空路径：允许正常构造，不要求 SVG 资源已经加载完成。

Scene 构造阶段只负责校验参数，不检查资源文件是否存在。资源加载、SVG 解析及栅格化错误继续由现有机制处理。

保留 `WidgetryIconProps::default()`，但其默认值不代表能够直接构造有效图标。`max_size` 和 `colors` 的默认行为不变。

## 运行时图标切换

保留 `WidgetryIcon::set_svg_in_world()` 和 `WidgetryIcon::set_svg()`。

这两个接口用于运行时切换已经构造的图标，不承担创建空图标后再补充初始路径的职责。

不修改现有 SVG 更新、缓存、加载等待和图片替换机制。

## 影响范围

检查使用 `@WidgetryIcon` 的内部控件、示例和测试，确认调用方已经提供有效的初始路径。

如果存在依赖空路径占位、之后再调用 `set_svg_in_world()` 的用法，应调整为构造时提供初始 SVG。

不改变绕过 `WidgetryIconProps`、直接构造底层 `WidgetryIcon` 组件的内部接口。

## 验收要求

- 未指定路径或显式传入空路径时，Scene 构造返回可捕获的 `BevyError`，不发生 panic。
- 提供非空路径时，Scene 构造成功，不依赖资源是否已经加载完成。
- 原有 SVG 资源加载及错误处理行为不变。
- `max_size`、颜色覆盖及默认样式行为不变。
- `set_svg_in_world()` 和 `set_svg()` 仍能正常切换图标。
- 相关调用方、示例及测试通过回归验证。

本次修改仅涉及 `WidgetryIcon` 的初始化校验及直接受影响的代码、测试和说明。其他控件的默认配置与初始化行为保持现状。
