# radio_group Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 动态 Option 集合、有效非空 / 空集合错误与恢复、当前 Selected Index、Group Focus / Disabled。
- 各 Option Checked 和交互 / Theme 配色。

## Stimuli

- BSN 构造、Pointer / Keyboard、程序化 Set、Theme、Disabled / Enabled。
- 延后父子挂接、ChildOf 替换、Group / Option 标记移除和恢复、Root 回收。

## Guards

- Option 必须是 Group 的直接 Child；Index / Root 有效。
- 同值程序化 Set 不产生用户业务 Event。

## Transitions

- 初始化选择首项；输入切换选项；程序 Setter 修改 Checked 投影。
- 增删保留选中实体；选中首 / 中 / 末项删除及连续删除按旧顺序选后继 / 前驱，批量删除与脱离层级同样修复；空集合诊断去重后恢复首项。
- Disabled / Re-enabled 的用户输入边界。

## Invariants

- 一个 Group 恰有一个 Checked；不同 Group 的选择相互独立。
- 程序更新静默；来自用户的通知与实际选择一致。
- 非法程序请求在队列执行时以 Severity::Error 传播并记录 Root / Index ERROR，不部分修改 Checked 或初始化状态，不阻止同批有效操作。
- Theme / Layout Patch 不改变选项顺序与业务状态。
- 父 / 子颜色覆盖更新与清除遵守局部优先级，选择更新后前景传播至实际 Label；其他 Group 不受影响，Focus 转移不改变 Option 调色语义或产生业务通知。
- 结构校验失败以 Severity::Error 传播，不同有效 Group 不受阻碍；静态故障不逐帧重验或重复记录，恢复与再次故障按真实状态转换记录。
- 已初始化 Group 的结构恢复保留原选择；Root 回收不伪造恢复日志或业务通知。

## Couplings

- Group Disabled × Children 请求。
- Focus × Keyboard，Theme × Option Foreground / Background。
- 动态结构 × Checked 身份与通知静默、Disabled 替补、命令执行时索引。

## Coverage Map

- `widgetry_radio_group.rs`：初始化、交互、场景组合、程序设置、颜色与失效结构诊断。
- 动态选项测试覆盖选中项删除与批量删除、追加 Checked 项不抢选且不重写既有 Checked、移除未选项保持身份、清空恢复与诊断、禁用替补、双 Group 隔离和排队删除后的索引选择。临时 Checked 项新增后立即删除不覆盖原删除位置；旧项全部替换或清空后恢复时，新项的初始 Checked 不覆盖默认首项规则。
- 程序设置覆盖未初始化 / 已初始化 Group、合法同值与不同值、Disabled、越界、错误类型或已销毁 Root，以及排队后类型 / 层级失效。
- 结构生命周期覆盖延后挂接、替换旧父级和新父级、ChildOf 不变时移除 Group 标记、Option 标记恢复及持续故障后的 Root 回收。内部 Unit Test 另保护稳定帧零结构校验、变化仅影响对应 Group，以及后安装 Plugin 的一次性补偿。
- 内部样式 Unit Test 保护稳定帧零 Option 配色、Hover 局部刷新与移除、父配置缺失后的真实恢复、Dot 输出修复与 Indicator 新建后延后挂接；持续故障不靠逐帧重试，修复及再次故障均保留诊断语义。

## 覆盖边界

运行时任意重排 Option 不属于当前公开行为。动态增删通过 BSN 构造与正常 ECS 层级 / Despawn 操作验证；空集合恢复同时覆盖新建与复用原 Option，两者都重置为首项。

