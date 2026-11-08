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
- Indicator 与当前状态一致，Theme 变化保留逻辑状态和标记 Identity。

## Couplings

- Input × Disabled、State × Indicator。
- Runtime Set / Queue × 真实执行时机；Theme × Icon 初始化。

## Coverage Map

- `checkbox.rs`：二态与三态公开行为、输入、排队更新、配色、Indicator 和故障传播。
- `headless.rs`：独立二态 HeadlessPlugin 未安装 Icon Plugin、不生成默认 Indicator 时，Mouse/Custom Pointer 的真实布局命中、Pressed/Checked、自更新与唯一输入请求；键盘重复/释放、焦点、原生 Checked 程序写入及本地/祖先 Disabled。与默认 Style 共存时不新增部件、不改自有颜色、不重复自更新；不接管仅带官方 Checkbox 的其他实体。
- `tri_headless.rs`：无 Icon Plugin/默认 Indicator 的三态独立行为，Mouse/Custom Pointer 完整三态循环与提交后观察、初始化及同值静默、Commands 执行时读取状态、禁用期间程序设置、焦点/重复/继承禁用、ActivateOnPress 独占与取消清理。状态从不由原生 Checked 表示；与默认 Plugin 共存不重复提交、后挂载行为保留已有状态且不通知、自有颜色不被覆盖。
- 源码中 `tri_state.rs`、`style.rs` 的 Unit Test 保护局部状态转换及样式优先级。
- `diagnostics.rs`：颜色 Core 原因链/origin/状态与字段、托管 Owner、同值静默，真实状态/颜色 Command 的 Error Severity 与源头 ID，初始颜色经 Scene 同源传播并清理 Root。
- Style Unit Test 分别破坏多个 Root 的颜色输出与 mark geometry，验证全部诊断到达宿主、健康第三个继续更新、持续同源、实际修复恢复当前投影且只记录一次 INFO、恢复后再次损坏生成新 ID；已有故障回归保留部件 Entity 与 Root 上下文。

