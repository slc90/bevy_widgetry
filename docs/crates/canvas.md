# Canvas

Canvas 当前提供脱离 UI 与 GPU 独立存在的 ECS Document 和矩形模型；此增量尚未提供 View、交互与导出。

## Document 与矩形

`CanvasDocument` 是无固定边界的业务 Entity，Document 坐标 X 向右、Y 向下，负坐标合法。矩形以 `ChildOf` 归属 Document，可包含任意多个；删除 Document 会级联删除其子图形。模型不需要 Camera、Image、Mesh 或 UI 插件，也不依赖 View 的存在。

`CanvasRectangle { size }` 保存中心对称的局部宽高，均须有限且至少为 1 个 Document 单位。官方 `Transform` 是中心位置、旋转与整体 Scale 的唯一权威；使用绕 Z 轴的正规化旋转，XY Scale 非零，允许非均匀 Scale。所有数值须有限，变换后的边界也须可表示。Document 不是渲染 Scene，Document Entity 本身的 Transform 不参与形状几何解释。

`CanvasPaint { fill, stroke }` 中 `fill: None` 表示无填充，`Some(Color::NONE)` 保留指定透明颜色的语义。`CanvasStroke { color, width }` 的宽度为正有限的局部 Document 单位，沿矩形边界居中；Transform 的 Scale 同时作用于填充和描边。颜色转换后的通道必须有限。

`CanvasDocument::create_rectangle`、`update_rectangle` 和 `remove_rectangle` 提供同步 World CRUD；创建/更新先校验完整数据，更新和删除还校验图形确实属于目标 Document。非法请求记录错误并返回 `Severity::Error`，保留原有数据。图形 Entity 即稳定身份，不维护第二份 Shape 列表或变换副本。

模型组件是业务数据，可通过原生 ECS 访问；直接修改时须遵守相同有效性约束，或使用上述校验型 CRUD。官方 Transform 保持原生读写方式。`CanvasRectangle::validate` 与 `bounds` 返回校验错误，供数据消费者处理；`corners` 对有效数据给出变换后的四角，`bounds` 计算包含居中描边和整体变换的最小 AABB，均不依赖 GPU。
