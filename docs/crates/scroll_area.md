# Scroll Area

插件保留 Pointer 焦点与当前控件内部键盘操作，不自动安装 `TabNavigationPlugin`。

## 功能概述

提供用于展示超出可见范围内容的通用 ScrollArea，支持横向、纵向及两轴滚动。

## 当前设计思路

ScrollArea 以 BSN `content` / `children` 组合调用方 UI，以独立的 Viewport 和 Scrollbar 表达可见范围及交互。垂直模式的内容容器宽度限制为 Viewport 宽度，支持文本在视口内换行；横向和双轴模式保留固有内容宽度。`content` 可修饰布局方向与 Padding，容器的尺寸与滚动几何由 ScrollArea 管理。Scrollbar 可见策略与真正允许滚动的 Axis 分离。

## 主要功能

- 横向、纵向或双轴 Scroll Axis。
- 各轴独立的 Auto、Always、Hidden Scrollbar Policy。
- Wheel、Scrollbar Thumb Drag、Keyboard Navigation 和程序化 ScrollPosition。
- 垂直 PageUp / PageDown / Home / End。
- `WidgetryScrollIntoView` 将目标调整到最近 ScrollArea 的可见区域。
- Thumb 状态与 Theme / 颜色覆盖同步。

## 行为与状态契约

默认垂直滚动、Auto Scrollbar，并启用 Keyboard Scrolling。Hidden 只隐藏 Bar，不禁止该轴滚动；关闭 Keyboard Scrolling 不关闭 Wheel / API。`ScrollPosition` 使用 Bevy 官方 Component 的更新方式。`IntoView` 目标已完整可见时不改变 Scroll；有效滚动操作需要限制在合法 Viewport 范围。嵌套 ScrollArea 优先处理最近 Viewport。

独立 ScrollArea 在首次构造、部件变化、有效 Disabled、Hover、拖动状态、Theme 或颜色覆盖变化时刷新滚动条配色。Thumb 使用 Disabled → Dragged → Hovered → Normal 的优先级；仅拖动状态组件被写入但 `dragging` 未变时，不重复解析颜色。稳定区域不重新遍历全部 Bar / Thumb 配色。禁用时暂存的 Scrollbar 仍参与颜色更新，程序化滚动与必要几何同步继续有效。由组合控件接管的 ScrollArea 使用所属控件提供的颜色；移除归属后恢复独立配色。

## 边界情况与错误处理

内容未溢出、Viewport 改变或 Scrollbar Gutter 出现时，需要按布局结果恢复合理的 Scroll Offset。Disabled 可以阻止用户输入，不能阻止合法程序化 Scroll 或必要 Thumb Geometry 更新。无效 Pointer Drag 生命周期必须清理。

`ScrollAreaError` / `ScrollAreaFailure` 区分非法颜色目标、托管颜色拒绝、非有限或负的滚动条宽度、必需部件缺失、已配置滚动条的目标 / 方向不匹配及 Core 原因。部件诊断保存 Root、Viewport、Content、相关轴与显隐策略、已知尺寸和具体 Entity / 部件；初次几何配置保存归属，后续损坏仍可定位原部件。同步颜色 API 返回领域失败，Scene / Command / System 边界显式转成 `Severity::Error`。

首次没有 ComputedNode 时等待布局；空内容、隐藏或零尺寸不报错。已测量后丢失必需 ComputedNode、已拥有 Viewport 丢失 ScrollPosition、Bar / Thumb 损坏属于真实失败。Core 在有效禁用时移除原生 ScrollArea、Scrollbar 在禁用时暂存，均不属于部件损坏。背景移除仍按原契约重建。

几何与样式沿用现有工作集合，各 Root 按业务键保留持续异常的 ID / origin 并在恢复时记录 INFO；每次批量更新聚合所有失败 Root。几何恢复会重新安排该 Root 的样式更新，补做异常期间跳过的 Root / Thumb 输入。样式消费和几何检查共享同一结构失败身份，组合控件只增加所属部件上下文。Keyboard / IntoView 先识别所属区域，在必需状态缺失时通过队列校验传播原诊断；无所属区域、正常布局等待和输入 Guard 保持静默。未处理的失败交给宿主，传播不重复创建源头 ERROR 或重捕获回溯。

## 已知限制

- Scrollbar 是否可见不表示滚动 Axis 是否启用。
- 实际 Layout 几何需要正常 Bevy UI 准备流程；仅构造 Scene 不代表已测得滚动范围。
- `ScrollIntoView` 只处理目标所在最近的 ScrollArea。

