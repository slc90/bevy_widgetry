# tooltip Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 无候选 / 等待 / 显示 / Warm、Anchor 与 Owner Identity。
- Theme、Disabled、动态内容及 Popup 可见性。

## Stimuli

- 真实 / 控制 Hover、时间推进、Anchor / Owner Despawn、Theme。
- Content Factory 调用、动态 Text / Icon 准备。

## Guards

- 必须提供 Content Factory；同 Anchor 不重复构造。
- Disabled Anchor 仍允许 Tooltip 显示。

## Transitions

- Cold / Warm Wait → Show → Exit / Cleanup。
- 切换 Anchor、Pointer 失效、Target Despawn 后重新决策。

## Invariants

- 同时最多一个 Popup；Factory 只在显示时执行。
- 失效 / 离开后不留孤儿 Entity。
- 新 Popup Subtree 在实际 Picking 前具备 IGNORE 与 UI 消费准备。

## Couplings

- Hover Ownership × Timing / Warm Window。
- Theme × Popup Shell 与动态 Text/Icon。
- Anchor 覆盖色更新 / 清除 × 可见 Popup，主题切换保留局部覆盖；隐藏后 Warm 重现使用当前 palette 与新 Factory 内容。

## Coverage Map

- `tooltip.rs`：公开 Scene、Hover / Timer、Factory、Popup Lifecycle、Theme 和真实内容准备。
- 源码 `headless.rs`、`style.rs` 局部测试负责精确计时、前景命中与私有 Guard。
- `style.rs` 局部测试另保护构造完成时立即采用 Anchor 覆盖色、稳定显示零重复配色、无关 Anchor 更新及 Popup 重新绑定；`tooltip.rs` 验证可见覆盖 / 清除 / Theme 与富 Text/Icon 前景同步，颜色变化不重建内容。

