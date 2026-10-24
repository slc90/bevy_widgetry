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
