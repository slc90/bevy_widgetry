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

## Couplings

- Model CRUD × Selection / Active Repair。
- Focus / Disabled × Pointer / Keyboard。
- Renderer Content × Row Reuse / Text / Icon 准备。

## Coverage Map

- `contract.rs`：Typed Source、Scene 配置、Shell 与错误路径。
- `behavior.rs`：真实输入、Selection / Active、共享 Model View 隔离和 Repair。
- `virtualization.rs`：Viewport Range、Row Reuse、Revision 与当帧 Content。
- `style.rs`：Theme、Focus、Hover / Pressed 与 Disabled 样式。

## 覆盖边界

已有测试包含共享 Source 的 View 隔离，但尚未形成计划中的“不同逐 View Renderer + Gallery 并排演示”的完整新增验收。

