# tree Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Model Expanded / Collapsed、Selected Entity 与 Unknown / Loading / Loaded。
- 可见 / 离屏 Node、View Focus / Disabled、Renderer Subtree。

## Stimuli

- BSN Scene、公开 Expand / Collapse / Selection API。
- ECS Hierarchy / Component Mutation、真实 Pointer / Keyboard / Wheel、Lazy Reply、Theme。

## Guards

- Source / Node / Renderer 有效、配置合法。
- Disabled 阻止用户输入；同值选择不重复通知。
- Expander 不等于 Row Selection。

## Transitions

- Expand / Collapse、Lazy Request / Completion、Selection / Clear。
- 外部 Hierarchy Mutation 后可见顺序重建，删除节点 Repair。
- View Spawn / Despawn 不销毁业务 Node。

## Invariants

- Node Entity 是业务身份；未受影响的 Entry ID / Revision 稳定。
- Model Authority 先提交再通知，自动 Repair 静默。
- 多 View 共享 Expanded / Selection，保留各自 Disabled / Scroll。
- Renderer 在动态 Component / Theme 更新时生成正确可见内容。

## Couplings

- Lazy Loading × Expand / Collapse。
- Hierarchy Mutation × Visible Projection / Selection Repair。
- 多 View × Shared Model，独立 Disabled 边界。

## Coverage Map

- `contract.rs`：Scene / Source / Props / Shell 错误处理。
- `model_behavior.rs`：Headless Expansion、Selection、Lazy 与外部 Hierarchy。
- `view.rs`：真实输入、共享 Source、虚拟化、Renderer、Focus / Disabled。
- `rendering.rs`：真实 UI Pipeline 中的新 Text / Expander 同帧准备。

## 覆盖边界

当前测试覆盖共享 Model 的基本双 View 行为；第二阶段拟新增的集中 Gallery 示例与进一步共享数据演示尚未实现。

