# 移除 HoldButton 的自定义无障碍节点

`WidgetryHoldButtonHeadless` 通过自己的按压时长、状态机及完成阈值决定何时发送 `Activate`。角色为 Button 的 `AccessibilityNode` 是附加语义，与计时、Pointer 命中、键盘确认及状态回退并非同一职责。

## 源码调整

`crates/hold_button/src/headless.rs` 的 `WidgetryHoldButtonHeadless` Required Component 中显式构造了 `bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::Button))`。移除这一声明，只保留 `Node`、`Hovered`、默认 `TabIndex(-1)`、`WidgetryHoldButtonState`、`WidgetryHoldButtonProgress` 与 `Timing` 等原有必要组件。

在 `crates/hold_button/Cargo.toml` 删除 `accesskit.workspace = true`。`WidgetryHoldButtonHeadlessPlugin` 与默认 Style 仍使用原输入和状态模型，既不增加官方 Button 替代输入，也不新增项目特定的 Agent 语义层。当前功能文档没有要求手写无障碍节点，无需为纯内部移除额外创造说明。

## 行为和验证

保留既有 Integration Test 与内部状态机测试，覆盖 Mouse / Custom Pointer 圆形命中、短按拒绝、达到阈值立即且只触发一次、释放重按、拖出取消/重新进入守卫、继承 Disabled / Visibility、聚焦的 Enter / Space 和按键重复限制。测试只关注可观察的时长、状态、事件与输入行为，不增加 AccessKit 断言。

如调整引起实际 GUI 风险，使用 BRP 验证 Press / Release 与长按成功时的视觉反馈，不能将程序化触发 `Activate` 当作真实长按的替代。

## 边界

不移除 `TabIndex` 或 Focus 支持；新的 Accessibility 实现和额外插件均不在范围内。与前后方案仅为清理顺序相邻，没有实现依赖。
