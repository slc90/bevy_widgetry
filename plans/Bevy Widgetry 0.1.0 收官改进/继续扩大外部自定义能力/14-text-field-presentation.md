# 开放 TextField 可选自动高度适配

在公开 Editor 绑定上提供可选的 AutoSize 适配能力。自动高度依赖真实文字排版，但不应强制所有 TextField Headless 使用默认 Frame、默认根布局或默认行数策略。

本增量直接依赖上一份 Editor 绑定。下一份 NumberInput 使用基础编辑能力，不要求开启 AutoSize；二者的相邻顺序不代表 NumberInput 必须自动增高。

```mermaid
flowchart LR
    E["公开绑定的 Editor"] --> P["官方最新排版"]
    P --> A["可选 AutoSize：视觉行数夹取"]
    A --> L["Editor visible_lines"]
    L --> C["官方 ContentSize 与 viewport"]
    S["默认或自定义布局"] --> E
    H["计数 / 提示 / 装饰槽"] --> S
```

## 测量编辑区，不测整个控件

AutoSize 继续从 Parley 最新排版读取视觉行数，包含软换行和预编辑排版。依据 min_rows/max_rows 夹取，更新真实 Editor 的 visible_lines，让官方 ContentSize 决定编辑区高度；超过上限沿用官方 viewport 滚动。

默认配置仍是 2–6 行，要求 multiline=true 且 `1 <= min_rows <= max_rows`。未启用 AutoSize 时保留单行或固定多行高度语义。配置仍只在构造时确定，不增加运行期模式编辑器。

可选适配器只管理它明确绑定的编辑区域。计数、提示、Prefix/Suffix 或外部标题的高度不计入编辑行数；较高装饰槽不拉伸真实 Editor。文本、宽度、槽宽、字体和行高变化后依正常帧调度收敛，不用反复重建 Editor。

## 与任意外部布局组合

移除 AutoSize 对私有 `Parts.frame` 的定位依赖，测量目标使用公开 Editor 绑定。外部 Scene 可以把计数放在标题旁、把提示放在根外的装饰布局中，但这些不会成为自动高度计算的输入。

默认根 `Node.height=Auto` 的约束继续适用于默认完整 Style；自定义结构的高度约束应由选择 AutoSize 的适配器明确说明，不能无条件向任意 Headless 根写 Node 几何。没有挂 AutoSize 的自定义控件不受默认自动布局系统扫描或修改。

计数与提示不是本增量新增的状态能力。沿用上一份方案中的公开正文数据和默认 Style 投影：默认计数仍在边框外下方右对齐，提示在 Frame/计数之后；自定义 Style 可以自行展示派生内容。

## 默认行为与实现范围

重点是 `crates/text_field/src/auto_size.rs`、相关注册与默认 Scene 组合。把独立编辑能力和可选布局适配区分开，而不是把自动高度整段删掉后要求外部重写。

保留官方编辑完成后的排版读取时机、默认根的固定高度冲突诊断、恢复 Auto 后继续伸缩的行为。Theme 只改变颜色，不能导致文字丢失、选区重置或 AutoSize 从头初始化。

## Gallery 与完成条件

在 TextField 页自定义示例中增加可伸缩多行区：窄宽度触发软换行，旁边放一个较高的自定义装饰节点，验证只测量 Editor。保留原有默认单行、多行、计数、提示示例，不改变它们的排列。

- 空文本、软换行、IME 预编辑、长提示、宽度改变、字体/行高改变分别验证；编辑行数在配置范围内，超过上限改用滚动。
- 没有 AutoSize 的 Headless 控件不被改高度；启用的自定义控件不需要默认 Frame。
- 默认根固定高度冲突首次报错、持续时不重复，恢复 Auto 后继续工作；计数/提示高度不混入测量。
- 四套主题下默认 Frame、辅助文本和整体高度与原表现一致，真实编辑状态保持不变。

## 边界

AutoSize 是可选布局能力，不是新的文本模型。错误绑定、重复 Owner 或缺官方排版资源继续返回领域诊断；正常等待排版不伪装成错误。不扩展为任意布局引擎或尺寸动画系统。
