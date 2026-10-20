# table Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

`frozen.rs` 保护冻结/滚动区域的稳定 X、共同 Y、离屏行有界虚拟化、透明区域真实 Clip、冻结区滚轮路由、N=0/1/2/超过列数、窄视口/列宽/列增删不减少配置、共享 View 隔离、DPI/UiScale 与分数滚动下 Header/Cell/行号对齐、跨区拖选和捕获释放、两区 Resize 身份、Flexible 宽度 API 使用完整视口、真实字体在 Resize 首次 Update 后的 glyph/clip、全冻结时键盘 Y Reveal，以及编辑草稿跨区换父和空行/空列。`view.rs` 继续保护六个实际根区域的构造契约，其余既有场景回归 N=0。字形布局回归不覆盖已知的 GPU 显示空白问题。

`clipboard.rs` 主要保护完整逻辑选区的强类型 TSV：单格/行列/范围/全选、过滤后的显示顺序、离屏数据、双 View 与 Source 列覆盖隔离、Tab/CR/LF/引号/Unicode/空值无损往返、缺失/类型不匹配/失败格式化器的原子拒绝和稳定 Cell 错误身份。原生 Clipboard 用例集中串行执行，覆盖失败时旧内容不变、成功写入、不含 Header、空投影/Disabled/编辑中 guard；真实 TextInputPlugin + TextField 验证编辑器 Ctrl+C 文本选区拥有权与 Esc 后恢复 Table 复制。提交入口的零次/一次写入由 Module Unit Test 保护。

投影清空或 Model 删除端点后同帧立即复制按自动修复后的逻辑选区处理，不依赖后续 reconciliation 帧，也不将正常失效选区报告为错误。

## States

- Model Row / Column Stable ID、Selection None / Row / Column / Cell。
- Focused Cell、Viewport X/Y、Renderer Registry、Layout / Width 和 Resize Gesture。
- Source 有效 / 失效、Visible / Offscreen Cell 和各区域 Theme / Disabled。
- 各 View 的 Override 与 Style padding / border，逐部件 Hover 与样式输出失效 / 恢复。

## Stimuli

- BSN 构造、Model CRUD / Projection Revision、Renderer 注册与替换。
- Pointer / Keyboard / Scroll、Programmatic Selection / Width、Resize Drag / Cancel、Theme。

## Guards

- Source、Schema、Renderer 与 ID 有效；Disabled 阻止用户输入。
- Primary Pointer / Focus / Gesture Owner 必须有效，Resize Target 消失即取消。

## Transitions

- Model Data → Cell / Header Projection。
- Selection / Focused Cell Navigation；程序化清空和 Model Repair。
- Resize Start → Width Changed → End / Cancel。
- X/Y Viewport 切换只实例化与当前矩形相交的 Cells。

## Invariants

- Row / Column Identity 和 Selection 在移动后稳定、删除后修复。
- 真实 State 先提交再通知；同值和自动 Repair 静默。
- Cell / Header 内容与对应 Value Type、Revision 一致。
- 静止时不重复投影 Schema，不重复发布没有变化的 Shell State。
- Resize Gesture 每次只有一个合法终止事件。
- Cell / Header Registry 隔离，替换仅影响相应 Factory；缺失 Renderer / Registry 报 Error，不以 Fallback 代替。
- 重叠可见 Cell / Header 保留 Entity，空 Axis 和两轴变化保持 Pair 唯一；Revision / Type / Renderer 替换只重建相应内容。
- Resize Callback Mutation 参与当前 Projection；Mouse Resize 的 Cursor Lease 随开始 / 离开 / 终止管理，Custom 不影响 OS Cursor。

## Couplings

- 两轴 Scroll × Header 同步与 Visible Range。
- Selection / Focus × Virtualization / Disabled。
- Resize × DPI / Column Lifecycle / Layout。
- Renderer Type Registry × Cell / Header Value。

## Coverage Map

- `renderers.rs`：Cell / Header Typed Registry、异构内容与缺失 Renderer 错误。
- `view.rs`：四区布局、Scene、Source / Revision 与 Theme / Disabled。
- `view.rs` 还覆盖共享 Model 的两侧独立配色、禁用侧数据更新、内容 / Renderer 替换、两轴滚动后的新建 / 复用 Cell 在同帧完成 Text 前景、Font、glyph、Visibility、Stack 和 Layout 准备；字体前置使用现有 test_utils 真实 UI 设施。
- 源码样式单元测试保护稳定帧零重复根解析 / 区域配色，单 Cell Hover / Disabled 及移除的局部工作量，旧 / 新 Row、Column、Cell Selection 与 Focus 区域，纯 Style 几何变化不重算颜色，Override / Theme、Selected 输出修复，以及持续 Error 的宿主传播、诊断去重和真实恢复。
- `virtualization.rs`：二维可见范围、Row / Column Header 复用与静止投影。
- `interaction.rs`：Selection、Focused Cell、真实输入、Resize 及各种取消时序。
- `interaction.rs` 的共享 Model 双 View 用例覆盖不同初始 Layout / 列宽、单侧改宽、Pointer / Keyboard 的事件归属和 Focus 转移、独立两轴 Scroll / Disabled，以及共享 Edit / Insert / Move / Remove 后的 Row / Column Stable ID、各自静默 Repair 和 View 销毁后的 Model 所有权。
- `common/mod.rs`：共享 Fixture 与输入 Helper，不单独拥有业务覆盖。

## 覆盖边界

- 编辑组合回归使用真实 TextField 验证本帧 pending text edit 与 Enter / Ctrl+Enter 的请求值一致，使用真实已打开 ComboBox 验证 Esc 取消整个 Table 草稿；Cell/Body 局部 Disabled 阻止请求并丢弃草稿。
- 三个 View 同帧排队提交、应用首个请求 Observer 关闭整个页面时，后续提交跳过已销毁 View，不 panic，也不销毁共享 Model。

- `editing.rs`：默认关闭、类型默认及 Source/Column 覆盖、错误类型只读、ReadOnly Hint 与 Disabled guard；Enter/双击启动、Esc/外部 Press 取消、编辑器及锚定 Popup 内部 Press 保留、多行 Ctrl+Enter、解析失败；Pending 先于强类型通知、重复提交及 Pending 取消 guard、Rejected 重试、新旧 request ID、错误 View/已删除 Row 回执拒绝；Revision 和虚拟化保留编辑器，Accepted 结束，不由 Table 写 Model。

Gallery 的 Shared Model 示例用同一个 Model 展示不同 Layout / 列宽和区域 Style。Renderer 仍按 App 的值类型注册；逐 View Column Schema 和 Renderer 不在当前能力内。BRP 可自动验证 Custom Pointer 与键盘交互，OS Mouse 及实际显示性能不由这些 Headless 用例证明。


## 行投影与受控更新

- `view.rs`：默认与受控 View 独立序列、实际 Cell Text 顺序、原子提交 rows/sort/filters、重复或过期 ID 拒绝且保持旧事实、空投影/重置、queued 执行时重新校验；共享 Model 插入/移动不自动进入受控结果，删除清理及 Row/Column 指标失效、重排后内容 Revision 与 Shell Identity；同 World tick 连续结构编辑与同步 API 的缓存一致性。
- `interaction.rs`：键盘与 Reveal 使用显示索引、过滤后的 Selection/Focus 静默修复、空投影和过滤行程序 Focus 拒绝。
- `virtualization.rs`：2000×40 的过滤倒序、双轴滚动下完整逻辑行到有限 Cells 的投影、显示行号、实际强类型内容与 Revision 不串位。
- `benches/update.rs` 的 row_projection/projected_idle/projected_scroll 测量受控更新和受控 View 稳定/滚动路径，校验实际新序列、Model 不变、Cell 内容与 Entity 数量有界；不代表完整 GPU/Present 性能。

## 表头请求

- `header.rs`：可选排序/筛选按钮的真实 Press/Click/Release、事件 ColumnId 和目标 View、基于已确认状态的排序循环/换列、未回填重复请求、未改变 Model/Selection/投影、多列过滤指示；Disabled/配置关闭/列删除 guard，以及两轴虚拟化后重建的稳定按钮身份和标题选择。
- 原子失败回填由 `view.rs` 的完整 rows/sort/filters 验证承担；原有 `interaction.rs` 保持 Resize Start/End/Cancel 回归覆盖。
- `benches/update.rs` 的 header_idle/header_scroll/header_commit 覆盖启用入口后的稳定、滚动和确认指标更新，检查按钮数量有界；只测量 Headless CPU/ECS 路径。

## 连续范围与全选

- States：单项与 Rows/Columns/Cells 稳定 ID 端点、All、独立 Focus、拖选指针会话与捕获。
- `interaction.rs`：同类型 Shift 多次延伸保留锚点、跨类型重设；受控显示次序下范围解析、状态先提交及去重；Corner/Ctrl+A 全选当前完整投影与空投影；拖选阈值内微小移动保留原选区/焦点及 Shift 点击锚点，跨横纵 Scroll 保留拖选锚点，终止保留范围；Disabled/任一端点行列删除/终点过滤/取消/销毁释放捕获并阻止晚到输入，失效端点修复静默。
- `view.rs`：共享 Model 相同端点在两个不同投影中产生不同连续范围；重排后旧/新范围成员高亮更新，单侧清空不改变另一侧。
- `virtualization.rs`：2000×40 数据过滤倒序后的 All、跨离屏矩形、逻辑范围完整且可见 Shell 数量有界，新建 Cell 的 Selected 与显示位置一致。
- `benches/update.rs` 的 range_selection/all_selection/range_scroll 测量范围提交与滚动，验证逻辑状态、可见 Selected 和区域配色，保留旧 Selection / Resize / Header Benchmark。
- `interaction.rs` 的真实 PointerInput 队列覆盖 Move/Release 同帧及跨帧释放：不产生临时单格通知，释放后下一次正常 Click 恢复可用。
- 拖选期间 Renderer 缺失导致投影丢弃时立即释放捕获，补齐 Renderer 不复活旧选区或通知；Source 在 Update 阶段销毁同样清理。
