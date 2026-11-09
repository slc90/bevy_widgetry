# Canvas 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## 编译配置
- 默认测试涵盖模型、View／Renderer、输入与 SVG。`png_export` 开启时增加 PNG 公开 API、渲染与导出生命周期验证。
- 真实 Windows／DX12 的 `png_gpu.rs` 测试需要显式执行被忽略用例；功能关闭时不编译对应 API、导出系统和 PNG 专属 Benchmark。

## States
- Document／CanvasShape／CanvasGeometry、Paint、Transform、叠放顺序与图形身份。
- 多 View 的 Document 绑定、Pan／Zoom／Selection、绘图／编辑工具和 Render／Mesh 资源。
- Pointer Capture／Focus、预览草稿、矩形／椭圆／Path 参数及节点拓扑。
- SVG／PNG 文件导出、GPU Readback、异步 Job 与导出请求生命周期。

## Stimuli
- 公共 World／Commands 模型更新、Shape 新增／删除／重排、Renderer／View 创建与销毁。
- 真实 Pointer／Keyboard／Wheel、选取／拖动／缩放／旋转、矩形／椭圆／Path 绘制和节点编辑。
- Theme／Paint／Fill Rule／Stroke 样式、窗口比例／UiScale、Model／View 变化及异步导出结果。

## Guards
- Document、图形身份、几何参数、Paint、Transform 和操作目标必须合法；无效／跨 Document 请求原子拒绝。
- 绘图／移动／旋转／缩放的输入守卫由当前工具、Selection、Focus、Disabled 和 Pointer Capture 决定；不完整草稿不可提交。
- 导出前验证文档内容、输出区域、颜色和倍率；缺少 Renderer／GPU 或失效 Job 时不得创建错误文件或部分资源。

## Transitions
- Shape 创建／修改／删除及 Document 重排；多 View 绑定／解绑、可见性与资源重建。
- Pan／Zoom／Select → Move／Scale／Rotate；绘图工具从预览／捕获到提交或取消，Path 从草稿到拓扑／节点编辑。
- SVG 同步文件写入及 PNG 异步准备 → GPU 读回 → 文件写入／结果事件／清理；取消、超时或失败走明确终态。

## Invariants
- Document 为形状、顺序与几何的业务权威；Shape Entity、CanvasGeometry、Paint／Transform 的合法提交保持一致，失败不部分更新。
- 创建、删除和变更叠放顺序不借用 Model Z 冒充顺序；共享多 View 命中、显示及导出遵循同一 Document 顺序。
- 多个 View 可以共享 Mesh 和 Model，但 Pan、Zoom、Selection、Focus、Camera、RenderLayers 与预览互相独立；解绑／销毁按最后引用正确回收资源。
- Renderer 在 Geometry／Paint／Transform 改变时刷新所需网格，Transform-only 不无故重建；空几何不残留旧像素或幽灵命中。
- Pan／Zoom 使用真实 Picking 和窗口／UI 比例换算，Wheel 保持光标锚点，跨帧／同帧手势按输入顺序推进；取消只释放本手势持有的捕获。
- Selection 以 Document 中的真实形状身份表示，形状删除和 View 解绑后修复；子控件 Focus／Wheel／Delete 不由 Canvas 抢占。
- Move／Scale／Rotate 对矩形、椭圆和 Path 均保持不相关的 Geometry／Paint／顺序；镜像、旋转、边界缩放和 Ctrl／Shift 修饰按既有工具语义执行。
- 预览不会提前改写业务 Document；合法释放才创建最终形状并切换工具，取消／无效草稿回收 Mesh／Entity／Capture。
- Path 多子路径、Fill Rule、Cap／Join、Pen 与节点拓扑变化遵循有效几何、描边和选择身份，不因样式切换修改原始路径命令。
- SVG 导出按实际 Transform、颜色、透明度和顺序生成可解析文件；不支持的颜色或失败写入保留原目标文件。
- PNG 以提交时快照和真实 GPU 完成结果作为文件依据，成功通知时输出已有效落盘；后台取消、迟到结果、超时和磁盘故障不重复完成或泄漏资源。
- 公开通知按已提交 Authority 发出；模型、Renderer、颜色和异步错误在 ECS 边界保留原始 source／origin、业务目标和必要故障上下文。

## Couplings
- Document Order × Picking／Renderer／SVG／PNG；多 View × Mesh／资源生命周期。
- Pointer Capture × Pan／Zoom／绘图／编辑；Selection × Focus／模型删除；Path 拓扑 × 几何投影。
- Export Snapshot × GPU Readback／Job／文件原子性；Theme／Paint × Mesh／Stroke／Alpha。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `batch.rs` | 多形状批量移动／删除、顺序和提交原子性。 |
| `deletion.rs` | 真实键盘删除、Focus 归属和双 View 清理。 |
| `diagnostics.rs` | 模型、输入、渲染、导出与颜色错误的来源和宿主传播。 |
| `drawing.rs` | 矩形预览绘制、提交／取消和多 View 隔离。 |
| `ellipse.rs` | 参数椭圆创建、几何边界、更新及失败拒绝。 |
| `ellipse_drawing.rs` | 椭圆预览绘制、抓取、提交和中断。 |
| `input.rs` | 真实 Pointer Pan／Zoom、坐标换算和同帧事件顺序。 |
| `model.rs` | 无 View／GPU 的 Document、Transform、Paint 与生命周期。 |
| `movement.rs` | 已选图形移动、捕获、Zoom 和异常取消。 |
| `multiselection.rs` | 独立选择集合、区域／多目标操作与视图隔离。 |
| `nodes.rs` | Path 节点选择、控制点／切线编辑和生命周期。 |
| `order.rs` | Document 图形叠放顺序、调整与合法性。 |
| `parameters.rs` | 图形参数及 Paint 更新的公共语义和原子性。 |
| `path.rs` | Path 命令、多个子路径和图形身份。 |
| `path_append.rs` | Path 命令追加、几何验证与失败保留。 |
| `pen.rs` | Pen 草稿、Path 生成与预览／提交状态。 |
| `png.rs` | 无 GPU 或不满足导出前置条件时的 PNG API 错误。 |
| `png_gpu.rs` | 显式 Windows／DX12 GPU 像素、快照、异步 PNG 与资源生命周期。 |
| `renderer.rs` | 真实 BSN／Layout／Mesh、离屏资源和多 View Render。 |
| `rotation.rs` | 交互旋转、角度连续性、吸附及控制柄。 |
| `scaling.rs` | 八向缩放、镜像、比例与固定锚点。 |
| `selection.rs` | 真实 Picking 命中、单选、视图隔离和状态修复。 |
| `selection_geometry.rs` | 选择区域与控制柄几何、变换和边界。 |
| `shape.rs` | 通用 CanvasShape／CanvasGeometry、typed 操作与身份一致性。 |
| `svg.rs` | SVG 文件布局、变换、颜色、Alpha、绘制顺序和失败保护。 |
| `topology.rs` | Path 拓扑、节点连接与编辑后有效性。 |
| `transforms.rs` | 通用图形移动／缩放／旋转、几何不变量和手势取消。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/geometry.rs` | 几何 AABB、Transform、描边和形状边界计算。 |
| `src/export/png.rs` | PNG 尺寸、内存预算、异步生命周期与超时／取消。 |
| `src/model.rs` | Document／Shape 局部语义和模型有效性。 |
| `src/error.rs` | 导出与模型错误的包装、原始 origin 和多失败传播。 |
