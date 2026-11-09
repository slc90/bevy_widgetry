# list_view 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Source、Stable ID／Revision、Selection／Active／Focus；Root／Item／Container Disabled。
- 可见区、离屏行、虚拟化 Row 与 Renderer 内容状态。

## Stimuli
- Typed Scene、Model CRUD、Selection API、真实 Pointer／Keyboard／Wheel、Scroll／Resize、Theme 与 Item Metadata。

## Guards
- Source、Renderer、Item Height 和目标身份有效；Disabled 阻止用户确认，但合法程序化操作仍可执行。
- Row Reuse 后的旧 Press／Selection 不能误作用到新的 Stable ID。

## Transitions
- 初始化、选择／多选／清空、Active 导航、源数据增删移动、Selection Repair、投影过滤重排与 Row Reuse。

## Invariants
- Selection Authority 先提交再通知；合法同值和自动 Repair 静默，Active／Selection 分别修复。
- Stable ID 不随原始顺序或显示投影漂移；多个共享 Source 的 View 各自维护 Selection／Focus／Disabled。
- 虚拟化保持有界可见 Row 数，正确复用身份并在当前帧完成新 Text／Icon 内容准备；Content Revision 只更新相关项。
- 过滤和重排保留隐藏选择及原始／显示索引语义；数据损坏、失效 Source 和 Renderer 错误不破坏健康实例。
- Theme／Override／Disabled 与动态 Row 的受管理配色保持一致；运行时注册和状态来源保持唯一。

## Couplings
- Model CRUD × Selection／Active Repair；投影 × 虚拟化／拖放；Disabled／Focus × 输入；Renderer Reuse × Theme／内容准备。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `behavior.rs` | 真实输入、单／多 View 行为、Selection 与 Active 修复。 |
| `contract.rs` | Typed Source、Scene、Renderer 和结构错误。 |
| `diagnostics.rs` | 公开选择／数据请求的领域诊断与 Command 传播。 |
| `drag.rs` | Row 拖放、投影顺序、虚拟化身份、取消与非法提交。 |
| `multi_selection.rs` | 多选集合、键盘修饰、范围／全选和 View 隔离。 |
| `projection.rs` | 过滤重排、隐藏选择、原始／显示索引及身份稳定性。 |
| `runtime_diagnostics.rs` | 多个 Root／Renderer 的持续故障、传播、恢复与隔离。 |
| `selection_api.rs` | 公开集合 Authority、事件、索引和延后操作。 |
| `style.rs` | Theme、Focus、Hover、Disabled、前景与覆盖样式。 |
| `virtualization.rs` | 有界可见投影、Row Reuse、Revision 和动态内容同步。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/model.rs` | Source／Stable ID 更新、耗尽和操作序列不变量。 |
| `src/style.rs` | 配色与部件 Owner 的局部更新契约。 |
