# Table

## 功能概述

提供具有异构 Cell 值、二维虚拟化和连续 Row / Column / Cell Selection 的 Table 组件。适用于可滚动、可调整列宽的数据表格。

## 当前设计思路

Model 保存 Generic Row 与 Column 定义，按 Column Projection 提供 Cell Value。按值类型注册的 Cell / Header Renderer 负责内容；Table View 分别管理行投影、Layout、Selection、Focused Cell、Scroll 和交互状态。可见 Cell 只是 Row / Column ID 组合的投影。

区域配色缓存于各 View，Theme / Override 变化重新解析调色板；稳定帧的配色路径不再克隆 Runtime、Style 或遍历可见区域计算颜色。Hover / 局部 Disabled 只失效实际部件，Selection 和 Focus 更新旧、新受影响区域；根 Disabled 更新该 View 的区域。新 Shell、内容版本替换及输出移除触发对应部件刷新，Scroll / 列宽坐标投影不自动使所有颜色失效。Style 的 padding / border 只在配置变化与部件初始化时更新，保持在 Foreground 传播和 UI 准备之前完成。每个 View 独立持有派生缓存，不改变共享 Model、Renderer 或二维虚拟化规则。

## 主要功能

- Model 支持 Row / Column Insert、Remove、Move、Edit，保持 Stable ID 和 Revision。
- 分别显示 Body、Column Header、Row Header 与 Corner，Headers 按各自 Scroll Axis 对齐。
- Cell / Header 可拥有不同值类型和 BSN Renderer。
- Layout 支持 Row Height、Header 尺寸、Fixed / Flexible Column Width 与最小列宽。
- Layout 的 `frozen_columns` 按每个 View 的前 N 个显示列冻结，冻结与滚动两区独立裁剪并同步纵向滚动。
- 可选列标题拖拽保存当前 View 的独立显示顺序，默认关闭；显式重置恢复当前 Model 次序。
- Pointer 选择连续 Row、Column 或 Cell 范围，Cell 拖选、同类型 Shift 扩展，以及当前 View 的 Ctrl+A / Corner 全选；Keyboard 在 Cell 间移动 Focus 并 Reveal。
- 公开 API 可调整 Selection、Focused Cell 与列宽。
- Column Header Resize Handle 提供 Start、Resized、End 或 Cancel 通知。
- 按区域配置 Theme 与颜色覆盖，支持 Disabled 状态。

## 行为与状态契约

Row / Column Stable ID 仅在所属 Model 内有效；移动保持身份，删除后不复用。Selection 与 Focused Cell 分别代表已选目标和导航位置，程序设置先提交再通知，同值不重复通知；Model Repair 默认静默。两个 View 可以共享一个 Model，但拥有各自行投影、Layout、列宽、Selection、Focus、Scroll 和 Disabled；Renderer Registry 按 Cell / Header 值类型在 App 级注册。

## 边界情况与错误处理

缺失 Renderer、无效 Model / Source、非法 Scene 配置或失效 ID 必须明确反馈。Resize 遇到 Column / Header 失效、Cancel 或 Disabled 时结束当前 Gesture，不能对另一个身份继续写宽度。滚动中的可见 Cell 复用不改变业务 ID。

运行结构、Renderer 注册/查找与配色 API 使用 `TableFailure` / `TableRuntimeFault`，区分失效部件、归属变化、缺失 Renderer/Registry、值类型不匹配与 generation 耗尽。Runtime 补充 View / Source、稳定 RowId / ColumnId、冻结或普通区域、输出部件及 Renderer 类型/版本。持续结构与样式故障按业务槽位复用原始诊断，实际重新验证成功后记录关联 INFO；布局等待、虚拟化销毁和 View 销毁不视为恢复。

同帧多个 View、Header、Cell 和样式部件的实际失败分别交给宿主，健康区域继续更新。每次实际 Scene 展开失败独立保留原始原因、Severity 与 payload；下层已受管错误保留首次 ID 与 Backtrace。失败仅清理该次新建的内容子树，已有业务实体脱离失败子树并保留已提交状态，不回滚 Model 或已写入的业务 Component。失败仍结束当前输入/编辑会话并静默清理选择，保持既有中断契约。

## 强类型原位编辑

编辑默认关闭。`WidgetryTableProps::editing` 或根实体的 `WidgetryTableEditing::new(predicate)` 开启编辑，predicate 按当前 World、Model Source 和稳定 Cell 身份判断业务资格。还须存在匹配值类型的 Editor。只读并非 Disabled：仍能聚焦、选择；编辑开启时轻微强调背景，并使用 ReadOnly Cursor Hint。只读单元格移除自身的默认光标声明，使 Kamuri 带锁图像或 System Cell 回退生效；退出只读时恢复默认声明。Disabled 仍优先显示禁用光标。

`WidgetryTableCellEditor<V>::new(mode, scene, read)` 用强类型初值生成 BSN SceneList，读取函数从编辑器根实体返回 `Result<V, WidgetryTableEditorReadError>`。`Rejected(String)` 表示正常解析 / 业务拒绝，保留草稿并显示反馈，不创建 ERROR；`Failed(TableEditFailure)` 表示基础设施失败，交给调用方或宿主，不伪装为验证反馈。`diagnostic(failure)` 可直接承接 TextField 等下层领域诊断，保留源头、ID 和原始回溯；`external(error)` 保留标准外部错误的具体类型与原因链。Table 只补充 View / Source / Cell / Editor / Pending / Request ID / 编辑阶段上下文，不重复记录源头。Renderer 与 Editor 独立。`WidgetryTableEditorAppExt` 注册类型默认 Editor 或绑定 `(Source Entity, ColumnId)` 的列覆盖；列覆盖优先，类型不匹配视为只读。运行时可通过 `WidgetryTableEditorRegistry` 注册或回收 Source 的覆盖项。

Enter / 双击进入编辑，单行 Enter、多行 Ctrl+Enter 提交。Esc 或编辑器外部的 Pointer Press 取消草稿，原输入继续传给目标；内部含下拉 Children 的输入不取消。脱离 Children 的弹出层可挂 `WidgetryTableEditorPopup { editor }` 标注归属。Editor 内部拥有导航与文本快捷键。

`begin_edit` / `submit_edit` / `cancel_edit` / `resolve_edit` 返回 `TableEditFailure`，对应 `queue_*` 显式转换后交给宿主。自身失败通过 `TableError::Edit` / `TableEditFault` 分类缺失部件、失效 Cell、未物化 Cell、Model 实例变更、初值类型、请求计数耗尽和回执不匹配原因；Editor Scene 失败保留原生 Severity 与 payload。多 View 同帧提交的实际失败全部传播，不因首个读取失败跳过其他 View。`WidgetryTableEditState` 只读查询 Cell、Editor Entity、Pending 和错误。编辑期间正常 Renderer 被替换，并保留至多一个额外 Cell Shell，使二维滚动或 Model Revision 改变时保留草稿及稳定身份；其他 Cell 仍按可见范围虚拟化。

原生键盘提交和 Editor 的可见提交按钮应触发 `WidgetryTableSubmitEdit { entity: view }` 意图。Table 在本帧 `EditableTextSystems` 应用字符、删除、粘贴及长度限制后读取草稿，避免同帧 Enter 丢失最后一次修改；处理期间不允许外部点击撤销这次提交意图。程序化 `submit_edit` / `queue_submit_edit` 立即读取调用时已经生效的编辑状态。Esc 从原始 FocusedInput 目标处理，因此内部下拉先消费 Escape 也会取消未提交草稿。Cell 或 Body 的有效 Disabled 与根 Disabled 同样屏蔽提交并取消草稿。

提交先标记 Pending，再发出 `WidgetryTableEditRequested { entity, cell, value, request_id }`；`value.downcast_ref::<V>()` 取得强类型值。Table 不写 Model。应用校验后通过 `row_mut` 等安全 API 写回，再 `resolve_edit(..., Accepted)`；`Rejected(error)` 保留草稿并显示错误，允许重试。Pending 阻止重复请求，Esc / 外部点击不撤销已发出的意图；确认不依赖 Revision，允许无值变化成功。共享 View 按 Model Revision 更新，编辑与选择独立。无效或过期请求、错误 View、已删除 Cell 的回执返回 Error。View / Source / Cell 失效时结束编辑；未提交草稿在关闭编辑资格或 Disabled 时取消，已提交请求仍可等待合法回执。

## TSV 复制

`WidgetryTableCopyFormatter<V>::new` 接受 `&V -> Result<String, WidgetryTableCopyFormatError>`。`diagnostic(failure)` 承接受管领域失败并保留原始 ID、origin 和原因链；`external(error)` 保留标准外部错误的具体类型。格式化失败统一补充 View / Model Source / 稳定 Cell / 值类型上下文，不通过字符串重新包装。`WidgetryTableCopyAppExt` 注册值类型默认或 `(Source, ColumnId)` 列覆盖，覆盖优先；类型不匹配明确失败，不回退到 UI 文本或 Debug。运行时 `WidgetryTableCopyFormatterRegistry` 可注册、移除类型默认或清理 Source 列覆盖，与 Renderer/Editor 注册表独立。

`selection_tsv(world, view)` 返回 `Result<Option<String>, TableCopyFailure>`，其中成功值是当前完整逻辑选区的文本；先刷新行投影，再冻结稳定 Row/Column ID 序列，以 Model 强类型值生成 TSV。顺序跟随当前 View，包含离屏 Cell，排除过滤行，不含列标题、行号、Corner 或图标。单格、整行列、连续范围和 All 共用同一路径。字段含 Tab、CR/LF 或双引号时加引号并将内部双引号加倍，不修剪或改写原始内容。

`copy_selection` / `queue_copy_selection` 在所有字段成功后一次写入 Bevy `Clipboard`，成功返回 true；无选区、空投影、根 Disabled 或编辑期间返回 false 且不写入。普通 Table 焦点的 Ctrl+C 调用相同入口；Editor 内快捷键归输入控件所有。任意缺失格式化器、错误类型、格式化/取值/序列化失败都会整体拒绝，返回 Error 并记录 View 和可定位的稳定 Cell 身份，保留旧剪贴板。`copy_selection` 同样返回 `TableCopyFailure`；自身失败由 `TableError::Copy` / `TableCopyFault` 区分缺失部件、失效行列和值、缺少或错误类型的 formatter、字段长度溢出和分配失败。原生写入失败保留 `ClipboardError` 和输出字节数，不在诊断中写入整段 TSV。Queue / Keyboard 边界显式转换并以 Severity::Error 交给宿主。TablePlugin 自动安装 Bevy ClipboardPlugin。

同帧投影清空、过滤或 Model 删除端点造成选区失效时，复制按空选区处理，不依赖下一帧自动修复；已成功解析选区内的真实取值或格式化失败仍整体报错。

## 已知限制

- TSV 消费软件需支持带引号的字段，才能无损还原包含 Tab、换行和引号的单元格。

- 目前同一个 Model 的多个 View 共享 Column Schema 与列取值函数，行显示投影可独立；不支持为同一 Model 的不同 View 分别定义 Column Schema。
- 相同 Cell 值类型的 Renderer 当前在 App 级注册，不提供任意逐 View Renderer Registry。
- 二维虚拟化仅构造可见范围，不表示任意规模的 Model 操作都具有常数成本。

## 严格冻结列

`WidgetryTableLayout::frozen_columns` 默认 0，可在构造 Layout 或运行时逐 View 配置。有效数量仅为 `min(N, column_count)`，始终保留原始 N；冻结区域超过视口宽度、列宽增大或视口变窄时也不减少冻结数量，不修改列宽。新增列未达到 N 时继续归入冻结区。

根实体的 Corner、RowHeaders、滚动 ColumnHeaders/Body，加上 `WidgetryTableFrozenColumnHeaders` / `WidgetryTableFrozenBody` 构成实际裁剪区域。完整列宽按 Table 可用区域求解，冻结列固定在左侧，剩余列沿 X 滚动；主 Body 的 `ScrollPosition` 是滚动 Authority，冻结 Body 与 RowHeaders 只投影其 Y。冻结区滚轮转交主 Body；透明背景不影响分区剪裁与 Pointer 命中。

两区都仅生成可见 Y 行和与各自可用 X 区间相交的列。超宽冻结配置中，被视口完全裁掉的列不物化，但仍属于前 N 列，不能被水平滚动露出。冻结配置或列宽改变时 Cell/Header 按稳定 ID 换到正确 Canvas，保留编辑草稿及正常 Shell 身份；Header 与 Cell 对齐，Resize Handle 保留列身份。键盘导航对冻结列仅 Reveal Y；矩形拖选跨边界时分别解析固定/滚动坐标。

## View 列顺序与标题拖拽

构造时通过 `column_reordering: WidgetryTableColumnReordering { enabled: true }` 启用标题主体拖拽，也可运行时修改该配置 Component 的 `enabled`。默认关闭。主体单击仍选择整列，移动达到 4 个窗口逻辑像素后显示插入线；仅在当前 Table 可见标题区内释放才提交，区外释放或 Cancel 不改变顺序。最右侧 6px Resize Handle 只改列宽，排序/筛选按钮及内嵌 Button 不启动列重排。

`WidgetryTableViewState` 通过 `ordered_columns()`、`column_id(display_index)`、`column_display_index(id)`、`has_manual_column_order()` 只读查询顺序。首次实际重排后以稳定 ColumnId 保存 View 偏好，不调用 Model 的 `move_column()`。普通行编辑、行投影、Renderer 替换或 Model 原始列重排不覆盖偏好；新增列按数据源次序追加到 View 末尾，删除仅清理对应 ID，其他列相对顺序不变。没有手动顺序时继续跟随 Model。

`WidgetryTable::<T>::reset_column_order(world, view)` 清除手动偏好并立即回到当前 Model 原始列顺序，`queue_reset_column_order` 在 Commands 执行时校验。重复重置返回 `Ok(false)`，不产生通知；重置中断当前列拖拽，之后可重新拖拽。实际手动顺序状态改变时先提交 ViewState，再发出该 View 的 `ColumnOrderChanged`；Model 自动修复与换源不额外发业务通知。无效 View / Source / 行类型返回 Error，失败不清除原顺序。

每个共享 View 独立持有列顺序，Model Source 或整个 Model 实例更换后不继承旧偏好，即使新旧 ColumnId 相同；旧 Shell 内容与进行中的输入/编辑会话也会清理。冻结区始终取新显示顺序的前 N 个位置，允许跨冻结边界拖入，N 与按 ColumnId 保存的列宽不被改动。Header/Cell 渲染、范围解析、高亮、键盘 Reveal 和 TSV 都按 View 列顺序执行，Cell 值及 Revision 仍通过 ColumnId 查 Model。

取消、禁用、移除配置、源实例更换、起始列删除或 Header 虚拟化离开后清除插入线和 capture，不提交旧落点。原生 Mouse 拖动的 cursor lease 随会话清理；Custom Pointer 验证不等于 OS Mouse 验收。


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

## 连续区域选择

每个 View 同时最多有一块逻辑选区。`WidgetryTableSelection` 保留 None / Row / Column / Cell，增加 `Rows { anchor, end }`、`Columns { anchor, end }`、`Cells { anchor, end }` 和 `All`；范围端点都是稳定 ID，单点范围规范化为单项，范围按当前 View 显示位置解析。Shift 点击只延伸同类型选区并保留初始锚点；跨类型按普通点击重设锚点。Ctrl 不追加离散区域。

`set_selection` 可同步设置上述范围。`selection_range(world, root)` 返回只读的 `WidgetryTableSelectionRange`，`rows()` / `columns()` 是当前完整显示序列中的半开范围，`contains` 查询一个显示位置。它不展开 Cell ID 集合，也不以可见 Shell 为数据范围。Focused Cell 仍是独立的导航位置；程序设置 Selection 不强行聚焦或滚动。

Ctrl+A 和 Corner 选择 `All`，涵盖当前完整行投影与全部列，包括离屏 Cell，排除过滤掉的行。重复全选不切换或取消；空行投影或零列时全选规范化为 None。All 随当前投影增删变化，带端点的范围随显示顺序重新解析；端点删除或过滤后静默清空失效范围，内部成员删除则自然缩短范围。失效端点的程序请求明确报错，旧状态保留。

单元格拖选达到 4 个窗口逻辑像素后捕获稳定 Body，实时更新矩形终点，结束保留最终选区。锚点不依赖物理 Cell Entity，滚动使原 Cell 销毁也不会换成另一个身份；滚动继续走 Body。行号、列标题、Resize Handle、嵌套 Button / EditableText 不作为单元格拖选起点。禁用、失效端点、取消、指针消失或 View 销毁会结束会话并释放本会话的捕获；不自动滚动超出 View 的区域，Pointer 超出时终点限制在当前可见边界。

单项变化沿用 RowSelected / ColumnSelected / CellSelected / SelectionCleared；范围与 All 使用 `RangeSelected(selection)`。均先提交真实状态再通知，同值不重复，初始化与自动修复静默。颜色只投影到可见区域，滚动新建的 Shell 依同一逻辑范围选中。


Model 的插入、可失败的 revision 更新、View 投影更新 / 重置、Selection / Focus / 列宽及选区查询返回 `TableFailure`。`TableError` 保留行列轴、稳定 ID、index / len、next ID / revision / structure version，以及所属 View / Model Source。投影请求区分未知 / 重复行、失效排序列 / 筛选列、失效 Selection / Focus 和列位置；失败不提交请求状态。原有缺失查询的 `Option`、越界移除 / 重排的 `false` 及同值 no-op 不改为失败，Structure Version 的既有 wrapping 算法不变。

布局纯校验使用 `TableLayoutFault` 保存具体字段、ColumnId、Fixed / Flexible 原值或总几何尺寸、行列数量与视口参数。Scene 构造和公开请求在拥有 View / Source 的层次创建诊断，Commands 边界显式转换；持续 Runtime 布局故障按 Source 与实际配置位值复用诊断（包括相同 NaN payload），真实修复后恢复，再次故障产生新 ID。单元格 `Arc<dyn Any>` 保持原业务类型，不为诊断复制整个数据集。
