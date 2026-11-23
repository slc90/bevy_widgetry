# Canvas 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## 编译配置
- 默认测试涵盖模型、View／Renderer、输入与 SVG。`png_export` 开启时增加 PNG 公开 API、渲染与导出生命周期验证。
- 真实 Windows／DX12 的 `png_gpu.rs` 测试需要显式执行被忽略用例；功能关闭时不编译对应 API、导出系统和 PNG 专属 Benchmark。

## States
- Document 默认图层、图层显隐与锁定、绘图项身份、Shape 几何、Connector 路由及标签、Paint、Transform 与层级叠放顺序。
- Document 编辑事务、撤销和重做位置、有容量限制的保留实体。
- 多 View 的 Document 绑定、Pan／Zoom／Selection、绘图／编辑工具和 Render／Mesh 资源。
- Pointer Capture／Focus、预览草稿、矩形／椭圆／Path 参数及节点拓扑。
- SVG／PNG 文件导出、GPU Readback、异步 Job 与导出请求生命周期。

## Stimuli
- 公共 World／Commands 模型更新、Shape 新增／删除／重排、Renderer／View 创建与销毁。
- 带 token 的复合修改、撤销／重做、容量调整与提交后的重做分支丢弃。
- 真实 Pointer／Keyboard／Wheel、选取／拖动／缩放／旋转、图形和 Connector 绘制、Path 节点和 Connector 路由点编辑。
- Theme／Paint／Fill Rule／Stroke 样式、窗口比例／UiScale、Model／View 变化及异步导出结果。

## Guards
- Document、图形身份、几何参数、Paint、Transform 和操作目标必须合法；无效／跨 Document 请求原子拒绝。
- 同一 Document 的编辑事务排斥普通写入及历史回放，过期 token 无法提交或修改。
- 绘图／移动／旋转／缩放的输入守卫由当前工具、Selection、Focus、Disabled 和 Pointer Capture 决定；不完整草稿不可提交。
- 导出前验证文档内容、输出区域、颜色和倍率；缺少 Renderer／GPU 或失效 Job 时不得创建错误文件或部分资源。

## Transitions
- Shape 创建／修改／删除及 Document 重排；多 View 绑定／解绑、可见性与资源重建。
- Pan／Zoom／Select → Move／Scale／Rotate；绘图工具从预览／捕获到提交或取消，Path 从草稿到拓扑／节点编辑。
- SVG 同步文件写入及 PNG 异步准备 → GPU 读回 → 文件写入／结果事件／清理；取消、超时或失败走明确终态。

## Invariants
- Document 为形状、顺序与几何的业务权威；Shape Entity、CanvasGeometry、Paint／Transform 的合法提交保持一致，失败不部分更新。
- 新图形属于默认图层，绘图项同时满足 ECS 归属和直属顺序登记；容器内重排与删除更新同一顺序，Document 销毁后级联移除子项。
- 图层顺序和容器直属顺序确定展平结果，Shape 查询只包含图形；共享多 View 命中、显示及导出遵循同一 Document 顺序。
- 图层操作保持成员身份和直属顺序。删除非空层需要明确策略，迁移必须使用可用目标层，文档始终有默认层。
- 分组要求成员在同一容器中画序连续，拒绝非连续选择并保留成员身份及全局展平画序。组边界使用 Path 实际绘制轮廓。嵌套变换在 View 代理、命中、节点编辑和导出中一致，组内路径属于单个 View。
- 历史回放恢复同一实体及其绘图层级，清理最后的历史引用或销毁 Document 后释放保留实体。共享历史不回退 View 的视角和仍有效的选择。
- 隐藏层不参与显示与导出，锁定层保留投影；两者均清理相关选择，绘制仅提交到当前视图的有效活动层。
- 多个 View 可以共享 Mesh 和 Model，但 Pan、Zoom、Selection、Focus、Camera、RenderLayers 与预览互相独立；解绑／销毁按最后引用正确回收资源。
- Renderer 在 Geometry／Paint／Transform 改变时刷新所需网格，Transform-only 不无故重建；空几何不残留旧像素或幽灵命中。
- Pan／Zoom 使用真实 Picking 和窗口／UI 比例换算，Wheel 保持光标锚点，跨帧／同帧手势按输入顺序推进；取消只释放本手势持有的捕获。
- Selection 以 Document 中的真实形状身份表示，形状删除和 View 解绑后修复；子控件 Focus／Wheel／Delete 不由 Canvas 抢占。
- Move／Scale／Rotate 对矩形、椭圆和 Path 均保持不相关的 Geometry／Paint／顺序；镜像、旋转、边界缩放和 Ctrl／Shift 修饰按既有工具语义执行。
- 预览不会提前改写业务 Document；合法释放才创建最终形状并切换工具，取消／无效草稿回收 Mesh／Entity／Capture。
- Path 多子路径、Fill Rule、Cap／Join、Pen 与节点拓扑变化遵循有效几何、描边和选择身份，不因样式切换修改原始路径命令。
- Connector 路由是可编辑数据。箭头取端点切线，标签位置按沿路由的长度计算。选择、移动、编组、复制及历史以整个 Connector 为单位。
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
| `basic_path_tools.rs` | 直线、折线和多边形的局部预览与完成。参数化工具的边数、交替半径、角度吸附和 ArcTo 参数，退化拖动、取消、历史、节点修改、复制、变换与 SVG 导出。 |
| `flowchart.rs` | 六种预设的局部轮廓、尺寸、取消、提交与历史，菱形的轮廓命中、编辑副本与分组导出。 |
| `connectors.rs` | 自由端点直线与折线路由、绘制取消与提交、路由点拖动、箭头和标签的投影与命中、整体移动、画序、图层／组、历史和 SVG 导出。 |
| `batch.rs` | 多形状批量移动／删除、顺序和提交原子性。 |
| `deletion.rs` | 真实键盘删除与历史回放、Focus 归属、手势合并和双 View 修复。 |
| `diagnostics.rs` | 模型、输入、渲染、导出与颜色错误的来源和宿主传播。 |
| `drawing.rs` | 矩形预览绘制、提交／取消和多 View 隔离。 |
| `ellipse.rs` | 参数椭圆创建、几何边界、更新及失败拒绝。 |
| `ellipse_drawing.rs` | 椭圆预览绘制、抓取、提交和中断。 |
| `input.rs` | 真实 Pointer Pan／Zoom、坐标换算和同帧事件顺序。 |
| `layers.rs` | 图层创建、排序、重命名、迁移和删除策略，双 View 的活动层、显隐、锁定、命中、导出与绘制提交。 |
| `groups.rs` | 嵌套组、分组精度、代理树与 SVG 投影、真实等比缩放手柄、跨 View 双击、禁用 View 的 Escape 和组内节点／绘图输入。 |
| `history.rs` | Headless 事务隔离、层级和组件差异回放、回放时观察者读取的父级与局部变换、容量与保留实体生命周期。 |
| `model.rs` | 无 View／GPU 的 Document、Transform、Paint 与生命周期。 |
| `movement.rs` | 已选图形移动、捕获、Zoom 和异常取消。 |
| `multiselection.rs` | 独立选择集合、区域／多目标操作与视图隔离。 |
| `nodes.rs` | Path 节点选择、控制点／切线编辑和生命周期。 |
| `order.rs` | 默认图层、绘图项身份、顺序查询、容器内重排与所有权。 |
| `parameters.rs` | 图形参数及 Paint 更新的公共语义和原子性。 |
| `path.rs` | Path 命令、多个子路径和图形身份。 |
| `path_append.rs` | Path 命令追加、几何验证与失败保留。 |
| `pen.rs` | Pen 草稿、Path 生成与预览／提交状态。 |
| `png.rs` | 无 GPU 或不满足导出前置条件时的 PNG API 错误。 |
| `png_gpu.rs` | 显式 Windows／DX12 GPU 像素、形状与 Connector 快照、异步 PNG 与资源生命周期。 |
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
| `src/order.rs` | 多图层与嵌套容器的展平、重排、删除和祖先归属。 |
| `src/error.rs` | 导出与模型错误的包装、原始 origin 和多失败传播。 |

### 文字绘图项

| 行为 | 覆盖 |
| --- | --- |
| 同字号字宽、显式换行、属性更新、无效输入、SVG 转义和撤销重做 | `text.rs` |
| 活动图层或组、独立副本、局部变换、删除与恢复 | `text.rs` |
| 点击放置、双 View 投影、草稿隔离、旋转组内放置、墨迹命中和形状与文字混合批量移动 | `text.rs` |
| TextField 聚焦时的 Delete 和 Ctrl+Z | `deletion.rs` |
| PNG 快照中多行字形的位置、颜色与末端对齐，以及区域外文字的导出就绪 | `png_gpu.rs`（DX12） |
