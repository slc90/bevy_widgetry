# Widgetry Table 分方案总览

## 整体目标

以两个内部 Axis、Cell Data Projection、Type Driven Renderer、2D Virtualization 和 Table-specific Interaction 建立 data driven、headless 的通用二维数据展示控件。外部保持 WidgetryTable<T>、BSN 与 SceneComponent 的使用方式。

本目录是补充测试后的 WidgetryTable 设计方案的完整内容拆分，不执行方案。每份小方案直接承载所需设计正文、测试场景与未决前提，原方案中的具体说明不由总览代替。

## 唯一执行顺序

Model 与数据投影 → Renderer → View 与 Layout → 二维 Virtualization → Interaction → Gallery。

1. [Model 与二维 Cell 数据投影](01-model-and-cell-data-source.md)：建立两个内部 Axis、Model identity / revision 和 ID pair 查询，保留数据 API 与失效处理的未决前提。
2. [异构 Cell 与 Header Renderer](02-cell-and-header-renderers.md)：建立彼此隔离的 Cell / Header registry，保留异构内容、注册示例、Content 职责及缺失注册等测试前提。
3. [BSN View、四区 Layout 与 Style](03-view-layout-and-style.md)：建立 BSN source 接入、四区滚动与区域 style，保留 shell / Content 分工及 width、source、ownership 的未决前提。
4. [二维 Viewport 与 Cell 生命周期](04-two-dimensional-virtualization.md)：建立二维可见 Cell 集合与完整生命周期，保留直接 Cell entity 层级、边界、数据刷新和状态投影测试。
5. [Selection、Focus 与 Table Interaction](05-selection-focus-and-interaction.md)：建立单一 selection、四方向 focus、整体 disabled 和选择 / resize events，并明确尚未定义的 guard、通知与修复语义。
6. [Gallery 展示与 BRP 运行时验收](06-gallery-and-runtime-validation.md)：保留全部 Demo 内容，按 Gallery 例外进行编译与 BRP 实际输入验收，关注二维 scroll 和动态 Content 的中间表现。

## 前后关系

Renderer 消费 Model 的类型值；View 组合 Model 与 renderer，并提供 viewport / layout；Virtualization 在其上维护二维 Cell；Interaction 将实际输入绑定到当前 logical identity；Gallery 消费完整公共能力进行运行时验收。前面小方案保留的跨领域 case，在依赖能力到位后由其主要 owner 验证完整 contract，不因此提前扩展该方案的实施范围。
