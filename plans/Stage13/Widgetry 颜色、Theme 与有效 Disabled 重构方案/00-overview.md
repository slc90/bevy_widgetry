# Widgetry 颜色重构：执行总览

本套方案把 Theme 独立、有效 Disabled、显式颜色覆盖、有限的前景色继承和 Gallery 展示整理成一条执行链。当前源码基线为 `slc90/bevy_widgetry@5d33dd561a3c9bb171d7dd3238a41903a0724e33`，目标环境保持 Windows 64 位 / Bevy 0.19.1。

[完整修改总方案](../widgetry-color-refactor-master.md) 保留整个设计；以下文件是可按顺序使用的独立小方案。每份都包含本阶段的设计正文、约束和验收条件。

## 唯一执行顺序

**01 Theme → 02 有效 Disabled → 03 颜色接入 → 04 Gallery → 05 全链验收**

| 顺序 | 小方案 | 负责的结果 |
|---|---|---|
| 01 | [建立独立 Widgetry Theme 与完整 AntD 参考配色](01-theme-and-palette.md) | 新 theme crate、完整控件/部件/状态颜色结构、Light/Dark 固定值、命名和主题通知。现有引用随新数据结构一起迁移。 |
| 02 | [建立有效 Disabled 继承并接通真实输入禁用](02-effective-disabled.md) | 分开本地禁用原因和继承结果，统一官方 InteractionDisabled 的有效投影；保留子级自身禁用并阻止真实输入。 |
| 03 | [接入统一颜色覆盖、前景色内容管理与全部控件](03-color-resolution-and-widget-integration.md) | 覆盖 API、WidgetryText/Icon、前景色作用域、组合部件归属和各控件 resolver/apply 一起接入。旧颜色通道和相关消费者在同一份方案中退出。 |
| 04 | [在现有 Gallery 展示完整主题色与真实覆盖效果](04-gallery-color-showcase.md) | 沿用已有页面与主题切换，增加完整色块目录、确定的局部覆盖/清除示例，以及有效禁用的真实演示。 |
| 05 | [完成 workspace 迁移收尾与全链行为验收](05-migration-and-verification.md) | 核查旧 API/writer 残留，完成行为矩阵、性能回归、Windows/BRP 运行验证和独立审查。 |

Theme 数据与 Disabled 的计算没有算法依赖，前两份的顺序是统一迁移顺序；颜色接入同时消费两者。Gallery 必须使用已经接通的公开颜色 API，不能用演示代码补库内部尚未完成的行为。最终验收不替代每份小方案的局部验证。

颜色覆盖、内容继承和所有消费端没有拆成多份半成品：它们必须共同建立唯一的颜色 writer，才能避免阶段交付时出现固定颜色失效或文字不跟随状态的问题。

本包仅包含修改方案。仓库未修改，编译、测试和 GUI 验证仍是实施交付条件。
