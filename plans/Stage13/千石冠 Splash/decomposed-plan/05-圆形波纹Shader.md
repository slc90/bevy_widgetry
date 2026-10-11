# 05｜实现从手掌扩散的圆形波纹揭幕

> 执行顺序第 5/7 份。基于 `master-plan.md` 的相应整段详细正文**直接重新分区**，并加入独立实施所需的范围与前后衔接。与后续阶段不是并行分支。对 Bevy 官方 API 的核对版本为 `v0.20.0`；源代码仍以实施时仓库当前提交为准。

## 目标

让离屏 Splash 画面从手掌位置开始局部透明并平滑显露真实 Gallery。

## 范围

UiMaterial fragment shader、uniform、波纹数学、alpha 混合、完结呈现；独立验证 Shader 正确，不改 Gallery 页面逻辑。

## 预期产出

能在 200ms 内清晰揭幕的 shader 材质、WESL/WGSL asset 和逐时刻画面。

## 与前后方案的关系

- 前序：[建立 Splash 资源加载与展示生命周期](04-加载与就绪生命周期.md)。
- 后序：[将完成的 Splash 无侵入接入 Gallery](06-Gallery正式集成.md)。
- 唯一完整执行顺序参见 [00-overview.md](00-overview.md)，在本阶段未过验收之前不应把问题留给后一个小方案。

## 工程现状与共同目标（原总方案保留内容）

Gallery 保持现有的 `main()` 顺序：`AppLogging::new`、`block_on(transparent_render_creation())`、`DefaultPlugins`、Widgetry/ Gallery 插件、`Startup` 创建主窗口/唯一 `Camera2d`/WidgetryWindow、`GalleryPage::Initializing → Button`，以及现有 `startup_cursor` 首帧流程。**不替换 DX12 初始化，不新建原生 Splash 窗口，不延后全部 Gallery 构造。**

Splash 在同一个 Gallery 窗口覆盖整个客户区；没有 Logo、进度条、仓库相关文字、独立小卡片或假加载时间。角色为《Slow Start》千石冠，以用户提供的[官方角色页](https://slow-start.com/character/?chara=kamuri)和本包 `assets/selected/approved_key_art.png` 为美术依据。上半身从底边探出，轻轻挥手，两根深蓝丝带随动，短促眨眼；最后由挥手手掌附近扩散的圆形波纹逐步显露真正的 Gallery。Splash 合成画面在转场前完全不透明；转场时露出的是**已经绘制的 Gallery 画面**，不是透明 Win32 桌面洞。

名义演出 1000ms，设计节奏：`0–200ms` 探头、`200–550ms` 挥手与丝带、`550–800ms` 眨眼及安定、`800–1000ms` 波纹交接。60 FPS 是目标刷新能力（单帧预算约 16.67ms），不是每 1/60 秒强制硬切图；使用经过时间决定状态，渲染性能需实机确认。Gallery 不就绪时，允许在最后姿势短暂等待，使总墙钟时间超过 1 秒，但**绝不提前揭幕空白或未就绪的 Gallery**。

### 已核对的事实及技术主线

- `Cargo.toml` 固定 Bevy 0.20.0，`default-features=false`，含 `bevy_ui`、`bevy_ui_render`、`bevy_render`、`bevy_core_pipeline`、`png` 和 `ui_picking`，**未启用 `bevy_sprite` / `bevy_sprite_render`**。不为本测试添加 Sprite 渲染功能；使用已经启用的 UI 图层能力。
- `gallery/src/main.rs` 的 `setup` 创建 `Camera2d` 和 deferred `widgetry_window` Scene，随后 `NextState<GalleryPage>::Button`；`gallery/src/pages/button.rs` 在 `OnEnter(Button)` 同步挂载页面。不能把 `Startup` 已完成当成 Gallery GPU 画面已完成。
- `crates/window/src/scene.rs` 的 `prepare_native_window` 设置 `transparent=true`、`decorations=false`、`CompositeAlphaMode::PreMultiplied`；`crates/window/src/render.rs` 手动创建 DX12 `DxgiFromVisual`。不修改此路径。
- `gallery/src/startup_cursor.rs` 已有 RenderApp 首次 present + UI background pipeline 检测与 `DwmFlush` 后的主线程信号；可以**最小化地复用/暴露一次性就绪信号**，但这只能证明当前检测的 Gallery 背景绘制，不等同于 Button 全部内容均完成渲染。
- Bevy 0.20.0 官方 `examples/ui/render_ui_to_texture.rs` 展示 `Image::new_target_texture`、`RenderTarget::Image`、`Camera { order: -1 }`、`UiTargetCamera`；`examples/ui/ui_transform.rs` 展示 `UiTransform` 的平移/旋转/缩放；`examples/ui/ui_material.rs` 展示 `UiMaterialPlugin`、`MaterialNode`、`AsBindGroup`、自定义 fragment shader 与运行时材质更新。
- **技术主线：** 千石冠的所有 UI 图层先由一个离屏 `Camera2d` 渲染成不透明 `Image`；主 Gallery Camera 仍向原窗口渲染 Widgetry UI；最顶层的全窗 `MaterialNode<SplashRevealMaterial>` 采样这张离屏纹理。揭幕期间，shader 仅把 Splash 对应区域的 **alpha** 降至 0，自然露出下层 Gallery；不必先将 Gallery 再复制到另一张纹理，也不必在 GPU 读取 Windows 桌面。
- 官方示例使用 `Bgra8UnormSrgb` 离屏纹理。实际格式、采样限制、WGSL/WESL 导入、alpha 混合与窗口预乘模式的组合，应以 0.20.0 示例为起点，在 Windows DX12 **实际验证**，不能照搬其他版本 API。

## 本阶段的详细方案（保留总方案对应正文）

## [S05] 通过 UiMaterial 实现圆形波纹揭幕 Shader

### 具体着色器设计

目标不是后处理整个 swapchain，也不是 DWM 缩略图；**只对主窗口最上层的 Splash 纹理四边形生成掩码**。Bevy 0.20.0 `UiMaterial`（`UiMaterialPlugin::<...>::default()`、`MaterialNode(handle)`、`#[derive(Asset,TypePath,AsBindGroup,Clone)]`）足以提供 shader uniform 和离屏 image texture/sampler。官方 WESL `assets/shaders/custom_ui_material.wesl` 使用 `import bevy_ui_render::ui_vertex_output::UiVertexOutput;`，本工程的 Shader 文件需确认以 `ShaderRef` 正确解析并通过同一 embedded source/内部 asset 路径加载，避免程序打包成 EXE 后丢失 shader 文件。

建议材质包含：离屏 Splash `Handle<Image>`；`center_px`（屏幕/viewport 实际像素坐标）；`viewport_px`（实际像素宽高）；`radius_px`；`feather_px`；淡色波纹边缘的颜色、宽度和强度等可调参数。尽量将变化数值合并成对齐的向量 uniform；不要每帧新增 Material 实例或重新编译 shader，更新同一个 `Assets<SplashRevealMaterial>` 对象即可。

以左上角为零点、实际像素为单位，定义 `p = uv * viewport_px`，`d = length(p - center_px)`。波纹外保留 Splash，波纹内显露 Gallery，基础可用 `alpha_splash = smoothstep(radius_px - feather_px, radius_px + feather_px, d)`：半径小时整屏 Splash 不透明，半径增长后中心 alpha→0，越过所有角点后整屏 alpha→0。可在前沿加一圈低强度环形柔光，但不得为了光圈把角色画面整体闪白。`radius_end >= max(distance(center, four viewport corners)) + feather`；该计算须支持非 16:9 窗口。

`center_px` 不能写死成“1920 屏幕的某点”。应该从 S01 的**手掌位置**经角色当前变换与画布 cover 映射得到屏幕坐标，交接开始时冻结这个中心，防止边缘突然跳动。动画末尾 Gallery 如果尚未就绪，半径应保持为零/全不透明（即使演出时钟已经到 800ms）。

Shader 输出纹理颜色及衰减 alpha；开始揭幕之前必须确认 S02/S04 的独立不透明兜底 Node 已被移除，且仍保留真正的 Splash MaterialNode。在 Bevy UI 的混合管线与 Win32 `CompositeAlphaMode::PreMultiplied` 共存下，必须检查 **RGB 是否被重复预乘、Gamma/sRGB、软边界的紫/黑边、波纹内部是否真的露出 Gallery，而不是露出桌面**。仅在 Splash UI 节点的 alpha 归零后让下层 Gallery 自然显露，不需要给 shader 再传一张 Gallery 截图纹理。若实测 `UiMaterial` 不能直接得到正确的整层透明合成，先定位 blend/排序问题，再讨论额外合成 RenderPass；不得悄然放弃圆形揭幕改成普通淡出。

### 交接完成与实际显示

最后一个 UI material uniform 半径满足覆盖所有四角，并实际经过窗口 present（不是只在主世界写了 uniform），再销毁 Splash 节点，以免出现一帧又恢复旧内容的闪烁。可用现有 render-side 首帧模式取得最少必要的绘制反馈。Wave 效果从手掌触发、在约 200ms 内完成，不延迟整个 Gallery 的逻辑创建。

### 本阶段验收边界

使用已完成的角色画面和一个可以辨认的 Gallery 底层 UI，捕获 0%、25%、50%、75%、100% 揭幕帧。检查画面中心、扩散方向、边缘、四角、DPI/窗口缩放、透明原生窗口和相机 render 顺序。任何中间帧不应出现桌面穿透（原生窗口圆角本来透明的区域除外）或黑色未初始化纹理。

---

## 全链路约束（跨阶段约束在本小方案内复现）

1. 只在 Gallery 程序中加 Splash 专用插件与资源；不向 Widgetry 公共库泄漏临时演示 API，不重构日志、透明 DX12 device 或 `GalleryPage` 页面生命周期。
2. 同一个原生 Window、现有 Gallery 主 Camera 不被替换；专用离屏 Camera 不渲染到窗口且不抢占 Gallery UI camera 目标。
3. Splash 从窗口有画面到揭幕前，必须有**不透明像素兜底**；素材加载、离屏管线编译期间也不能先闪现下层 Gallery。兜底可用与插画一致的全窗纯色/渐变 UI 画布，并非附加品牌背景卡片。
4. 本次仅 2D，不做 3D、MP4 视频播放、完整帧序列 Atlas、Logo、音频、复杂发丝物理或通用 Splash 框架。
5. 1920×1080 是 16:9 **设计坐标**，实际素材尺寸为 **1672×941**，大多数 PNG 的透明区域很大。需要统一转换设计坐标、素材像素坐标、UI 逻辑像素、窗口物理像素及实际绘制 UV；不得直接假定它们相等。
6. 两根丝带独立图层、同一个头顶根部锚点、相关但不同相位的轻摆；手臂绕肩部锚点旋转；眨眼预先加载两张主体图，只在眼部出现像素变化。
7. 播放计时从 Splash **首次可见且所需 GPU 内容可用之后**才开始，不是从 `main()`、`Startup` 或调用 `AssetServer::load` 开始。Gallery 首屏未就绪不能触发揭幕。
8. 播放和揭幕期间连续驱动重绘，结束后回到既有 `WinitSettings::desktop_app()` 的按需更新；默认一次性播放，所有 RenderTarget、material、image handle、UI root 和 marker 的清理可审计。
9. 不以 `LoadState::Loaded` 代替纹理已经上传、管线已就绪或已经 present 的证明；确保 CPU 就绪与实际渲染可用分离。
10. 不以用户可见效果图替代素材程序级几何验证；尤其当前独立手臂和丝带仍是**待对位候选**，不应被文档宣称已经严丝合缝。
11. 资源或 Shader 失败时写明错误并按有限超时执行安全退化，不能无限占据画面；不要阻塞主线程等待 AssetServer。
12. 已有 `GALLERY_WAVEFORM_BENCH_OUTPUT`、`GALLERY_FILE_DIALOG_BENCH_OUTPUT`、`GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT` 模式原则上不插入这 1 秒动画，以免污染现有 benchmark 的测量语义。

## 核对资料

- [Bevy v0.20.0 — UI 渲染到纹理](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/render_ui_to_texture.rs)：`Image::new_target_texture`、`RenderTarget::Image`、`Camera.order`、`UiTargetCamera`。
- [Bevy v0.20.0 — UI Transform](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_transform.rs)：`UiTransform` 旋转、平移、缩放与 UI ImageNode。
- [Bevy v0.20.0 — UI Material](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_material.rs)：`UiMaterialPlugin`、`MaterialNode`、`AsBindGroup`、动态材质。
- [Bevy v0.20.0 — UI Material shader](https://github.com/bevyengine/bevy/blob/v0.20.0/assets/shaders/custom_ui_material.wesl)：WESL `UiVertexOutput`、颜色/贴图采样示例。
- [Bevy v0.20.0 — UI picking source](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/picking_backend.rs)：默认 UI 拾取阻挡与 `FocusPolicy` 不参与该后端。
- [Bevy v0.20.0 — UI z 排序定义](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/ui_node.rs)：`GlobalZIndex` 的跨层级行为。
- [Bevy v0.20.0 — Embedded Asset](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/asset/embedded_asset.rs)：不依赖运行时磁盘的素材嵌入。
- [Gallery main.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)、[Gallery assets.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/assets.rs)、[startup_cursor.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/startup_cursor.rs)、[window render.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/render.rs)：与真实宿主一致的入口、Asset、首帧与 DX12 行为。

> **版本警戒：** 网络搜索常返回 `bevy 0.19.1` 或 Bevy GitHub `main` 的代码。本计划已用 GitHub `v0.20.0` tag 对关键例子做定点核验；Codex 实施时也必须以 `=0.20.0` + 用户当前仓库为准。编译签名和具体 runtime 顺序始终以本机实际依赖版本为终审。
