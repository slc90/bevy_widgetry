# 增加 Connector 形状锚点与自动跟随

## 目标

让 Connector 端点可以附着到 Shape 的连接锚点。节点移动、缩放、旋转或所在
Group 变换时，Connector
自动更新端点；删除节点时连接线转换为自由端点，不留下失效 Entity 引用。

## 锚点模型

Connector Endpoint 扩展为两种形式：

-   `Free(Vec2)`：保存父容器局部坐标中的自由端点。
-   `Attached { target: Entity, anchor: CanvasAnchor }`：引用目标 Shape
    和稳定锚点。

首版锚点为 North、East、South、West 四个方向。Rectangle 和 Ellipse
根据各自参数计算精确锚点；闭合且只有一个外轮廓的
Path，通过从局部边界中心向四个方向与轮廓求交，并选择射线方向上最外侧的有效交点得到锚点，因此流程图预设
Path 可以连接。开放 Path、含多个子轮廓或无法得到稳定外边界的 Path
不提供连接锚点，继续使用自由端点。

锚点位置以 Shape 局部坐标表达。Renderer/Document
更新时使用同一层级变换解析将其转换为 Document 坐标，再转换到 Connector
所属父容器的局部坐标。这样 Connector 即使放在 Group
中，也能正确连接到另一个 Group 或 Layer 中的 Shape。

## 创建与跟随

Connector 端点拖动到可连接 Shape 的锚点容差范围内时，可以吸附并提交
Attached Endpoint；离开范围则保持 Free
Endpoint。锚点候选遵循对象吸附的屏幕容差与稳定候选选择规则。

目标 Shape 的 Transform、Geometry 或祖先 Group Transform
改变时，附着端点重新计算。Connector
的中间路由点保持原有相对位置；只调整附着端点以及必要的相邻线段。自动避障不在范围内。

目标 Shape 被删除时，先读取最后一次解析出的 Document-space
端点位置，再把 Attached Endpoint 转为 Free Endpoint。删除操作与
Connector 更新必须在同一个 Document 事务中提交，Undo
可以恢复原连接关系。若目标 Shape 所在 Layer 隐藏，连接到该 Shape 的
Connector 暂时不显示；Layer 重新显示后恢复连接线。

## Group、Layer 与吸附

Connector 可以与目标 Shape 位于不同 Layer 或 Group，但必须属于同一
Document。移动 Shape 到另一个 Layer 不改变其 Entity，因此连接保持。Group
Transform 改变时，锚点通过共享变换解析自动更新。

对象吸附系统增加 Connector Anchor
候选，并将其优先级置于普通边缘/中心和网格吸附之前。锚点吸附只在
Connector 端点编辑期间生效。

## 完成条件与验证

-   Rectangle、Ellipse 和流程图 Path 预设的四方向锚点位置正确。
-   Connector 端点附着、脱离和重新附着有测试。
-   Shape 移动、旋转、缩放、Geometry 修改和 Group 变换后，Connector
    端点正确跟随。
-   删除目标 Shape 后端点转为最后位置的自由端点；Undo 恢复原连接。
-   目标 Layer 隐藏时 Connector 隐藏，重新显示时恢复。
-   Connector 与目标 Shape 位于不同 Group/Layer 时仍正确转换坐标。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证端点吸附、节点拖动跟随和删除节点后的连接修复。

## 设计边界

不实现自动避障、自动正交布线、复杂路由求解或跨 Document
连接。连接锚点只作用于当前 Document 内支持锚点的 Shape。
