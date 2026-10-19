# Table

## 功能概述

提供具有异构 Cell 值、二维虚拟化和 Row / Column / Cell Selection 的 Table 组件。适用于可滚动、可调整列宽的数据表格。

## 当前设计思路

Model 保存 Generic Row 与 Column 定义，按 Column Projection 提供 Cell Value。按值类型注册的 Cell / Header Renderer 负责内容；Table View 分别管理行投影、Layout、Selection、Focused Cell、Scroll 和交互状态。可见 Cell 只是 Row / Column ID 组合的投影。

区域配色缓存于各 View，Theme / Override 变化重新解析调色板；稳定帧的配色路径不再克隆 Runtime、Style 或遍历可见区域计算颜色。Hover / 局部 Disabled 只失效实际部件，Selection 和 Focus 更新旧、新受影响区域；根 Disabled 更新该 View 的区域。新 Shell、内容版本替换及输出移除触发对应部件刷新，Scroll / 列宽坐标投影不自动使所有颜色失效。Style 的 padding / border 只在配置变化与部件初始化时更新，保持在 Foreground 传播和 UI 准备之前完成。每个 View 独立持有派生缓存，不改变共享 Model、Renderer 或二维虚拟化规则。

## 主要功能

- Model 支持 Row / Column Insert、Remove、Move、Edit，保持 Stable ID 和 Revision。
- 分别显示 Body、Column Header、Row Header 与 Corner，Headers 按各自 Scroll Axis 对齐。
- Cell / Header 可拥有不同值类型和 BSN Renderer。
- Layout 支持 Row Height、Header 尺寸、Fixed / Flexible Column Width 与最小列宽。
- Pointer 选择 Row、Column 或 Cell，Keyboard 在 Cell 间移动 Focus 并 Reveal。
- 公开 API 可调整 Selection、Focused Cell 与列宽。
- Column Header Resize Handle 提供 Start、Resized、End 或 Cancel 通知。
- 按区域配置 Theme 与颜色覆盖，支持 Disabled 状态。

## 行为与状态契约

Row / Column Stable ID 仅在所属 Model 内有效；移动保持身份，删除后不复用。Selection 与 Focused Cell 分别代表已选目标和导航位置，程序设置先提交再通知，同值不重复通知；Model Repair 默认静默。两个 View 可以共享一个 Model，但拥有各自行投影、Layout、列宽、Selection、Focus、Scroll 和 Disabled；Renderer Registry 按 Cell / Header 值类型在 App 级注册。

## 边界情况与错误处理

缺失 Renderer、无效 Model / Source、非法 Scene 配置或失效 ID 必须明确反馈。Resize 遇到 Column / Header 失效、Cancel 或 Disabled 时结束当前 Gesture，不能对另一个身份继续写宽度。滚动中的可见 Cell 复用不改变业务 ID。

区域样式所需输出缺失时继续以 Error 交给宿主，保留失效区域重试；持续同因只记录一次中文 ERROR，实际修复后记录 INFO，销毁 View 不伪造恢复。

## 已知限制

- 目前同一个 Model 的多个 View 共享 Column Schema 与列取值函数，行显示投影可独立；不支持为同一 Model 的不同 View 分别定义 Column Schema。
- 相同 Cell 值类型的 Renderer 当前在 App 级注册，不提供任意逐 View Renderer Registry。
- 二维虚拟化仅构造可见范围，不表示任意规模的 Model 操作都具有常数成本。


## 每个 View 的行投影

`WidgetryTableViewState` 是根实体上只读的行显示 Authority，`ordered_rows()`、`row_id(display_index)`、`display_index(row)` 提供完整显示序列及反查。默认跟随 Model 的全部行；各 View 可独立使用应用提交的子集和顺序，不复制业务行，也不改变 Model 原始顺序。

`WidgetryTable::update_view(world, root, WidgetryTableViewUpdate { rows, sort, filters })` 一次校验并提交完整有序 RowId、可选的单列 `WidgetryTableSort` 与活跃过滤列集合。方向为 Ascending/Descending，`None` 表示无排序指标；多列过滤标记可同时存在。重复/过期 RowId 或 ColumnId 明确返回 Error，不部分覆盖旧状态。合法同值返回 false；空 rows 是有效的空投影。`queue_update_view` 在 Commands 执行时校验。`reset_view` / `queue_reset_view` 清除受控结果与指标，恢复默认 Model 次序。

Table 不进行业务比较或过滤。应用负责在业务数据变化后重算并重新提交；受控 View 在等待期间保留原顺序和子集，不自动接纳新增行。真实删除的行 ID 与列指标会静默清理，避免使用过期身份；Row 内容更新仍按稳定 ID 和真实 Model 索引更新。默认 View 继续反映 Model 的插入、删除和移动。内部结构版本补充 ECS Tick，确保同一 World tick 内连续结构编辑和同步 API 仍能取得当前顺序。

垂直布局、行号、键盘上下导航、Focus Reveal 与滚动全部按显示索引工作；Cell Value/Revision 按 RowId 反查 Model 索引。Cell 仍以 `(RowId, ColumnId)` 复用，重排不串用内容。过滤隐藏的单项 Selection / Focus 会静默清理；程序化设置 Row/Cell 或 Focus 时，该行必须在当前 View 投影中。

应用可把投影重算 system 排在 `PostUpdate` 的 `WidgetryTableSystems::Projection` 前，保证新的行序列在同次 UI 构造中消费。当前不包含 Table 行拖拽、列隐藏或自动业务排序/筛选。

## 表头排序与筛选请求

`WidgetryTableProps::header_actions` / 根实体展示配置 `WidgetryTableHeaderActions` 的 `sortable`、`filterable` 集合独立启用对应 ColumnId 的按钮。标题主体仍选择整列，最右侧 6 px Handle 调整列宽；请求按钮不触发这两项行为。按钮身份可通过只读 `WidgetryTableHeaderAction` 查询。

`WidgetryTableEventKind::SortRequested { column, direction }` 请求当前已确认状态的下一步：无 → Ascending → Descending → 无；换列从 Ascending 开始。`FilterRequested(column)` 仅请求打开/更改该列业务过滤。请求不改变 Model、行投影、Selection 或排序/过滤指标；应用可不处理。只有成功的 `update_view` 才确认显示结果，排序指标单列排他，过滤指标可多列共存。

按钮指示从 ViewState 派生，虚拟化或 Header Renderer 替换后重建仍使用稳定 ColumnId；Disabled、已删除列、已关闭的入口及已销毁按钮不会发出请求。无效应用回填仍遵循原子拒绝契约，不部分改变旧指示与行序列。

表头指标使用内建 SVG：无排序显示双向 Chevron，升降序分别单向 Chevron，筛选区分空心/实心漏斗，不依赖宿主字体是否包含 Unicode 图标。
