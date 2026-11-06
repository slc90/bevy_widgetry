# 开放 ComboBox 搜索组合

将 ComboBox 的本地搜索开放为可选组合能力，通过公开绑定引用 TextField 编辑区域及现有 ListView。外部可以把搜索框和无结果提示安排在自己的 Popup 结构中，不必使用默认的直接 Children 布局。

本增量依赖上一份 ComboBox 部件/生命周期，以及公开 TextField 编辑能力。下一份 DatePicker 不依赖搜索，只是顺序相邻。

```mermaid
flowchart LR
    E["绑定的 Search Editor"] --> Q["唯一查询正文"]
    Q --> M["外部 Matcher"]
    S["共享原始 Model"] --> M
    M --> P["实例自己的 Stable ID Projection"]
    P --> L["同一个 ListView"]
    L --> A["Active 候选"]
    A --> C["确认后提交既有单选状态"]
```

## 搜索改变投影，不复制数据或选择

保留 `WidgetryComboBoxMatcher<T>` 的查询与原始条目匹配能力。每个实例更新自己的 Stable ID Projection，不复制或修改原 Model，也不影响共享数据的其他实例。

查询正文由绑定 TextField 的 EditableText 唯一持有。可以保留用于差异检测的缓存和待处理动作，但不能把另一个 String 作为第二个可变正文权威。公开 search_state 从真实状态读回查询、匹配数量和 Active。

过滤隐藏已选项时，Field 与正式 Selection 保留；真实删除才静默修复为空。每次打开清空查询，空查询恢复完整结果，聚焦搜索输入。Active 是候选导航状态，不等同已提交选择。

## 输入路由和关闭约束

搜索模式中的 Up/Down、Home/End 导航匹配且未禁用候选，Enter 确认，Escape 取消。Left/Right、删除、文字选择及 IME 沿用 TextField；导航键不再同时执行光标移动，Ctrl/Shift 不改变单选语义。

保留目前按编辑队列顺序处理搜索输入的方式，使结果行能赶在 UI Build/Layout 前创建。若需要提前使用官方 apply_pending_edits，应通过公开的受控编辑能力协作，不能让 Gallery 调用 TextField 私有函数；后续官方阶段仍负责 TextEditChange、排版和光标。

关闭或禁用时阻断隐藏输入和同批剩余确认，清理正确的焦点归属。无结果时搜索仍可编辑，不能因列表隐藏而把搜索框也变成不可用。确认后的选择和关闭继续走上一份的生命周期，不能再直接修改 Popup Visibility 绕过逻辑打开状态。

## 搜索框与提示不规定视觉布局

Headless 不要求 SearchField 是 Popup 的直接 Child，也不要求无结果提示存在固定标记、颜色或高度。显示 no_results、search_prompt 的 Scene Factory 与位置属于默认 Style 或外部 Style。

默认搜索区仍高 40px，列表按匹配行数和 max_visible_items 计算高度，输入框与无结果内容不占数据行配额。默认无结果提示与 Field Placeholder 独立；过滤和恢复后行背景/点击区保持撑满 Popup 内宽。默认颜色托管和整体外观不变。

## Gallery 与完成条件

在 ComboBox 自定义示例中加入不同位置的搜索区域和自定义无结果提示；继续使用同一 ListView 引擎，不制作新的选择列表。

主要处理 `crates/combo_box/src/search.rs`、默认搜索 Scene、typed 注册以及 TextField 公开交互适配。

- 两个 ComboBox 共享 Model 时查询互不影响，过滤隐藏选中项不清空 Selection，真实删除正常修复。
- 打开清查询、Active 导航、Enter/Escape、无结果继续编辑、IME 与同帧关闭后确认都有覆盖。
- 外部搜索区多层嵌套、默认与自定义 TextField 都能通过绑定工作，不要求私有 Frame。
- 默认搜索布局、行配额、无结果高度与四套配色一致；查询变化不重建无关数据，关闭后无隐藏输入。

## 范围

不新增远程搜索、异步加载、自由文本提交或多选 ComboBox。本增量仍是原 Model 上的本地单选过滤，沿用既有诊断、队列顺序和 typed 注册幂等性。
