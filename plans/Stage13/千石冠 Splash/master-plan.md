# 千石冠 Splash：Bevy 0.20 完整技术实施总方案

> 适用仓库：[`slc90/bevy_widgetry`](https://github.com/slc90/bevy_widgetry)，以当前 `main` 上 `bevy = "=0.20.0"` 为准。目标平台 Windows x64，使用现有透明 DX12 窗口。本文是交给 Codex 的**实施设计**，不是已经完成的代码，也不代表已经过 Windows 编译或实机性能验收。
>
> **来源区分：** 千石冠视觉、1 秒演出、60 FPS、圆形波纹、保留启动流程等来自双方先前确认的方案；离屏 UI + `UiMaterial`、资源同步、接口文件安排是本次依据 **Bevy 0.20.0 正式源码/示例**及仓库实况补充的技术设计。凡需 Windows 实测才能定论的事项明确列为验收项。

## 总体目标与最终效果

Gallery 保持现有的 `main()` 顺序：`AppLogging::new`、`block_on(transparent_render_creation())`、`DefaultPlugins`、Widgetry/ Gallery 插件、`Startup` 创建主窗口/唯一 `Camera2d`/WidgetryWindow、`GalleryPage::Initializing → Button`，以及现有 `startup_cursor` 首帧流程。**不替换 DX12 初始化，不新建原生 Splash 窗口，不延后全部 Gallery 构造。**

Splash 在同一个 Gallery 窗口覆盖整个客户区；没有 Logo、进度条、仓库相关文字、独立小卡片或假加载时间。角色为《Slow Start》千石冠，以用户提供的[官方角色页](https://slow-start.com/character/?chara=kamuri)和本包 `assets/selected/approved_key_art.png` 为美术依据。上半身从底边探出，轻轻挥手，两根深蓝丝带随动，短促眨眼；最后由挥手手掌附近扩散的圆形波纹逐步显露真正的 Gallery。Splash 合成画面在转场前完全不透明；转场时露出的是**已经绘制的 Gallery 画面**，不是透明 Win32 桌面洞。

名义演出 1000ms，设计节奏：`0–200ms` 探头、`200–550ms` 挥手与丝带、`550–800ms` 眨眼及安定、`800–1000ms` 波纹交接。60 FPS 是目标刷新能力（单帧预算约 16.67ms），不是每 1/60 秒强制硬切图；使用经过时间决定状态，渲染性能需实机确认。Gallery 不就绪时，允许在最后姿势短暂等待，使总墙钟时间超过 1 秒，但**绝不提前揭幕空白或未就绪的 Gallery**。

## 已核对的工程事实与总体技术取舍

- `Cargo.toml` 固定 Bevy 0.20.0，`default-features=false`，含 `bevy_ui`、`bevy_ui_render`、`bevy_render`、`bevy_core_pipeline`、`png` 和 `ui_picking`，**未启用 `bevy_sprite` / `bevy_sprite_render`**。不为本测试添加 Sprite 渲染功能；使用已经启用的 UI 图层能力。
- `gallery/src/main.rs` 的 `setup` 创建 `Camera2d` 和 deferred `widgetry_window` Scene，随后 `NextState<GalleryPage>::Button`；`gallery/src/pages/button.rs` 在 `OnEnter(Button)` 同步挂载页面。不能把 `Startup` 已完成当成 Gallery GPU 画面已完成。
- `crates/window/src/scene.rs` 的 `prepare_native_window` 设置 `transparent=true`、`decorations=false`、`CompositeAlphaMode::PreMultiplied`；`crates/window/src/render.rs` 手动创建 DX12 `DxgiFromVisual`。不修改此路径。
- `gallery/src/startup_cursor.rs` 已有 RenderApp 首次 present + UI background pipeline 检测与 `DwmFlush` 后的主线程信号；可以**最小化地复用/暴露一次性就绪信号**，但这只能证明当前检测的 Gallery 背景绘制，不等同于 Button 全部内容均完成渲染。
- Bevy 0.20.0 官方 `examples/ui/render_ui_to_texture.rs` 展示 `Image::new_target_texture`、`RenderTarget::Image`、`Camera { order: -1 }`、`UiTargetCamera`；`examples/ui/ui_transform.rs` 展示 `UiTransform` 的平移/旋转/缩放；`examples/ui/ui_material.rs` 展示 `UiMaterialPlugin`、`MaterialNode`、`AsBindGroup`、自定义 fragment shader 与运行时材质更新。
- **技术主线：** 千石冠的所有 UI 图层先由一个离屏 `Camera2d` 渲染成不透明 `Image`；主 Gallery Camera 仍向原窗口渲染 Widgetry UI；最顶层的全窗 `MaterialNode<SplashRevealMaterial>` 采样这张离屏纹理。揭幕期间，shader 仅把 Splash 对应区域的 **alpha** 降至 0，自然露出下层 Gallery；不必先将 Gallery 再复制到另一张纹理，也不必在 GPU 读取 Windows 桌面。
- 官方示例使用 `Bgra8UnormSrgb` 离屏纹理。实际格式、采样限制、WGSL/WESL 导入、alpha 混合与窗口预乘模式的组合，应以 0.20.0 示例为起点，在 Windows DX12 **实际验证**，不能照搬其他版本 API。

## 跨阶段不变量（每个受影响阶段都必须遵守）

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

---

## [S01] 标准化角色与画布素材

### 设计依据及真实素材状态

本包 `assets/selected/` 放的是已经在对话中视觉接受的候选，包括 `approved_key_art.png`（完整背景+角色、**不透明**）、`body_open.png`（无丝带睁眼透明主体）、`body_closed_aligned.png`（Python 局部眼部替换修正版）、`arm_candidate.png`、`ribbon_upper_candidate.png`、`ribbon_lower_candidate.png`。配套官方设定、合并丝带参照、全幅挥手图在 `assets/reference/`；失败/历史版本与 blink 验证动图在 `assets/experiments/`、`assets/qa/`。具体原文件名、尺寸、SHA256、透明 bbox 参阅 `metadata/asset-manifest.json`。

**现状必须如实对待：** 选定完整画面和主要图层尺寸为 1672×941，原图并非原生 1080p。`body_open` 与 `body_closed_aligned` 的像素差异恰好位于 `(662,378)–(1022,616)` 的眼部邻域，约占画面 3.509%；眼部范围外相同。该修正已由用户视觉确认。**独立手臂、上下丝带是图像生成候选而不是对原基准图直接做的逐像素精确裁切**，默认全尺寸叠放可能错位；尤其上下丝带图的根部坐标不同，不能认为自动重合。部分角色发丝透明边缘还有彩色/黑色噪点，需要质量检查。

### 素材契约与产出

建立统一的 `artboard` 空间（设计 1920×1080，或选择 1672×941 作为规范母版后提供到设计坐标的线性映射，二选一并全程贯彻），生成可机器读取的 `layer_map.json`（名称、素材路径、源尺寸、透明裁剪 bbox、绘制起点、z 顺序、默认 pivot、父子关系、缩放/安全框、版本与 checksum）。裁掉透明大画布以节省像素填充成本时，保存裁剪偏移；禁止由于裁剪丢失头顶细长丝带或发尾。

角色默认姿势与已通过的睁眼/闭眼图必须用**相同的布局和 pivot**，切换时只更改 `ImageNode.image` 对应的 Handle，绝不改变布局或根节点变换。验证像素差异只落在眼部替换区域，包括 alpha 差异；禁止把未经修正的原始闭眼图误用作运行时素材。

手臂必须补画移动后可见的肩部/袖口/身后发丝。两根丝带最终需要独立的、**汇合在头顶同一点**的图层；允许保持整体单根形状并用小幅角度摆动，不要求多段骨骼或复杂形变。被丝带遮挡的头顶部位应有足够完整底图，以避免摆动时显露破洞。

必须提供没有任何角色像素的**完整不透明动画底画**。当前 `approved_key_art.png` 是角色与背景合成，**不能**直接作为底画，否则主体探头时会出现第二个静止的千石冠。可以以原画为美术参考重新补全背景，或采用适配同一画风的全窗口渐变/轻微装饰；若采用纯色/渐变只作为场景自身的设计画布，不是额外浮层 UI 卡片。艺术补画无法由普通图像裁剪算法无损还原时，不要虚构已经修好：保留明确的待美术验收项。

拼装预览至少包含静止默认、手臂相对肩部旋转正反角、丝带各自旋转、闭眼/睁眼切换和无背景透明洞检查。默认姿势与 `approved_key_art` 的人物位置、脸型、服饰、双丝带视觉语义尽量一致，并记录任何无法完全保真的局部差异。完成本阶段才称 `ready_for_animation`，而非简单复制 PNG 即视为完成。

### 本阶段验收边界

成功标准是透明 PNG 可通过相同坐标表稳定还原角色；手臂活动后不存在明显肩部缺口；两根丝带共享根部而不飘离；眨眼整体不跳变；底画不含第二份角色。产物为标准化贴图、布局映射、预览图与素材 QA 报告。未满足则留在本阶段修正，不能把视觉问题转嫁给后续 Bevy Shader。

---

## [S02] 建立可离屏合成的 Splash 静态场景

### 渲染结构（优先路径）

保持原 Gallery 目标：`Camera2d` → 原始透明 Windows 主窗口 → WidgetryWindow/UI。新增 Splash 专用 `Camera2d`，其 `Camera.order` 置为低于 Gallery 主 Camera（参考官方示例 `order:-1`），目标绑定 `RenderTarget::Image(handle.into())`。用 `Image::new_target_texture(width,height,TextureFormat::Bgra8UnormSrgb,None)` 创建可采样离屏贴图，交由 `Assets<Image>` 管理。请依据当前官方 API 和 Windows DX12 实测适配格式、纹理 usage 与尺寸更新，不盲目搬用 Bevy 0.19 或 main 分支代码。

把用于绘制角色的根 UI `Node` 显式加上 `UiTargetCamera(offscreen_camera)`；其下是覆盖 100% 的不透明画布、`ImageNode` 角色主体、独立手臂、上下丝带。`ImageNode.image_mode` 采用 `NodeImageMode::Stretch` 时，宽高必须由统一画布布局决定，不能把全尺寸 1672×941 素材任意拉成当前窗口尺寸。建议采用**绝对定位的分层元素**，按上一阶段的 `layer_map.json` 填入定位和枢轴信息。

`UiTransform` 在 Bevy 0.20 支持 `translation: Val2`、`rotation: Rot2`（源文档注明顺时针）、`scale: Vec2`；图层的独立运动由此完成。单根丝带若裁成局部图，必须将图层贴图的根点映射到**相同的 UI pivot 容器**；若直接在 1672×941 全透明画布的中心旋转，会绕错误位置大幅扫动。必要时使用空枢轴 Node 作为父节点，以其局部坐标对子图偏移。

在主 Gallery Camera 对应的另一个根 UI Node 上创建覆盖全窗口的 `MaterialNode<SplashRevealMaterial>`，明确 `UiTargetCamera(gallery_camera)`，使用能压过 Gallery 根和各种 popup 的 `GlobalZIndex`，默认材质只不透明地采样离屏 Image。**不能把 Splash 显示节点挂在会被 Window 内容滚动、裁剪或主题布局约束的 `GalleryPageHost` 中。** 这层既是动画最终的完整画面，也是以后圆形揭幕的唯一遮罩/渲染表面。

官方 `GlobalZIndex` 能跨不同 UI 层级排序，优于假定兄弟 `ZIndex` 一定压过全部弹窗。虽然动画专用 UI 节点不需要响应点击，主窗口这一层要在 Splash 生效时承担**屏蔽 Gallery 指针输入**的职责，完整输入控制留待集成阶段。

### 分辨率、清屏与布局

1920×1080 仅定义设计时的角色构图和原始波纹中心基准。窗口初始配置带 `scale_factor_override(1.25)`，且可改变缩放与大小：离屏纹理像素、UI 的 `Val::Px` 逻辑尺寸、Window 物理像素必须分别计算。原则上使用固定纵横比的**等比覆盖**（cover；超宽/超窄窗口允许裁剪非安全区），为安全区边缘设限；不允许两轴独立缩放压扁角色。需要给 Shader 传实际 viewport 像素尺寸。

离屏 render target 尺寸可以先采用设计级 16:9 贴图（如 1920×1080），再在最终集成阶段根据窗口变化重建或分辨率分档；明确记录性能/内存取舍和 resize 行为。每帧清屏及全窗画布都应产生 alpha=1 的有效颜色；主窗口最早一帧 Splash 贴图还没上屏时，仍应有与动画画面相符的全窗**不透明兜底层**。离屏采样图未准备好时不要暴露 Gallery；这只是在画布准备期保证不闪烁，并非新增一块艺术背景。

### 本阶段验收边界

证明默认姿势可在独立离屏 Camera 渲染、被主相机的全屏 UI 材质采样并覆盖 Gallery；全窗口没有静态角色重影、黑色错误清屏、上下倒置、跨窗口串画面或因 DPI 导致的非等比拉伸。**兜底遮挡层必须与 Splash 材质为不同 UI 节点：先让兜底压住 Gallery，再准备实际 Splash 贴图；实际材质已成功参与窗口 present 后才撤除兜底，以免揭幕时兜底仍在下面挡住 Gallery。** 此阶段无须实现完整角色运动和波纹，但 offscreen → fullscreen 的 GPU 通路必须实机可见。

---

## [S03] 制作按真实时间推进的 1 秒角色实时演出

### 时间轴与运动模型

使用专用 `SplashAnimationClock` 保存**从首次可见帧放行后的播放时间**，转换为归一化进度 `t∈[0,1]`；每次更新计算关键姿势而非累加角度误差。时间来自 Bevy 的真实时间增量并根据窗口不可见、最小化、暂停等状态控制推进，避免窗口失去绘制机会后恢复时直接跳过整个 1 秒。渲染帧数可变，动作曲线是时间函数，不依赖累计 `frame_index`。

| 时间区间 | 核心动作 | 布局与动画条件 |
|---|---|---|
| 0–200ms | 千石冠从底边探头 | 角色整体沿 Y 轴上移；起始时主角不应暴露到窗口，末尾稍过冲再回落（例如 130–200ms 的减速与回弹）；具体移动像素取决于 S01 的 alpha bbox 与视口映射，而非硬编码“上移 200px” |
| 200–550ms | 轻轻挥手 | 以肩部为 pivot 带动手臂做一到两次小幅往返旋转；不能简单平移一张已伸展的大画布；头发暂整体跟随主体，避免一次性加入物理模拟 |
| 200–800ms | 双丝带随动 | 两根丝带共享根部坐标；不同幅度/相位/衰减，动作随角色探头与挥手响应。初期建议小幅数度的角度摆动，视觉上不分家、不穿帮 |
| 550–800ms | 眨眼与恢复 | 睁眼主体 Handle → `body_closed_aligned.png` → 睁眼主体 Handle，闭眼约 50–100ms（60 FPS 下约 3–6 帧的视觉停留），不再让另一张生成图带来衣服/头发闪动 |
| 800–1000ms | 演出尾段与交接 | 保留挥手完成后的稳定姿势，准备从手掌位置开始的圆形波纹。若 Gallery 未就绪，冻结在稳定姿态而**非重新循环演出**；交接开始后播 200ms 的独立揭幕计时 |

缓动建议：探头使用 ease-out + 轻微 overshoot；挥手使用平滑的往返曲线；丝带用延迟/阻尼近似，不求物理准确但须连贯；眨眼先两帧素材切换，不引入眼睛变形 shader。所有关键状态由一个播放器协调，避免多个系统分别自行累计时间而不同步。

### Bevy 更新与资源交互

- 主体只使用一个 `ImageNode`，预先保留强引用 `Handle<Image>`（Open/Closed），仅在表情状态切换时赋值，不在每帧调用 `AssetServer::load`。
- 手臂、两个 Ribbon 各使用 `UiTransform`，从目标 pose 直接赋值；父级承担角色整体平移，避免应用双重变换。
- 以现有 `WinitSettings::desktop_app()` 为前提，动画/揭幕活动期间用 `MessageWriter<RequestRedraw>` 驱动持续绘制；游戏循环不是自动固定 60Hz。若硬件刷新率小于 60Hz 或 DX12 阶段编译/加载造成卡顿，只记录实际帧率与帧时间，不声称保证 60 FPS。
- Splash 结束或错误退化后必须停止主动重绘，不能让空闲 Gallery 永远进入高速刷新。
- 演示调试环境可允许开发开关重复播放与慢速观察，但默认启动只播放一次；已启用 benchmark 时应跳过 Splash（见 S06）。

### 本阶段验收边界

提供可反复播放的实机动画、可跳转时间点的开发态静帧和实际帧时间记录。完整验证探头起点无角色闪现、手臂连接、两根丝带固定根点、闭眼与睁眼画面稳定，动画最终保持准备交接的姿势。

---

## [S04] Splash 生命周期、素材加载和首帧就绪管理

### 状态职责（建议状态机）

引入仅属于 Splash 的生命周期资源，而不复用 `GalleryPage` State。建议阶段：`OpaqueCover` → `WaitingAssets` → `WaitingFirstSplashPresent` → `Playing` → `HoldingForGallery`（仅条件未满足时）→ `Revealing` → `Finished`；失败则进入 `Fallback` 并最终退出，不能死循环。

- `OpaqueCover`：原有 Gallery 窗口一创建，主 Camera UI 即存在全窗不透明覆盖；不等待 asset 才插入覆盖，否则会闪露 Gallery。
- `WaitingAssets`：一次性调用 `AssetServer::load` 获取全部必需 PNG（若使用 embedded assets 则沿现有 registry），保留 Handle。使用 `AssetServer::get_load_state(handle.id())`、`Assets<Image>::get` 或资源事件判断 CPU 资源加载成功/失败；Shader 和 offscreen 纹理则分别检查管线准备。CPU 的 `Loaded` **不是** GPU 可采样的证据。
- `WaitingFirstSplashPresent`：离屏 Camera 已渲染、主相机的 Splash `MaterialNode` 管线和 image bind group 成功参加绘制，且第一帧确实交给目标窗口 present 后，才开始 1 秒演出时钟。可参考 Gallery 现有 `startup_cursor.rs` 的 RenderApp 阶段 + `DwmFlush` 处理，但应有单独的 Splash 展示标志，不能直接把 Gallery 背景已出现当成 Splash 已出现。**得到真实已呈现信号以后撤除兜底背景 Node，保留完全不透明的 Splash MaterialNode；不能让兜底层存活到 Revealing 阶段，因为它会堵住波纹透明区的 Gallery。**
- `Playing`：0–800ms 推进角色曲线，短促眨眼；持续 `RequestRedraw`。如果画面最小化/隐藏，暂停累计动画时间或采取经过验证的恢复策略，不能恢复时直接完成波纹。
- `HoldingForGallery`：角色停在 800ms 最终姿势，Splash 全不透明，同时等待 `GalleryReady`。若先前 Gallery 已就绪，就无需停留。
- `Revealing`：独立 200ms 计时，更新材质 uniform，使波纹半径增长；确认末帧已经呈现后退出覆盖。
- `Finished`：释放临时 Camera/scene/root/material/render texture Handle 和输入屏蔽资源，只保留合理的日志/一次性诊断数据。不要调用 `App::exit`，正常 Gallery 继续运行。

### Gallery 首屏就绪信号

CPU 结构条件：`State<GalleryPage> == Button`、唯一 `GalleryPageHost`、唯一 `GalleryPageContent(Button)`、`WidgetryWindow` 有效。GPU 结构条件：原有 `startup_cursor` 的**真实背景 draw + present + DwmFlush** 判定已出现，或者另一个至少等强度的实机验证。它不自动证明所有文字/按钮已 draw；如需要严格“首屏全部可交互”保证，补充首次 Button 关键 UI 批次就绪检测或清楚记录首屏可见判定的范围。禁止仅凭 `OnEnter(Button)`、`Assets<Image>` 或 `Window.visible` 作结论。

应该把这个只读条件做成 `GalleryFirstFrameReady` 之类的**粘性令牌**，因为 `startup_cursor::enable_cursor` 当前会移除 `StartupCursor` 资源；新模块不能在令牌尚未读取时因为原资源销毁而丢失信息。只给现有机制增加就绪信号，不改变首帧逻辑本身，也不重新执行 DX12 adapter/device 创建。

### 等待超时、容错与可观测性

素材不存在/载入失败、WESL Shader 编译失败、RenderTarget 规格不支持、图像采样 bind group 无法建立、窗口被关闭、Gallery 首屏构造错误，都需要明确 `tracing` 日志，区分 `SplashAssetFailed`、`SplashPipelineFailed`、`GalleryNotReady` 等原因。设置有限超时（例如数秒、具体阈值可配置并实机调整）；**日志+释放阻挡层+暴露原 Gallery/错误处理**优于永远停在无文字的启动画面。超时前保持不透明，不进行假进度显示，不同步阻塞主线程，也不伪装成“已经正常完成”。

建议记录 `splash_assets_ready_ms`、`splash_first_present_ms`、`gallery_ready_ms`、`reveal_start_ms`、`reveal_done_ms`，用于诊断真实时长，而不是把帧率估计替代时序观察。

### 本阶段验收边界

通过人为延迟图片加载、失败路径和 Gallery 首屏延迟，验证不会闪白/提前揭幕、不会无限卡住、动画时间从实际可见时开始，且默认顺利结束后不存在残留循环任务或无效 Handle。

---

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

## [S07] 完成实机验证、回归与 Codex 交付验收

### 验证维度

**静态与单元测试：** S01 所有素材读取格式、alpha 裁剪及 pivots 有明确机器检查；睁眼/闭眼纹理宽高相同，眼睛外严格一致；shader 的圆形距离与四角最大半径数学正确；状态机不可能绕过 `GalleryReady` 进入 `Revealing`，任意 `Failed` 和超时路径最后会清理。布局在 16:9、超宽、窄窗与初始 Gallery 缩放（1.25）下不裁掉必需角色表情/挥手。

**功能性实机：** Windows x64 启动，分别记录首次冷启动与再次运行；抓取开始前覆盖、探头后、挥手、闭眼、波纹 25%/50%/75%、全部揭幕、结束后 Gallery 交互等时间点。重点看没有首次裸露 Gallery、背景双人重影、头顶两根丝带离根、肩部穿帮、闭眼时全身抖动、波纹边缘黑边/桌面穿透以及关停瞬间闪烁。

**就绪与失败路径：** 人为延迟 Splash 贴图、延迟 Gallery Button 构造/图片准备、模拟 shader 资源找不到、关闭窗口、DPI 改动、最小化再恢复。CPU asset Loaded 但 GPU 未呈现时不会开始播放；Gallery not ready 时停在稳定姿势，不提前揭幕；资源失败时有错误日志和有限时间退出，而非挂死。

**输入及回归：** Splash 进行时点击 Sidebar、按 Tab、滚轮、拖标题栏、鼠标停留弹层区域均不能误触底层；完成后各输入全部可恢复。`GalleryPage` 切换、主题和缩放、`WidgetryCursorTheme` 首帧切换及既有 BRP/benchmark 逻辑不受破坏。其它独立 Gallery window 不会意外拥有 Splash。

**性能：** 开启轻量统计（可使用 `FrameTimeDiagnosticsPlugin`，或对时钟/呈现时刻记录），提供暖启动与首次 shader pipeline 编译分开观察的帧时间 p50、p95、最长帧、实际完整演出墙钟时间、资源准备等待时间，60 FPS 目标对应 16.67ms 的理论帧预算。只有在实际 Windows DX12 设备上测得结果才能说明已达到目标。离屏+主窗口两个 Camera 所增加的 GPU 消耗需量化。停止后空闲 Gallery 不应持续不必要的 60 FPS redraw。

**仓库规范：** 遵循 workspace `rust 2024`、`unsafe_code=forbid`、`clippy::unwrap_used/expect_used/panic/todo/unimplemented/unreachable=deny` 等约束；优先只修改 Gallery 自身。以 Windows 适用工具链执行格式、编译、单元测试、Clippy 与可交互实测，记下具体命令和结果。**本交付包自身没有执行 Windows 编译/运行**，不能把计划列出的测试说成已通过。

### 完整完成标准

只有 A 素材几何验收、B 离屏场景、C 1 秒角色动画、D 资源与首帧状态管理、E 圆形波纹 Shader、F Gallery 正式交接、G Windows 回归**全部完成**，才算交付完成。成果不只是截图或视频，而是仓库可运行源码、内嵌运行时素材、可复现的检查/诊断信息，以及存在的剩余风险说明。

---

## 官方参考与实现依据（固定版本）

- [Bevy v0.20.0 — UI 渲染到纹理](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/render_ui_to_texture.rs)：`Image::new_target_texture`、`RenderTarget::Image`、`Camera.order`、`UiTargetCamera`。
- [Bevy v0.20.0 — UI Transform](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_transform.rs)：`UiTransform` 旋转、平移、缩放与 UI ImageNode。
- [Bevy v0.20.0 — UI Material](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/ui/ui_material.rs)：`UiMaterialPlugin`、`MaterialNode`、`AsBindGroup`、动态材质。
- [Bevy v0.20.0 — UI Material shader](https://github.com/bevyengine/bevy/blob/v0.20.0/assets/shaders/custom_ui_material.wesl)：WESL `UiVertexOutput`、颜色/贴图采样示例。
- [Bevy v0.20.0 — UI picking source](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/picking_backend.rs)：默认 UI 拾取阻挡与 `FocusPolicy` 不参与该后端。
- [Bevy v0.20.0 — UI z 排序定义](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/ui_node.rs)：`GlobalZIndex` 的跨层级行为。
- [Bevy v0.20.0 — Embedded Asset](https://github.com/bevyengine/bevy/blob/v0.20.0/examples/asset/embedded_asset.rs)：不依赖运行时磁盘的素材嵌入。
- [Gallery main.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)、[Gallery assets.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/assets.rs)、[startup_cursor.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/startup_cursor.rs)、[window render.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/render.rs)：与真实宿主一致的入口、Asset、首帧与 DX12 行为。

> **版本警戒：** 网络搜索常返回 `bevy 0.19.1` 或 Bevy GitHub `main` 的代码。本计划已用 GitHub `v0.20.0` tag 对关键例子做定点核验；Codex 实施时也必须以 `=0.20.0` + 用户当前仓库为准。编译签名和具体 runtime 顺序始终以本机实际依赖版本为终审。
