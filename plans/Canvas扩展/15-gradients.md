# 为 CanvasPaint 增加线性与径向渐变填充

## 目标

支持常用的线性与径向渐变填充，并提供可交互的渐变手柄和色标编辑。渐变参数属于
Document Paint；手柄、色标面板和临时预览属于 View/Host UI。

## Paint 类型

当前 `CanvasPaint.fill` 为 `Option<Color>`。扩展后改为
`Option<CanvasFill>`，其中 `CanvasFill` 为： - `Solid(Color)`。 -
`LinearGradient { start, end, stops }`。 -
`RadialGradient { center, radius, stops }`。

这是一项明确的公开 API 类型迁移；现有 `fill: Some(Color)` 调用需要改为
`fill: Some(CanvasFill::Solid(Color))`，并同步更新
Gallery、测试和功能文档。不要同时保留 `fill` 和 `gradient`
两个可独立设置的字段，以免出现两个填充权威。

每种渐变包含 2 至 8 个 Color Stop。Stop 偏移位于
`[0, 1]`，按非递减顺序保存；相同偏移允许出现，用于表达硬边颜色切换。LinearGradient
保存局部起点和终点；RadialGradient
保存局部中心与半径。径向渐变首版不包含独立焦点。

渐变坐标使用 Shape 局部坐标。Shape 移动、旋转或缩放时，渐变随 Shape
一起变换；Group 变换按统一层级矩阵影响最终呈现。

## 编辑体验

选中 Shape 后可以将 Fill 切换为线性或径向渐变。View
显示渐变方向/范围手柄；拖动手柄更新局部坐标。色标面板支持添加、删除、移动
Stop 和修改颜色。Stop 数量、偏移与半径在 Document API 提交前校验。

渐变编辑拖动使用一个编辑事务；一次手柄拖动只产生一个 Undo
条目。临时拖动预览属于 View，不提前写入共享 Document。

## Renderer 与资源

现有 Renderer 使用 `ColorMaterial` 表达纯色 Fill/Stroke，几何由共享
MeshCache 派生。渐变 Fill 需要独立的 2D Material/Shader
路径，读取渐变参数并在片元阶段计算颜色；Stroke
继续使用现有纯色路径。Mesh
几何缓存与渐变材质参数缓存分离，修改色标或方向不应无故重新细分
Geometry。

每个 View 的材质/代理资源独立管理，Document Paint 是唯一数据权威。View
销毁时释放自己的材质引用，不得破坏其他 View 仍在使用的同一 Document。

## 导出

SVG 导出生成 LinearGradient/RadialGradient 定义和对应引用，保留
Stop、局部坐标与 Shape Transform。PNG 导出使用实际 GPU
Renderer。导出只覆盖当前支持的渐变语义，不扩展 SVG Import 或完整 Paint
Server。

## 完成条件与验证

-   Solid Fill 迁移后原有纯色效果保持一致。
-   Linear/Radial Gradient 的数据校验、Stop 增删/排序、局部坐标和
    Transform 有测试。
-   相同 Stop 偏移产生确定的硬边结果。
-   渐变更新不使无关 Geometry Mesh 失效。
-   多 View 显示同一 Document 时材质与缓存生命周期正确。
-   渐变手柄和 Stop 编辑通过 BRP 验证。
-   SVG/PNG 导出对渐变有可重复的图像验证。
-   若渐变路径影响持续渲染成本，按实际场景测量 Frame Time 与长帧。
-   更新 Coverage Model、Canvas 功能文档和设计文档。

## 设计边界

首版只支持 Shape 填充渐变，不支持渐变描边、Pattern、Mesh Gradient、Noise
Gradient、滤镜或遮罩。CanvasPaint 描述用户绘图内容，不是 Canvas
控件主题。
