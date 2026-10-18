# Combo Box

## 功能概述

提供从 `WidgetryListModel<T>` 中选择单个选项的通用 ComboBox。Field 显示当前 Selection，Popup 提供列表浏览。

## 当前设计思路

ComboBox 在业务上组合 Button、ListView 和 ScrollArea。数据由 Model 持有，Selection 使用 Stable Item ID；Field 与 Popup 分别由 Renderer 构造内容。Popup 的可见性不应产生第二份选择 Authority。Field 与内部 ListView 的颜色归属在构造时绑定，BSN 延后挂接及后续层级 / 部件变化在 StyleOwners 阶段补偿；稳定实例不再逐帧遍历层级建立归属。移除归属标记会在首次颜色消费前修复，部件脱离或 Root 失效时清理对应派生关系。

组合配色响应 Field Hover / Pressed、Popup 可见性、有效 Disabled、颜色 Override / Theme 和部件生命周期变化；稳定实例不重复解析或应用整套颜色。根缓存真正样式 Owner 提供的 palette，托管 ComboBox 的局部交互沿用该 palette，解除托管后恢复自己的 Theme / Override。内部 ListView 使用组合 Owner 的列表配色，独立处理 Selection、Active、Focus 与可见行输入及滚动复用。Field `(id, index, revision)` 内容缓存保持独立，颜色变化不重建 Renderer，新的 Text / Icon 在当帧前景传播中获得正确颜色。

## 主要功能

- `register_widgetry_combo_box::<T>` 注册具体数据类型能力。
- 支持文本、Icon 或任意 BSN Renderer 内容。
- 可配置 `item_height`、`max_visible_items` 及 Placeholder Scene 工厂 `WidgetryComboBoxContent`。
- Popup 支持 Pointer、Keyboard Navigation 与 Scroll；选择后关闭，Escape 或外部点击也可以关闭。
- Model Insert / Remove / Move / Edit 后，列表及 Field 跟随最新数据。
- `select_id` 按 Stable ID 选择，`select_index` 在 Commands 执行时解析原始 Model 索引；`clear_selection` 清空。`selected_id(world, combo)` 从本体只读查询唯一 Selection，无需应用查找内部列表。
- 使用 `ValueChange<Option<WidgetryListItemId>>` 通知变化。

## 行为与状态契约

构造后无论 Model 是否非空均保持 `None`，Field 显示可配置 Placeholder；默认文案为 `Select…`，应用可用 Scene 工厂提供本地化文字或其他内容。清空、删除选中项后恢复 Placeholder，不自动选择其他项目；新增数据也不触发首选。Field 直接按原始 ID 读取 Model，投影过滤隐藏不影响有效选择。移动项目不改变其 Stable ID。真实 Selection 改变后先提交内部 ListView 的单选 Authority，再以 ComboBox 为 Source 通知；同值不重复通知，自动删除修复静默。程序化设置不强行关闭现有 Popup。Disabled 拒绝用户输入，但保留公开程序化 Selection 能力。

## 边界情况与错误处理

Model Source、Renderer、行高等配置必须有效；失效 Source 或无效程序化请求需要诊断。Popup 关闭后隐藏列表不得继续消费键盘选择，关闭与同帧焦点切换时也要保留输入归属正确性。

## 已知限制

- 使用前需要为具体 `T` 注册能力，不能只安装非泛型 Plugin。
- Renderer 控制内容形态；ComboBox 不内建所有业务类型的选项展示。
- 当前 Popup 与 Field 的生命周期和 Focus 行为取决于相应 Widget 的组合契约。

