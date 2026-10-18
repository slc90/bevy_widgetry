# Core

## 功能概述

提供 Widgetry 多个控件共同需要的 UI 基础设施，包括内容配色、默认字体、SVG Icon、有效 Disabled、Pointer State、Scene 错误处理和 UI 准备阶段能力。

## 当前设计思路

Core 尽量把跨 Widget 的机制集中管理，同时让各 Widget 保有自己的业务 State。共享能力通过专门 Plugin、Component 或公开适配方法装配，不要求应用接受一套取代 Bevy 的 UI Runtime。

## 主要功能

- 默认字体设置：只对适用的新增 `TextFont` 使用缺省字体，保留调用方的显式字体。
- `WidgetryText` 和内容 Foreground：按最近颜色作用域、Theme 与覆盖解析 Text / TextSpan。
- `WidgetryIcon`：通过 SVG 生成图像，可设置、替换或清除显式颜色与 SVG 输入。
- 有效 Disabled：组合祖先继承、本地请求和控件原生限制，并投影到官方输入状态。
- Pointer：把有效 Mouse、Custom、Touch 的 Hover 命中投影到官方 Hovered / DirectlyHovered；管理所标记控件的 Pressed Ownership 清理。
- Scene：提供同步及 Deferred 构造的错误传播、失败预约 Entity 清理。
- 提供动态 UI 同帧准备与共享 Z-index 标识等能力。
- `WidgetryInputTarget` 提供 Workspace 内组合控件根到实际输入实体的路由元数据；Window 的 Tab 与初始焦点解析使用同一目标，不承载控件的编辑或业务状态。

## 行为与状态契约

内部限制可通过 `set_intrinsic_disabled_for<R>` 按理由类型独立设置和撤销；已有 `set_intrinsic_disabled` 使用独立默认理由。所有内部理由按 OR 组合，撤销其中一项不会解除其他理由。

UI Hierarchy 的 Disabled 有效值采用 OR 继承，本地请求即使在继承期间修改也必须保留。官方 Hovered 包含子代命中，DirectlyHovered 限于直接命中；共享 Pointer 投影拥有相应写入权。Widgetry 内容颜色只影响使用标记的 Text，原生 Text 仍保留自身定义。Scene Props 在构造后不充当运行时状态。

Disabled 不通过 Native Window Owner 关系跨窗口传播。单独禁用的内容使用对应 Theme Fallback。`WidgetryPointerPressed` 只清理标记控件所拥有的真实 Pointer Press 会话，不接管预先存在的程序化 Pressed 或官方正常 Activation。默认字体可通过 `WidgetryFontPlugin` 或 `WidgetryAppExt::set_default_font` 启用，诊断状态与 Z-index 标识供自定义控件复用。

## 边界情况与错误处理

- 无效 Scene 展开、必要 UI 结构缺失和非法 Icon 更新通过 `BevyError` / 宿主 Handler 传递。
- 构造失败仅清理本次新建的预约空 Entity，不回滚已经提交的业务副作用。
- SVG 加载失败、等待或过时更新需要按 Icon 的当前请求身份判断，不应破坏此前有效内容。
- 有效 Disabled 在 Deferred 操作实际 Apply / Flush 前不代表已生效。

## 已知限制

- Font、Icon 和 Scene 各能力有各自的装配前提；Core 不是全局业务 Model。
- 官方 Hover State 的独立写入者需要与共享 Pointer Plugin 协调。
- Headless UI 证明的是 ECS / Layout 阶段行为，不等于原生窗口的完整显示验证。

