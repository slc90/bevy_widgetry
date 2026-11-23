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

选择按绘图项 Entity 保存，与其他 View 独立。普通点击替换为单选，Ctrl+点击切换成员，空白点击清空，Ctrl+空白保持选择。首次点击未选中的绘图项只选择，再次按下已选项才启动移动。默认命中整组，进入组内后可以选择该组的直属成员。

命中使用真实填充和描边，不把椭圆外接矩形角落或 Path 孔洞当作实心区域。透明填充与无填充不同：显式存在的透明填充仍参与命中。

### 移动、缩放和旋转

单选时提供整体缩放与旋转控点，缩放修改 Transform 而不是局部几何；按住 Shift 旋转时，角度吸附到文档坐标中的 15° 整数倍。多选支持整批移动和 Delete，但不提供多选整体缩放／旋转控点。

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

### 绘制直线、折线和多边形

用 `CanvasView::set_mode_in_world` 或 `set_mode` 进入 `CanvasMode::Line(paint)`、`Polyline(paint)` 或 `Polygon(paint)`。Gallery 的主视图提供三个对应工具按钮。

- **直线**按下确定起点，拖动预览，释放后提交开放的两点 Path。为线段配置 `CanvasPaint.stroke`。
- **折线**单击添加顶点，移动指针时预览下一段。至少两个不同顶点后用 Enter 或在最后一个顶点双击提交。折线只使用描边。
- **多边形**以相同方式放置至少三个不同顶点，预览回到起点的轮廓，提交时生成 `Close`。多边形支持填充和描边。

触屏可连续点按放置顶点。折线和多边形只接受有效点击，拖动会打断连续双击的判定。成功提交在活动图层或当前组中创建一个可节点编辑和导出的 `CanvasPath`，选择新项并返回 Select。一次提交对应一个撤销条目。Esc、指针取消、失去焦点或切换工具清除当前 View 的草稿。草稿期间共享文档、其他 View 和导出内容保持原状。

### 拖动绘制正多边形、星形和圆弧

通过 `CanvasMode::Parametric(tool, paint)` 选择工具。Gallery 中再次点击当前工具按钮可切换预设，主视图读数显示所选参数。

| `CanvasPathTool` | 参数与手势 |
|---|---|
| `RegularPolygon { sides }` | 3 至 255 条边。从中心拖到第一个外顶点确定半径和起始角，之后沿正角度等距生成顶点。 |
| `Star { points, inner_ratio }` | 3 至 255 个外顶点，内外半径比例在开区间 (0, 1) 内。从中心拖到第一个外顶点，轮流连接内外顶点并闭合。 |
| `Arc { radii, rotation, large_arc, sweep }` | 从起点拖到终点。双轴半径使用文档单位，旋转使用弧度，两个方向标志沿用 SVG 端点圆弧语义。用描边绘制开放弧线。 |

正多边形和星形按住 Shift 时，第一个外顶点的方向吸附到最近的 15°，外半径保持拖动距离。参数非法时设置工具返回错误。零长度拖动或给定半径无法连接两端时没有预览，释放时不创建图形。成功释放会选中新建 Path、返回 Select，并产生一次可重做的撤销记录。生成结果可沿用 Path 节点编辑、变换和导出接口。

### 流程图形状预设

选择 `CanvasMode::Flowchart(preset, paint)` 后，从外接矩形的一角拖到对角即可创建流程图节点。Gallery 提供开始／结束、判断、输入／输出、数据库、文档和块箭头六个按钮。开始／结束使用最大圆角的 `CanvasRectangle`，其余预设生成带菱形、斜边、圆柱弧线、波浪底边或箭头轮廓的闭合 Path。可用普通图形编辑工具调整样式、节点、尺寸、旋转和顺序。

程序化创建时，`CanvasFlowchartPreset::geometry(size)` 返回以局部原点为中心的几何，宽高均须为有限值且至少为 1 个文档单位。拖动工具在当前 View 预览，取消即丢弃；释放后在活动层或编辑中的组内创建图形，并记录一次 Undo。

### 使用 Pen 绘制 Path

```rust
CanvasView::set_mode(&mut commands, view, CanvasMode::Pen(CanvasPaint {
    fill: None,
    stroke: Some(CanvasStroke { width: 2.0, ..default() }),
    ..default()
}));
```

Pen 持有当前 View 的草稿。获得 Canvas 焦点后，L / Q / B / A 切换段类型，C 闭合，N 开始新子路径，Enter 完成，Esc 丢弃草稿。按 Esc 取消草稿或正在放置的落点时，仍停留在当前组。Pen 空闲时按 Esc 返回上一级。完成、取消或返回上一级后，Pen 工具仍处于启用状态。

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

每份 Document 初始包含一个可见、未锁定的 `Layer 1`。每个 Shape 都具有 `CanvasItem` 身份，保留自己的 Entity、Geometry、Paint 和 Transform。Document 支持在 Headless World 中创建与修改。

`layer_order` 返回从底到顶的图层 ID，`item_order` 查询指定容器的直属绘图项，`draw_order` 依图层与容器顺序展平绘图项。`shape_order` 只返回其中的 Shape。`reorder_item` 和 `reorder_shape` 都在绘图项所属的容器内调整顺序。

绘制、命中、SVG 与 PNG 使用同一套叠放顺序，图形的 `Transform.z` 不参与排序。每个图形的填充后紧接它自己的描边。

```rust
CanvasDocument::remove_shapes(world, document, &[rectangle, ellipse])
    .map_err(|failure| failure.into_bevy_error())?;
```

键盘 Delete 只在有效 CanvasView 根拥有焦点时删除选择，不接管内部 TextField 的删除键。删除最后一个图形后文档仍存在，可以继续绘制。

## 撤销与重做

同一 Document 的所有 View 共享编辑历史。CanvasView 根获得焦点后，Ctrl+Z 撤销，Ctrl+Shift+Z 或 Ctrl+Y 重做。视角、工具、活动图层和草稿仍由各 View 独立维护。回到较早的 Document 状态时，View 会清理已失效的选择和组内编辑路径。

`CanvasDocument::undo` 和 `redo` 在 Headless World 中也可使用。没有对应记录时返回 `Ok(false)`，可通过 `can_undo` 和 `can_redo` 查询当前可执行性。每次有效的修改产生一条记录，批量操作作为一条记录提交。拖动实时更新文档，并在释放或取消时将实际保留的修改合并为一条记录。

要合并多次程序化修改，调用 `CanvasDocument::edit`，将闭包收到的 `CanvasEditToken` 传给各写入 API。闭包返回时会结束事务。若闭包返回错误，此前已成功的修改仍可作为一条记录撤销。事务打开期间，直接传入 Document Entity 的写操作以及撤销、重做均返回 `CanvasErrorKind::Busy`。另一个 View 仍可观察与调整视角。

历史默认保留最近 100 条事务，可用 `set_history_capacity` 调整，也可用 `clear_history` 清空。撤销后提交新修改会丢弃重做分支。删除或撤销创建会从绘制层级分离实体，历史回放会恢复原 Entity。保留的实体带有内部 `CanvasTombstone` 标记，并在相关历史记录被清理或文档销毁后释放。

回放会先检查需要改写组件或变更容器成员资格的实体的所有权。直接通过 ECS 将这些实体移到另一份 Document 后，原文档回放对应事务会返回所有权错误。

## 图层管理

`CanvasDocument::layers` 提供从底到顶的图层信息。`CanvasLayerId` 绑定所属文档，`get()` 返回文档内稳定的局部编号。新增图层时可指定插入位置，之后可重命名、调整顺序和移动直属绘图项。

矩形、椭圆和 Path 的 `create_*_in_layer` 方法接收显式图层 ID。原有创建方法写入 `default_layer`。删除默认层时，剩余顺序中的第一层接任默认层。

隐藏层的内容从视图、命中与 SVG／PNG 导出中移除，显示后按原顺序恢复。锁定层继续绘制与导出，视图选择和编辑只接受可见、未锁定的图层内容。Document 的显式 API 可修改锁定层。

`CanvasView::active_layer` 查询该视图的活动图层，`set_active_layer_in_world` 可切换至可用图层。绘图草稿只能提交到开始绘制时的活动层，活动层变化会取消草稿。活动层变为不可用时，视图选择第一个可见且未锁定的图层，全部不可用时暂停提交。共享 Document 的视图可在不同图层绘图。

`remove_layer` 可以用 `OnlyIfEmpty` 删除空层，用 `MoveContentsTo` 将内容追加到同一文档内可用的目标层，或用 `DeleteContents` 级联删除图层及其内容。每份 Document 至少保留一个图层。

## 嵌套分组与组内编辑

`CanvasDocument::group_items` 将同一个图层或父组中画序连续的两个以上绘图项组成有名称的 `CanvasGroup`。组原点位于所选项在父容器中的绘制边界中心。成员保留 Entity、原有视觉位置和全局叠放顺序，组占用原有的连续画序范围。`ungroup` 将组的变换组合回成员，并在该组的位置按序展开直属项。如果坐标转换无法保留成员的视觉位置，分组或取消分组会在写入前拒绝。删除组的最后一个成员时，空组也会被移除。

`update_group_transform` 支持平移、绕 Z 旋转和正数等比缩放。组的缩放手柄保持宽高比。`container_of` 查询直属父容器，`document_transform` 返回从绘图项局部坐标到文档坐标的 `Affine3A`。`reorder_item` 可以在直属容器内调整整组的顺序。

在选择模式下双击进入组，Esc 返回上一级。同步 API 为 `enter_group_in_world` 和 `leave_group_in_world`，对应排队入口为 `enter_group` 和 `leave_group`。`CanvasView::group_path` 查询从外到内的当前编辑路径。两个共享文档的 View 可以分别停在不同层级。进入组会激活该组所属图层，明确切换到另一图层时会退出组内编辑。组内绘制的矩形、椭圆和 Pen 按预览轮廓提交到该组。若坐标换算使轮廓偏移超出亚像素容差，提交会在写入前拒绝。所属层隐藏、锁定或组被移除时，View 回到仍可编辑的最近祖先。

在 Gallery 中，先 Ctrl+点击选择同一容器中的成员，再使用分组按钮。选中整组后可拖动、旋转、等比缩放或取消分组。

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

## 放置和编辑文字

在主视图选择「放置文字」，单击目标位置。Canvas 将文字中心放在点击处，加入当前组或活动图层，并选中新文字。Gallery 属性面板支持显式换行、字号、十六进制颜色和起始／居中／末尾对齐。点击「应用文字属性」后两个视图同步更新。选择框和自动导出区域包含伸出排版框的字形笔画。字号使用文档单位，拖动、变换、编组、层次移动、删除和撤销使用普通绘图项的操作。

宿主可用 `CanvasDocument::create_text` 或 `create_text_in_layer` 创建文字，从 `CanvasText::style` 读取属性，通过 `update_text` 提交修改。`duplicate_text` 将副本添加到原容器，保留局部变换。`draw_order` 包含文字，`shape_order` 只返回矢量形状。每项使用一种字体、字号和颜色，长行保持单行。SVG 导出保留可编辑的文字和内置默认字体，PNG 导出将文字和其他可见项一起绘制。

## 绘制和编辑连接线

在 Gallery 主视图选择「连接线」。依次单击起点、需要的中间路由点和终点，按 Enter 或双击终点提交。两个确认点生成直线，加入中间点后生成手动折线。Esc 取消当前草稿。悬停预览只在绘制视图显示，提交产生一条文档历史记录，并选中新连接线。Gallery 默认使用终点箭头，在开始落点前再次点击工具可切换单向和双向箭头。

在选择模式中点击线段、箭头或标签可选中整条连接线。拖动整体会一起移动自由端点和中间点，附着端点保持在目标形状上。进入节点模式后可分别拖动端点与中间点，每次拖动形成一条历史记录。Gallery 属性面板可应用标签内容、字号和颜色，清空内容后应用可删除标签。

宿主通过 `CanvasDocument::create_connector` 或 `create_connector_in_layer` 创建连接线，并通过 `update_connector` 修改路由、样式或标签。路由坐标以绘图项局部坐标表示，创建时使用所在容器的坐标。样式支持独立的起点和终点三角箭头，以及描边颜色、宽度、端点和连接样式。标签的 `position` 取值为 `0.0..=1.0`，按局部路由的累计长度计算，文字以该点为中心。

`duplicate_connector` 将路线、样式、标签与局部变换复制到同一容器。`draw_order` 包含连接线。连接线可重排、编组、移动图层、删除并撤销或重做。SVG 导出写入描边路径、箭头轮廓和标签文字，PNG 渲染已提交的完整连接线。

启用 `CanvasObjectSnap::connector_anchors` 后，创建连接线或拖动端点时可以捕捉形状的 North、East、South、West 锚点。中间路由点一直是自由点。锚点位置随形状尺寸、局部变换和祖先组变换更新，跨图层和跨组连接使用同一 Document 坐标。拖离锚点范围会解除附着，删除目标则保留端点最后的局部位置。撤销删除可以恢复附着。目标所在图层隐藏期间，连接线一并从 View 和导出中隐藏。路线长度为零时，View 在重合点显示一个可选中的标记，导出会省略该连接线。

宿主可通过 `CanvasDocument::shape_anchor` 查询锚点的 Document 坐标，通过 `set_connector_endpoint` 提交 `CanvasConnectorEndpoint::Free` 或 `Attached`，通过 `connector_endpoint` 查询端点。自由端点的读写使用直属容器坐标。矩形与椭圆提供精确的四向位置，单闭合 Path 在局部轮廓上取相应方向的最外侧交点。其余 Path 可继续使用自由端点。读取 `CanvasConnector::route` 会获得当前解析后的绘图项局部几何。

## 网格与吸附

Gallery 提供主视图的网格显示、网格吸附和对象吸附开关。宿主可以用 `CanvasViewProps::grid` 初始化网格设置，通过 `CanvasView::grid` 查询，通过 `set_grid_in_world` 或 `set_grid` 修改。`CanvasGridSnap` 可分别控制绘制、移动、Path 节点和 Connector 路由点。有效修改提交后触发 `CanvasGridChanged`。默认间距为 25 文档单位，每 4 格突出一条主线，显示和吸附均默认关闭。间距须为正有限值，`major_every` 接受正整数或 `None`。

网格以文档原点为基准。平移和缩放改变可见范围。缩小时，视图按密度选择显示主线或更稀疏的网格线。吸附始终使用完整的基础间距。在 6 个 View 逻辑像素内，绘制点和编辑节点吸附到最近交点。Connector 的路由点开关同时控制创建和编辑。正多边形和星形优先将外顶点吸附到网格，超出容差时按住 Shift 可对齐角度。移动绘图项通常使用其局部原点，Connector 优先使用第一个自由路由点。多选拖动以按下的绘图项为基准，所有成员共享同一位移。

显示与吸附可独立使用。共享 Document 的视图各自维护设置，撤销和重做只回放文档修改。程序化 Document API 保留调用方提供的坐标。网格画在各自视图的图形下方，绘图项继续决定命中、画序及 SVG／PNG 导出内容。

通过 `CanvasViewProps::object_snap` 或 `CanvasView::set_object_snap_in_world`、`set_object_snap` 设置对象吸附。用 `CanvasView::object_snap` 查询当前设置，实际变更会触发 `CanvasObjectSnapChanged`。边缘及中心、Path 锚点和 Connector 形状锚点可分别启用，默认均关闭，容差默认为 8 个 View 逻辑像素。容差须为正有限值。

绘制点、Path 节点、Connector 路由点和移动操作可以捕捉 Path 锚点，也可以在两个轴上分别贴合绘图项边缘或中心线。移动时使用整个选择的文档边界，Path 锚点捕捉沿用抓取项的移动基准点。Connector 端点优先寻找可附着锚点，其余候选按 Path 锚点、对象边缘或中心、网格的顺序判断。同距离优先取前景目标，以稳定标识消解剩余并列。隐藏图层的内容从候选中排除，可见锁定图层仍提供参考。普通对象吸附会排除当前选择。

拖动期间，参考线显示当前对齐的轴。已捕捉的目标有额外的 2 个逻辑像素释放范围。操作结束、取消或修改对象吸附设置后清除参考线。
