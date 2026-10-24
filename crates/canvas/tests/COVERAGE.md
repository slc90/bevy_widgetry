# Canvas Integration Test Coverage

## Model

`model.rs` 覆盖没有 UI/渲染插件的纯 World 模型：多个矩形共享 Document、负坐标、原生 Transform 更新、无填充与透明填充语义、更新与移除、Document 的 ChildOf 级联清理。

非法尺寸、Paint、Transform、无效 Document、已删除图形和跨 Document 请求应返回 Error Severity 与必要诊断，不创建额外 Entity，不修改旧几何与 Paint。纯几何测试覆盖居中描边、旋转及非均匀整体 Scale 的 Document AABB 和四角；局部尺寸下限为 1 单位。

## Renderer 与 View API

`renderer.rs` 通过真实 BSN、UI Layout、Transform/Visibility 调度验证：1/1.5/2 倍窗口比例下的逻辑尺寸与物理目标、关闭 rounding 后的分数像素布局、同帧布局与相机矩阵更新、Document Y 方向转换、4× MSAA 配置、填充与居中描边的顶点/索引、变换同帧传播与几何复用、None 填充隐藏、尺寸/描边修改、隐藏恢复、持续帧资源有界和 View 销毁不删除 Document。

Document 引用 API 覆盖同步/Commands、合法同值无通知、状态先提交后通知、排队执行时 Document 已失效的拒绝与宿主错误传播。真实 GPU 的 Alpha/MSAA/边缘表现由任务 BRP 验证，不由 Headless 配置断言替代。

变换回归覆盖模型 Z Scale 为负、零及正值时描边始终位于填充之前，XY 镜像不重建几何，模型 Z Scale 不被改写。

多 View 生命周期覆盖共享 Mesh Handle、独立 Camera/Image/RenderLayers、几何和变换同步、隐藏恢复与 View 重建、解绑后仅回收最后一个引用、删除图形与 Document 后的资源清理。Document 删除后保留 View 和布局、引用自动清空并允许重新绑定；隐藏 View 同样修复。DocumentChanged Observer 可以销毁 View，Renderer 不访问失效实体。真实跨 View 画面同步、透明目标清空由任务 BRP 验证。
