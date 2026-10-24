# Canvas

Canvas 提供独立 ECS Document、矩形模型和普通 Bevy UI CanvasView。当前已实现矩形显示，尚未提供编辑交互与导出。

## Document 与矩形

`CanvasDocument` 是无固定边界的业务 Entity，Document 坐标 X 向右、Y 向下，负坐标合法。矩形以 `ChildOf` 归属 Document，可包含任意多个；删除 Document 会级联删除其子图形。模型不需要 Camera、Image、Mesh 或 UI 插件，也不依赖 View 的存在。

`CanvasRectangle { size }` 保存中心对称的局部宽高，均须有限且至少为 1 个 Document 单位。官方 `Transform` 是中心位置、旋转与整体 Scale 的唯一权威；使用绕 Z 轴的正规化旋转，XY Scale 非零，允许非均匀 Scale。所有数值须有限，变换后的边界也须可表示。Document 不是渲染 Scene，Document Entity 本身的 Transform 不参与形状几何解释。

`CanvasPaint { fill, stroke }` 中 `fill: None` 表示无填充，`Some(Color::NONE)` 保留指定透明颜色的语义。`CanvasStroke { color, width }` 的宽度为正有限的局部 Document 单位，沿矩形边界居中；Transform 的 Scale 同时作用于填充和描边。颜色转换后的通道必须有限。

`CanvasDocument::create_rectangle`、`update_rectangle` 和 `remove_rectangle` 提供同步 World CRUD；创建/更新先校验完整数据，更新和删除还校验图形确实属于目标 Document。非法请求记录错误并返回 `Severity::Error`，保留原有数据。图形 Entity 即稳定身份，不维护第二份 Shape 列表或变换副本。

模型组件是业务数据，可通过原生 ECS 访问；直接修改时须遵守相同有效性约束，或使用上述校验型 CRUD。官方 Transform 保持原生读写方式。`CanvasRectangle::validate` 与 `bounds` 返回校验错误，供数据消费者处理；`corners` 对有效数据给出变换后的四角，`bounds` 计算包含居中描边和整体变换的最小 AABB，均不依赖 GPU。

## Canvas View 与渲染

安装 `CanvasPlugin` 后，通过 BSN `@CanvasView { @document: {Some(document)} }` 构造并使用普通 Node 设置布局。Facade 默认启用 `canvas` Feature，通过 `bevy_widgetry::canvas` 导出公共 API，也可仅启用该 Feature。Props 仅初始化引用；`CanvasView::document` 查询，`set_document_in_world` / Commands `set_document` 更新。有效同值不通知，实际提交后发送 `CanvasDocumentChanged`；无效 View/Document 返回错误。None 可显示空画布。

每个 View 有独立 Camera、Image 和全局唯一离屏层号，使用 Core `offscreen` 的维护和回收机制。Camera 透明清屏，采用 4× MSAA；需要背景时可在 UI Node 设置 BackgroundColor，不写入 Document。View 中心默认对应原点，1 Document 单位等于 1 UI 逻辑像素；目标 Image 使用布局物理尺寸，投影使用逻辑尺寸。Document 的 Y 向下通过专属绘图容器的一次 Y 反射转换，Document Transform 不变。

填充和居中描边分别使用二维 Mesh 与 ColorMaterial，支持无填充、无描边和透明颜色。描边几何为内外四边连接的环形，过宽描边的内部尺寸钳至零。位置、旋转、整体 Scale 和颜色更新不重新生成几何；局部尺寸或描边宽度变化更新原 Mesh Asset。图形代理在 Transform 传播前更新，相机在布局后、Frustum 检查前更新，避免投影与内容落后一帧。

隐藏或零尺寸时停用 View 相机；恢复后使用当前模型。销毁 View 回收专属 Scene、相机、Image、层号、Mesh 和 Material，Document 与子图形继续存在。当前 Mesh 属于各 View 的代理，不挂到 Document；模型始终为唯一图形权威。

实际 Bevy 0.20 的二维 RenderTarget 使用预乘 RGB，而原生 Viewport 按 straight alpha 合成，直接显示会对透明颜色再次乘 Alpha。Canvas 因此使用 Bevy 自带 `FullscreenMaterial` 在相机输出前将 RGB 除以非零 Alpha；Alpha 为零时输出全零。该适配只改变颜色表示，保留原生 Camera2d / ColorMaterial / MSAA / Viewport 路径，不负责几何绘制。WESL 由 Asset 的 `BuiltinShader::CanvasStraightAlpha` 提供；不会影响其他 Widget 或窗口相机。

绘图代理只投影 XY Scale，模型的 Z Scale 不参与平面几何，也不缩放填充/描边之间的内部深度偏移；因此负值或零 Z Scale 不会颠倒或压平 Paint 的顺序，模型 Transform 保持原值。
