# Radio Group

## 功能概述

提供一组固定互斥选项的 `WidgetryRadioGroup` / `WidgetryRadioOption`，适合模式切换与单选设置。

## 当前设计思路

RadioGroup 管理一个固定的直接 Children 选项集合，使用 Bevy 官方 `Checked` 投影互斥选择。控件由 BSN 构造，不为重复 UI 内容另建平行构造方式。

Option 配色只响应 Checked、Hover、有效 Disabled、父 / 子颜色覆盖、主题和必要的部件生命周期变化。单 Option 输入只刷新对应项，Group 颜色变化刷新本组，稳定状态不重复解析全部选项。移除 Hover 按未悬停处理；Indicator / Dot 构造、延后挂接和输出修复在首次有效颜色阶段重新刷新。Group Focus 继续只使用既有容器样式，不为 Option 增加新的视觉状态。

## 主要功能

- 初始化时选择第一项。
- Pointer 点击选项，Focus 后使用方向键切换。
- 用户选择通过 Group Source 上的 `ValueChange<usize>` 通知。
- `set_selected` 按 Index 进行程序化选择。
- Root Disabled 继承到选项，Theme / Focus / Hover / Pressed 更新外观。

## 行为与状态契约

Group 至少要有一个 Option，每个 Option 必须为 Group 的直接 Child。Index 对应创建时 Children 的固定顺序，运行时不支持随意重组选项。一个 Group 保持恰有一个 Checked；程序化 `set_selected` 不发用户选择通知，Theme 更新也不触发业务变化。不同 Group 保持独立 Selection。

## 边界情况与错误处理

`set_selected` 在 Commands 实际执行时检查 Root、非空的直接 Option 集合与 Index。无效 Root（含已销毁或错误类型）、缺失或不合法的 Children、越界 Index 均记录带 Root / Index 的 ERROR，并以 `Severity::Error` 交给宿主 Error Handler。失败请求不修改 Checked 或初始化状态，也不发送业务通知；同批队列中其他合法 Group 的操作仍可执行。合法同值选择成功且静默。

某个不完整 Option 的错误不应阻止其他有效实例更新。Disabled 阻止用户修改，但保留合法程序化设置能力。

Option 样式诊断将父级颜色配置缺失与 Indicator / Dot 结构失效统一交给该 Option 的 FailureState。相同持续故障只记录一次 ERROR，不同故障重新记录；实际恢复后记录一次 INFO，恢复后再次失效仍会报告。日志包含 Option 与 Group，多个 Option 独立记录。日志去重不减少错误传播：每次实际样式调用失败仍以 `Severity::Error` 交给宿主，正常且未曾失败的实例不输出恢复日志。

样式故障后保留诊断状态，由后续输入、父配置恢复或部件修复信号重新校验；不以逐帧全量重试诊断故障。单个失效 Option 不阻止其余待更新实例。

结构诊断在 Group / Option 首次出现、相关父子关系变化或标记失效后，于下一次正常输入派发前执行。构造期延后挂接可在后续结构变化时恢复；已初始化实例重新校验时保留现有 Checked，不重新选择首项。稳定结构不逐帧重验，同一持续故障只记录一次 ERROR，实际恢复后记录一次 INFO，再次失效重新记录。每次实际校验失败仍以 `Severity::Error` 传播；不因异常持续而重复执行校验。Root 回收不产生恢复通知。程序化请求继续在执行路径独立检查当前结构。

## 已知限制

- 选项集合设计为构造期固定结构，不提供动态增删 Option API。
- Tab Navigation 由调用方在适当祖先设置 `TabGroup`。
- `ValueChange` 的 Index 身份取决于 Option 的固定 Children 顺序。

