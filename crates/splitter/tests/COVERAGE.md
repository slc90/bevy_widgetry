# Splitter coverage

`layout.rs` 使用真实 Bevy UI Layout 与 BSN Scene，覆盖主轴方向、Pane 本体 Patch、默认及初始化 Clamp、双侧 min/max 与根范围、单侧无上限、父级溢出、被动 Resize 目标保持、嵌套独立性和错误通路。状态维度为方向、保存比例、初始化阶段与原生约束；输入包括构造、父级尺寸及 Pane Node 变化。初始化保持通知静默。

`constraints.rs` 覆盖百分比约束与逻辑像素/Scale Factor 的耦合，以及外部 ScrollArea 包装最小尺寸溢出的真实布局；ScrollPosition 与 Overflow 保持其原生职责。动态给两个百分比最小尺寸 Pane 设置像素最大值时，先应用根范围再校验收敛后的单侧限制，合法更新不进入 error handler。

持续约束错误时，等待根重排保留诊断状态，不虚发恢复日志；只有实际有效布局恢复后才记录一次恢复。

GUI 视觉、外部 ScrollArea 滚动由任务级 BRP 验证；不将合成 ComputedNode 或测试通过当作 GUI 证据。

`box_layout.rs` 保护分数 min 尺寸及 1.0/1.25/1.5 Scale Factor 下的首次比例修正，横/纵向 ContentBox Pane 与根的 padding/border 只计入一次，并验证 Bevy 原生 border 像素吸附后的双侧最大范围。百分比 padding 保留主轴比例关系，根范围稳定后才提交初始化比例，避免临时尺寸永久改变初值修正结果。
