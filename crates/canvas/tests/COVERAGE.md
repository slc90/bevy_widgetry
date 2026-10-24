# Canvas Integration Test Coverage

## Model

`model.rs` 覆盖没有 UI/渲染插件的纯 World 模型：多个矩形共享 Document、负坐标、原生 Transform 更新、无填充与透明填充语义、更新与移除、Document 的 ChildOf 级联清理。

非法尺寸、Paint、Transform、无效 Document、已删除图形和跨 Document 请求应返回 Error Severity 与必要诊断，不创建额外 Entity，不修改旧几何与 Paint。纯几何测试覆盖居中描边、旋转及非均匀整体 Scale 的 Document AABB 和四角；局部尺寸下限为 1 单位。

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

`selection.rs` 覆盖 View 自有 Selection 的同步/Commands API、合法同值/清空、先提交后通知、跨 Document 拒绝及宿主错误传播；解绑清空。真实 Mouse/Custom Picking 覆盖旋转、XY 镜像/非均匀缩放且 Z Scale=0 的矩形、空心内部、描边外缘和外侧空白；Window/UiScale 与 Zoom/Pan 坐标换算、两 View 不同选择、Space 优先平移且保留选择、首次按压选择后拖动不移动模型。嵌套 Button 保有真实 InputFocus，Canvas 不接管其按压或平移。隐藏/Disabled/失焦/零尺寸/Display::None/解绑守卫，以及模型改属其他 Document 后无悬空查询。

`renderer.rs` 覆盖选择轮廓与实际旋转边缘平行、不同 Zoom 下逻辑线宽保持、模型/Mesh/Paint 不变、取消选择和 View 销毁的资源释放、多个 View 的轮廓资源隔离；图形移除后的选择修复通知允许销毁 View，不访问过期实体。`common/` 仅为 Pointer 测试共享真实 UI/Picking fixture，不提供生产 API。

## 已选矩形移动

`movement.rs` 通过真实Mouse/Custom Picking覆盖首次选择不捕获/不移动、再次按下已选图形才移动，抓取偏移、Zoom 0.5/1/2、Window比例与UiScale、Viewport坐标，移动中按下/松开Space用途锁定、移出区域继续、释放后停止。原模型rotation/scale/Z/size/Paint和共享Mesh Handle保持，两个View的代理同步且其Pan/Zoom/Selection独立。

取消覆盖PointerCancel、隐藏、Disabled、Window失焦、View/Document/图形销毁、InputFocus转移、Display::None、零尺寸、Document解绑、Selection清空，停止后不再改模型。按原始输入顺序覆盖Move后非Primary释放仍继续、同帧Move→Primary Release/Cancel保留最后位移；移动中Wheel锚点不跳且后续位移使用新Zoom。共用手势路由的细粒度同帧Focus/Capture交接回归继续由`input.rs`保护；GPU画面及交互帧时间由任务BRP验证。

## 预览式矩形绘制

`drawing.rs` 覆盖Mode同步/Commands API、Paint与View校验、同值无通知、先提交后通知、模式互不覆盖。真实Mouse/Custom输入验证四个方向、Zoom/Window比例、Wheel锚点、按下/移动时Document不变且仅当前View有预览、释放后创建/自动选择/返回Select，另一View的Selection保持。首帧从View外进入后同帧Press→Move→Release仍提交。

无效小于1单位的单击、Esc真实KeyboardInput、PointerCancel、隐藏、Window失焦、View销毁、模式切换与Document解绑均不提交，预览Entity/Mesh/Material和捕获均回收。Space优先Pan且中途松开仍不改为绘制；普通取消保留工具模式，Esc和有效/无效释放回到Select。真实预览仅当前View可见和释放后双View同步由任务BRP验证。

首帧Move入View→Press→Release无Drag仍退出Rectangle模式，并覆盖同帧再Press/Move/Release不能沿用旧绘制。完成通知切换Paint并在同帧重新Press时，复用预览同步填充/描边颜色、Alpha模式和描边宽度，覆盖不透明→半透明/无填充；不保留上一手势的渲染数据。ModeChanged的Deferred回调可以销毁/解绑View，旧完成路径不创建过期图形。

## 八向尺寸调整

`resizing.rs` 通过Mouse/Custom真实Picking覆盖八个控制点优先于内部移动、旋转与非均匀/镜像Scale、Z Scale=0/2、Zoom 0.1/0.5/1/2/10、Window比例1/1.5/2和UiScale；按需将View中心移至高Zoom控制点，验证局部尺寸/中心与固定对边/对角，rotation/scale/Z/Paint保持，另一View的模型代理同步且Pan/Zoom/Selection不变。越界拖动钳至局部1单位，多次越过后不翻转或漂移；Wheel中抓取不跳、PointerCancel释放后停止。

`renderer.rs`继续覆盖独立View覆盖物回收，新增八个控制点的40顶点Mesh及8逻辑像素视觉大小。`common/coordinates.rs`仅被selection/resizing测试按需引用，为真实UI位置换算提供共享助手；移动测试使用不与控制点重叠的内点负载，空白选择测试的外点位于控制点容差之外。
