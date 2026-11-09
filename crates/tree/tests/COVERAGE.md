# tree 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 节点 Expanded／Collapsed、Selection／Active 与 Unknown／Loading／Loaded 状态；View Focus／Disabled、可见／离屏 Row 和 Renderer 子树。
- 多 View 共享原始 ECS 树与 Lazy Loading，但各自维护展开、选择和滚动状态。

## Stimuli
- BSN Scene、公开展开／折叠／选择／移动 API、ECS Hierarchy／Component 变更。
- 真实 Pointer／Keyboard／Wheel、拖放、Lazy Reply、Theme／Override 和 View 生命周期。

## Guards
- Source、Node、View／Renderer 与配置必须有效；Indent 非负有限，Item Height 为正且有限。
- Disabled 阻止相应用户输入，合法程序化状态更新仍允许；Expander 操作不等同 Row Selection。
- 跨树、循环、失效或不满足 Lazy 状态要求的节点移动必须原子拒绝。
- BSN 构造中间态可以延后同步，完整 Shell 的失效依赖必须报告并传播错误。

## Transitions
- Expand／Collapse、Lazy Request／Completion、选择／清空／多选。
- 外部层级修改、移动／拖放、删除后的可见顺序重建与选择修复；View 创建／销毁与 Renderer 重建。

## Invariants
- Node Entity 直接作为内部 ListView Row、Selection 和 Active 身份；展开、折叠、Row Reuse 和重挂接保持业务 Entity 不变。
- Selection／Expanded Authority 先提交再通知，合法同值与自动 Repair 静默；折叠不清除仍有效的隐藏 Selection，删除失效节点才修复。
- 多 View 的 Selection、Expanded、Active、Focus、Disabled、Scroll 和颜色覆盖相互独立；View 销毁不销毁共享业务树。
- Lazy Request 不因再次展开重复，完成回复只对符合当前展开状态的 View 投影。
- 动态 Renderer／Text／Expander 在既有同帧准备契约下完成内容、归属、字体、尺寸和配色更新；稳定帧不重复处理无变化状态。
- 虚拟化和 Renderer 重建不把旧命中或部件 Owner 错配给新 Row；非法移动和结构失败不部分修改树或破坏无关业务子树。
- 错误传播保留 Root／Model／请求、原始 source／origin、持续故障与真实恢复的诊断语义。

## Couplings
- Lazy Loading × Expand／Collapse；Hierarchy Mutation × Projection／Selection Repair。
- 多 View × 共享 Model／独立 Disabled；Drag × Virtualization／Renderer；Theme × 动态部件 Owner。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `contract.rs` | Scene、Source、Props、Shell 的构造和错误契约。 |
| `drag.rs` | 真实 Pointer 拖放、合法落点、虚拟化／Renderer 变化和取消。 |
| `model_behavior.rs` | Headless 展开、选择、Lazy Loading 与外部层级变化。 |
| `movement.rs` | 节点移动、层级／顺序变化、非法请求和提交原子性。 |
| `multi_selection.rs` | Ctrl／Shift 多选、范围／全选、批量操作和视图隔离。 |
| `rendering.rs` | 新建 Text／Expander 的同帧 UI 内容准备。 |
| `view.rs` | 真实输入、虚拟化、Theme／Owner、Focus／Disabled 和共享树显示。 |
| `view_state.rs` | 多 View 独立状态、共享 Lazy、删除／销毁与修复。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/movement.rs` | 节点移动的局部规则、No-op、诊断与实际层级一致性。 |
| `src/renderer.rs` | Renderer 身份、错误传播、故障恢复和业务子树保留。 |
| `src/view.rs` | Source／View 状态修复、持续错误及多实例隔离。 |
| `src/view/style.rs` | 内部颜色 Owner、局部样式更新、缓存稳定性及修复。 |
