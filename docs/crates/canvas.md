# Canvas

Canvas 提供独立 ECS Document、矩形、参数椭圆与原始命令 Path 模型和普通 Bevy UI CanvasView。当前提供矩形、椭圆与多子路径显示、View 平移/缩放与三类图形的真实几何单选，支持三类已选图形整体移动和一次性矩形/椭圆绘制，支持八向整体缩放与旋转，支持独立SVG矢量导出和异步透明PNG导出。

## Document 与矩形

`CanvasDocument` 是无固定边界的业务 Entity，Document 坐标 X 向右、Y 向下，负坐标合法。每个图形 Entity 带有统一的 `CanvasShape` 身份和唯一的 `CanvasGeometry` 几何组件，以 `ChildOf` 归属一个 Document；删除 Document 会级联删除其子图形。模型不需要 Camera、Image、Mesh 或 UI 插件，也不依赖 View 的存在。

`CanvasGeometry::Rectangle(CanvasRectangle { size, corner_radius })` 保存中心对称的局部宽高，均须有限且至少为 1 个 Document 单位；corner_radius为统一局部圆角半径，须有限且在0至min(size)/2内，0保持直角。圆角由派生轮廓生成真实填充与描边Mesh，命中会穿透角落空白；整体变换后的bounds按圆角支持函数计算，SVG使用rx/ry，PNG保持同一几何；当前支持 Rectangle、Ellipse 与 Path 分支，`CanvasRectangle` 本身不是 Component。可通过匹配几何枚举或 `CanvasGeometry::rectangle()` 只读查询矩形数据。官方 `Transform` 是中心位置、旋转与整体 Scale 的唯一权威；使用绕 Z 轴的正规化旋转，XY Scale 非零，允许非均匀 Scale。所有数值须有限，变换后的边界也须可表示。Document 不是渲染 Scene，Document Entity 本身的 Transform 不参与形状几何解释。

`CanvasPaint { fill, fill_rule, stroke }` 中 `fill: None` 表示无填充，`Some(Color::NONE)` 保留指定透明颜色的语义。`CanvasFillRule` 支持 NonZero（默认）与 EvenOdd；同向嵌套轮廓在 NonZero 下填满、EvenOdd 下形成孔洞，反向内轮廓在两种规则下均可形成孔洞，自交按所选规则解释。Rectangle、Ellipse 与 Path 使用相同 Paint 语义。

`CanvasStroke { color, width, cap, join }` 的宽度为正有限的局部 Document 单位，沿边界居中；默认黑色、宽度1、Butt端点与Miter连接。`CanvasLineCap` 支持 Butt / Round / Square；`CanvasLineJoin` 支持 Miter / Round / Bevel，Miter limit固定为4，超过时回退Bevel。开放端点按所选cap延伸，闭合轮廓使用join；零长度线段/闭合子路径在Round/Square下形成圆/方点，只有MoveTo的子路径不描边。Round连接只补外侧圆扇形，不向短线段内侧添加圆盘。Transform 的 Scale 同时作用于填充和描边，颜色转换后的通道必须有限。矩形绘制预览与提交、已有矩形SVG/PNG导出均使用对应样式。

`CanvasDocument::create_rectangle`、`update_rectangle` 和通用 `remove_shape` 提供同步 World CRUD；创建/更新先校验完整数据，更新和删除还校验完整的 Shape 身份及所属 Document，矩形更新核对几何类型。非法请求记录错误并返回 `Severity::Error`，保留原有数据。图形 Entity 即稳定身份，Document 私有顺序序列保存从底至顶的唯一顺序，不复制几何或变换。

模型组件可通过原生 ECS 查询；公开业务写入统一通过 Document API，外部直接修改 Shape、Geometry、Paint 或 Transform 不属于保证一致性的业务接口。BRP 的直接字段修改仅用于调试。`CanvasRectangle::validate` 与 `bounds` 返回校验错误，供数据消费者处理；`corners` 对有效数据给出变换后的四角，`bounds` 计算包含居中描边和整体变换的最小 AABB，均不依赖 GPU。提交后现有消费者从同一几何组件同步显示、选择、手势和导出，不维护另一份可编辑矩形数据。

## 参数椭圆

`CanvasEllipse { radii }` 保留中心对称的两轴半径，`Default` 为半径 1 的圆。半径必须有限且为正，允许小于 1 单位，局部直径、描边外缘和变换后的边界须可表示。`create_ellipse` / `update_ellipse` 沿用同一 Document 所属关系、Paint 与二维 Transform 校验；typed 更新拒绝不同几何类型，不提供图形种类转换。椭圆加入 Document 顺序并与矩形共同显示，多个 View 共享同一图形的 Mesh。

椭圆填充从单位圆派生八段三次曲线的闭合 Path，由 lyon 按 1/4096 的相对容差细分，再映射两轴半径；原半径仍为唯一可编辑几何，派生路径不进入 Document。描边先用 f64 计算两侧等距点，再分别按两轴外缘半径归一化并交给 lyon，避免高偏心率椭圆的短轴或微小轮廓被合并，以及巨大坐标长度计算溢出。达到短轴直径的宽描边已没有内孔，直接对凸外侧等距轮廓做单次填充，按归一化轮廓到弦的最大距离自适应细分，保留高偏心率椭圆端部的圆弧；其余描边由同一自适应采样的内外等距点构造三角带，合并为同向 NonZero 区域再填充，避免半透明重叠混色。仅 Transform、颜色变化不细分，半径、fill_rule或描边width/cap/join改变原地更新缓存 Mesh；光滑闭合椭圆的cap/join不改变可见轮廓。Renderer 缓存并使用实际 Mesh AABB，包含曲线近似和描边细分的外延。共享缓存同时保存实际顶点的局部min/max供命中预筛选及选择框使用，不以理想椭圆边界排除近似曲线的外露边缘，也不从中心/半长重建远子路径的最小坐标。

`CanvasEllipse::bounds` 返回理想椭圆及其居中描边经二维 Transform 后的 AABB，用 f64 计算支撑长度避免中间平方溢出。当前椭圆支持程序化 CRUD、显示、单选、删除及整体移动/旋转/缩放；椭圆交互绘制和参数控点尚未提供。SVG/PNG 当前只支持矩形，Document 含椭圆时返回 `UnsupportedGeometry`，在打开文件及分配导出资源前拒绝整个请求，不省略椭圆生成不完整文件。

## 原始命令 Path

`CanvasGeometry::Path(CanvasPath { commands })` 保存 `MoveTo`、`LineTo`、`QuadTo`、`CubicTo`、`ArcTo` 和 `Close` 原始命令；`CanvasGeometry` 因可变长命令数据提供 Clone，不再提供 Copy。`create_path` / `update_path` 沿用 typed CRUD、Document 所属和统一顺序，不能自动转换几何种类。一个 Shape 可同时包含开放/闭合的多个独立子路径，共用 Entity、Paint、Transform 和叠放位置。Path 必须非空并从 MoveTo 开始；控制点和端点必须有限。Close 后可以继续 MoveTo，也可以从刚闭合的起点继续绘制；没有开放子路径时重复 Close 被拒绝。

ArcTo 使用 SVG 端点语义：`radii` 为非负局部两轴半径，`rotation` 为弧轴旋转弧度，`large_arc` / `sweep` 保留长短弧及方向；零半径等价 LineTo，同端点不增加弧段，过小半径按 SVG 规则扩大。原始弧命令不改写；双精度计算的派生三次曲线只用于显示及边界，短弧以端点为参考计算，避免巨大半径的中心/角度相减消去。

Path 显式传递所选NonZero/EvenOdd填充规则，支持Butt/Round/Square端点和Miter/Round/Bevel连接；默认NonZero、Butt、Miter与miter limit=4。填充和描边仍分别只绘制一次；描边凸片合并为同向 NonZero 区域，避免自交/子路径重叠时半透明重复混色。各条曲线在自身按两轴归一化的坐标内细分，局部线段描边从中心线向两侧分别用双精度构造凸片，保留极端宽长比下的近处锚点和Butt端面；外侧连接同样使用凸片。极端宽长比先按零坐标平面分区，避免GPU裁剪相反巨大端点时丢失短边，中心线长度同时约束连接容差。派生包围范围相交的子路径共同计算填充/描边覆盖；尺度悬殊的重叠区域先用f64裁剪成局部绘制区域，再按原始轮廓方向及所选填充规则细分，保留小孔洞及交叉描边单次混色。单独凸片可直接细分；宽描边为避免过度分区按宽度约束预算，检测到平移归一化合并可表达坐标时，仅在对应轴使用保留零原点的二进制缩放，并按实际缩放后特征尺寸约束lyon连接容差，保护局部透明间隙。未回退时也按外露顶点到非相邻边的实际间隙约束连接容差，仅当点到边的整个连接区间已经被描边union覆盖时视作已有连接，不因两个凸片在别处共享角点而忽略实际间隙。所有局部片段最终仍属于同一Shape的共享填充/描边Mesh。连接只补外侧角，内侧由线段描边本身形成。`bounds` 保守包含曲线控制包围盒、所选端点/连接的描边外延及整体变换，不宣称最小 AABB。

多个 View 共享同一 Path 的填充/描边 Mesh；命令、fill_rule或描边width/cap/join变化原地刷新，纯 Transform/颜色更新不重新细分。缓存保留不可变的原几何快照，View 代理只持有自身材质及共享 Mesh，不重复保存可编辑命令。Path代理使用专用View层与GPU裁剪，避免极远子路径造成共享AABB的CPU视锥裁剪舍入误判。当前 Path 提供程序化 CRUD、显示、真实几何单选、删除及整体移动/旋转/缩放；节点编辑及通用 SVG/PNG 导出由后续增量接入，含 Path 的导出返回 UnsupportedGeometry，保留已有目标文件。

可重复的 Headless Benchmark 见 `crates/canvas/benches/path_views.rs`，覆盖100/1000条多子路径、两个选中同一Shape的View及单图形命令/Transform更新后的完整App帧；transform_only使用update_transform交替平移、旋转、非均匀/镜像Scale，包含双选择框刷新。共享Paint Mesh有界，选择框专属Mesh各一；不包含GPU与窗口呈现。

`CanvasDocument::append_rectangle(world, document, shape, rect)`、`append_circle(..., center, radius)`、`append_ellipse(..., center, radii)`、`append_line(..., from, to)`、`append_polyline(..., points)`和`append_polygon(..., points)`向现有Path追加局部坐标轮廓。每次以MoveTo开始新子路径；线/折线开放，矩形/多边形/圆椭圆闭合，圆椭圆保留两个ArcTo半弧。矩形Rect需有限正宽高，圆椭圆需有限中心和正有限半径、能精确表达center.x±radii.x的水平端点（否则拒绝，避免SVG半径修正或弧心偏移）；折线至少2点，多边形至少3点，所有点必须有限。线段允许零长度，矩形不受参数Rectangle的1单位尺寸下限限制。

追加复用原Path的Entity、Paint、Transform及Document顺序，不创建参数图形或来源元信息；生成命令可继续通过普通Path更新编辑。完整身份/所属/种类、参数和整体Path校验后才提交，拒绝时记录错误并保留全部原数据。`crates/canvas/benches/path_append.rs`测量100/1000/10000条原命令上的矩形追加与公开API恢复成本，包含两次整体校验，工作量不随样本累积；不包含渲染。

## Document 叠放顺序

新建图形追加到顶部；`CanvasDocument::shape_order(world, document)` 返回从底至顶的图形 Entity 序列，并同步修复失效或不再归属本 Document 的成员。`reorder_shape(world, document, shape, CanvasOrder)` 支持 Raise（上移一位）、Lower（下移一位）、Front（置顶）与 Back（置底）；实际改变返回 true，边界合法同值返回 false，无效 Document、实体或所属关系明确拒绝。删除图形同时移除顺序条目；各 View 共享这份业务顺序。

View 画面、从顶到底的命中、SVG 与 PNG 快照均使用此顺序。每个图形的填充后接自身描边，后方图形的描边会被前方图形的填充遮挡。原生 Transform 的 Z 值保留，但不再影响图形先后关系或导致相机裁掉图形。View 和 PNG 将业务顺序投影为有界渲染深度；选择轮廓及未提交预览置于业务图形之上，不属于 Document 顺序。不提供 Layer 或 Group。

## Canvas View 与渲染

安装 `CanvasPlugin` 后，通过 BSN `@CanvasView { @document: {Some(document)} }` 构造并使用普通 Node 设置布局。Facade 默认启用 `canvas` Feature，通过 `bevy_widgetry::canvas` 导出公共 API，也可仅启用该 Feature。Props 仅初始化引用；`CanvasView::document` 查询，`set_document_in_world` / Commands `set_document` 更新。有效同值不通知，实际提交后发送 `CanvasDocumentChanged`；无效 View/Document 返回错误。None 可显示空画布。

每个 View 有独立 Camera、Image 和全局唯一离屏层号，使用 Core `offscreen` 的维护和回收机制。Camera 透明清屏，采用 4× MSAA；需要背景时可在 UI Node 设置 BackgroundColor，不写入 Document。View 中心默认对应原点，Zoom=1 时 1 Document 单位等于 1 UI 逻辑像素；目标 Image 使用布局物理尺寸，投影使用逻辑尺寸除以 Zoom。Document 的 Y 向下通过专属绘图容器的一次 Y 反射转换，Document Transform 不变。

填充和居中描边分别使用二维 Mesh 与 ColorMaterial，支持无填充、无描边和透明颜色。描边几何为内外四边连接的环形，过宽描边的内部尺寸钳至零。位置、旋转、整体 Scale 和颜色更新不重新生成几何；局部尺寸或描边宽度变化更新原 Mesh Asset。图形代理在 Transform 传播前更新，相机在布局后、Frustum 检查前更新，避免投影与内容落后一帧。

同一个 Document 可同时被多个 View 引用。每个图形的填充/描边 Mesh 统一缓存，各 View 的 Mesh2d 引用相同 Handle；几何变化原地更新共享 Asset，Transform、Material、代理 Entity、Camera、Image 和层号仍由 View 独立维护。缓存以实际渲染代理引用为准，在最后一个引用消失后回收，不依赖 View 的销毁顺序；之后再次绑定可从 Document 重建。

隐藏或零尺寸时停用 View 相机；恢复后使用当前模型。销毁 View 回收专属 Scene、相机、Image、层号和 Material，Document 与子图形继续存在，也不删除其他 View 仍引用的 Mesh。移除矩形会清理所有 View 的对应代理与不再引用的 Mesh。销毁 Document 后，各 View 自动将引用清为 None，并在提交后发送 CanvasDocumentChanged；UI 实体、布局尺寸、相机和透明目标保持，画面为空，不自动创建 Document。通过公开 API 重新绑定有效 Document 后恢复显示。代理不挂到 Document，模型始终为唯一图形权威。

实际 Bevy 0.20 的二维 RenderTarget 使用预乘 RGB，而原生 Viewport 按 straight alpha 合成，直接显示会对透明颜色再次乘 Alpha。Canvas 因此使用 Bevy 自带 `FullscreenMaterial` 在相机输出前将 RGB 除以非零 Alpha；Alpha 为零时输出全零。该适配只改变颜色表示，保留原生 Camera2d / ColorMaterial / MSAA / Viewport 路径，不负责几何绘制。WESL 由 Asset 的 `BuiltinShader::CanvasStraightAlpha` 提供；不会影响其他 Widget 或窗口相机。

绘图代理只投影 XY Scale，模型的 Z Scale 不参与平面几何，也不缩放填充/描边之间的内部深度偏移；因此负值或零 Z Scale 不会颠倒或压平 Paint 的顺序，模型 Transform 保持原值。

## View 平移

View 观察中心默认是 Document `(0,0)`，可用 Props `center` 初始化，通过 `CanvasView::center` 查询，`set_center_in_world` / Commands `set_center` 更新。中心必须有限，允许负坐标；非法请求保留状态并返回 Error。实际提交后发送 `CanvasCenterChanged`，合法同值不通知。中心仅影响当前 View 的相机，不写回 Document 或其他 View。

有效 Document 的可用 View 支持 Space + 左键拖动。输入从 UI 区域获取，按 UI 相机比例、Viewport 偏移和 Node 变换转换到局部逻辑坐标；内容跟随鼠标，观察中心反向移动。开始后捕获 Pointer ID 和目标，直到左键释放；中途松开 Space 仍保持平移，移出可视区域后仍可继续。普通左键和中键不启动平移。

已有捕获的活动平移按原始输入顺序消费同一 Pointer 在主手势结束前的其他按钮按压，不触发其他控件的点击或焦点切换；其他按钮释放不终止平移。新手势首帧保留 Bevy 的事件派发顺序，不在 Primary Press 实际执行前预测并消费后续输入；Canvas 本身仅响应 Primary 平移。Primary Release / Cancel 之前的位移按原始输入顺序提交，终止之后的新按压仍走真实命中路由。回调取消或转移捕获时，旧手势立即结束，并按本帧 Backend 命中或新捕获 owner 恢复路由。

PointerCancel、焦点转移或窗口失焦、隐藏、Disabled、零尺寸、Document 解绑/销毁及 View 销毁均终止手势并归还捕获。View 使用现有 InputFocus 与 PointerCaptureMap；平移与矩形移动共用按序输入与捕获清理。

## View 缩放

Zoom 默认 1，可通过 Props `zoom` 初始化，`CanvasView::zoom` 查询，`set_zoom_in_world` / Commands `set_zoom` 更新。合法范围为 `[0.1,10]`；NaN、无穷大和范围外值返回 Error，保留原状态。合法同值不通知，实际提交后发送 `CanvasZoomChanged`。程序化更新以视口中心为锚点，保留当前 Document 观察中心。

指针位于可用 Canvas 内且目标窗口有焦点时，垂直滚轮以光标为锚点缩放，无须先取得键盘焦点。Line 单位每单位乘以 `exp(0.1)`，Pixel 每 100 单位对应一个 Line；水平或非有限输入不缩放。滚轮钳至倍率边界，同值不额外修正中心；不可用或区域外的输入不执行缩放。Canvas 不接管命中其他 Widget 的滚轮。

光标锚点缩放将新 Zoom 与中心一起提交后再通知，中心实际改变时也发送 `CanvasCenterChanged`；通知回调再次改变中心或销毁 View 时不继续发送过期的中心通知。缩放与平移不改变模型 Transform、size、Paint 或共享 Mesh。平移逻辑位移除以当前 Zoom，滚轮不重置活动 Pan；捕获期间的 Move / Wheel 按原始顺序处理。运行时 Window Scale Factor、UiScale、Node 变换和布局变化继续沿用 UI 局部逻辑坐标，窗口比例不写入 Zoom。

## View 单选

每个 View 默认没有 Selection，`CanvasView::selection` 查询，`set_selection_in_world` / Commands `set_selection` 设置一个 Rectangle/Ellipse/Path Shape Entity 或 None。目标必须属于当前有效 Document；无效 View、过期 Entity、跨 Document 或缺失完整 Shape 身份的目标返回 Error，保留原状态。实际提交后发送 `CanvasSelectionChanged`，合法同值与重复清空不通知。Document 绑定实际改变时一并清空选择；失效或改属其他 Document 的选择不从查询返回，Renderer 下一次同步时清理状态和轮廓并通知 None。修复通知回调可以销毁 View。

普通左键按下从 View 局部 UI 坐标换算到 Document，按 Document 顺序从顶到底检查 Rectangle/Ellipse/Path 的真实填充和描边。包围框仅做预筛选，不是可点击面；Path 的 NonZero/EvenOdd 孔洞、多子路径之间空白和椭圆包围框角落会穿透到下方图形。填充复用共享 Mesh 的实际三角覆盖，fill=Some 即使 alpha=0 仍参与命中，fill=None 的内部不命中。描边在实际 cap/join/width 覆盖外额外保留4个窗口逻辑屏幕像素的容差，投影包含 Zoom、UI Node 变换、UiScale 和窗口比例。Shape 仅投影 XY，Z Scale 不参与命中。任意子路径命中均选择整个 Shape。命中另一个图形切换选择，空白清空；多个 View 的 Selection 独立。点击获取现有 InputFocus，点击其他 Widget 后焦点归实际目标；命中嵌套子 Widget 时 Canvas 不冒领选择或 Pan。Space+左键优先平移，保留选择；首次按在尚未选中的图形上只改变选择，该次按压后移动不会修改模型。三类图形均可在再次命中时移动，整体控制柄的命中独立于复杂Path的填充区域。

各类图形的选择框使用实际共享填充/描边 Mesh 的局部边界并应用整体 Transform，沿含描边外缘的旋转边界绘制，外留 3 个 UI 逻辑像素间隔，线宽保持 2 个逻辑像素，不随 Zoom 变粗。轮廓属于 View 离屏 Scene 的独立 Mesh/Material，不修改 CanvasPaint，不是 Document 子图形；不属于导出数据。取消选择或销毁 View 会回收其专属资源。顶部外侧另有独立旋转点。Rectangle/Ellipse/Path共用整体缩放与旋转控制柄；这些操作只提交Transform，保留局部Geometry。

## 已选图形整体移动

后续新的左键按下命中当前已选Shape时捕获该 Pointer，从此刻的 Document 坐标记录中心抓取偏移；移动时以当前 View 中心、Zoom 和局部 UI 坐标求出新的 Document 中心，只修改该图形的 Transform.translation，Z、rotation、scale、局部Geometry与Paint 保持。首次选择时不启动移动，Space+左键优先 Pan；一旦手势开始，随后按下/松开 Space 不改变用途。拖到 View 外继续，滚轮缩放后保留抓取点，以新 Zoom 换算后续位移。

移动更新共享 Document，所有 View 的图形和选择轮廓同步跟随，填充/描边 Mesh Handle 保持；其他 View 的 Pan/Zoom/Selection 不变。移动继承 Pan 的同 Pointer 非 Primary 按压隔离及按原始顺序提交终止前最后位移的契约。

Primary Release、PointerCancel、Focus 转移、Window 失焦、隐藏、Display::None、Disabled、零尺寸、Document 解绑/销毁、图形失效/改属其他 Document、Selection 改变或 View 销毁均终止移动并归还自身捕获；不释放其他 owner 的捕获。终止后不恢复旧手势，也不回滚已提交的模型位置。

## 矩形与椭圆绘制

View 默认 `CanvasMode::Select`；通过 `CanvasView::mode` 查询，`set_mode_in_world` / Commands `set_mode` 切换到 `CanvasMode::Rectangle(CanvasPaint)` 或 `CanvasMode::Ellipse(CanvasPaint)`。Paint 预先校验，非法 View/Paint 返回 Error 并保留原模式；合法同值不通知。实际改变时先提交、取消当前 Pointer 手势，再发送 `CanvasModeChanged`。模式和 Paint 属于当前 View，不改变其他 View 的工具。

矩形/椭圆模式下普通左键按下记录 Document 起点，移动仅更新当前 View 的对应类型预览。预览是该 View 离屏 Scene 的独立 Mesh/Material，没有 Document 图形 Entity，不进入共享图形缓存、其他 View 或导出结果。椭圆草稿任意轴小于1个Document单位时不生成预览，避免零半径进入描边细分。Space+左键仍优先平移，开始后用途锁定；中途通过 API 切换模式会取消旧手势，不能转换为另一个操作。

左键释放时以起止点的 min/max 构造正尺寸和中心，支持四向拖动；宽高均至少 1 个 Document 单位且有限才创建。Rectangle保持局部宽高，Ellipse保留两轴半径（拖动范围的一半），不转换为Path。新图形进入Document顺序顶部，其他View同步显示且保留各自选择和视角。无效单击不写 Document；有效提交后自动选中新 Entity，并返回 Select。按 Esc 丢弃预览并退出绘制模式；普通 PointerCancel、失焦、不可用 View 或 Document 解绑/销毁取消预览并释放捕获，不提交半成品。预览在完成、取消和 View 销毁时回收专属 Mesh/Material。模式切回 Select 的通知回调若销毁/解绑 View 或再次切换模式，旧提交不继续创建图形。

## 八向整体缩放

三类已选图形的含描边外缘显示四角与四边中点共八个方形控制柄；视觉边长固定8个UI逻辑像素，命中半径6个逻辑像素，优先于内部移动，重叠时选最近柄。空间平移和Rectangle工具保持既有优先级。整体框取共享网格实际局部边界，不把复杂Path的空白框当作填充命中区域。

按下时保存原Transform、局部边界、控制柄和Document指针位置。后续位移按原二维变换求逆，角柄改变两个Scale分量，边柄只改对应分量，必要时调整translation以保持相反边/角的Document位置；rotation及Z保持。允许跨固定点形成镜像，恰为零或向零舍入时将Scale绝对值钳到f32最小可表示非零值以保持可逆；零跨度轴保留原Scale。Rectangle.size、Ellipse.radii、Path原始命令与Paint均不改变，局部Mesh无需重新细分，描边与整体Scale一起变化。

`CanvasDocument::update_transform(world, document, shape, transform)`为三类图形提供同一同步入口，完整校验身份/所属、现有Geometry与Paint以及新Transform后只提交Transform；无效请求记录错误、返回Error且保留原数据。整体交互复用此入口，不生成几何转换或新Shape。

双View同步看到整体变换与选择框，自有Pan/Zoom/Selection保持。Wheel保持抓取关系，后续输入使用当前View坐标。Release完成；PointerCancel、失焦、隐藏、Disabled、零尺寸、解绑、选择/图形失效及View销毁均终止并释放自身捕获，保留已提交Transform，不恢复旧手势，也不增加撤销或Esc回滚。

## 旋转与角度吸附

第九个控制点是顶部边中点外侧的圆点，以连接线与尺寸点区分；沿实际变换后的顶部外法向偏移 24 个 UI 逻辑像素，圆点直径固定 8 个逻辑像素。旋转点先于尺寸点和内部移动命中，同样只属于当前 View 的编辑覆盖物。Space Pan 和 Rectangle 模式仍保持优先级；已开始的旋转不会因随后按 Space 改为平移。

三类图形的旋转手势记录原Transform与按下时围绕整体选择框中心的方向，根据当前Document指针方向更新Transform.rotation；选择框中心、局部Geometry、scale、Z和Paint保持。Path的局部边界中心不等于Transform原点时，同时调整translation以保持旋转中心固定。跨越 ±180° 或整圈时按方向连续显示，局部填充/描边 Mesh 不重建，所有 View 看到相同旋转，各自导航与选择不变。按住任一 Shift 将绝对角度吸附到 15° 的整数倍；按下/释放 Shift 时，即使指针静止也重新计算吸附/自由角度。

旋转继承既有 Pointer 捕获与失效清理，不添加枢轴或撤销系统。取消保留已提交的rotation及必要translation；Selection/Document/图形失效或 View 不可用后停止更新并归还自身捕获。

## 删除

键盘焦点精确落在可用 CanvasView 根且输入窗口有焦点时，Delete 删除当前有效选择的 Document Shape。焦点位于 TextField、其他控件或 Canvas 子控件时不接管 Delete；未选中时无操作。所有引用同一图形的 View 在正常同步中清空失效 Selection 并通知 None，针对已删除图形的手势终止并归还捕获；代理和最后引用的 Mesh 回收，不修改其他图形、View 中心或 Zoom。删除最后一个图形后 Document 与 View 保持，空画布可以继续绘制。

## SVG 矢量导出

`CanvasDocument::export_svg(world, document, path, region)` 同步生成并写入透明背景 SVG，合法导出直接覆盖目标文件。`region: Option<Rect>` 为 Document 坐标裁剪区域；None 自动合并所有已提交矩形包含居中描边及整体旋转/非均匀Scale的最小AABB，负坐标保留为viewBox原点。显式区域必须有限且正宽高；图形与区域不相交时合法输出透明区域。

SVG通过rect与二维matrix保留中心、局部尺寸、Transform、无填充、填充/描边Alpha及居中描边；图形按Document顺序输出，每个图形的填充及描边连续，与View相同；SVG根显式声明color-interpolation="linearRGB"，按Canvas的线性Alpha合成语义输出。部分SVG消费者（包括当前resvg）忽略该属性，其重叠半透明颜色可能不同；导出保留标准声明，不将这类渲染器结果当作Canvas颜色真值。输出不依赖任何View，不包含背景、选择控制点或未提交预览。`CanvasExportError` 区分EmptyDocument、InvalidDocument、InvalidGeometry、InvalidRegion、UnsupportedColor和Io；SVG仅接受转换后sRGB及Alpha通道均在[0,1]的颜色，模型允许但SVG无法等价表达的HDR/负通道/越界Alpha在写文件前拒绝，不预先钳制源色。Document为空时即使指定区域仍报EmptyDocument。完整几何/区域校验先于打开目标文件，因此校验失败不创建或截断文件；I/O失败返回错误。宿主将该类型错误交给Bevy时应显式使用`BevyError::error`。

## PNG 离屏导出

`CanvasDocument::export_png(world, document, path, CanvasPngOptions { region, scale })` 在完整Document/区域/倍率校验后返回请求Entity。默认region与SVG相同，scale默认1；像素宽高分别为区域尺寸乘倍率后向上取整，投影映射完整指定区域，分数边界不截断。倍率须有限且为正；目标须满足实际RenderDevice纹理和Buffer上限，并通过包含4×MSAA、读回与CPU编码副本的512 MiB工作集估算（每像素80字节加对齐读回）。非法请求不分配导出目标，不创建或截断目标文件；空Document即使显式region也返回EmptyDocument。需要已安装CanvasPlugin并有实际GPU RenderDevice，否则返回RendererUnavailable。CanvasPlugin自动复用或安装App共享的JobManagerPlugin；Manager停止接纳后返回Job(ShuttingDown)，不分配导出渲染资源。

请求快照只含已提交矩形，后续Document修改不改变本次输出。每个请求从Core公共离屏层分配独立Image/RenderLayers，使用专用Camera与几何代理重新栅格化，4×MSAA及StraightAlpha转换保留透明RGBA。图形按Document顺序排列，填充后紧接自身描边，再映射为导出专属深度，不因模型Z超出相机范围而丢图形。独立于View是否存在、Pan/Zoom、背景、编辑控制点和窗口比例；已有View的相机、目标及导航不被修改。

RenderApp确认Mesh/Material/Image及后处理管线就绪并完成渲染后，才对导出Image使用Bevy异步读回；不是现有View或窗口截图。同一次导出登记一个CPU Job，执行体等待ECS/GPU截图交接，在后台完成RGBA的PNG编码/直接覆盖写盘，再等待必要资源清理；不另建截图Job或编码Job。请求Entity携带只读`CanvasPngExport`，可通过`job_id()`查询唯一JobId并请求Manager取消。收到该Job的Finished后发送`CanvasPngExportFinished { entity, job_id, document, path, result }`，result为实际写盘成功的像素尺寸或CanvasExportError。事件内可查询真实Job终态，通知后自动移除执行记录。请求提交成功不等于文件完成，应用须监听这个完成事件；读回前后GPU等待超过30秒报Render错误，编码/写盘等待实际任务结果。完成或失败后请求Entity自动移除，专属Image/Scene/Camera/Mesh/Material和层号回收，完成回调可自行移除请求Entity。

准备或尚未提交截图时可确认取消并清理；GPU读回已经提交时保持Stopping等待实际读回，不能提前销毁Image/Scene或复用层号。GPU等待超过30秒将业务结果标记为Render错误，已提交读回仍需结束并释放资源后才得到Failed及一次完成通知；若底层一直不返回，须由宿主应用的全局关闭超时决策处理。迟到或重复截图核查原请求Entity、JobId、截图Entity和阶段，不重复编码或写盘。请求Entity提前销毁会请求取消，私有收割状态继续持有必要资源直至读回/编码实际结束，并发送一次原身份的完成事件。

编码开始前响应取消且完成清理后调用acknowledge_cancel，Job为Canceled，事件返回CanvasExportError::Canceled；不可中断的同步编码/写盘已经完成时，晚取消仍为Completed，真实编码或写盘错误仍为Failed。失败或取消不伪装成功；SVG继续同步导出，不登记Job。Pending期间在Last发送RequestRedraw，确保无新输入时GPU准备、清理及最终通知仍可推进；所有导出收尾后停止额外重绘。

可重复的DX12导出Benchmark见`crates/canvas/benches/png_export.rs`：64个已提交矩形，512/2048像素方形目标，测量请求到渲染、读回、编码写盘、资源回收及完成通知的整个流程。GPU初始化在计时外，包含轮询等待；不代表Native Window呈现帧时间或60 FPS验收。

顺序模型与大规模重排的可重复 Benchmark 见 `crates/canvas/benches/document_order.rs`，覆盖 100/1000/10000 个图形，交替置顶/置底包含实际数据移动及一次公开顺序查询；初始化在计时外。该结果仅表示 Headless Document 操作成本，不代表 GUI 呈现帧时间。

椭圆双 View 的可重复 Headless Benchmark 见 `crates/canvas/benches/ellipse_views.rs`，覆盖 100/1000 个椭圆及两个 View，包含普通椭圆描边、高偏心率宽描边以及非均匀缩放下的高偏心率薄描边，分别测量单图形 Transform-only 更新或半径更新后的完整 App 帧；初始化与首次细分在计时外，不包含 GPU 提交和窗口呈现。

## Gallery 展示

Gallery 的 Canvas 页面通过 Facade 安装 CanvasPlugin，并组合外部工具栏和两个引用同一 Document 的 CanvasView。演示页面拥有 Document 并在离页时清理；视图的导航/选择和绘制预览各自独立。导出区域/倍率、目标路径与异步完成反馈由 Gallery 管理，详见 [Gallery Canvas](../gallery.md#canvas)。

真实 Picking 点击与完整 Headless 帧的可重复 Benchmark 见 `crates/canvas/benches/selection.rs`：100/1000条重叠的带孔洞Path及底部Ellipse，双View中交替点击孔洞与环边，覆盖穿透全部Path和直接命中顶部Path，不包含GPU提交和窗口呈现。


## 参数图形控点编辑

Select模式下，Rectangle/Ellipse在整体方点内侧另显示八个菱形参数尺寸点，矩形左上角内侧另有小圆点编辑统一圆角半径。菱形向中心内缩14个逻辑像素，小圆点内缩26个逻辑像素，菱形至多占控制点到中心距离的35%、圆角点至多65%，避免小图形控点越过中心。参数点直径8、命中半径5个逻辑像素；旋转点优先，整体Scale与参数点按距离共同选最近点（相同距离时保留Scale），参数点重叠时选最近点，Path不显示参数点。工具栏无需切换到另一种编辑模式。

尺寸拖动按按下时的二维Transform求局部位移，角点修改宽高/两轴半径，边点修改一个分量，调整translation保持相反几何边/角位置；rotation、XY/Z Scale、Z位置、Paint及Document顺序保留。几何尺寸钳至1个Document单位（Ellipse半径0.5），不因尺寸过零镜像或转换类型；原有整体Scale仍可镜像。缩小矩形时圆角钳至新尺寸上限；圆角拖动按局部XY位移平均值调整并钳在合法区间。通过Document typed API实时提交，所有共享View复用并更新原Mesh句柄，选择覆盖层同步；纯Transform操作仍不重新细分。

参数手势沿用Pointer身份/目标与独占捕获。Release结束，普通PointerCancel、失焦、隐藏/禁用、换绑、删除/失效或选择清空清理捕获并停止后续更新，保留已经提交的几何与变换。不新增Esc回滚。
