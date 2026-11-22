# 增加 Connector、折线路由与箭头

## 目标

提供区别于普通 Path 的流程图连接线。Connector 是有端点语义的 Document
Item，首个增量支持自由端点、直线/手动折线路由、箭头和标签；后续再增加形状锚定。

## Connector 数据

新增 `CanvasConnector` Item，保存： - 起点与终点的自由坐标。 -
可选的中间路由点。 - 描边颜色、宽度、端点与连接样式。 -
起点和终点箭头类型。 - 可选的 Connector Label。

无中间路由点时为直线；有中间点时为手动折线。Connector
的路由数据是正式模型，Renderer 从它生成
Path/Mesh，不把生成网格作为可编辑事实。

`CanvasConnectorLabel` 保存文本、字号、颜色和沿 Connector
路径的位置参数。它是 Connector 自身的数据，不是独立 CanvasText
Entity；这样复制、删除和移动 Connector
时标签自然随之处理。流程图节点上的文字仍使用独立 CanvasText。

## 箭头与编辑

起点和终点分别支持无箭头或基础实心三角箭头，因此可以表达单向与双向连接。箭头几何根据端点切线生成，并纳入命中检测和导出。

Connector 工具通过 Pointer
操作创建起点、终点和可选折线中间点。创建期间只显示当前 View
预览；完成后提交正式 Connector。选中 Connector
后可移动端点和中间路由点；移动整个 Connector 时保持内部点的相对位置。

Connector 使用独立 Paint/Stroke 语义，不要求 `CanvasGeometry` 也变成
Connector 的第二份权威。Renderer 可以复用现有 Path
几何细分逻辑，但路由点与端点只保存在 `CanvasConnector` 中。

## 顺序、Group 与导出

Connector 是普通 CanvasItem，可以进入 Layer 或
Group。其路由点在所属父容器的局部坐标中保存，Group Transform 统一作用于
Connector 和其他子项。SVG 导出写入路径与箭头几何；PNG 使用实际 Renderer
结果。

## 完成条件与验证

-   自由端点直线、手动折线、箭头、双向箭头和标签均有确定性测试。
-   Connector 选择、移动、路由点编辑、删除和顺序正确。
-   箭头方向与路径端点切线一致，命中检测覆盖线段和箭头。
-   Connector 可进入 Group/Layer，并参与复制、Undo/Redo 和导出。
-   取消绘制不产生 Document 项；成功创建产生一个历史条目。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证直线/折线 Connector、箭头和标签。

## 设计边界

首个增量不实现形状锚定、自动吸附、自动避障、自动正交布线或自动布局。连接端点先作为自由点，锚定在后续独立增量中实现。
