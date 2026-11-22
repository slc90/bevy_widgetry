# 增加 Canvas 复制、剪切、粘贴与重复

## 目标

支持复制 Shape、Text、Connector 和
Group，并保留嵌套结构、样式与连接关系，方便重复制作图标、流程图节点和表情贴纸。

## Clipboard 内容

增加应用内 `CanvasClipboard` Resource，保存 Canvas 可编辑 Item
的快照。不接入操作系统全局剪贴板，不存储 PNG 截图。

复制时从当前 View 的选择中提取最上层被选 Item：如果同时选中了一个 Group
和它的子项，只复制 Group 一次，避免重复。快照保留： - Shape
Geometry、Paint、Transform。 - Text 内容、字号、颜色和对齐。 - Group
层级与直属顺序。 - Connector 路由、箭头、标签和端点关系。

## Connector 引用重映射

如果 Connector 的两个端点都引用复制集合中的 Shape，Paste 时将目标 Entity
重映射到新
Shape。如果只有一端目标在复制集合内，复制后的该端保留内部引用，外部端点转换为当时解析位置的
Free Endpoint。不能让复制后的 Connector 继续误连到原图形。

## 操作语义

-   **Copy**：更新 CanvasClipboard，不修改 Document，不产生 Undo 条目。
-   **Cut**：先成功生成 Clipboard
    快照，再以一个事务删除原选择；复制失败时原内容保持不变。
-   **Paste**：在当前活动 Layer 创建新 Item，重建 Group 层级和 Connector
    引用，整体偏移一个固定的 Document-space 距离（默认 16 单位）。
-   **Duplicate**：在当前 Document 中复制并直接提交，使用与 Paste 相同的
    Entity 重映射和偏移规则。

跨 Document Paste 允许发生；目标 Layer 必须属于目标 Document
且可见、未锁定。若活动 Layer 无效，使用目标 Document 的默认可用
Layer。Clipboard 不保存来源 Document 的 Layer ID，避免跨文档粘贴引用失效
Layer。

## 历史与原子性

Cut、Paste、Duplicate 各形成一个 Undo 条目。Paste 在全部 Item、Group 和
Connector 引用验证成功后一次提交；失败不能留下部分创建的 Entity
或顺序引用。Undo/Redo 必须整体恢复层级和连接关系。

## 完成条件与验证

-   Shape、Text、Group、Connector 的 Copy/Paste/Duplicate
    保留内容与样式。
-   Group 复制保留嵌套顺序；重复选择父子项不会生成重复子树。
-   Connector 内部引用正确重映射，外部引用按自由端点策略处理。
-   Cut 复制失败时不删除原内容。
-   Paste 失败时无部分 Entity、ItemOrder 或 LayerStack 写入。
-   Paste 进入正确 Layer，偏移规则稳定。
-   跨 Document 粘贴不保留来源 Layer ID 或错误 Entity 引用。
-   Copy 不进入历史；Cut/Paste/Duplicate 各自整体可 Undo/Redo。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证复制、粘贴、重复和剪切。

## 设计边界

不实现系统剪贴板格式、跨应用拖放或图片剪贴板。Clipboard 只保存 Canvas
可编辑数据。
