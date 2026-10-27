# Canvas Integration Test Coverage

## Model

`model.rs` 覆盖没有 UI/渲染插件的纯 World 模型：多个矩形共享 Document、负坐标、原生 Transform 更新、无填充与透明填充语义、更新与移除、Document 的 ChildOf 级联清理。

`shape.rs` 覆盖公共 CanvasShape 身份与单一 CanvasGeometry 权威、typed 矩形更新及通用删除；跨 Document 和缺失身份请求拒绝后保留旧几何、Transform、Paint 与 Entity 数量。现有交互、显示与导出测试统一从 CanvasGeometry 读取矩形，保持原有回归契约。

非法尺寸、Paint、Transform、无效 Document、已删除图形和跨 Document 请求应返回 Error Severity 与必要诊断，不创建额外 Entity，不修改旧几何与 Paint。纯几何测试覆盖居中描边、旋转及非均匀整体 Scale 的 Document AABB 和四角；局部尺寸下限为 1 单位。

## 参数椭圆

`ellipse.rs` 覆盖参数椭圆的微小正半径、非法/非有限半径、变换和边界溢出、跨 Document 与几何类型更新拒绝、提交失败不部分写入、共享身份/顺序及删除；椭圆与矩形混合时尚未接入的 SVG/PNG 导出明确拒绝并保留原文件。`renderer.rs` 覆盖双 View 共享椭圆 Mesh、原地半径更新、Transform-only 无 Mesh Modified、源参数与几何类型保持、真实网格 AABB 和资源回收。

## Document 叠放顺序

`order.rs` 验证创建追加、相邻交换、置顶/置底、边界 no-op、无效/跨 Document 拒绝、删除/外部失效修复及序列唯一性；Transform.z 保留且不影响顺序。`selection.rs` 的双 View 真实 Mouse/Custom Picking 验证按序重排后的顶部命中、自有选择保持及每个图形的填充/描边连续的代理深度。`svg.rs` 验证线性颜色声明与连续 Paint 次序；`png_gpu.rs` 以真实 DX12 输出和 SVG 栅格像素对比，覆盖前方填充遮住后方描边、重排反转遮挡、模型 Z 与顺序无关、提交后的快照保持原顺序。

## Renderer 与 View API

`renderer.rs` 通过真实 BSN、UI Layout、Transform/Visibility 调度验证：1/1.5/2 倍窗口比例下的逻辑尺寸与物理目标、关闭 rounding 后的分数像素布局、同帧布局与相机矩阵更新、Document Y 方向转换、4× MSAA 配置、填充与居中描边的顶点/索引、变换同帧传播与几何复用、None 填充隐藏、尺寸/描边修改、隐藏恢复、持续帧资源有界和 View 销毁不删除 Document。

Document 引用 API 覆盖同步/Commands、合法同值无通知、状态先提交后通知、排队执行时 Document 已失效的拒绝与宿主错误传播。真实 GPU 的 Alpha/MSAA/边缘表现由任务 BRP 验证，不由 Headless 配置断言替代。

变换回归覆盖模型 Z Scale 为负、零及正值时描边始终位于填充之前，XY 镜像不重建几何，模型 Z Scale 不被改写。

多 View 生命周期覆盖共享 Mesh Handle、独立 Camera/Image/RenderLayers、几何和变换同步、隐藏恢复与 View 重建、解绑后仅回收最后一个引用、删除图形与 Document 后的资源清理。Document 删除后保留 View 和布局、引用自动清空并允许重新绑定；隐藏 View 同样修复。DocumentChanged Observer 可以销毁 View，Renderer 不访问失效实体。真实跨 View 画面同步、透明目标清空由任务 BRP 验证。

## View 平移

`input.rs` 经真实 Picking PointerInput 验证 Mouse/Custom Pointer 的 Space+左键捕获、局部坐标平移、松开 Space 后手势锁定、移出 View 后继续、Release 后停止；普通左键与中键不平移，其他 Pointer 不接管手势。覆盖 Window Scale 1/1.5/2、UiScale、UI 相机 Viewport 偏移、旋转/缩放 Node 的坐标转换；独立 View 中心与 Document Transform 不变，离屏相机 GlobalTransform 同帧更新。

取消覆盖 PointerCancel、隐藏、Display::None、零尺寸、Disabled、Window 失焦、InputFocus 转移、View 销毁及 Document 销毁，捕获归还且状态不再更新。中心 API 覆盖同步/Commands、有限值校验、合法同值无通知、状态先提交后通知和错误日志。

输入顺序回归覆盖捕获期间在 View 外松开同 Pointer 的右键/中键仍继续平移；同帧 Move → Primary Release / Cancel 先应用最后位移再结束，避免 Bevy 提前清除 Capture 导致丢帧。相同 Pointer 可在释放 A 后转到 B 并独立平移。

同帧 Primary Release → 移动至 B → Primary Press 按真实命中启动 B，不能把新按压继续路由到 A。

同帧 Secondary Press → Move → Primary Release 仍先提交旧主手势的最后位移，焦点路由变化不应丢弃已发生的移动。活动手势按原始输入顺序提交，Primary 终止后的新手势及其同帧移动由常规 Picking 接管。

已有捕获的活动平移在终止之前，同 Pointer 的非 Primary 按压被消费为 no-op，不能触发 B 的 PointerPress；终止之后的非 Primary 按压仍由真实目标 B 接收。

CenterChanged 回调转移 InputFocus 或 PointerCaptureMap owner 时，旧手势立即停止处理后续动作；不吞掉真实目标 B 的后续按压，不释放新 owner 的捕获。

通知回调取消的接收者回归同时覆盖有/无Primary Release的帧，避免捕获强制HoverMap残留旧View。首帧Press→Move→Secondary Press保留真实事件，不因预判的手势状态吞掉回调取消后的按压；覆盖Mouse/Custom及Focus/Capture两种回调转移。

首帧PointerDrag通知回调转移捕获后，下一帧仍发送A的PointerOut与B的PointerOver，不能提前覆盖HoverMap而跳过迁移。

从View外移入并在同一帧Primary Press→Release且没有Drag时，帧末归还捕获，未按键的后续Move不平移；覆盖Mouse/Custom，不能依赖发给上一帧命中目标的PointerRelease。

首帧移入→Press→Release→无键Move→Press→Move时，新按压从自身位置建立起点，不把两次按压之间的无键位移算入平移；后续保持与释放仍正确。

## View 缩放

`input.rs` 覆盖 Zoom World/Commands API 的中心锚点、默认值、合法同值、有限范围拒绝、错误日志与宿主传播、多 View 隔离。真实 Mouse/Custom Wheel 验证光标下 Document 点在连续与反向缩放后稳定、极值不额外 Pan、Window/UiScale 与 Viewport 转换、运行时窗口比例变化及旋转缩放 Node、Pixel/Line 灵敏度、Zoom 与中心先提交后通知。活动 Pan 使用新倍率且不重置，同帧 Move→Wheel→Move 按原始输入顺序处理。隐藏、Disabled、失焦、解绑、零尺寸、Display::None、区域外和命中其他 Widget 的滚轮不缩放，其他目标仍收到 Scroll。

`renderer.rs` 验证不同倍率下逻辑投影、目标纹理保持布局物理分辨率、运行时比例/布局变化、多 View 倍率隔离及 Mesh Handle/顶点、Document Transform 不变。GPU 表现和真实窗口比例切换由任务 BRP 验证。

嵌套Widget回归通过真实Picking命中Canvas子Button：子级收到Scroll后即使继续冒泡，祖先Canvas也不缩放或截断传播；不仅覆盖并列Widget。

## View 单选

`selection.rs` 覆盖 View 自有 Selection 的同步/Commands API、合法同值/清空、先提交后通知、跨 Document 拒绝及宿主错误传播；解绑清空。真实 Mouse/Custom Picking 覆盖旋转、XY 镜像/非均匀缩放且 Z Scale=0 的矩形、空心内部穿透、描边外缘和外侧空白；Window/UiScale 与 Zoom/Pan 坐标换算、两 View 不同选择、Space 优先平移且保留选择、首次按压选择后拖动不移动模型。嵌套 Button 保有真实 InputFocus，Canvas 不接管其按压或平移。隐藏/Disabled/失焦/零尺寸/Display::None/解绑守卫，以及模型改属其他 Document 后无悬空查询。

`renderer.rs` 覆盖选择轮廓与实际旋转边缘平行、不同 Zoom 下逻辑线宽保持、模型/Mesh/Paint 不变、取消选择和 View 销毁的资源释放、多个 View 的轮廓资源隔离；图形移除后的选择修复通知允许销毁 View，不访问过期实体。`common/` 仅为 Pointer 测试共享真实 UI/Picking fixture，不提供生产 API。

## 已选矩形移动

`movement.rs` 通过真实Mouse/Custom Picking覆盖首次选择不捕获/不移动、再次按下已选图形才移动，抓取偏移、Zoom 0.5/1/2、Window比例与UiScale、Viewport坐标，移动中按下/松开Space用途锁定、移出区域继续、释放后停止。原模型rotation/scale/Z/size/Paint和共享Mesh Handle保持，两个View的代理同步且其Pan/Zoom/Selection独立。

取消覆盖PointerCancel、隐藏、Disabled、Window失焦、View/Document/图形销毁、InputFocus转移、Display::None、零尺寸、Document解绑、Selection清空，停止后不再改模型。按原始输入顺序覆盖Move后非Primary释放仍继续、同帧Move→Primary Release/Cancel保留最后位移；移动中Wheel锚点不跳且后续位移使用新Zoom。共用手势路由的细粒度同帧Focus/Capture交接回归继续由`input.rs`保护；GPU画面及交互帧时间由任务BRP验证。

## 预览式矩形绘制

`drawing.rs` 覆盖Mode同步/Commands API、Paint与View校验、同值无通知、先提交后通知、模式互不覆盖。真实Mouse/Custom输入验证四个方向、Zoom/Window比例、Wheel锚点、按下/移动时Document不变且仅当前View有预览、释放后创建/自动选择/返回Select，另一View的Selection保持。首帧从View外进入后同帧Press→Move→Release仍提交。

无效小于1单位的单击、Esc真实KeyboardInput、PointerCancel、隐藏、Window失焦、View销毁、模式切换与Document解绑均不提交，预览Entity/Mesh/Material和捕获均回收。Space优先Pan且中途松开仍不改为绘制；普通取消保留工具模式，Esc和有效/无效释放回到Select。真实预览仅当前View可见和释放后双View同步由任务BRP验证。

首帧Move入View→Press→Release无Drag仍退出Rectangle模式，并覆盖同帧再Press/Move/Release不能沿用旧绘制。完成通知切换Paint并在同帧重新Press时，复用预览同步填充/描边颜色、Alpha模式和描边宽度，覆盖不透明→半透明/无填充；不保留上一手势的渲染数据。ModeChanged的Deferred回调可以销毁/解绑View，旧完成路径不创建过期图形。

## 八向整体缩放

`scaling.rs`通过Mouse/Custom真实Picking覆盖矩形八个整体控制柄、旋转/非均匀/镜像Scale、Z Scale=0/2、Zoom=0.1/0.5/1/2/10、Window=1/1.5/2及UiScale。验证Scale分量与固定对边/角，局部size、rotation、Z及Paint保持；双View代理同步且Pan/Zoom/Selection独立。跨固定点允许镜像，Wheel保留抓取，PointerCancel释放后保留已提交Transform。

`transforms.rs`验证Rectangle/Ellipse/多开放闭合子路径Path的update_transform事务校验、真实先选后拖、整体非均匀/跨零镜像缩放及固定角、围绕实际选择框中心旋转，原始Geometry/Paint/顺序与共享Paint Mesh不变；零高度Path只缩放有跨度轴。Ellipse/Path的Move/Scale/Rotate分别覆盖PointerCancel、Focus转移、Window失焦、Display::None、解绑、图形删除、选择清空、隐藏、Disabled及View销毁后的捕获释放和已提交变换保留。矩形共用输入的顺序、滚轮、Space优先及Shift回归保留在既有测试中。

## 旋转与15度吸附

`rotation.rs`以Mouse/Custom真实Picking覆盖独立旋转点、原中心方向差、rotation独立更新，Window/UiScale/Zoom和XY镜像/非均匀Scale、Z Scale零值；中心/局部size/Scale/Z/Paint和填充/描边Mesh Handle保持，多个View模型代理同步且自有状态独立。方向跨越175/185/195/350/355/365度保持物理旋转连续，Selection清空后停止。

真实KeyboardInput覆盖左右Shift在指针静止时15度吸附和松开恢复自由角度，Wheel锚点及PointerCancel清理。Renderer的57顶点覆盖物包含8方形尺寸点、旋转连接线和12边圆点，圆点直径固定8个逻辑像素。实际GUI控点表现、修饰键锁定与帧时间由任务BRP验证。

## 删除

删除测试通过真实KeyboardInput/FocusedInput与Mouse/Custom捕获，覆盖精确Canvas根焦点、子控件焦点不冒领、隐藏与窗口失焦守卫、空选择不通知；删除共享图形后双View通知None时模型已不存在，捕获释放且Mesh/轮廓回收，其他图形和View导航保持；删除最后图形后Document/View继续存在。已有Renderer测试保护外部CRUD删除和通知回调销毁View。真实WidgetryTextField嵌套Canvas的logical Home/Delete输入将abc删为bc，所选图形保留；TextField库仅作为测试依赖。BRP的Delete为Unidentified，GUI只能验证不误删Canvas模型，不能冒称文本删除通过。

同帧回归将键盘Dispatch安排在PointerInput::receive之后、Canvas消费原始输入之前：Move→Primary Release与Delete同帧时保留Pan最后位移，Rectangle预览仍完成提交；Delete仅立即停止针对被删图形的Move/Resize/Rotate。

## SVG 文件导出

`svg.rs` 在无View/GPU的World验证Document负坐标、旋转/非均匀镜像Scale、Z Scale=0的二维matrix与角点、含居中描边AABB、透明颜色和fill=None；用usvg/roxmltree解析真实目标文件，用resvg栅格结果检查多图形Z次序、Alpha及透明外部区域。显式区域可落在图形外且得到透明输出。校验失败覆盖空Document（含指定区域）、非法/非有限区域、非法几何与已有文件保留；合法覆盖与I/O错误单独验证。文件是唯一公开输出形式。

合成/遮挡回归：解析导出声明linearRGB，以真实输出的颜色/Alpha与绘制顺序计算填充和描边叠加，核对线性合成后的sRGB颜色；同 Z、间隔 0.005 及 ±9000 的图形均按 Document 顺序，保持每个图形的 fill/stroke 连续。当前resvg忽略color-interpolation，其像素断言仅用于几何/Alpha，不用于这项颜色真值。

HDR/负颜色通道与越界Alpha分别作用于fill/stroke时，模型CRUD仍合法，SVG返回UnsupportedColor且原文件字节保持；避免在Alpha合成前钳制源色而改变结果。

## PNG 异步文件导出

`export/png.rs` Unit Test覆盖1/2/4倍130×90目标、非整数向上取整、非法倍率、GPU纹理/对齐Buffer上限、内存预算与极大数值乘法溢出拒绝。`png.rs`在无Renderer的World验证EmptyDocument（含显式区域）、InvalidScale、RendererUnavailable不写文件、不分配渲染资源。

`png_gpu.rs`默认ignore，显式`cargo test -p bevy_widgetry_canvas --test png_gpu -- --ignored`运行真实Windows/DX12：无View下并发1/2/4倍重新栅格化、130×90/260×180/520×360真实RGBA文件，中心/描边/重叠的线性混色及Alpha；旋转与镜像非均匀Scale、AABB透明角与4倍结果不同于1倍最近邻拉伸；非整数显式区域与图形不相交得到27×19透明图。完成事件内立即读取有效PNG尺寸，验证实际文件先于成功通知；I/O失败通过同一事件返回。两View离屏渲染且各自选择/导航不同，导出像素与无View一致，导航/选择和Asset/Camera计数保持；快照建立后修改模型不改变本次导出。GPU存在但CanvasPlugin未安装时返回RendererUnavailable；超大目标拒绝保留文件与资源数；删除最后图形后EmptyDocument保留已有PNG。该测试不声称Native Window或交互FPS验收。

PNG内部生命周期回归通过真实Bevy `trigger_screenshots`投递读回：共享Manager关闭拒收且不分配资源；Queued执行体已Running但GPU未准备时取消、Preparing取消、读回中全局关闭、请求Entity提前销毁、重复Observer与晚取消仍Completed、磁盘失败的原CanvasExportError。30秒超时后仍保留已提交Screenshot的Scene/Image/层号至读回实际交接，之后只发一次Failed通知；迟到结果不写盘，不发生失效Entity框架错误。事件内核对JobId及Manager终态，最终收尾后操作记录/资源清空，Last持续RequestRedraw且通知后停止额外重绘。真实GPU测试也在完成事件中核对Job终态与实际输出文件。

椭圆显示 GPU 回归同样位于 `png_gpu.rs`：直接异步读回 CanvasView 专属目标，比较半径 100、1e-5、1e20 与对应等比 Scale 的实际像素，覆盖填充和描边；半径 10 或 1e-5、宽度 40、Alpha=0.5 的描边内部只应混色一次；宽度 1 的细描边保留内孔及边缘；很宽的半透明描边不应改变其下的圆形填充轮廓。高偏心率宽描边覆盖长轴沿 X/Y 两种方向、四个端部圆弧内点及轮廓外透明点，防止固定参数角采样切平端部。薄描边覆盖半径比 1e8 的 X/Y 两种方向及非均匀 Scale，将两轴映射为可见圆形后检查短轴描边、内孔和轮廓外透明区域，避免统一归一化丢失短轴。测试不通过尚未支持椭圆的 PNG 导出实现间接验证，也不声称 Native Window 输入证据。

## 填充与描边样式

`png_gpu.rs` 在真实DX12下切换NonZero/EvenOdd，覆盖同向与反向嵌套轮廓、五角星自交、原始命令及Shape身份保持；逐次切换Butt/Round/Square与Miter/Round/Bevel，检查开放端点、闭合轮廓cap不生效、零长度线/闭合子路径及MoveTo-only、短折返Round连接、EvenOdd填充下描边仍单次Alpha覆盖。Rectangle三种join的CanvasView与PNG输出像素对照resvg解析实际SVG文件的结果，保证现有导出同步样式。`renderer.rs` 逐次更新fill_rule、cap、join和width，核对双View共享Handle和网格原地刷新、Asset数量有界、颜色/Transform不改Mesh。`drawing.rs` 比较Round/Bevel矩形预览与提交后双View的实际描边顶点。

## Path 命令与多子路径

`path_append.rs` 覆盖基础轮廓追加：矩形/圆/椭圆/线段/折线/多边形各自开始新子路径，开放/闭合及ArcTo语义、已有命令不改写、后续可普通节点编辑，单一Shape身份/顺序/Transform/Paint/Entity数量保持。无效身份/所属/种类、过期目标、非有限/退化参数、点数不足、不可精确表示的水平端点及整体变换溢出均返回Error与必要日志，失败后不部分追加。`renderer.rs`覆盖逐种追加后双View同步、共享Mesh原地更新且资源有界。大坐标下半径0.1/0.15导致的端点舍入均拒绝，精确可表示的0.125保留实际曲线100单位Y半径与X边界。可见像素由Gallery BRP验收。

`path.rs` 覆盖typed CRUD/原始命令保留、身份/所属/种类守卫与拒绝后的事务保留，非有限端点/控制点、非法顺序/弧参数和变换溢出；四组弧标志、零/极小/巨大半径修正、Close后继续路径及未接入导出的文件保留。`renderer.rs` 保护双View共享、命令失效、Transform-only不改Mesh及删除回收。`png_gpu.rs` 显式DX12读回验证多个独立子路径、Quad/Cubic/Arc、NonZero填充、Butt端点和半透明描边单次覆盖；远距离子路径不影响小轮廓像素；包围盒相交的远直线达到1e12时保留局部填充/描边，同一子路径内往返远线仍保留单位矩形填充，非相邻近邻顶点与远点构成自交路径时保留局部填充，长边凹轮廓保留单轴窄缝，细长重叠三角形在非均匀Scale后保留填充且顶点数量有界，宽描边旁存在远几何时保留局部透明间隙，错开端点的斜向宽描边在单轴回退后仍保留沿线透明间隙，未触发坐标回退时也保护斜向端点与非相邻边的间隙及重复子路径的同一间隙，两斜向凸片共享角点仍保留另一端的三角间隙，单线/交叉子路径在2^34、2^80和2^100宽度时保留Butt端面及单次Alpha，巨大外轮廓仍保留反向小孔洞，使用真实生产裁剪路径；包围范围重叠时保留NonZero同/反向轮廓及交叉描边单次混色。短线段锐角连接只扩展外侧，内侧不添加Miter或Bevel，避免越过短线段；巨大描边不改变填充曲线，填充与描边各自归一化。未声称Path编辑工具或原生OS交互验收。

## 通用几何单选

`selection_geometry.rs`通过Mouse/Custom真实Picking覆盖EvenOdd孔洞穿透下方Ellipse、NonZero填满、Document重排后的顶部命中、多子路径远端选择整Shape及包围框空白；旋转/镜像/非均匀Scale和Z Scale=0下Ellipse真实几何及包围框角落、透明Some填充有效与None填充不命中；开放线描边在Zoom=0.25/1/4、Window/UiScale=1.5及旋转非均匀UI Node下始终保留4窗口逻辑屏幕像素容差。两个View选择和覆盖层独立，删除Shape仅移除相关框并修复状态；三类整体手势由transforms.rs覆盖。已有矩形输入、控点、导出隔离与生命周期回归继续保留。

几何宽相数值回归：半径1e8、旋转-0.1663的Ellipse，真实Mesh边缘超出理想Ellipse AABB时，View平移到边缘后真实点击仍选中；预筛选与选择框使用同一共享网格原始min/max。
