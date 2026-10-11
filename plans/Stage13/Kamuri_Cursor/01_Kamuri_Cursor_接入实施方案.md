# 千石冠 Cursor 主题接入实施方案（交给 Codex）

**目标仓库**：[`slc90/bevy_widgetry`](https://github.com/slc90/bevy_widgetry)（`main`，只读核查于 2026-10-11）  
**目标环境**：Bevy `=0.20.0`；Windows x64；`widget_gallery` 使用 `WinitSettings::desktop_app()`。  
**目标**：将已经设计的 **44 种千石冠 Cursor** 接入现有 Widgetry 原生鼠标光标协调器；**仅 Pointer 交付现成的 12 帧动画**，其他 **43 种先保持静态**。其余已选定的 13 种动画是以后增量制作的范围，不属于本轮实现。  
**交付类型**：实施规范、设计参考素材、Pointer 实际候选素材和可复用的动画生产规程；**尚未修改任何 GitHub 源码**。

## 0. Codex 开始前必读（仓库规则不可绕过）

1. 阅读根目录 [`AGENTS.md`](https://github.com/slc90/bevy_widgetry/blob/main/AGENTS.md)、`rules/project-context.md`、`docs/architecture.md`、`rules/task-scope.md`、`rules/development.md`、`rules/code.md`。
2. 按本次修改涉及的内容继续阅读 `rules/architecture.md`、`rules/dependencies.md`、`rules/documentation.md`、`rules/testing.md`、`rules/widget-api.md`、`rules/gui-debugging.md`，以及需要时的 `rules/benchmark.md`、`rules/logging.md`、`rules/git.md`。
3. 按规则在仓库根目录创建并维护 `<任务名称>-进度.md`；依 Type → Red → Green → Refactor 开发；在提交前移除进度文件。
4. 代码相关改动完成后遵循 `AGENTS.md` 的**独立 Code Review** 全流程。目标操作系统 Windows 64 位；用 PowerShell 7 (`pwsh`)；禁止为其他平台添加无需求分支。
5. `plans/` 是仓库历史规划而不是 Codex 开发上下文。**本包 `docs/` 是用户明确提供的当次任务规格**；解包后由 Codex 显式阅读，不把它误当成仓库可主动检索的旧 `plans/`。

## 1. 已核实的仓库事实与改动入口

| 当前文件                                  | 当前事实                                                                                                          | 本次预期修改                                                                                      |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| 根 `Cargo.toml`                           | `bevy = "=0.20.0"` 且 `default-features=false`，显式 features 中尚无 `custom_cursor` / `png`                      | 依据 Bevy 0.20 的实际编译情况启用 `custom_cursor` 及 PNG 图像解码所需 feature                     |
| `crates/cursor/src/lib.rs`                | 导出 `WidgetryCursorPlugin`、`CursorLease`；禁止同时安装官方 `CursorIconPlugin`                                   | 对外增加**最小必要**主题选择/扩展光标语义入口，不改旧入口语义                                     |
| `crates/cursor/src/coordination.rs`       | `Last.before(CursorSystems::Update)`、唯一窗口光标 writer；Mouse hover/祖先继承/Disabled/drag lease/native 优先级 | 将「逻辑上选中什么光标」与「当前动画帧的实际 CursorIcon」分离；增加主题映射与 per-window 播放状态 |
| `crates/cursor/src/lease.rs`              | `Dragging(SystemCursorIcon)` / `Native`，原生控件依赖旧 `CursorLease::dragging`                                   | **保留**旧构造方式；如自定义拖拽语义有真实需求再作兼容扩展                                        |
| `crates/cursor/tests/coordination.rs`     | 有 hover、遮挡、Disabled、窗口隔离、drag/native、无变化不写入等测试                                               | 基于原测试新增主题、动画和错误路径测试；不削减已有 contract                                       |
| `crates/asset/src/lib.rs`、`constants.rs` | 集中内嵌字体/SVG，使用 `EmbeddedAssetRegistry`、语义 `BuiltinIcon`                                                | 集中登记 Cursor PNG 资产及 `BuiltinCursorImage` 等**语义标识**；其他库 crate 不写物理路径         |
| `crates/bevy_widgetry/src/lib.rs`         | `cursor` facade 仅导出 `CursorLease, WidgetryCursorPlugin`                                                        | 必要时 re-export 公共主题切换入口；不要把实现堆在 facade                                          |
| `crates/theme`                            | 既有 `Light/Dark/Pink Dream/Kamuri Violet` 颜色主题                                                               | **不默认强行绑定**光标与颜色主题；按明确的应用配置开启冠光标                                      |
| `gallery/src/main.rs`                     | 低功耗 `WinitSettings::desktop_app()`，已装 BRP runtime                                                           | 安装/展示可选光标主题、提供覆盖 44 语义的验证入口，**不全局改为 Continuous**                      |
| `docs/architecture.md`                    | 已记录 workspace 和依赖图                                                                                         | 若 `cursor -> asset` 等真实依赖发生变化，应在同一改动同步更新                                     |

特别注意 `coordinate()` 现有 `current.set_if_neq(next)`：它当前每次都按语义重新生成 `CursorIcon::System(...)`。改成动画后，如果每轮仍将 `Pointer` 覆盖为第 0 帧，动画就永远无法前进。应让语义变化触发重建，**同一语义下播放系统只更新图集帧索引**，并保持“无变化不写入”的现有 invariant。

## 2. 美术交付的真实状态（实施门槛）

本包清单：`asset_manifest.json` / `.csv`。

- `design_references/`：**44 个条目**的原始设计参考板，分别按 Cursor 类型命名。其中有多张参考板来自同一原始组合海报。**经文件格式审计，44/44 张均为带背景、标题和示意区的 RGB 展示板（43 张 1448×1086，1 张 1449×1086）；绝不可直接当系统光标 PNG 使用。**
- `pointer_animation/frames_master_320/`：`Pointer` **12 张 320×320 RGBA 透明帧**，从已确认的点击动画整理而来（代码可再规范化）。其 GIF 和透明 APNG 也在 `previews/`。
- `pointer_animation/pointer_atlas_128.png`：**已生成** 512×384 RGBA、4×3、每帧 128×128 的候选运行时图集。
- `pointer_animation/frames_runtime_128/`：同一内容的 12 张 128×128 RGBA 连续帧。
- `pointer_animation/pointer_static_128.png`：Pointer 的单帧 fallback。
- `pointer_animation/pointer.meta.toml`：候选参数：12 帧、8 FPS、循环、hotspot `[44,108]`。**这个 hotspot 是根据按钮中心估计的，需要 Windows 真鼠标验证，不是已实测定稿。**
- `pointer_animation/keyframes_original/`：Pointer 动画的 4 张完整原创关键姿势（透明 PNG）；可作为以后修帧的美术依据。
- `scripts/build_pointer_atlas.py`：可重复执行的构建脚本；实际已对图集做 12 帧像素一致性校验。

**注：部分展示板的内部说明栏/标题沿用了生成时的其他 Cursor 名称；以清单的语义映射及实际功能图形核对为准，不能信任板内每个标签。**

**不得宣称 44 款现成可运行。** 除 Pointer 外，**43 款静态系统光标还需要把各自概念参考提取/重新输出为真正的透明 RGBA 独立图片，统一尺寸并确定 hotspot**。背景去除不能损坏浅紫发丝、阴影和抗锯齿；纯粹矩形裁切设计板或简单把浅色背景抠除通常不合格。Codex 应先做素材 QA，无法高质量还原的条目明确标记为 **asset blocker**，交由美术/模型补做，不能以污染的图片冒充完成。

### 2.1 44 种 Cursor 范围

**34 种已有系统语义**：

`Default`, `Pointer`, `ContextMenu`, `Help`, `Text`, `VerticalText`, `Cell`, `Crosshair`, `Grab`, `Grabbing`, `Move`, `Copy`, `Alias`, `NoDrop`,
`EResize`, `NResize`, `NeResize`, `NwResize`, `SResize`, `SeResize`, `SwResize`, `WResize`, `EwResize`, `NsResize`, `NeswResize`, `NwseResize`, `ColResize`, `RowResize`,
`AllScroll`, `ZoomIn`, `ZoomOut`, `Progress`, `Wait`, `NotAllowed`。

**10 种新增业务语义**：

`AddSelection`, `RemoveSelection`, `RangeSelection`, `DragSort`, `InsertDrop`, `ReplaceDrop`, `HorizontalValueAdjust`, `VerticalValueAdjust`, `PanReady`, `Panning`。

实际代码里请以 Bevy 0.20 的 `SystemCursorIcon` 枚举校验拼写，并将**业务扩展 10 种**留在 Widgetry 独立语义层：它们不是 Bevy 已内建的 `SystemCursorIcon` 变体。

### 2.2 本轮动画范围

**正式可接入动画只有 `Pointer`**（12×8FPS，即每轮 1.5s）。另外 13 种是以后根据第二份文档扩展的优先名单：

`Wait`, `Progress`, `Help`, `Grab`, `Grabbing`, `DragSort`, `InsertDrop`, `ReplaceDrop`, `ZoomIn`, `ZoomOut`, `HorizontalValueAdjust`, `VerticalValueAdjust`, `Panning`。

当前实现 **13 种暂时使用各自静态图**。余下 30 种始终静态，无须实现额外演出。

## 3. 期望架构及优先级

### 3.1 不更换现有协调器

使用现有 `WidgetryCursorPlugin` 作为唯一拥有 Window `CursorIcon` 写入权的系统。内部推荐把流程清晰拆为：

```text
真实 Mouse / PointerInput、HoverMap、PointerHits、PointerCaptureMap
    ↓
nearest ancestor EntityCursor（及 Widgetry 扩展语义），Disabled override
    ↓
窗口独立的逻辑 CursorKind / CursorIntent
    ↓
优先级决议：Native lease > Dragging lease > Focused hover > Default
    ↓
系统光标 or 已启用的 Kamuri 主题素材映射
    ↓
per-Window 活跃动画状态 + 固定 hotspot
    ↓
CursorIcon::System / CursorIcon::Custom（仍由同一 writer 应用）
    ↓
Bevy CursorSystems::Update → Windows 原生 Cursor
```

**不得破坏的语义**：

1. 只有 `PointerId::Mouse` 改变 OS 光标；BRP 的 `Custom Pointer` / touch 不得代替真实 Mouse 抢控制权。
2. hover 仍以有效顶层 hit / capture 优先、从命中实体向最近祖先继承声明；`InteractionDisabled` 沿祖先 OR，并覆盖为 NotAllowed。
3. `CursorLease::dragging` 在 drag 期间锁定语义；release、Cancel、owner disable/despawn、window loss、blur、跨窗口等情况按现有行为释放。
4. `CursorLease::native` 让出 OS cursor 写入权。**不在原生窗口移动/缩放期间强推主题帧**。
5. 每个 Window 独立保有当前语义和动画进度；窗口切换不得串帧。
6. 同一语义没有帧变化时不重复写 `CursorIcon`；不要为了播放驱动顺便重置 UI hover / picking。
7. 保留原有 `EntityCursor::Custom` 的调用者指定图像；如无明确覆盖指令，不把它重映射为冠图案。

### 3.2 主题开关

新增最小的**可选**主题配置，例如 `System`（原生系统默认）和 `Kamuri`；不要重新实现颜色 `WidgetryThemeMode`。

- 默认是否启用冠主题由宿主明确配置；库调用者不做选择时维持旧系统光标行为。
- Gallery 提供切换开关或独立测试界面，能看出 System/Kamuri 差别。
- 未加载完成、图片解码失败、平台不能应用自定义 Cursor 等情况采取**系统光标 fallback**（必要时记录 ERROR，但不能阻塞操作）。
- 不引入由不存在的品牌/平台/未来插件驱动的多层配置体系。

### 3.3 34 种系统语义的映射

对 `EntityCursor::System(SystemCursorIcon::<K>)`、drag lease 的 `SystemCursorIcon` 进行主题资源查表：

- 未启用主题 → 保持 `CursorIcon::System(K)`。
- 已启用且 K 的**正式透明资产**已可用 → `CursorIcon::Custom(CustomCursor::Image(..))`。
- K 尚无可用的合格正式资产 → **保留系统 Cursor(K)**，绝不指向一张概念海报。
- 对 `Pointer`：查表结果是动画 atlas + 当前 index；不是每帧重新创建随机 PNG。
- 后续启用 `Wait` 等动画时，只需向该语义的资源描述添加 atlas/时间规则，不改输入派发和 hover 系统。

### 3.4 10 种 Widgetry 扩展语义

推荐用一个小而明确的 `WidgetryCursorHint`（名称最终由 Codex 决定）给 UI entity 声明上述 10 种**Widgetry 独有的光标语义**，其祖先解析和 disabled 规则与现有 `EntityCursor` 协同，不能另起独立 picking 系统。

- 如果同一 entity 同时有 `EntityCursor` 和扩展声明，**预先定义兼容优先级**；建议**既有 `EntityCursor` 优先**。跨祖先仍“距离命中实体最近的有效声明优先”。
- 这 10 种 **只应在确有对应 Widget/业务操作的范围启用**，不替换 `SystemCursorIcon`。
- 不为了把十种示意图全部显示出来而改动每个无关 Widget；Gallery 可以提供明确演示目标。
- `DragSort`、`InsertDrop` 等的 drag 状态，如果需要离开 hover 后仍保持图标，依托现有 lease 生命周期进行**最小必要**的扩展，绝不规避 native lease。
- 不在本次为了未来的完整手势识别实现新拖放框架；无触发者的扩展类型可先作为可声明/可演示语义。

### 3.5 播放状态与时序

`Pointer`：

- 进入当前实际活跃语义 Pointer 时从 frame 0 启动（若仍处于相同语义则不要重启）。
- `fps = 8` → **125 ms/帧**；12 帧循环；每帧热点相同。
- 同一 Window `Pointer -> Pointer` 稳定时只推进帧；`Pointer -> 其他` 立即停；再次进入 Pointer 重启。
- `Native` 期间停止主题动画写入；离开 Native 后根据当时 Mouse hover/lease 重新决议。窗口 focus 或生灭按原有规则处理。
- 使用 Bevy 0.20 `CursorIcon::Custom(CustomCursor::Image(CustomCursorImage { handle, texture_atlas: Some(TextureAtlas { layout, index }), hotspot: (44,108), .. }))` 的具体字段以 **编译时实际 0.20.0 API**为准。
- `TextureAtlasLayout::from_grid(UVec2::new(128,128), 4, 3, None, None)`。不要把 512×384 图集作为“单张大图”应用给窗口，否则 hotspot 会偏、尺寸也错。
- **固定 hotspot** 是相对每格 128×128 的坐标，不是整张 atlas 的坐标；不得跟随人物/按钮动画抖动。
- 优先修改现有 `coordinate()` 写入流程，而不是在其后新增第二个争抢窗口 `CursorIcon` 的 writer。

### 3.6 低功耗窗口的关键技术点

`gallery/src/main.rs` 用 `WinitSettings::desktop_app()`，这意味着鼠标不动时 `App::Update` 不会按 8 FPS 自行运行。**只添加 `Timer` 不足以驱动光标动画**。

- 不将 Gallery 或整个 Widgetry 全局改为 `Continuous` / `game()`。
- 在**至少一个有焦点的 Window 正在播放 Pointer 动画**时，使用 Bevy 的 `RequestRedraw` / 合理的事件循环唤醒手段使定时系统继续获得帧更新；动画结束/被暂停后停止主动唤醒。
- 可以初版使用「仅有活跃动画才持续 RequestRedraw、按 125ms 更新实际帧」；如需更低 CPU 唤醒频率，Codex 可使用已有标准唤醒机制按时触发，**但不要引入 unsafe 或无关新 crate**。
- 真实鼠标**停住不动**时仍须可看见 Pointer 完整循环，而不是需要鼠标移动才播放。
- 同时测试窗口失焦、多窗口、native lease 期间没有无意义唤醒；`RequestRedraw` 应遵循项目 BRP / Gallery 规则。
- 不增加空闲时 60Hz 更新常驻开销。

参考官方示例：

- https://bevy.org/examples/window/custom-cursor-image/ （`custom_cursor` feature、`CustomCursorImage` + Texture Atlas）
- https://bevy.org/examples/window/low-power/ （`desktop_app`、`RequestRedraw`）

## 4. 修改文件建议与分工

| 工作包                     | 涉及目标                                                                      | Codex 应完成的内容                                                                                                              |
| -------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| A. 素材制备                | 本包 `design_references/` → 正式资产待入库区                                  | 对 43 个静态光标逐一制成**真正的透明 RGBA**、去背景/去文字、无边缘污染，按统一尺寸和 hotspot 标定；标记失败项并停用对应主题映射 |
| B. Bevy 依赖               | 根 `Cargo.toml`                                                               | 确认并启用 `custom_cursor`，PNG 解码 feature；编译测试                                                                          |
| C. 内嵌素材注册            | `crates/asset/src/constants.rs`、`src/lib.rs`、`crates/asset/assets/cursors/` | 仅纳入已通过 QA 的正式图；提供 `BuiltinCursorImage` 语义路径；必要时注册 embed plugin                                           |
| D. Cursor theme + animator | `crates/cursor/src/{lib,coordination, ...}.rs`、`Cargo.toml`                  | 逻辑语义与显示帧分离；per-window 状态；主题 opt-in；加载/Fallback；帧驱动；保护旧 leases                                        |
| E. Facade 与文档           | `crates/bevy_widgetry/src/lib.rs` 等                                          | 只暴露使用方必需的主题/扩展声明入口；补 rustdoc                                                                                 |
| F. Gallery 可视验证        | `gallery/src/main.rs` 及既有 page/展示模块                                    | 系统/冠对照、Pointer 动画、44 语义展示入口或分批覆盖场景，不破坏原 Gallery page 结构                                            |
| G. 架构同步                | `docs/architecture.md`                                                        | 只有实际依赖图、crate 角色变化时同步更新                                                                                        |
| H. 验证                    | `crates/cursor/tests/`、BRP + Windows 真鼠标                                  | 旧行为回归 + 新主题/动画 + 真实系统热点验收                                                                                     |

新增 module 数量、名称按职责需要确定，不因为“文件多更整齐”而机械拆分；遵守 `rules/architecture.md`。

## 5. 素材规格与接入检查

### 5.1 当前 Pointer（候选参数，不以估算冒充实测）

| 参数              | 值                                                                     |
| ----------------- | ---------------------------------------------------------------------- |
| 图集              | `pointer_animation/pointer_atlas_128.png`                              |
| 文件实际尺寸      | 512×384 RGBA                                                           |
| 网格              | 4 列 × 3 行，无间距、无 margin                                         |
| 每帧              | 128×128 RGBA                                                           |
| 时间              | 12 帧，8 FPS，循环 1.5 s                                               |
| hotspot           | `[44,108]`，按钮中心估计，**必须用真实 Windows 鼠标确认**              |
| 高分辨率母版      | `pointer_atlas_master_320.png`；建议保留便于后续重新导出               |
| 单帧系统 fallback | `pointer_static_128.png`                                               |
| 素材更新方式      | 先用 `scripts/build_pointer_atlas.py` 生成图集，运行时 Bevy 不负责拼图 |

⚠️ 128×128 视觉尺寸较大，最终光标尺寸应通过 Windows 真实体验判断，可用脚本输出 96 / 64 等统一尺寸版本并**同比例重算 hotspot**；不是固定必须 128。

### 5.2 其余 43 种静态图（素材关口）

对每个条目，记录：

- 唯一类型名与设计参考路径（`asset_manifest.json`）；
- 正式 RGBA 源图片、最终像素尺寸、Alpha 边缘；
- 光标热点坐标，说明锚点位于何处；
- 可辨识性验证：建议展示 24、32、48、64、96/128 px 等尺寸的视觉预览；
- 与光标语义一致：resize 的方向、text 插入位置、禁止标记、drag 的抓取点；
- 人体符合已经确认的 Q 版 C 风格：淡紫长发、半垂蓝眼、**恰好两条向上飘的深蓝丝带**。

**不可自动假设**静态图片都用中心 hotspot。`Default/Help/Progress` 通常与尖端有关，`Text/Crosshair` 通常与中心有关，`Pointer` 与按钮接触点有关；以每张实际构图为准。

如果没有原透明母版，Codex 可先进行最小化提取试验并做人工可读的 QC 汇总，**无法保质的条目应请求新素材**；严禁把 RGB 概念板带标题压成 128×128 伪造运行时资源。

## 6. 验证与验收口径

### 6.1 自动测试（按新增合同设计，而非机械逐函数）

- Theme System 模式下现有 `crates/cursor/tests/coordination.rs` 原有断言行为不变：最近祖先、Disabled、捕获、z-order、窗口独立、native/drag、owner invalid、no-change。
- Kamuri 模式：34 种系统语义映射表覆盖；10 种扩展语义能被声明和决议；缺图或未加载保留系统 Cursor（扩展类型应有明确的系统 fallback）。
- `Pointer`：进入从 0 开始、每 125ms index 递进、12→0 循环、离开停、重进重启；相同语义不重启。
- 与 `Native`、`Dragging`、多 Window、失焦/关闭、disabled、Capture 的状态交错不会留下旧动画。
- 未变化不写 `CursorIcon`；主动唤醒只在需要播放时。
- `pointer_atlas_128.png`：像素 512×384、每格 128、总 12 帧、每格 RGBA；热点合法 `[0,128)` 且固定。自动测试不要把 hotspot“主观合适”当成可程序化证明。

### 6.2 构建/质量

在 Windows `pwsh` 执行仓库规则要求的相关验证（具体范围由改动决定）：

```pwsh
cargo fmt --all -- --check
cargo check --workspace
cargo test -p bevy_widgetry_cursor
cargo clippy --workspace --all-targets
cargo run -p widget_gallery
```

**不能在此包中宣称编译、单元测试、BRP 或真机鼠标已经通过**，这些由 Codex 后续实施和真实环境确认。

### 6.3 GUI 验收必须区分 BRP 和真实 OS Mouse

- BRP `brp_launch(widget_gallery)`，用 ECS 查询或截图确认相应目标 UI 的 hover/disabled/drag 状态、主题切换和 Pointer 资源加载；按 `rules/gui-debugging.md` 释放 BRP Custom Pointer 会话。
- **注意：BRP 常规鼠标模拟使用 `PointerId::Custom`，不负责移动系统 OS 鼠标，也不等价于 `PointerId::Mouse`**。真实 OS 光标贴图、动画帧率、hotspot 与标题栏 native move/resize 的验收须**真实 Windows 鼠标人工操作**；不可用 BRP 的 ECS state 冒充已验收。
- 停住真实鼠标 3 秒观察动画是否继续；hover/离开、按下/释放、Disabled、drag、native move/resize、窗口切换均要确认。实时画质观测 128 与 64/96 尺寸。
- 观察瞬时问题：切换一帧空白、hover flicker、动画 frame0 重置、丢失热点、回到桌面后仍保留错误光标。

## 7. Codex 建议实施顺序（每步均可独立记录验收）

1. **核验素材、确认 license 与接入范围**。读取本包 manifest，校验 Pointer 的图集和 43 款静态图的真实状态。不要因为不能自动抠好图就继续大改代码并宣称全套完成。
2. **验证单个 Pointer Native Cursor 最小闭环**。启用 features、注册该图集，在现有 `WidgetryCursorPlugin` 中实现 opt-in + per-window 播放；静止鼠标能够 8 FPS 循环；热点真机可校准；System 模式保持原行为。
3. **增加静态 34 种与扩展 10 种的 semantic mapping**。接受 43 款素材尚需加工的现实；正式资源逐款通过 QA 后再启用对应映射；缺图落回 System。
4. **融合 10 类扩展声明和 Drag/Native 优先级**，在 gallery 和相关现有 Widget 的确有语义时使用；避免拓展为新手势系统。
5. **按新合同扩展测试、Gallery 验收和 docs**。运行 fmt/check/test/clippy、BRP + 真人 Mouse；按 `AGENTS.md` 完成独立 Code Review。
6. **交付报告**明确区分：已编译/未验证、已接入/仅参考、启用的正式 Cursor 类型、未过素材 QA 列表、hotspot 实测结果及 license 决策。不能笼统称“44 个完成”。

## 8. 交付决策与风险

- **素材 readiness**：原先的 44 张是设计参考板，**不是 44 个运行时透明 Cursor**；其余 43 个 sprites 必须单独完成。
- **动画 readiness**：`Pointer` 12 帧候选图集已经生成并且源帧 RGBA 经程序校验；尚未通过 Windows 真鼠标/Bevy运行时验证，hotspot 初值应视为可调整。
- **原生接口限制**：测试 Windows Winit 自定义 Cursor 更新效率和应用生命周期；本机独显不消除 Windows 自定义光标大小、转换成本或事件循环唤醒要求。
- **用户先前确认的功能语言**：角色直接点击有实体反馈的按钮；不引入常规箭头手型覆盖 `Pointer`；糖果作为小装饰；其他 Cursor 保持各自清晰语义。不要“为了统一”覆盖视觉设计。
- **不扩大工程范围**：不重构 Theme/Window/整个输入系统；不因为将来可能动画化就一口气实现其他 13 组空动画状态。

## 9. 给 Codex 的最终完成定义

**完成**只有在下列事实分别被证明时才能报告：

1. 现有 Widgetry Cursor 语义完全保持且回归测试通过；
2. `Pointer` 动画在 Windows 真鼠标静止状态下完整循环、hotspot 点击准确、不影响 native lease；
3. 目标静态 Cursor 逐项存在**合格透明资源**且可正确映射；素材不足的明确列出 blocker，不得冒充完成；
4. Gallery 的 System/Kamuri 对照和必要交互验证可复现；
5. 项目 `AGENTS.md` 规定的工程流程、文档同步、测试和独立 Code Review 都执行完毕；
6. 公共/私有资产分发决策明确，未获授权不自动公开提交角色素材。

**本方案只描述要做的事，具体实施以仓库实际源码和编译结果为准。**
