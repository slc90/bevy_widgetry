# check_box 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 二态 Checked／Unchecked 与三态 Unchecked／Checked／Indeterminate，Focus、Disabled、Theme 和 Indicator 就绪状态。

## Stimuli
- Pointer、Keyboard、公开 Set／Cycle、Commands 延后执行、Theme 与资产准备。

## Guards
- Disabled 阻止用户切换，但合法程序化设置继续生效；无效 Root 明确报错，重复键不产生重复操作。

## Transitions
- 三态按 Unchecked → Checked → Indeterminate → Unchecked 循环；同值请求静默，禁用／恢复保留状态。

## Invariants
- 三态先更新 Authority 再发送 `ValueChange`；二态遵循原生 Bevy Checked 写入时机。
- Indicator 与真实状态一致；Theme 改变不重置选中状态或标记身份；Headless 不要求默认图标子树。
- 本地／祖先 Disabled、实际输入与排队操作不产生重复写入或错误事件；无效颜色更新保留旧配置与原诊断身份。

## Couplings
- 输入 × Disabled；状态 × Indicator；Theme × Icon 初始化；Headless × 默认 Plugin 共存。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `checkbox.rs` | 默认二态／三态、真实输入、程序更新、颜色及 Indicator 行为。 |
| `diagnostics.rs` | 状态／颜色的领域失败、托管归属和宿主诊断。 |
| `headless.rs` | 独立二态 Headless 的真实命中、焦点及继承禁用。 |
| `tri_headless.rs` | 独立三态 Headless、循环、通知、排队与输入去重。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/tri_state.rs` | 三态局部状态转换与边界条件。 |
| `src/style.rs` | 配色优先级、部件缺失、恢复及样式诊断。 |
