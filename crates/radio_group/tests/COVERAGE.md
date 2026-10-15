# radio_group Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 非空固定 Option 集合、当前 Selected Index、Group Focus / Disabled。
- 各 Option Checked 和交互 / Theme 配色。

## Stimuli

- BSN 构造、Pointer / Keyboard、程序化 Set、Theme、Disabled / Enabled。
- 延后父子挂接、ChildOf 替换、Group / Option 标记移除和恢复、Root 回收。

## Guards

- Option 必须是 Group 的直接 Child；Index / Root 有效。
- 同值程序化 Set 不产生用户业务 Event。

## Transitions

- 初始化选择首项；输入切换选项；程序 Setter 修改 Checked 投影。
- Disabled / Re-enabled 的用户输入边界。

## Invariants

- 一个 Group 恰有一个 Checked；不同 Group 的选择相互独立。
- 程序更新静默；来自用户的通知与实际选择一致。
- 非法程序请求在队列执行时以 Severity::Error 传播并记录 Root / Index ERROR，不部分修改 Checked 或初始化状态，不阻止同批有效操作。
- Theme / Layout Patch 不改变选项顺序与业务状态。
- 结构校验失败以 Severity::Error 传播，不同有效 Group 不受阻碍；静态故障不逐帧重验或重复记录，恢复与再次故障按真实状态转换记录。
- 已初始化 Group 的结构恢复保留原选择；Root 回收不伪造恢复日志或业务通知。

## Couplings

- Group Disabled × Children 请求。
- Focus × Keyboard，Theme × Option Foreground / Background。

## Coverage Map

- `widgetry_radio_group.rs`：初始化、交互、场景组合、程序设置、颜色与失效结构诊断。
- 程序设置覆盖未初始化 / 已初始化 Group、合法同值与不同值、Disabled、越界、错误类型或已销毁 Root，以及排队后类型 / 层级失效。
- 结构生命周期覆盖延后挂接、替换旧父级和新父级、ChildOf 不变时移除 Group 标记、Option 标记恢复及持续故障后的 Root 回收。内部 Unit Test 另保护稳定帧零结构校验、变化仅影响对应 Group，以及后安装 Plugin 的一次性补偿。

## 覆盖边界

动态增删 / 重排 Option 不属于当前公开行为，测试针对构造期固定结构。

