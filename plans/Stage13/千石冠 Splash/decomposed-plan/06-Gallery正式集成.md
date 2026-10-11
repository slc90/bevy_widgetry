# 06｜将完成的 Splash 无侵入接入 Gallery

> 执行顺序第 6/7 份。基于 `master-plan.md` 的相应整段详细正文**直接重新分区**，并加入独立实施所需的范围与前后衔接。与后续阶段不是并行分支。对 Bevy 官方 API 的核对版本为 `v0.20.0`；源代码仍以实施时仓库当前提交为准。

## 目标

实现真实主窗口的一次性 Splash 播放、输入隔离与正确恢复。

## 范围

main/Startup、嵌入资源、GalleryReady、已有首帧/光标机制、基准排除、跨窗口边界；保留日志和 DX12 初始化。

## 预期产出

可启动的 widget_gallery 完整链路，不修改公共控件行为。

## 与前后方案的关系

- 前序：[实现从手掌扩散的圆形波纹揭幕](05-圆形波纹Shader.md)。
- 后序：[完成 Windows DX12 实机验证与交付验收](07-实机与回归验收.md)。
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

## [S06] 将 Splash 无侵入地交接到现有 Gallery

### 仓库落点与集成边界

新增尽量局限于 `gallery/src/splash/`（可包含 `mod.rs`、`assets.rs`、`scene.rs`、`animation.rs`、`lifecycle.rs`、`reveal.rs`、Shader 等，实际文件按职责确定），而非修改 `crates/window`/`crates/cursor` 的公共接口。少量接入点：

- `gallery/src/main.rs`：注册 `splash` 模块、插件/系统；在现有 `setup` 创建原 Gallery 主 Camera、Window shell 时同步准备 Splash 不透明遮挡层，避免另外一个 Startup system 无序造成首帧 Gallery 曝光；不得重新创建 Window 或重新 `block_on(transparent_render_creation())`。
- `gallery/src/assets.rs` 和 `gallery/src/assets/constants.rs`：按当前 `EmbeddedAssetRegistry::insert_asset(watched_path(...), &embedded_path!(...), bytes)` 方式嵌入运行时所需 PNG；输出使用 `AssetPath`/`AssetServer`，与现有 GalleryAssetPlugin 保持一致。只嵌入通过 S01 验收的**最终**素材，历史对比图和 GIF 留在交付包，不放进可执行文件。
- `gallery/src/gallery.rs` 与 `gallery/src/pages/button.rs`：维持 `Initializing → Button` 和 `OnEnter/OnExit` 挂载/卸载策略，Splash 不引入 `GalleryPage::Splash`；通过读现有 State、host/content marker 获得 CPU 首屏就绪条件。
- `gallery/src/startup_cursor.rs`：仅在必要时**发布已通过原始首帧判定的粘性 Ready 信号**给 Splash；保留原来首次呈现后切 Kamuri Cursor 的时机、`DwmFlush`、请求重绘行为，不能让 Splash 破坏这段既有逻辑。

### UI 遮挡层与输入策略

离屏场景本身不需要鼠标拾取；主窗口全屏 `MaterialNode` 需要能够覆盖所有 Widgetry 页面、header/侧栏和弹层。Bevy 0.20 的 UI picking 后端明示**忽略 `FocusPolicy`**，而默认 UI 节点可拦截下层 picking；因此不要把 `FocusPolicy::Block` 误当作万能拦截。给 Overlay 定义明确的 `Pickable` / 输入策略，并实测鼠标点击、滚轮、触摸、Tab/快捷键、已有文本焦点和窗口标题栏拖动/关闭按钮：Splash 活跃时不能把交互传给 Gallery，结束后必须恢复正常。

隐藏 Gallery 逻辑节点不可作为阻挡手段：Gallery 必须在揭幕时已经画在 Splash 后面。对于键盘焦点和程序级快捷键，必要时在 Gallery 的入口处理或输入焦点层做短期 gating，并在 Finish/Fallback 清理；避免修改全部 Widget 的公共行为。

### 与现有性能基准、异常和多窗口的关系

默认运行只播一次；可提供 `GALLERY_SPLASH_DISABLE` 或单独开发测试开关，且已有三项 `GALLERY_*_BENCH_OUTPUT` 不应包含 Splash 1 秒。录日志仍使用现有 AppLogging；如果基准测试明确需要测 Splash，则应有专用测试入口而不是污染原测量。Gallery 其它窗口（Window 页独立窗口）不绘制 Splash，离屏 Camera/主覆盖 UI 根显式绑定 Gallery primary camera，不能因默认 UI camera 自动选择而误出现在第二个窗口。

当 Gallery 窗口关闭、最小化、尺寸变化、DPI/scale factor 改变、首次 present 暂停或中途 Shader/asset 错误发生时，不要留下空 Camera、失效 image bind group、无限 redraw 或永久输入屏蔽。先完成 Splash 的实际画面交接，再做资源卸载/销毁；RenderApp 可能滞后一帧清理，需实机观察，避免窗口瞬闪。

### 本阶段验收边界

真实运行 `widget_gallery`（Windows x64）时，启动顺序和原 Gallery 功能均保持，Splash 占满原主窗口，后台 Button 页按原逻辑构造，角色演出和 Gallery readiness 同步，在画面安全且完整时启动圆形揭幕；结束后普通鼠标、键盘、Popup、滚动、窗口移动缩放和原 Cursor 切换全部可用。

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
