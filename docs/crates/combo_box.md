# Combo Box

## 功能概述

提供从 `WidgetryListModel<T>` 中选择单个选项的通用 ComboBox。Field 显示当前 Selection，Popup 提供列表浏览。

## 当前设计思路

ComboBox 在业务上组合 Button、ListView 和 ScrollArea。数据由 Model 持有，Selection 使用 Stable Item ID；Field 与 Popup 分别由 Renderer 构造内容。Popup 的可见性不应产生第二份选择 Authority。Field 与内部 ListView 的颜色归属在构造时绑定，BSN 延后挂接及后续层级 / 部件变化在 StyleOwners 阶段补偿；稳定实例不再逐帧遍历层级建立归属。移除归属标记会在首次颜色消费前修复，部件脱离或 Root 失效时清理对应派生关系。

## 主要功能

- `register_widgetry_combo_box::<T>` 注册具体数据类型能力。
- 支持文本、Icon 或任意 BSN Renderer 内容。
- 可配置 `item_height` 与 `max_visible_items`。
- Popup 支持 Pointer、Keyboard Navigation 与 Scroll；选择后关闭，Escape 或外部点击也可以关闭。
- Model Insert / Remove / Move / Edit 后，列表及 Field 跟随最新数据。
- 通过 Stable ID 设置或清空 Selection，使用 `ValueChange<Option<WidgetryListItemId>>` 通知变化。

## 行为与状态契约

非空 Model 首次初始化时默认选择第一项，空 Model 保持未选中。移动项目不改变其 Stable ID。真实 Selection 改变后先提交 Authority，再以 ComboBox 为 Source 通知；同值不重复通知。程序化设置不强行关闭现有 Popup。Disabled 拒绝用户输入，但保留公开程序化 Selection 能力。

## 边界情况与错误处理

Model Source、Renderer、行高等配置必须有效；失效 Source 或无效程序化请求需要诊断。Popup 关闭后隐藏列表不得继续消费键盘选择，关闭与同帧焦点切换时也要保留输入归属正确性。

## 已知限制

- 使用前需要为具体 `T` 注册能力，不能只安装非泛型 Plugin。
- Renderer 控制内容形态；ComboBox 不内建所有业务类型的选项展示。
- 当前 Popup 与 Field 的生命周期和 Focus 行为取决于相应 Widget 的组合契约。

