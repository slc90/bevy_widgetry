# List View

## 功能概述

提供 `WidgetryListModel<T>` 与泛型 `WidgetryListView<T>`，用于单选、键盘浏览和可虚拟化的纵向条目列表。

## 当前设计思路

Model 持有业务数据和稳定身份，View 持有自己的 Selection、Active、Focus、Scroll 等交互状态。物理 Row 仅表示当前 Viewport 的投影，滚动可回收和复用对应 Entity。逐 View Renderer 使用 BSN 组合条目内容；共享同一 Model 的 View 可使用不同 Renderer，不共享交互状态。

## 主要功能

- Model 支持 Insert、Remove、Move、Edit 和 Clear，并维护 Stable Item ID、Revision 及 Disabled Metadata。
- View 根据 Viewport 实例化可见行，离屏条目不长期保留完整 UI Tree。
- Pointer 选择、Keyboard 方向键 / Home / End、PageUp / PageDown、Space / Enter 和 Reveal 行为。
- 公开 API 可设置或清除 Selection、设置 Active；Selection 改变时通过 `ValueChange` 发送 Stable ID 或 None。
- 支持 Theme、Item Disabled、Hover、Pressed、Focus 与颜色覆盖。

## 行为与状态契约

在 App 中注册对应数据类型，并提供匹配的 Model Source、Renderer 与有限正数的 `item_height`。Selection 与 Active 分离：程序选择可以改变被选项目，导航位置独立维护。Model Move 不改变 Stable ID；删除后的无效 Selection / Active 按契约修复。多个 View 可共享 Model，但各自 Selection、Active、滚动、Disabled 和用户通知互不绑定。自有 Selection 先提交再通知，同值不通知，自动 Repair 默认不通知。

配色按变化更新：Theme、覆盖值、根 Disabled 或 Focus 等变化刷新对应 View，行内 Hover / Pressed / Selected / Disabled 变化定位到相应行，稳定帧不重复解析全部可见行颜色。Focus 转移刷新旧、新目标；行换 ID、Renderer 内容替换及滚动新建行均在内容准备前获得当前样式。ComboBox、Tree 等组合 Owner 提供的调色板继续负责嵌入 ListView，局部行交互沿用该调色板。

## 边界情况与错误处理

失效 Source、类型不匹配、Renderer 缺失或无效高度属于构造 / 运行时错误；排队程序化操作在执行时验证 Root、Source 和 ID。Disabled 限制用户输入，但有效程序化操作仍可执行。行复用后旧 Press 与旧业务 ID 不得作用于新条目。

## 已知限制

- 当前是单项 Selection，而非内建多选。
- Renderer 不由 Model 自动提供，消费者必须注册并传入合适的构造逻辑。
- Viewport 尚未完成布局时，可见投影及 Reveal 需要遵循实际布局生命周期。

