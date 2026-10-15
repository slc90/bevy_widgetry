# button Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Normal、Hover、Pressed、Disabled；当前 Theme 和覆盖颜色。
- 自定义 Node / Children 与 Foreground 内容。

## Stimuli

- Scene 构造、Pointer 输入、添加 / 移除交互 Component、Theme 切换与 Override。

## Guards

- Disabled 阻止用户 Activation；样式不改调用方 Layout 或 Children。

## Transitions

- Normal ↔ Hover ↔ Pressed ↔ Disabled。
- 解除 Disabled 后正常恢复 Hover / Pressed 或 Normal。

## Invariants

- 优先级为 Disabled > Pressed > Hover > Normal。
- 真实输入可触发官方 `Activate`，被禁用时不触发。
- Foreground 传到受管理内容，Theme 更新不改业务 Structure。

## Couplings

- 颜色 Override × 当前 Theme / 交互状态。
- Button 容器与 Text / Icon Children 的同帧配色传播。

## Coverage Map

- `widgetry_button.rs`：基础状态、样式转换、Override、Theme、Scene Patch、内容传播与 Pointer Activation。
- 源码 Unit Test 保护更局部的样式优先级规则。

