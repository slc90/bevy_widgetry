# table Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

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

Gallery 的 Shared Model 示例用同一个 Model 展示不同 Layout / 列宽和区域 Style。Renderer 仍按 App 的值类型注册；逐 View Column Schema 和 Renderer 不在当前能力内。BRP 可自动验证 Custom Pointer 与键盘交互，OS Mouse 及实际显示性能不由这些 Headless 用例证明。

