# 增加网格显示与网格吸附

## 目标

为精确绘图提供每个 View
独立的网格显示和网格吸附。网格是编辑辅助覆盖物，不属于 Document
图形内容。

## View 配置

CanvasView 增加网格配置： - 是否显示网格。 - 是否启用网格吸附。 -
网格间距，使用 Document 单位。 -
可选的主网格间隔，用于在较大缩放范围内保持可读。

两个 View 观察同一 Document
时，可以使用不同网格间距和显隐状态。网格设置不进入 Document，也不随
Undo/Redo 回退。

## 显示与坐标

网格根据当前 View 的 center、zoom 和可见区域绘制。吸附间距始终按
Document 单位计算；Zoom
只影响屏幕显示密度和鼠标容差，不改变实际网格步长。

网格只作为 Renderer/Editor 的 View overlay： - 不进入 Document 的
Layer/Item 顺序。 - 不参与图形命中。 - 不进入 SVG/PNG 导出。 -
视图销毁时随 View 资源一起释放。

## 吸附规则

网格吸附作用于当前绘制点、移动目标点、Path 节点和 Connector
自由端点。吸附计算先把输入换算到 Document
坐标，再按网格间距取最近交点。屏幕容差使用逻辑像素表达并换算到当前
Zoom，避免放大后吸附范围过大。

吸附仅影响用户交互；程序化 Document API
仍使用调用方提供的精确坐标，不暗中修改数据。

## 完成条件与验证

-   网格显示随 Pan/Zoom 正确更新，不出现屏幕坐标漂移。
-   不同 View 的网格配置互相独立。
-   网格吸附在不同 Zoom 下保持相同 Document 间距。
-   网格不参与命中、Document 顺序和导出。
-   绘制、移动、Path 节点和 Connector 自由端点可独立启用/关闭网格吸附。
-   吸附取消或工具切换不会残留 View overlay。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证网格显示、开关和真实吸附，并检查 Screenshot。

## 设计边界

不实现标尺、持久辅助线、Artboard 网格、页面布局或复杂约束系统。
