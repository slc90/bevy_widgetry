# Scroll Area

## 功能概述

提供用于展示超出可见范围内容的通用 ScrollArea，支持横向、纵向及两轴滚动。

## 当前设计思路

ScrollArea 以 BSN `content` / `children` 组合调用方 UI，以独立的 Viewport 和 Scrollbar 表达可见范围及交互。Scrollbar 可见策略与真正允许滚动的 Axis 分离。

## 主要功能

- 横向、纵向或双轴 Scroll Axis。
- 各轴独立的 Auto、Always、Hidden Scrollbar Policy。
- Wheel、Scrollbar Thumb Drag、Keyboard Navigation 和程序化 ScrollPosition。
- 垂直 PageUp / PageDown / Home / End。
- `WidgetryScrollIntoView` 将目标调整到最近 ScrollArea 的可见区域。
- Thumb 状态与 Theme / 颜色覆盖同步。

## 行为与状态契约

默认垂直滚动、Auto Scrollbar，并启用 Keyboard Scrolling。Hidden 只隐藏 Bar，不禁止该轴滚动；关闭 Keyboard Scrolling 不关闭 Wheel / API。`ScrollPosition` 使用 Bevy 官方 Component 的更新方式。`IntoView` 目标已完整可见时不改变 Scroll；有效滚动操作需要限制在合法 Viewport 范围。嵌套 ScrollArea 优先处理最近 Viewport。

## 边界情况与错误处理

内容未溢出、Viewport 改变或 Scrollbar Gutter 出现时，需要按布局结果恢复合理的 Scroll Offset。Disabled 可以阻止用户输入，不能阻止合法程序化 Scroll 或必要 Thumb Geometry 更新。无效 Pointer Drag 生命周期必须清理。

## 已知限制

- Scrollbar 是否可见不表示滚动 Axis 是否启用。
- 实际 Layout 几何需要正常 Bevy UI 准备流程；仅构造 Scene 不代表已测得滚动范围。
- `ScrollIntoView` 只处理目标所在最近的 ScrollArea。

