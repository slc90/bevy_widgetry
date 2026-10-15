# table Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Model Row / Column Stable ID、Selection None / Row / Column / Cell。
- Focused Cell、Viewport X/Y、Renderer Registry、Layout / Width 和 Resize Gesture。
- Source 有效 / 失效、Visible / Offscreen Cell 和各区域 Theme / Disabled。

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

## Couplings

- 两轴 Scroll × Header 同步与 Visible Range。
- Selection / Focus × Virtualization / Disabled。
- Resize × DPI / Column Lifecycle / Layout。
- Renderer Type Registry × Cell / Header Value。

## Coverage Map

- `renderers.rs`：Cell / Header Typed Registry、异构内容与缺失 Renderer 错误。
- `view.rs`：四区布局、Scene、Source / Revision 与 Theme / Disabled。
- `virtualization.rs`：二维可见范围、Row / Column Header 复用与静止投影。
- `interaction.rs`：Selection、Focused Cell、真实输入、Resize 及各种取消时序。
- `common/mod.rs`：共享 Fixture 与输入 Helper，不单独拥有业务覆盖。

## 覆盖边界

当前用例没有实现第二阶段所提“共享一个 Model 的两个 View 使用不同 Layout / Column Width 的集中 Gallery 演示”。Renderer 仍按 App 的值类型注册。

