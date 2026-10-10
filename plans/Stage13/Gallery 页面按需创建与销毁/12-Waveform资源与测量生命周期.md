# 12｜将 Waveform 生产者与页面状态按需创建并兼容基准测试

## 目标

让 Waveform 的 Live/Replay 生产者仅在页面激活期间运行和占用对应 Resource，同时不破坏现有测量系统。

## 范围

`gallery/src/pages/waveform.rs`、`gallery/src/waveform_benchmark.rs` 及公共页壳的 benchmark 布局约束。

## 预期产出

Waveform 首次进入才初始化生成器和状态，退出后不再运行演示 producer，重新进入从新时间基准生成；现有 bench 可以如常通过导航测量。

## 与前后方案的关系

依赖前面的 Page State、页面挂载和同步退出契约；Window 页作为最后一种特殊窗口生命周期随后迁移。

## 方案正文

### 当前跨页面资源

`WaveformDemoPlugin::build()` 当前 `.init_resource::<DemoSources>()`，`DemoSources::from_world()` 会建立 `Arc<LiveSource>` Basic(4,600)、Stress(64,704000)、`Arc<ReplaySource>`，加载 `GalleryWaveform::BasicReplay` 的 `Handle<ReplayAsset>`，并保存 `elapsed`、`last`、`replay_start`、`generated_at`、`frame_ms`、`producer_ms`、`burst`、`hold_until` 等生产时序。`scene(&DemoSources)` 创建 Basic Live/Replay/Stress 三个 Waveform View，各有 `WaveformDemoState` Component；`color_examples` 克隆 Basic LiveSource 的 Arc。

`OnEnter(Waveform)` 才初始化新的 `DemoSources`（继续使用 `FromWorld` 与 AssetServer），插入 Resource，再用它们克隆需要的 Arc 构造 BSN Scene，并挂至 PageHost；演示时序从 Duration::ZERO 和新的 Instant 开始。`OnExit(Waveform)` 先同步销毁所有 Waveform UI/颜色示例，再移除 `DemoSources` Resource，使其 Arc 引用与页面持有的 Handle 正常释放。**不强制删除整个全局 AssetServer 或共享 `Assets<ReplayAsset>`**：Handle 生命周期不等于全局资源拥有权，旧资源若仍被其他引用使用不能误删。

### System 注册与门控

`WaveformRenderPlugin` 和 `.register_type::<WaveformDemoState>()` 仍是应用级一次注册，不因页面进入而反复 add_plugins。`drive` 保持 `Update.before(WaveformSystems::Update)`，加 `run_if(in_state(GalleryPage::Waveform))`；`sync_color_cursor` 保持 `Update.after(drive).before(WaveformSystems::Update)` 并同样 gate；`status` 保持 `Last.in_set(WaveformDemoSystems::Status)` 且 gate。原因是 `drive(world)` 在未发现可见 waveform 时仍直接 `world.resource_mut::<DemoSources>()`，`status` 强制要求 `Res<DemoSources>`；清掉 Resource 后如全局运行会失败。按状态 gate 后，其他页面不会访问被释放的 producer 数据。原 `visible()` 仅用于之前隐藏页防止继续驱动，可保留与功能无冲突的逻辑，本次不做额外重构。

### 基准测试接口保持

`gallery/src/waveform_benchmark.rs::install()` 是应用级安装，`Last::measure` 依赖 `WaveformDemoSystems::Status` 的调度关系，也会在找到 Name=`StressWaveform` 且尺寸达标后读取 `WaveformDemoSources` 的延迟、生产等数据。因此 `WaveformDemoSystems::Status` label、`WaveformDemoState` 注册、`StressWaveform` Name、`output_width` 等时机不可改坏。**不要**把这个 Benchmark 的 `Measurement` 误当成 Waveform 页面 Resource 删除。测量例程在缺少 StressWaveform 时直接返回，可以等待 BRP 导航进入 Waveform；`gallery/benches/waveform.rs` 本来就通过 readiness 与鼠标点击切页。仍需在实际 Windows 测量环境核对：`GALLERY_WAVEFORM_BENCH_OUTPUT` 启用时公共 `page()` 不加颜色/ScrollArea 包装，StressViewport 必须维持原 1600×640 显示证据，计时从就绪后开始，frames.csv/完成标记继续生成。

如果用户在 benchmark 会话中切走 Waveform 后又回来，新的 producer / state 会重置，而应用级 Measurement 仍可能延续一次测量；不要在本次无依据地改变 benchmark 的测量生命周期，至少应把“测量期间保持 Waveform 页面”作为验证场景与使用前提。

**源代码：** [waveform.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/waveform.rs)、[waveform_benchmark.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/waveform_benchmark.rs)、[gallery/benches/waveform.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/benches/waveform.rs)。
