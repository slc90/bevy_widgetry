# 增加图形对齐与等间距分布

## 目标

提供图标、流程图和贴纸排版需要的对齐与分布操作。输入来自当前 View
的选择，实际修改 Document 中 Item 的 Transform。

## 对齐操作

支持： - 左边缘、右边缘对齐。 - 顶边、底边对齐。 -
水平中心、垂直中心对齐。 - 相对于选择集合对齐。 - 相对于当前 View
可见文档矩形对齐。

当前 Canvas 没有固定 Artboard，因此"对齐到画布"明确表示对齐到当前 View
的可见 Document-space 矩形。该矩形由 View 的 viewport、center 和 zoom
计算，不是 Document Entity 的 Transform，也不是另一个 View 的可见范围。

## 分布操作

支持水平等间距与垂直等间距，至少需要三个 Item。先按对应轴的 AABB
排序，保持第一个和最后一个 Item 的位置，中间 Item
按相等间距重新定位。Item 尺寸不变。

对齐与分布支持 Shape、Text 和 Group。Connector
不作为对齐/分布目标，因为移动整条 Connector
可能与附着锚点关系冲突；Connector 随其锚定 Shape
更新端点，不参与普通对象分布。

## 边界计算与提交

所有边界都在 Document 坐标计算。旋转 Shape
先变换局部边界，再取轴对齐包围盒；Group
使用所有后代可见几何边界的并集。该 AABB
语义同时用于对象吸附和智能参考线。

批量对齐/分布通过一个 Document 事务提交。任何无效 Entity、跨 Document
Item、隐藏/锁定 Layer 中的选中项或不支持的 Connector
都使整个操作失败，不得只修改部分对象。绘制顺序不因对齐而改变。

## 完成条件与验证

-   六种基本对齐、选择集合对齐、当前 View
    区域对齐、水平/垂直分布均有确定性测试。
-   计算覆盖负坐标、不同尺寸、旋转 Shape 和嵌套 Group。
-   旋转对象统一使用 Document-space AABB。
-   分布至少三个 Item；两个或更少时返回未改变结果。
-   Connector 被选中时操作明确拒绝，不部分移动其他 Item。
-   批量更新失败时所有 Transform 保持旧值。
-   两个 View 使用各自可见区域对齐时，写入共享 Document 的结果正确。
-   一次对齐/分布只产生一个 Undo 条目。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证对齐和分布，并检查实际位置。

## 设计边界

不实现自动布局、流程图自动排版、Artboard
或复杂约束求解。对齐与分布是明确的一次性编辑命令。
