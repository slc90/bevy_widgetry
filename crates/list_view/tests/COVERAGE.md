# list_view Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Model Source 与 Stable ID / Revision、Selection、Active、Focus。
- Root / Item / Container Disabled，Viewport Range、可见 / 离屏 Row 生命周期。

## Stimuli

- Typed Scene / Renderer 注册；Model CRUD、公开 Setter / Clear。
- Pointer、Keyboard、Wheel、Scroll / Resize、Theme 与 Item Metadata 更新。

## Guards

- Source / Renderer / Item Height 合法。
- Disabled 阻止用户操作，程序更新保持可用。
- 旧 Press 不能跨 Row Reuse 选择新 Identity。

## Transitions

- 初始构造、选择 / 清空、Active Navigation、数据删除 Repair。
- 移动 / 更新数据之后刷新对应可见内容；Scroll / Resize 重建 Range。

## Invariants

- 真实 Selection 先提交再通知，合法同值和自动 Repair 静默。
- Stable ID 保持、Active / Selection 各自修复。
- 虚拟化只保留需要的可见 Row；Content Revision 只影响对应项。
- Theme / Disabled 不破坏 Row 内的业务内容与 Identity。
- 行复用换 ID、Renderer 内容替换和滚动返回旧数据后，当前 Override 与前景在生成帧生效；清除 Override 恢复主题。
- Root / Viewport / Content / Item Disabled 分别保护用户确认入口；多个共享 Source 的 View 不共享 Selection 或用户通知。
- Model 删除按旧 Active 位置修复，Source 失效后报错、恢复后重新投影；Typed Runtime 注册幂等，Props 不形成第二份运行期状态。
- 可见投影无 Overscan，重叠 Row 复用；Identity / Revision 决定 Subtree 重建，生成帧内完成真实 Text / Icon 消费准备。

## Couplings

- Model CRUD × Selection / Active Repair。
- Focus / Disabled × Pointer / Keyboard。
- Renderer Content × Row Reuse / Text / Icon 准备。

## Coverage Map

- `contract.rs`：Typed Source、Scene 配置、Shell 与错误路径。
- `selection_api.rs`：集合 Authority、提交后集合/差量事件、ID/原始索引/显示索引、执行时解析与过期 ID 拒绝。
- `multi_selection.rs`：Ctrl 切换、Shift 投影范围与锚点回退、整源批量全选、Single 配置、禁用守卫及共享 View 独立集合。
- `behavior.rs`：真实输入、Selection / Active、共享 Model View 隔离和 Repair。
- `projection.rs`：共享 Model 独立过滤/重排、原始索引与显示索引区分、隐藏选择保留和删除修复，以及 Entity 身份、临时拥有数据和无 Revision 数据源。
- `virtualization.rs`：Viewport Range、Row Reuse、Revision 与当帧 Content。
- `style.rs`：Theme、Focus、Hover / Pressed 与 Disabled 样式。
- 私有单元工作量回归：稳定帧零配色解析、单行交互不扩散、Focus 旧新目标隔离，以及 Owner 调色板在局部变化和新内容中保持。计数不进入公共 API。

## 覆盖边界

`behavior.rs` 在已有共享 Source 用例上覆盖文字与图标文字双 Renderer：Pointer / Keyboard 的通知归属、Focus 转移、禁用一侧后的组合 Insert / Edit / Move / Remove 和静默 Repair，以及独立 Wheel 与禁用恢复。Renderer 真实像素准备仍由 `virtualization.rs` 负责；Gallery 并排操作与语言切换由任务级 BRP 验证。

`benches/update.rs` 固定 Model 数量与 Viewport，分别测 idle、hover_one 和 scroll；Hover 刺激在 Picking 后提交并核对颜色输出。Headless Update / Layout 耗时不代表原生窗口 Present 或持续交互 60 FPS。


- `drag.rs`：真实 Pointer 事件前/后锚点穷举、过滤隐藏项和多选保持、原始索引变更、失效 ID、Disabled / Cancel / Row 复用 / 无合法悬停拒绝、同位置不触发 Changed、共享 Model 不被拖拽修改、View 显示顺序/选择/锚点/Active 隔离；Camera DPI / viewport 与额外 UiScale 的坐标转换，以及取消拖动后抑制本次释放 Click、后续点击恢复。
- `benches/update.rs` 的 `draggable_idle` / `draggable_scroll` 对照默认关闭路径，仍验证有界 Row 和实际滚动内容。

Tree 组合通过 `crates/tree/tests/multi_selection.rs` 验证 VisibleUnion 的隐藏集合保留、真实 Ctrl+A 与 Disabled View 程序化全选；普通 Source 范围的全模型选择继续由本 crate 的 `multi_selection.rs` 保护。

Tree 专用内部桥接的三段落点、Inside禁止反馈与实时加载状态、运行时开关取消、Entity复用和万项滚动组合由 `crates/tree/tests/drag.rs` 保护；本 crate 保留原列表上下半区与过滤排序回归。

`drag.rs` 另保护完整手动顺序的数据刷新/新增末尾/删除、过滤隐藏身份恢复、过滤状态下 Before/After 只移动实际源 ID、关闭后保序与显式重置、主动投影策略替换及同数值 ID/revision Source 更换重置、旧命中 Row 销毁与实际 Text 内容刷新。`projection.rs` 验证只读外部数据源不需要写入适配即可启用 View 拖拽。

`drag.rs` 工作量回归保护无数据变化的拖拽提交不重新求值投影资格；`benches/drag.rs` 在 1k/10k/100k 项上经真实 Pointer 事件反复换位并完成一次 Headless Update，核对 Model 不变、每次投影实际改变与可见行有界，记录 CPU/Layout 耗时，不覆盖原生窗口 Present。
