# bevy_widgetry · Pointer 方案总览

让普通 Widget 的输入、hover 和终止处理统一到 Pointer，同时保留真实鼠标体验和原生 Window 操作。

[完整总方案](../master-plan.md)保存本次设计正文与源码依据。以下小方案按唯一顺序执行；详细约束、实现边界和验收内容均在各自文件中。

## 执行顺序

1. [建立 Mouse 与 Custom 共用的交互测试基线](01-双身份交互测试基线.md)

   建立 Mouse 与 Custom 共用的行为断言和真实 Picking 测试基础，记录尚未修复的差异。 可复用的双身份测试入口、真实命中 fixture 和有证据的基线。

2. [在 core 中统一 Hovered 和 DirectlyHovered 的 Pointer 语义](02-共享Hover状态Pointer化.md)

   在 core 为官方 Hovered/DirectlyHovered 提供唯一的通用 Pointer 写入者。 共享适配插件、独立 Widget 装配覆盖、Changed 安静性及多种命中条件测试。

3. [让 Tooltip 跟随有效 Pointer，而不是固定 Mouse](03-Tooltip有效Pointer生命周期.md)

   让单个 Tooltip 跟随有效 Pointer，而不是固定读取 Mouse。 设备无关的 Tooltip 状态机、原有时间边界与交接回归。

4. [补齐普通控件的按下、拖动和取消闭环](04-普通控件交互终止闭环.md)

   补齐经测试证实的按下、拖动、取消和指针失效边界，同时保留正常业务行为。 按会话 ownership 幂等收尾的普通交互，以及每个生产修改对应的失败用例和回归证据。

5. [接入新版 BRP 并完成 Gallery 真实交互交接](05-新版BRP接入与Gallery验收.md)

   将固定新版 BRP 接入 Gallery，验证真实鼠标、Custom 与串行交接的完整体验。 一致的消费版本、真实 GUI 与人工原生记录，以及准确的已测/未测说明。

## 跨仓库位置

先执行 BRP 01 → 02 → 03 → 04 → 05 → 06，再执行 Widgetry 01 → 02 → 03 → 04 → 05。Widgetry 前面的本地适配并不技术依赖 BRP 发布，这里给出的是统一执行顺序；最终 Gallery 接入才要求固定的 BRP 版本产物。
