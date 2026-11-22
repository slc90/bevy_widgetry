# 增加直线、折线与多边形工具

## 目标

在现有 Rectangle、Ellipse、Pen 和 PathNodeEdit
之外，增加最常用的直线、折线和多边形绘制工具。所有结果仍保存为可继续编辑的
`CanvasPath`，不增加新的底层 Mesh 类型。

## 绘制语义

-   **Line**：Pointer Down 确定起点，拖动显示线段预览，Release 提交包含
    MoveTo/LineTo 的开放 Path。
-   **Polyline**：每次点击添加一个顶点，移动 Pointer 时预览当前段；Enter
    或双击完成，至少两个有效顶点才提交。
-   **Polygon**：连续点击添加顶点，预览闭合轮廓；Enter
    或双击完成，至少三个有效顶点才提交，并生成 Close 命令。

新工具加入 `CanvasMode` 或等价 View 工具状态。绘制预览只属于当前
View，不进入 Document、其他 View 或导出。完成时通过 Document API
在当前活动 Layer 创建 Path，并按现有工具契约选择新 Item；Escape、Pointer
Cancel、失焦或切换工具清理预览，不留下半成品。

## 与现有 Path 的关系

Line、Polyline、Polygon 都生成现有 `CanvasPathCommand`。生成后可以进入
`PathNodeEdit`，继续移动锚点、控制点或修改子路径。Line 使用无填充 Paint
时只绘制描边；Polygon
可使用填充和描边。不要把这些工具生成的路径转成不可编辑的网格。

绘制坐标仍遵循当前 Canvas 的窗口/UI → View local → Document coordinate
转换。后续 Grid/Object Snapping
增量会把吸附接入统一输入坐标路径；本增量不为每个工具单独实现吸附算法。

## 完成条件与验证

-   Line、Polyline、Polygon 的预览、提交、取消和无效点数均有确定性测试。
-   Line 生成开放 Path；Polyline 保留顶点顺序；Polygon 正确闭合。
-   提交后可选择、移动、节点编辑和导出。
-   预览取消不修改 Document，也不产生历史记录；成功提交形成一个 Undo
    条目。
-   多 View 中草稿仅出现在发起绘制的 View，提交后其他 View 观察到结果。
-   现有 Rectangle、Ellipse、Pen 和 PathNodeEdit 行为不回归。
-   更新 Coverage Model、Canvas 功能文档与设计文档。
-   Gallery 通过 BRP 验证三种工具的真实输入与截图。

## 设计边界

不增加自由手绘、压力笔刷、像素笔刷、自动路径拟合或路径布尔运算。
