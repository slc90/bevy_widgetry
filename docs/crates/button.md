# Button

## 功能概述

提供可组合调用方自定义 BSN Children 的 Button，适合触发操作或作为复合 Widget 的交互入口。

## 当前设计思路

采用 Bevy Button 的 Activation 机制，Widgetry 负责外观与内容配色。内容、尺寸与布局由调用方提供，不引入专用的文字或 Icon 构造接口。

## 主要功能

- 通过 `WidgetryButton` Scene 创建控件并组合 Text、Icon 或其他 Children。
- 通过官方 `Activate` Observer 响应用户触发。
- 配色随 Hover、Pressed、Disabled、Theme 与颜色覆盖变化。
- Foreground 颜色可传递到 Widgetry 管理的内容。

## 行为与状态契约

状态优先级为 Disabled → Pressed → Hover → Normal。`InteractionDisabled` 拒绝用户 Activation，移除禁用后可以继续响应交互。外观变化不应重置调用方提供的 `Node` Layout Patch 或 Children。

## 边界情况与错误处理

Theme 或颜色覆盖变更后，按控件当前状态解析外观。普通 Press/Release 及被禁用的输入不应凭空产生业务操作；构造时由 BSN 和对应 Plugin 提供必要基础能力。

## 已知限制

- 不规定 Button 的业务动作；由应用的 Observer 实现。
- 不自动提供固定标签、Icon 或字体内容。
- Bevy 官方 Button 激活契约仍然适用。

