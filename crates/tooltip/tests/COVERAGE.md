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
- `independent.rs`：不经默认字体或 UI Plugin 安装纯 Headless，无 Factory/默认 Style；保护异位 Popup 归属、持续悬停单次 Show、Anchor Despawn / 能力移除、重复与过期登记/Hide、排队构造前失效、Scene 失败清理、原生 WindowFocused 消息转换的 Cancel / Hide，以及安装默认 Plugin 时不接管自定义实例。
- 源码 `headless.rs`、`style.rs` 局部测试负责精确计时、前景命中与私有 Guard。
- `style.rs` 局部测试另保护构造完成时立即采用 Anchor 覆盖色、稳定显示零重复配色、无关 Anchor 更新及 Popup 重新绑定；`tooltip.rs` 验证可见覆盖 / 清除 / Theme 与富 Text/Icon 前景同步，颜色变化不重建内容。


- `diagnostics.rs`：颜色领域分类、Core source / origin 传播、托管拒绝、缺失内容通过 Scene 保持分类，独立无效请求拥有不同 ID。
- `style.rs` 局部诊断测试覆盖运行期 Factory 缺失内容、嵌套 Scene 原诊断和 Anchor/Popup 上下文、失败 Popup 清理，以及两 Popup 样式失败批量上报、重复观察保持 origin、恢复 INFO。正常显示、隐藏、重复请求和无目标过滤无诊断事件。
