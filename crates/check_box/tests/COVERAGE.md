# check_box Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 二态 Checked / Unchecked；三态 Unchecked / Checked / Indeterminate。
- Focus、Disabled、Theme、Indicator Materialization。

## Stimuli

- Pointer、Keyboard Space / Enter、公开 Set / Cycle Queue、Theme、Asset 就绪和状态恢复。

## Guards

- Disabled 阻止用户变更，合法程序化 Set 仍执行。
- 无效 Root 必须报错；Repeat Key Press 不应重复提交操作。

## Transitions

- 三态按 Unchecked → Checked → Indeterminate → Unchecked 循环。
- 同值请求不产生新通知；禁用与恢复保留既有状态。

## Invariants

- 三态先提交 Authority 再发送 `ValueChange`。
- 官方二态 Checked 遵循 Bevy 的写入时序。
- Indicator / Accessibility 与当前状态一致，Theme 变化保留逻辑状态和标记 Identity。

## Couplings

- Input × Disabled、State × Indicator 与 Accessibility。
- Runtime Set / Queue × 真实执行时机；Theme × Icon 初始化。

## Coverage Map

- `checkbox.rs`：二态与三态公开行为、输入、排队更新、配色、Indicator 和故障传播。
- 源码中 `tri_state.rs`、`style.rs` 的 Unit Test 保护局部状态转换及样式优先级。

