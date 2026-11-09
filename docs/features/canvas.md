# Canvas

Canvas 提供可共享的二维文档、普通 UI 画布视图、矩形／椭圆／Path 编辑，以及 SVG 和 PNG 导出。Document 保存正式图形，View 保存观察中心、缩放、选择和当前工具；多个 View 可以编辑同一份文档。

```rust
use bevy::prelude::*;
use bevy_widgetry::canvas::*;

app.add_plugins(CanvasPlugin);
```

## 文档与图形

### 创建文档和矩形

```rust
let document = world.spawn(CanvasDocument).id();
let rectangle = CanvasDocument::create_rectangle(
    world,
    document,
    CanvasRectangle { size: Vec2::new(120.0, 80.0), corner_radius: 8.0 },
    Transform::from_xyz(100.0, 60.0, 0.0),
    CanvasPaint {
        fill: Some(Color::srgb(0.2, 0.5, 0.9)),
        stroke: Some(CanvasStroke { width: 2.0, ..default() }),
        ..default()
    },
).map_err(|failure| failure.into_bevy_error())?;
```

Document 坐标 X 向右、Y 向下，允许负坐标。矩形局部坐标以中心为原点；`Transform` 控制整体位置、旋转与 Scale。Document 自己的 Transform 不作为图形的额外几何变换。

矩形宽高至少为 1 个 Document 单位，圆角不超过短边的一半。非法几何、所属关系或变换在提交前拒绝，更新不通过自动钳制掩盖错误。

### 创建椭圆

```rust
let ellipse = CanvasDocument::create_ellipse(
    world, document,
    CanvasEllipse { radii: Vec2::new(60.0, 30.0) },
    Transform::from_xyz(260.0, 60.0, 0.0),
    CanvasPaint::default(),
).map_err(|failure| failure.into_bevy_error())?;
```

椭圆保存两轴半径，不转换成可编辑的多边形。`CanvasGeometry` 区分 Rectangle、Ellipse 与 Path；类型化更新不会悄悄改变图形种类。

### 修改整体变换

```rust
let transform = Transform {
    translation: Vec3::new(150.0, 100.0, 0.0),
    rotation: Quat::from_rotation_z(0.25),
    scale: Vec3::new(1.5, 1.0, 1.0),
};
CanvasDocument::update_transform(world, document, rectangle, transform)
    .map_err(|failure| failure.into_bevy_error())?;
```

整体变换不改局部尺寸、Path 命令或 Paint。公开写入使用 Document API；直接替换几何和变换组件不能代替这些操作中的完整校验。

## 画布视图

### 在 UI 中显示文档

```rust
let view = world.spawn_scene(bsn! {
    @CanvasView {
        @document: {Some(document)},
        @center: Vec2::new(150.0, 80.0),
        @zoom: 1.0,
    }
    Node { width: px(640), height: px(400) }
    BackgroundColor(Color::srgb(0.10, 0.11, 0.13))
})?.id();
```

Zoom=1 时，一个 Document 单位对应一个 UI 逻辑像素。背景属于 View 的 UI，不写入 Document，也不会成为矢量导出的背景。

同一 Document 可以绑定多个 View。删除 View 释放它的离屏资源，不删除文档；删除 Document 后，引用它的 View 变为空画布，仍可重新绑定。

### 平移、缩放和换文档

```rust
CanvasView::set_center(&mut commands, view, Vec2::new(200.0, 100.0));
CanvasView::set_zoom(&mut commands, view, 2.0);
CanvasView::set_document(&mut commands, view, Some(other_document));
```

同步版本分别为 `set_center_in_world`、`set_zoom_in_world`、`set_document_in_world`。Zoom 合法范围为 `[0.1, 10.0]`；程序化缩放保持观察中心，用户滚轮以指针位置为锚点。

Space + 鼠标主键拖动平移。手势开始后用途固定，不会因中途松开 Space 改成图形移动。捕获期间拖出视口仍可继续；取消、失焦、禁用或解绑会结束手势。

## 选择与变换

### 选择指定图形

```rust
CanvasView::set_selection_in_world(world, view, [rectangle, ellipse])
    .map_err(|failure| failure.into_bevy_error())?;
let selected = CanvasView::selection(world, view)
    .map_err(|failure| failure.into_bevy_error())?;
```

选择按图形 Entity 保存，与其他 View 独立。普通点击替换为单选，Ctrl+点击切换成员；空白点击清空，Ctrl+空白保持选择。首次点击未选中图形只选择，再次按下已选图形才启动移动。

命中使用真实填充和描边，不把椭圆外接矩形角落或 Path 孔洞当作实心区域。透明填充与无填充不同：显式存在的透明填充仍参与命中。

### 移动、缩放和旋转

单选时提供整体缩放与旋转控点，缩放修改 Transform 而不是局部几何；Shift 旋转按 15° 吸附。多选支持整批移动和 Delete，但不提供多选整体缩放／旋转控点。

```rust
CanvasDocument::update_transforms(world, document, &[
    (rectangle, Transform::from_xyz(120.0, 120.0, 0.0)),
    (ellipse, Transform::from_xyz(280.0, 120.0, 0.0)),
]).map_err(|failure| failure.into_bevy_error())?;
```

批量更新先校验所有成员，再一起提交，任何非法成员都会保留整批旧值。普通手势取消保留已提交位置，不自动执行撤销或 Esc 回滚。

## 绘制工具

### 绘制矩形和椭圆

```rust
CanvasView::set_mode(
    &mut commands, view,
    CanvasMode::Rectangle(CanvasPaint::default()),
);
```

切换为 `CanvasMode::Ellipse(paint)` 即绘制椭圆。拖动时只出现本 View 的预览；释放后尺寸有效才创建正式图形，自动选中新图形并回到 Select。取消不留下半成品。Space 平移仍优先。

### 使用 Pen 绘制 Path

```rust
CanvasView::set_mode(&mut commands, view, CanvasMode::Pen(CanvasPaint {
    fill: None,
    stroke: Some(CanvasStroke { width: 2.0, ..default() }),
    ..default()
}));
```

Pen 持有当前 View 的草稿。获得 Canvas 焦点后，L / Q / B / A 切换段类型，C 闭合，N 开始新子路径，Enter 完成，Esc 丢弃草稿。完成或 Esc 后仍保留 Pen 工具；模式切换才离开工具。

草稿不进入 Document、其他 View 或导出。实际提交后才产生正式 Shape。

## Path 与参数编辑

### 管理路径数据

`CanvasPath` 保存 MoveTo、LineTo、QuadTo、CubicTo、ArcTo、Close 原始命令。一个 Shape 可包含多个开放或闭合子路径，共享 Paint、Transform 和叠放位置。

`create_path` / `update_path` 提交完整 Path；`append_rectangle`、`append_circle`、`append_ellipse`、`append_line`、`append_polyline`、`append_polygon` 向已有 Path 追加独立轮廓。

填充规则支持 NonZero 与 EvenOdd；描边支持 Butt / Round / Square 端点和 Miter / Round / Bevel 连接。设置 `fill: None` 可以得到只有描边的路径。

### 拖动路径节点

```rust
CanvasView::set_mode(&mut commands, view, CanvasMode::PathNodeEdit);
```

节点模式编辑已选 Path 的原始端点和贝塞尔控制点，不将 Rectangle / Ellipse 转换为 Path。ArcTo 拖动端点时保留半径、弧轴旋转与方向标记。节点拖动实时提交几何，普通取消不回滚已提交值。

拓扑修改使用 `insert_path_anchor`、`remove_path_anchor`、`set_path_segment`、`set_path_subpath_closed`。命令下标以调用时的原路径解释，成功后可能变化，因此下一次操作重新读取命令；失败不会留下半条候选路径。

## 图形顺序与删除

Document 保存唯一的从底到顶顺序，绘制、命中、SVG 与 PNG 使用相同顺序。Transform.z 不代替该顺序。每个图形的填充后紧接它自己的描边，不把全部填充与全部描边分开排序。

```rust
CanvasDocument::remove_shapes(world, document, &[rectangle, ellipse])
    .map_err(|failure| failure.into_bevy_error())?;
```

键盘 Delete 只在有效 CanvasView 根拥有焦点时删除选择，不接管内部 TextField 的删除键。删除最后一个图形后文档仍存在，可以继续绘制。

## SVG 导出

### 导出完整文档或指定区域

```rust
CanvasDocument::export_svg(world, document, "drawing.svg", None)
    .map_err(|failure| failure.into_bevy_error())?;
```

`None` 自动合并已提交图形的边界；显式 `Some(Rect)` 使用 Document 坐标区域。导出保留参数矩形、椭圆、原始 Path 命令、二维变换与透明度，不包含背景、选择框、控点和未提交草稿。

导出成功会覆盖目标文件。完整模型、区域及颜色校验先于打开文件；空 Document 拒绝导出。部分 SVG 查看器的透明混合效果可能与 Canvas 的线性合成不同，这不改变模型中的颜色。

## PNG 导出

### 开启与发起导出

Facade 启用 `canvas_png_export`；直接依赖 Canvas crate 时启用 `png_export`。PNG 使用实际 GPU 渲染，与同步 SVG 不同。

```rust
let request = CanvasDocument::export_png(
    world, document, "drawing.png",
    CanvasPngOptions { region: None, scale: 2.0 },
).map_err(|failure| failure.into_bevy_error())?;
```

返回的是导出请求 Entity，不是“文件已完成”。请求固定已提交模型快照，之后继续编辑不会改变本次输出。输出与 View 是否存在、Pan/Zoom 或屏幕比例无关。

### 处理完成与取消

```rust
fn exported(event: On<CanvasPngExportFinished>) -> Result {
    match &event.result {
        Ok(size) => info!(?size, "PNG 已写入"),
        Err(CanvasPngStop::Canceled) => info!("导出取消"),
        Err(CanvasPngStop::Failed(error)) => {
            return Err(error.clone().into_bevy_error());
        }
    }
    Ok(())
}
```

这里将真实失败交给宿主错误处理器；应用也可以在失败分支实际执行提示或放弃策略后，再记录处理结果。通过请求的 `CanvasPngExport::job_id()` 可以取得受管任务身份并发起取消。完成、取消或失败后释放请求专属资源；取消不会回滚 Document。
