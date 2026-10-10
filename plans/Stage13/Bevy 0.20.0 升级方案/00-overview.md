# Bevy 0.20.0 升级方案总览

把 bevy_widgetry 全工作区升级到 Bevy 0.20.0，保持现有控件与 Windows 桌面行为。按下方唯一顺序执行，每份方案完成后整个工作区都必须能够编译；具体范围、依赖、迁移说明和验收条件直接写在各份方案中。

## 执行顺序

1. [全工作区切换到 Bevy 0.20 并恢复完整构建](01-workspace-cutover.md)

   依赖、宏语法和所有硬性 API 修复一起交付，不留下“版本改完但其他 crate 暂时不能编译”的中间成果。

2. [对齐 BSN 表达与 Scene 构造契约](02-scene-contracts.md)

   保留 SceneComponent、模板上下文和实体引用语义，检查失败回收与错误传播。

3. [对齐 Pointer 事件与交互状态生命周期](03-pointer-lifecycle.md)

   核对事件身份、目标和按键所有权，覆盖取消、失效、禁用和跨窗口输入。

4. [恢复 TextField、焦点与多窗口 IME 的完整行为](04-text-input.md)

   处理 TextInput、编辑视口、只读/禁用守卫、Tab/Escape，以及多窗口键盘与 IME 路由。

5. [对齐 UI 布局、颜色与增量更新时序](05-ui-layout.md)

   检查调度依赖、字体与尺寸单位、裁剪、滚动收敛，以及 retained UI 下的变化跟踪。

6. [验证 Windows 渲染、Waveform 与 BRP 运行链路](06-window-rendering.md)

   检查透明合成、首帧呈现、窗口资源归属、内建材质 Waveform 和 BRP v0.4.0 接入。

7. [同步当前说明并完成全仓验收](07-docs-and-acceptance.md)

   更新当前版本与 MCP 使用说明，保留历史计划原文，完成所有 Cargo 目标与真实 Gallery 的最终检查。

第一份方案的范围较大，因为根依赖切换会同时影响多个 crate、测试工具和 Gallery，不能把编译断开的状态当成一个小方案的完成结果。后续方案分别验证更深的行为契约，不是为前面补交编译修复。

本次不设置基线记录阶段，也不修改仓库。文档给出的是实施与验收要求；尚未实际执行 Windows 构建或 GUI 验证。
