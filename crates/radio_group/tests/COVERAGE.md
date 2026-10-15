# radio_group Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 非空固定 Option 集合、当前 Selected Index、Group Focus / Disabled。
- 各 Option Checked 和交互 / Theme 配色。

## Stimuli

- BSN 构造、Pointer / Keyboard、程序化 Set、Theme、Disabled / Enabled。

## Guards

- Option 必须是 Group 的直接 Child；Index / Root 有效。
- 同值程序化 Set 不产生用户业务 Event。

## Transitions

- 初始化选择首项；输入切换选项；程序 Setter 修改 Checked 投影。
- Disabled / Re-enabled 的用户输入边界。

## Invariants

- 一个 Group 恰有一个 Checked；不同 Group 的选择相互独立。
- 程序更新静默；来自用户的通知与实际选择一致。
- Theme / Layout Patch 不改变选项顺序与业务状态。

## Couplings

- Group Disabled × Children 请求。
- Focus × Keyboard，Theme × Option Foreground / Background。

## Coverage Map

- `widgetry_radio_group.rs`：初始化、交互、场景组合、程序设置、颜色与失效结构诊断。

## 覆盖边界

动态增删 / 重排 Option 不属于当前公开行为，测试针对构造期固定结构。

