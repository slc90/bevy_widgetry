# Stage12 规则与控件修改方案总览

## 整体目标

固化已确认的 Widget 输入 / 输出契约，按控件完成自有 state 的公开更新、只读查询、提交后通知、清空和错误语义迁移；直接复用官方 Component 保持原样。

本目录依据 Stage12 输入 / 输出盘点和已确认的规则共识整理，只交付方案，尚未实施 AGENTS、rules 或源码。原始盘点的现状、已确认规则、例外及未决前提均在各小方案正文保留，不能把规划或旧盘点视为已完成的运行时验收。

## 唯一执行顺序

规则修改 → Button → 二态 CheckBox → 三态 CheckBox → RadioGroup → TextField → ReadOnlyTextField → ScrollArea → ListView → ComboBox → Tree → Table → Waveform → Tooltip → Window → MessageBox → Icon。

| 顺序 | 小方案 | 内容概述 |
| --- | --- | --- |
| 01 | [规则修改](01-固化Widget-API规则与开发入口.md) | 新增 Widget API 规则并补充开发入口，澄清 ECS 直接操作和测试示例的范围。只固化已确认契约，保留未决实施设计。 |
| 02 | [Button](02-Button-保留官方激活契约.md) | 明确 Activate 是操作通知，保留官方输入与 Component 契约。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 03 | [二态 CheckBox](03-二态-CheckBox-保留-Checked-与官方变化路径.md) | 明确 Checked 的官方例外范围，保留现有程序输入与通知。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 04 | [三态 CheckBox](04-三态-CheckBox-统一-state-提交与变化通知.md) | 使 UI 与程序更新都在 WidgetryCheckState 提交之后通知，并让公开 Component 查询只读。 迁移 API 与通知时同步直接消费者。 |
| 05 | [RadioGroup](05-RadioGroup-明确官方-Checked-与-index-API-边界.md) | 保留官方 Checked authority，明确固定 index 输入和多层通知的现有契约。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 06 | [TextField](06-TextField-明确-EditableText-与-TextEditChange-语义.md) | 保持官方文本机制，避免把 TextEditChange 写成严格的文本 value changed。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 07 | [ReadOnlyTextField](07-ReadOnlyTextField-明确只读输入过滤边界.md) | 明确 read-only、disabled 和程序修改的边界，保留官方通知。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 08 | [ScrollArea](08-ScrollArea-保留官方滚动-state-与请求机制.md) | 明确 ScrollPosition 查询与 ScrollIntoView 请求，保留官方滚动行为。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 09 | [ListView](09-ListView-统一只读-state-与程序选择通知.md) | 收敛自有 selection / active 的公开更新入口，使程序选择和显式清空在 state 提交后通知。 迁移 API 与通知时同步直接消费者。 |
| 10 | [ComboBox](10-ComboBox-统一-root-选择与清空通知.md) | 使 UI 与程序选择 / 清空都在内部 authority 提交后向 root 通知，并保留各路径 Popup 附带行为。 迁移 API 与通知时同步直接消费者。 |
| 11 | [Tree](11-Tree-统一-Model-选择通知与无效目标错误.md) | 使 Model UI / 程序 selection 和清空提交后通知，区分非法目标与合法无变化。 迁移 API 与通知时同步直接消费者。 |
| 12 | [Table](12-Table-统一选择通知与-resize-结束取消语义.md) | 统一自有 state 更新的已提交变化通知，区分 resize 正常结束与取消。 迁移 API 与通知时同步直接消费者。 |
| 13 | [Waveform](13-Waveform-明确外部输入与已提交-runtime-边界.md) | 明确数据输入和已提交结果如何适用自有 state 规则，保留数据读取及失败契约。 先明确受影响的必要边界，不擅自添加新通知。 |
| 14 | [Tooltip](14-Tooltip-保留内部-hover-lifecycle-契约.md) | 保持内部 hover lifecycle，明确没有公开显示控制或完成通知。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 15 | [Window](15-Window-保留-native-state-与-resize-Message.md) | 保持 native Window 官方输入输出，明确它与 Table gesture 的不同契约。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 16 | [MessageBox](16-MessageBox-保留一次性结果与关闭顺序.md) | 明确既有先决议再通知，保留一次性结果及资源关闭顺序。 以契约澄清和保持边界为主，不人为增加行为修改。 |
| 17 | [Icon](17-Icon-明确-Widget-API-与异步显示边界.md) | 明确自有 Component setter 如何适用只读查询，并保持异步 asset / 颜色行为。 先明确受影响的必要边界，不擅自添加新通知。 |

## 前后关系

规则方案为全体 Widget 的约束来源。ListView 的 state / event 是 ComboBox 与 Tree 的实际基础；触发公共 API 变更的方案同时完成消费者的必要适配，避免直到后续方案才能恢复编译。Window / Button 提供 MessageBox 的现有窗口与激活基础；其他相邻方案无技术硬依赖处，仅采用上面的统一顺序。

各方案单独保留范围、具体行为、例外、缺口、验证意图、预期产出和前后关系。实施前按其必要前提明确 API / payload；不因为已有拆分文档就宣称 Stage12 全部完成。
