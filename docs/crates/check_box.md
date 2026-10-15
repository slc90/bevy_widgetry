# Check Box

## 功能概述

提供二态 `WidgetryCheckBox` 和三态 `WidgetryTriStateCheckbox`，用于二值选择与部分选中等场景。两者都可接收调用方的 BSN Children。

## 当前设计思路

二态 Checkbox 复用 Bevy 官方 `Checked` State。三态 Checkbox 由 Widgetry 自有 `WidgetryCheckState` 管理状态，并提供专用的 Setter、Cycle 操作及状态通知。两套状态来源不强行统一。

## 主要功能

- 二态 Checked / Unchecked 显示与输入。
- 三态 Unchecked、Checked、Indeterminate 显示与指示 Icon。
- 三态支持 Pointer Activation，以及 Focus 后 Space / Enter 切换。
- `set_state` 可指定三态值；`cycle_state` 按 Unchecked → Checked → Indeterminate → Unchecked 顺序切换。
- 状态对应的 Accessibility、Theme 与内容配色同步。

## 行为与状态契约

三态默认 Unchecked；其真实状态改变时先提交再发送 `ValueChange`，同值不通知。`Commands` 操作在真正执行时才更新。Disabled 阻止用户操作，但不阻止合约允许的程序化设置。二态仍遵循官方 `Checked` 的使用方式。

## 边界情况与错误处理

无效的三态 Root 或内部必要指示结构缺失属于错误，不应无声忽略。Theme 切换及 Indicator 加载不应改变逻辑选中状态；状态与视觉投影需要保持一致。

## 已知限制

- 三态的程序化 API 与通知不用于替代 Bevy 官方二态 Component。
- SVG Indicator 依赖 Asset 和 Icon 加载就绪；构造与内容绘制存在正常 Asset 生命周期。

