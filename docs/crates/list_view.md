# List View

## 功能概述

提供 `WidgetryListModel<T>` 与泛型 `WidgetryListView<T, K = WidgetryListItemId>`，用于多选、键盘浏览和可虚拟化的纵向条目列表。

## 当前设计思路

Model 持有业务数据和稳定身份，View 持有自己的 Selection、Active、Focus、Scroll 等交互状态。物理 Row 仅表示当前 Viewport 的投影，滚动可回收和复用对应 Entity。每个 View 持有 `WidgetryListProjection<K>`，按独立 ID 顺序渲染原始数据的子集或重排结果，不复制业务数据。逐 View Renderer 使用 BSN 组合条目内容，回调统一接收 `&T`，不传入位置索引；共享同一 Model 的 View 可使用不同 Renderer，不共享交互状态。

## 主要功能

- Model 支持 Insert、Remove、Move、Edit 和 Clear，并维护 Stable Item ID、Revision 及 Disabled Metadata。
- View 根据 Viewport 实例化可见行，离屏条目不长期保留完整 UI Tree。
- Pointer 选择、Keyboard 方向键 / Home / End、PageUp / PageDown、Space / Enter 和 Reveal 行为。
- 公开 API 可设置或清除 Selection、设置 Active；Selection 改变时通过 `WidgetryListSelectionChanged<K>` 发送当前集合及 added / removed。
- 支持 Theme、Item Disabled、Hover、Pressed、Focus 与颜色覆盖。

## 行为与状态契约

在 App 中注册对应数据类型，并提供匹配的 Model Source、Renderer 与有限正数的 `item_height`。Selection 与 Active 分离：程序选择可以改变被选项目，导航位置独立维护。Model Move 不改变 Stable ID；删除后的无效 Selection / Active 按契约修复。多个 View 可共享 Model，但各自 Selection、Active、滚动、Disabled 和用户通知互不绑定。自有 Selection 先提交再通知，同值不通知，自动 Repair 默认不通知。

配色按变化更新：Theme、覆盖值、根 Disabled 或 Focus 等变化刷新对应 View，行内 Hover / Pressed / Selected / Disabled 变化定位到相应行，稳定帧不重复解析全部可见行颜色。Focus 转移刷新旧、新目标；行换 ID、Renderer 内容替换及滚动新建行均在内容准备前获得当前样式。ComboBox、Tree 等组合 Owner 提供的调色板继续负责嵌入 ListView，局部行交互沿用该调色板。

## 边界情况与错误处理

失效 Source、类型不匹配、Renderer 缺失或无效高度属于构造 / 运行时错误；排队程序化操作在执行时验证 Root、Source 和 ID。Disabled 限制用户输入，但有效程序化操作仍可执行。行复用后旧 Press 与旧业务 ID 不得作用于新条目。

## 已知限制

- Renderer 不由 Model 自动提供，消费者必须注册并传入合适的构造逻辑。
- Viewport 尚未完成布局时，可见投影及 Reveal 需要遵循实际布局生命周期。


## 数据源与投影

普通数据通过 `register_widgetry_list_view::<T>()` 注册，按稳定 ID 读取 Model 的借用值；ID→原始索引映射随结构变更维护。其他身份类型通过 `register_widgetry_list_source::<T, K, D>()` 注册 `WidgetryListDataSource`，可返回借用或拥有的行值、Disabled 和可选 Revision。适配器的 `changed_at` 返回反映所有相关数据变化的 ECS Tick；无 Revision 的行在数据源变化时保守重建可见内容。

`projection` 构造配置或 `set_projection_in_world` / `set_projection` 设置每个 View 的过滤与排列函数。配置函数只处理 ID；未知及重复 ID 被剔除。数据源或配置变化后自动重建投影，先修复导航再虚拟化。`WidgetryListViewItem.index` 是显示索引，`id` 是原始身份；编辑数据必须按 ID 反查原始索引。`select_index` 使用原始 Model 索引，`select_display_index` 使用本 View 显示索引，`select_id` 使用稳定身份；Active 的 `set_active_display_index` 使用显示顺序。过滤隐藏不清空仍存在的选择，真正删除才静默修复。

## Selection 子系统

`WidgetryListSelectionState<K>` 是唯一选择 Authority，持有稳定 ID 集合与范围锚点，通过 `selected_ids()`、`contains()`、`selected_id()` 和 `anchor()` 只读查询。初始集合为空，`selected_id()` 仅在恰好选中一项时返回身份。`WidgetryListViewState<K>` 只暴露独立的 Active，内部导航缓存不保存另一份 Selection。

普通 View 默认 `WidgetryListSelectionMode::Multiple`：普通点击或 Enter/Space 替换为单项，Ctrl 切换成员并设置稳定身份锚点，Shift 按本 View 投影顺序替换为锚点到目标的连续范围，跨离屏项并跳过禁用项。锚点被过滤隐藏时回退到本次目标；真正删除时静默清除无效锚点。Ctrl+A 一次提交整个原始数据源中未禁用的身份，包括被过滤与离屏项。Root、Viewport、Content 禁用时阻止此快捷键。`Single` 模式忽略多选修饰符并禁用全选，ComboBox 与当前 Tree 包装显式使用此模式。

`WidgetryListView::selection(world, root)` 读取根实体的选择。`select_id`、`select_index`、`select_display_index`、`replace_selection`、`add_id`、`deselect_id`、`clear_selection` 各有 Commands 与 `_in_world` 入口。集合替换先验证所有身份，再原子提交；Single 模式拒绝多项集合。程序操作不受 Disabled 限制。同步入口返回 `Result<bool, BevyError>`，同集合无变更通知；按索引排队时在执行时解析身份。真实选择提交后才发集合事件，事件的 `source` 属于具体 View，`selected` 共享不可变快照，`added` / `removed` 描述本次集合增减，集合与差量不承诺迭代顺序。初始化及自动删除修复静默。

组合控件复用同一 Selection，内部静默入口只用于初始化及既有 Authority 投影；ComboBox 向外继续发送单值 `ValueChange<Option<WidgetryListItemId>>`。共享 Source 不共享选择、Active、焦点或通知。行 Hover/Pressed 配色与可见 Row 选择投影只查询集合成员，不复制整个集合。

## 拖拽排序

普通 Model 注册支持按 View 设置 `reorderable: true`，默认关闭。拖动始终只移动起始稳定 ID，不改变多选集合。当前投影行的上半部表示 Before，下半部表示 After，以行边框显示放置反馈；释放时重新验证 Source、ID、禁用和 Row 身份，再修改原始 Model 顺序。过滤隐藏项保持相对顺序，其他 View 自动重建自己的投影。已有显式排序规则仍优先决定最终显示顺序。

取消、非法目标、自身或相邻同位置放置不修改 Model；禁用或虚拟行换身份会取消过期拖动，并抑制本次释放时的 Click，保持选择集合。放置位置按命中 Camera 的 DPI 和 viewport 计算，与额外 UiScale 独立。仅在当前 View 内拖放，不支持跨 View 转移；不提供拖拽自动滚动。只读自定义数据源不能开启排序，构造时返回错误。
