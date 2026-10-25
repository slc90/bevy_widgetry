# Canvas

Canvas 提供独立 ECS Document、矩形模型和普通 Bevy UI CanvasView。当前提供矩形显示、View 平移/缩放与矩形单选，支持已选矩形移动和一次性矩形绘制，支持八向尺寸调整与旋转，尚未提供导出。

## Document 与矩形

`CanvasDocument` 是无固定边界的业务 Entity，Document 坐标 X 向右、Y 向下，负坐标合法。矩形以 `ChildOf` 归属 Document，可包含任意多个；删除 Document 会级联删除其子图形。模型不需要 Camera、Image、Mesh 或 UI 插件，也不依赖 View 的存在。

`CanvasRectangle { size }` 保存中心对称的局部宽高，均须有限且至少为 1 个 Document 单位。官方 `Transform` 是中心位置、旋转与整体 Scale 的唯一权威；使用绕 Z 轴的正规化旋转，XY Scale 非零，允许非均匀 Scale。所有数值须有限，变换后的边界也须可表示。Document 不是渲染 Scene，Document Entity 本身的 Transform 不参与形状几何解释。

`CanvasPaint { fill, stroke }` 中 `fill: None` 表示无填充，`Some(Color::NONE)` 保留指定透明颜色的语义。`CanvasStroke { color, width }` 的宽度为正有限的局部 Document 单位，沿矩形边界居中；Transform 的 Scale 同时作用于填充和描边。颜色转换后的通道必须有限。

`CanvasDocument::create_rectangle`、`update_rectangle` 和 `remove_rectangle` 提供同步 World CRUD；创建/更新先校验完整数据，更新和删除还校验图形确实属于目标 Document。非法请求记录错误并返回 `Severity::Error`，保留原有数据。图形 Entity 即稳定身份，不维护第二份 Shape 列表或变换副本。

模型组件是业务数据，可通过原生 ECS 访问；直接修改时须遵守相同有效性约束，或使用上述校验型 CRUD。官方 Transform 保持原生读写方式。`CanvasRectangle::validate` 与 `bounds` 返回校验错误，供数据消费者处理；`corners` 对有效数据给出变换后的四角，`bounds` 计算包含居中描边和整体变换的最小 AABB，均不依赖 GPU。

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

每个 View 默认没有 Selection，`CanvasView::selection` 查询，`set_selection_in_world` / Commands `set_selection` 设置一个矩形 Entity 或 None。目标必须属于当前有效 Document；无效 View、过期 Entity、跨 Document 或非矩形目标返回 Error，保留原状态。实际提交后发送 `CanvasSelectionChanged`，合法同值与重复清空不通知。Document 绑定实际改变时一并清空选择；失效或改属其他 Document 的选择不从查询返回，Renderer 下一次同步时清理状态和轮廓并通知 None。修复通知回调可以销毁 View。

普通左键按下从 View 局部 UI 坐标换算到 Document，再逆矩形的 XY Transform 完成命中，包含居中描边外缘。无填充或透明填充的矩形内部仍可选择，Z Scale 不参与平面求逆。命中另一个矩形切换选择，空白清空；多个 View 的 Selection 独立。点击获取现有 InputFocus，点击其他 Widget 后焦点归实际目标；命中嵌套子 Widget 时 Canvas 不冒领选择或 Pan。Space+左键优先平移，保留选择；首次按在尚未选中的矩形上只改变选择，该次按压后移动不会修改模型。重叠矩形的命中顺序未作保证。

选择轮廓沿含描边外缘的旋转边界绘制，外留 3 个 UI 逻辑像素间隔，线宽保持 2 个逻辑像素，不随 Zoom 变粗。轮廓属于 View 离屏 Scene 的独立 Mesh/Material，不修改 CanvasPaint，不是 Document 子图形；不属于导出数据。取消选择或销毁 View 会回收其专属资源。顶部外侧另有独立旋转点。

## 已选矩形移动

后续新的左键按下命中当前已选矩形时捕获该 Pointer，从此刻的 Document 坐标记录中心抓取偏移；移动时以当前 View 中心、Zoom 和局部 UI 坐标求出新的 Document 中心，只修改该图形的 Transform.translation，Z、rotation、scale、size、Paint 保持。首次选择时不启动移动，Space+左键优先 Pan；一旦手势开始，随后按下/松开 Space 不改变用途。拖到 View 外继续，滚轮缩放后保留抓取点，以新 Zoom 换算后续位移。

移动更新共享 Document，所有 View 的图形和选择轮廓同步跟随，填充/描边 Mesh Handle 保持；其他 View 的 Pan/Zoom/Selection 不变。移动继承 Pan 的同 Pointer 非 Primary 按压隔离及按原始顺序提交终止前最后位移的契约。

Primary Release、PointerCancel、Focus 转移、Window 失焦、隐藏、Display::None、Disabled、零尺寸、Document 解绑/销毁、图形失效/改属其他 Document、Selection 改变或 View 销毁均终止移动并归还自身捕获；不释放其他 owner 的捕获。终止后不恢复旧手势，也不回滚已提交的模型位置。

## 矩形绘制

View 默认 `CanvasMode::Select`；通过 `CanvasView::mode` 查询，`set_mode_in_world` / Commands `set_mode` 切换到 `CanvasMode::Rectangle(CanvasPaint)`。Paint 预先校验，非法 View/Paint 返回 Error 并保留原模式；合法同值不通知。实际改变时先提交、取消当前 Pointer 手势，再发送 `CanvasModeChanged`。模式和 Paint 属于当前 View，不改变其他 View 的工具。

矩形模式下普通左键按下记录 Document 起点，移动仅更新当前 View 的预览。预览是该 View 离屏 Scene 的独立 Mesh/Material，没有 Document 图形 Entity，不进入共享图形缓存或其他 View。Space+左键仍优先平移，开始后用途锁定；中途通过 API 切换模式会取消旧手势，不能转换为另一个操作。

左键释放时以起止点的 min/max 构造正尺寸和中心，支持四向拖动；宽高均至少 1 个 Document 单位且有限才创建矩形。无效单击不写 Document；有效提交后自动选中新 Entity，并返回 Select。按 Esc 丢弃预览并退出矩形模式；普通 PointerCancel、失焦、不可用 View 或 Document 解绑/销毁取消预览并释放捕获，不提交半成品。预览在完成、取消和 View 销毁时回收专属 Mesh/Material。模式切回 Select 的通知回调若销毁/解绑 View 或再次切换模式，旧提交不继续创建图形。

## 八向尺寸调整

选中矩形的含描边外缘显示四角与四边中点共八个方形控制点；视觉边长固定为 8 个 UI 逻辑像素，不随 Canvas Zoom 改变。控制点与选择轮廓共用当前 View 的编辑 Mesh/Material，不成为 Document 图形或共享图形 Mesh。命中采用 6 个逻辑像素半径并优先于图形内部移动，区域重叠时选择最近的控制点。Space Pan 与 Rectangle 模式仍按既有优先级处理。

按下时记录原 size、Transform、控制点和 Document 指针位置，后续指针位移经原旋转和 XY Scale 逆变换到局部方向。角点同时修改宽高，边点只改对应尺寸；实际修改 CanvasRectangle.size 与必要的 Transform.translation，保留 rotation、scale 和 Z。局部尺寸钳至至少 1，不允许翻转或交换控制点角色；固定对边/对角的 Document 位置保持，含描边与已有非均匀/镜像 Scale 的几何也沿用这一约束。

共享 Document 的其他 View 同步尺寸、Paint Mesh 和轮廓，自有 Pan/Zoom/Selection 保持。Wheel 缩放保持抓取点，后续位移使用当前 View 坐标换算。释放或取消采用已有独占 Pointer 手势的结束规则，失焦、隐藏、Disabled、零尺寸、Selection/Document/图形失效以及 View 销毁均归还捕获；取消保留已经提交的尺寸。

## 旋转与角度吸附

第九个控制点是顶部边中点外侧的圆点，以连接线与尺寸点区分；沿实际变换后的顶部外法向偏移 24 个 UI 逻辑像素，圆点直径固定 8 个逻辑像素。旋转点先于尺寸点和内部移动命中，同样只属于当前 View 的编辑覆盖物。Space Pan 和 Rectangle 模式仍保持优先级；已开始的旋转不会因随后按 Space 改为平移。

旋转手势记录原 Transform 与按下时围绕中心的方向，根据当前 Document 指针方向更新 Transform.rotation；中心、size、scale、Z 和 Paint 保持。跨越 ±180° 或整圈时按方向连续显示，局部填充/描边 Mesh 不重建，所有 View 看到相同旋转，各自导航与选择不变。按住任一 Shift 将绝对角度吸附到 15° 的整数倍；按下/释放 Shift 时，即使指针静止也重新计算吸附/自由角度。

旋转继承既有 Pointer 捕获与失效清理，不添加枢轴或撤销系统。取消保留已提交的 rotation；Selection/Document/图形失效或 View 不可用后停止更新并归还自身捕获。

## 删除

键盘焦点精确落在可用 CanvasView 根且输入窗口有焦点时，Delete 删除当前有效选择的 Document 矩形。焦点位于 TextField、其他控件或 Canvas 子控件时不接管 Delete；未选中时无操作。所有引用同一图形的 View 在正常同步中清空失效 Selection 并通知 None，针对已删除图形的手势终止并归还捕获；代理和最后引用的 Mesh 回收，不修改其他图形、View 中心或 Zoom。删除最后一个图形后 Document 与 View 保持，空画布可以继续绘制。
