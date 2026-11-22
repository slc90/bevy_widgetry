# 实现 Canvas Layer 管理

## 目标

在 `CanvasLayerStack`
上实现完整的基础图层操作：创建、删除、重命名、排序、显隐、锁定，以及将绘图项移动到其他
Layer。当前活动 Layer 属于 View，不属于共享 Document。

## Layer 是 Document 数据，活动 Layer 是 View 状态

每个 Layer 使用稳定 `CanvasLayerId`，保存名称、可见性、锁定状态和直属
Item 顺序。Document 的 LayerStack 顺序决定图层从底到顶的绘制顺序。Layer
不需要独立 ECS Entity，也没有独立 Transform。

`ViewState` 增加 `active_layer: Option<CanvasLayerId>`。两个 CanvasView
可以共享同一份 Document，但分别在不同 Layer 中绘制。切换 Document
时，View 校验活动 Layer；若旧 ID 不属于新
Document，则选择第一个可见且未锁定的 Layer。

新建绘图工具提交到当前 View 的活动 Layer。现有
`CanvasDocument::create_rectangle` 等无目标 Layer 的便捷 API
继续写入默认 Layer；增加显式 Layer 版本供程序化操作使用。

## 操作规则

-   **创建**：在指定索引插入 Layer，生成新的稳定 ID，默认可见且未锁定。
-   **重命名**：只修改名称，不改变 Layer ID、顺序和内容。
-   **排序**：调整 LayerStack 中的位置；所有 View 观察同一新顺序。
-   **隐藏**：隐藏 Layer 后，其所有 Item 不参与
    Renderer、Picking、对象吸附和 SVG/PNG 导出。重新显示后恢复原内容。
-   **锁定**：Layer 保持可见并继续参与导出，但其中 Item
    不能被用户选择或通过交互修改。锁定是交互 Guard，不禁止调用方通过
    Document API 进行明确的程序化修改。
-   **移动 Item**：将 Shape 或 Group 从一个 Layer 移到另一个 Layer，保留
    Entity 身份、局部数据和相对顺序。Group 的后代仍归属于该
    Group，不逐个迁移。
-   **删除**：不能删除最后一个 Layer。删除非空 Layer
    时必须显式提供处置策略：`MoveContentsTo(target_layer)` 或
    `DeleteContents`。迁移目标必须属于同一
    Document，且可见、未锁定；不允许静默丢弃内容。`DeleteContents`
    是明确的级联删除，并作为一个 Undo 事务提交。

隐藏或锁定 Layer 时，View 会清理该 Layer 中的选择成员；如果当前活动
Layer 被隐藏或锁定，切换到第一个可见且未锁定的 Layer；不存在可用 Layer
时活动 Layer 置空，绘图工具不能提交新内容，直到用户选择可用 Layer。

## 顺序与渲染

Renderer、命中检测、对象吸附和导出使用相同的 LayerStack 顺序。隐藏 Layer
完全跳过渲染和命中；锁定 Layer 保持显示和导出，但不接受交互选择。Layer
顺序变化不能改变 Item Entity 身份或 Group 内部顺序。

## Gallery 图层面板

在现有 Canvas Gallery 页面增加 Layer
面板，显示名称、顺序、显隐、锁定和当前活动 Layer。面板通过 Canvas 的公开
API 更新 LayerStack，不直接替换内部
Component。面板自身的展开状态、行选择高亮和拖动预览留在 Gallery。

## 完成条件与验证

-   Layer 创建、重命名、排序、显隐、锁定、移动和删除均有确定性测试。
-   默认 Layer 始终存在；删除最后一个 Layer 被拒绝。
-   删除非空 Layer 必须明确选择迁移或级联删除，失败时不丢失内容。
-   活动 Layer 在不同 View 间独立；Layer 数据变化在不同 View 间共享。
-   隐藏 Layer 不渲染、不命中、不参与吸附和导出；锁定 Layer
    可见且可导出，但不可交互选择。
-   Layer 变更后 selection 与 active_layer 被正确修复。
-   更新 Integration Test Coverage Model 和 Canvas 功能/设计文档。
-   Gallery 通过 BRP 验证 Layer 切换、隐藏/显示、锁定/解锁和排序，并检查
    Screenshot 与 ECS 状态。

## 设计边界

本增量不实现 Group 编辑、Layer
混合模式、图层透明度、蒙版或图层效果。Layer 是 Document 的数据容器，不是
Gallery 的纯 UI 分组。
