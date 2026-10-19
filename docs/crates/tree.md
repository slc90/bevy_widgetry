# Tree

## 功能概述

提供以 ECS Entity 表示业务 Node 的层级 Tree Model 与 TreeView，支持展开、Selection、Lazy Loading 与可见行虚拟化。

## 当前设计思路

`WidgetryTreeModel` 只保存原始 root。每个 `WidgetryTreeView` 的只读 `WidgetryTreeState` 保存独立的 Expanded 与可见元信息缓存；结合原始 Entity Hierarchy 生成本 View 的可见顺序。Selection 唯一 Authority 为其内部 `WidgetryListSelectionState<Entity>`，不在 Model 或 TreeState 重复存储。`WidgetryTreeView` 只投影可见 Node，并借助按 Component 类型注册的 Renderer 构造内容；业务 Node 不依赖其 Row Entity 的生命周期。内部列表使用 `WidgetryListView<WidgetryTreeVisibleItem, Entity>`，由 Tree 数据适配器直接提供 Entity 键和拥有的行展示数据，经 `WidgetryListProjection<Entity>` 驱动虚拟化，不再创建中间 `WidgetryListModel` 或另行分配 ListItemId。适配器以原始 root 下连续有效的 TreeNode 层级判断成员资格，隐藏后代仍属于数据源，脱离层级或移除标记后不再解析。

Tree 自身 ListView 与 Expander 的样式归属在构造时绑定，BSN 延后挂接、部件重建、层级变化及归属标记移除时，在 StyleOwners 阶段局部补偿。稳定 View 不再逐帧为已有部件发现归属。Expander 归最近的实际 TreeView 管理，不以共享 Model 作为颜色 Owner；嵌套 TreeView 独立维护部件，业务 Renderer 创建的普通 Button 不被接管。部件脱离或标记失效后清理派生归属，后续真实挂接 / 修复信号可重新绑定，失效节点不进入永久重试队列。

组合配色在 Theme 或 View Override 变化时重新解析该 View 的调色板；稳定帧不重复解析根配色或计算全部 Expander 颜色。Expander 的 Hover / Pressed / 有效 Disabled 及其移除只更新自身，新建或归属变化的部件使用当前 Owner 调色板。受管 ListView 沿用 Tree 提供的配色，并自行响应 Selection、Active、Focus 与行复用；共享原始树的逐 View 投影和 Renderer 内容变化继续正常执行，禁用 View 仍反映共享数据。配色更新不重建业务 Node，输出缺失的真实错误继续传播，修复后重新应用。

## 主要功能

- Root 下的 `WidgetryTreeNode` 表达业务层级和当前可见 Depth。
- 展开、收起、切换及设置 / 清空 Selection，通过 `WidgetryTreeEvent` 通知。
- 每 View 批量展开已加载结构与全部折叠；不触发远端加载，不清除合法隐藏选择。
- Expander Icon、缩进、行高和 Theme / Disabled 风格可配置。
- 行 Pointer 多选与 Keyboard Navigation，Ctrl 切换、Shift 按可见顺序选择范围，Ctrl+A 并入当前可见未禁用节点。隐藏锚点不能用于当前范围时，以当前目标建立新锚点。
- Unknown → Loading → Loaded 形式的 Lazy Children State，`ChildrenRequested` 通知调用方加载。
- 对异构业务 Component 注册 Renderer；按 Viewport 构造可见行。

## 行为与状态契约

同一个 Model 被多个 TreeView 使用时，Selection、展开、可见 Projection、Active、Focus、Scroll 与 Disabled 均按 View 独立；业务节点及父子结构、子节点顺序、加载状态共享。Selection Identity 是业务 Node Entity，不是行 Index；展开 / 收起影响可见性，保留隐藏后代的合法 Selection，不能销毁业务 Node。真实删除、移出原始 root 或移除有效 TreeNode 标记才静默清理失效选择。程序化 State 改变先提交再通知，同值不通知。外部 Hierarchy / Component 变化需要更新 Model 与可见投影。

## 边界情况与错误处理

Invalid Root / Node、缺失 Renderer、非法行高或缩进需要报告。Lazy Loading 由调用方响应 `ChildrenRequested`，Model 不替调用方执行文件系统 / 网络获取。Loading 期间重复展开不应重复发相同请求；数据删除后要修复失效 Selection。

`WidgetryTreeView::expand/collapse/toggle_expand/select(world, view, node)` 接受目标 TreeView Entity；`queue_expand/queue_collapse/queue_toggle_expand/queue_select` 在 Commands 执行时校验。`state(world, view)` 查询展开与可见节点，`selection(world, view)` 查询只读选择集合。程序化操作允许更新 Disabled View，用户输入受该 View 的有效 Disabled 约束。`WidgetryTreeEvent.entity` 是发生变化的具体 View；`SelectionChanged { selected, added, removed }` 表达已提交的集合变化，同值设置不通知。

懒加载元信息保存在共享节点上。多个 View 可独立展开同一 Unknown 节点，只有首次 `Unknown → Loading` 发出一次 `ChildrenRequested`；加载完成后只更新展开中的 View，折叠 View 保持折叠。

额外的 Source / Hierarchy 诊断响应构造、引用 Source 的 Model 插入 / 移除、内部 ListView / Expander 及相关层级变化，在正常输入前和 Reconcile / Renderer 构造完成后的边界检查受影响对象。稳定 View 不组织额外全量诊断扫描；每 View 的树投影仍执行业务所需的当前 Source 与内部列表前提校验。Source、树投影、View 层级与各 Expander 分别维护故障状态，同因持续异常不重复记录，本地结构错误不影响其他有效 View 的诊断；实际恢复后记录 INFO，再次故障重新报告，销毁不产生恢复信息。每次真实失败仍以 `Severity::Error` 交给宿主；归属补偿与层级检查共享 Expander 的同一故障状态。

调用方响应 Lazy Children 请求、添加业务 Children 后，通过 `set_loaded` 完成该请求；每个可见业务 Node 应匹配已注册的 Component Renderer。

## 已知限制

- Renderer Registry 当前按 Component 类型注册，不支持任意逐 View 相同类型 Renderer。
- Lazy Node 的数据获取与失败重试策略最终由宿主实现。


`replace_selection` / `queue_replace_selection` 以 Entity 集合更新该 View；`select_visible` / `queue_select_visible` 与 Ctrl+A 使用相同的 VisibleUnion 语义。全部展开使用 `expand_all` / `queue_expand_all`，遍历本地或明确 Loaded 的可展开结构，在 Unknown/Loading 节点停止，不新增加载请求，也不更改共享加载状态。`collapse_all` / `queue_collapse_all` 清除本 View 的全部 Expanded，保留 Selection。批量展开状态变化合并为一次 `ExpansionChanged { expanded, collapsed }` 通知，同值不通知；单项操作沿用 Expanded/Collapsed 事件。
