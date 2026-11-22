# 增加常用流程图形状预设

## 目标

为流程图和结构图提供常见节点轮廓。预设创建普通可编辑 Shape，优先使用现有
Rectangle 参数或 Path，而不是引入专用流程图渲染器。

## 首批预设

-   **Terminator / 圆角矩形**：使用 `CanvasRectangle.corner_radius`
    表达开始/结束节点。
-   **Decision / 菱形**：使用闭合 Path 表达判断节点。
-   **Input/Output / 平行四边形**：使用闭合 Path 表达输入输出节点。
-   **Database / 圆柱体**：使用闭合 Path 表达上下弧线和侧边。
-   **Document / 文档形状**：使用闭合 Path 表达带波浪底边的文档轮廓。
-   **Arrow / 独立箭头形状**：使用闭合 Path
    表达可旋转的块状箭头；它是普通 Shape，与 Connector
    的端点箭头装饰分开。

预设工具只决定初始几何和默认 Paint。创建后，图形仍然是普通
Shape，可以修改颜色、描边、Transform、Path 节点和顺序，也可以进入
Group、Layer、复制/粘贴、渐变与导出流程。

## API 与工具

提供类型明确的预设枚举或构造函数，避免 Gallery 手工拼接一组难以维护的
Path
命令。参数只包含当前预设真正需要的尺寸与圆角/比例。工具栏展示常用节点图形，用户选择后拖动创建。

预设创建使用当前活动 Layer；预览只属于当前 View。生成图形通过 Document
API 提交并记录一个 Undo 条目。

## 完成条件与验证

-   每个预设生成预期闭合轮廓，且尺寸/Transform 有效；独立箭头可通过
    Transform 旋转和缩放。
-   圆角矩形仍是 `CanvasRectangle`，其他预设为可编辑 Path。
-   创建后的预设可选择、变换、节点编辑、分组、复制和导出。
-   预览取消不写入 Document；成功提交可 Undo/Redo。
-   命中检测遵循真实填充/描边，不把 Path 外接矩形当作实心区域。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证所有预设创建与编辑。

## 设计边界

不建立 BPMN
完整符号库、流程图语义验证或自动布局系统。这里只提供常见节点轮廓。
