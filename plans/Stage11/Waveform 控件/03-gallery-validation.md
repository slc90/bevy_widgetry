# 建立 Gallery Live/Replay/Stress 场景并完成真实 GUI 与性能验收

## 目标

把前两份方案完成的 Waveform 作为真实控件接入 Gallery，以直观而可重复的场景同时验证正确性、Live/Replay 统一语义和目标高采样率性能。Gallery 只负责提供外部 source、cursor driver、测试资产和观测，不重新实现 Waveform 内部模式。

最终用 Widget Gallery + BRP 动态验证和 Gallery GUI benchmark 建立第一份可追溯 baseline，并以 **64 kHz × 64 channels × 10 s、稳定 60 FPS、持续输入无 backlog、dropped sample = 0** 作为本版本完成性能 gate。

## 范围

本方案负责：

- 新增 Waveform Gallery entry；
- Basic Live、Basic Replay、Stress Live 三个直观实例；
- synthetic source / producer 与 Live/Replay cursor driver；
- Replay 预生成 `.wfrm` 资产、生成工具与 Gallery embedded asset 注册；
- 必要的状态/FPS/frame-time 展示；
- 按项目 `rules/gui-debugging.md` 做任务级 BRP 动态验收；
- 完整 `ViewportNode + Camera2d + Mesh2d + render` GUI benchmark；
- 记录 baseline、性能预算结论和未验证项；
- 完成 facade/plugin/workspace/docs/assets/依赖面的最终全局一致性检查。

Gallery 自身不增加 unit/integration test。局部 correctness 和 ECS/BSN contract 已分布在前两份方案的 crate tests 中；Gallery 的价值是可观察 GUI 行为与完整 renderer 性能。

## 预期产出

完成后 Gallery 中能直接看到：一个 Basic Live、一个读取固定文件数据的 Basic Replay，以及一个真实压力参数的 Stress Live。Live/Replay 使用同一个 Waveform 实现而只替换外部 driver/source；BRP 能观察启动填充、滚动、颜色、lane、Replay loop 等动态行为；GUI benchmark 对目标场景给出可复现的 frame/latency/backlog/dropped-sample/memory 数据，并留下第一份 baseline。

## 与前后方案的关系

前置依赖是第一方案稳定的 headless/source/reducer contract 和第二方案完成的 BSN/ViewportNode/Mesh2d widget。Gallery 不允许为了演示方便添加 Waveform 内部 Live/Replay 分支，也不通过改变 renderer 正常配置、减少数据语义或隐藏 dropped sample 来取得性能结果。

这是当前拆分链的最后一份方案。完成 Gallery GUI/benchmark 后还要做全局影响检查，确认 facade、plugin、workspace dependency、architecture docs、asset 注册、依赖面和 Code Review 没有遗漏，然后才能认为第一版方案完整落地。

---

## Gallery 页面结构

新增一个 Waveform gallery entry，直观展示三个实例：

```text
Waveform
├── Basic Live
├── Basic Replay
└── Stress Live
```

Basic Live 和 Basic Replay 同时存在，而不是用一个实例加按钮切换。Gallery 的目的就是让行为一眼可比较，避免为了演示模式切换先引入一套控制 UI。

Live / Replay 不对应 Waveform 内部的两种 mode。两者只区别于外部：

```text
Basic Live
Synthetic source 持续追加
→ cursor = 已确认可用的 latest frame

Basic Replay
预生成固定数据
→ Replay driver 按时间推进 cursor
```

Waveform 仍然只看到 cursor、source 和固定 config，因此可以直接验证前面设计的核心结论：Live/Replay 在控件内部具有完全相同的 viewport、buffered range 和 renderer 行为。

---

## Basic Live

Basic Live 用小规模、肉眼容易判断的确定性信号验证正确性，不把压力测试参数和视觉诊断混在一起。

建议配置：

- 4 channels；
- 较低 sample rate，例如 1 kHz 或 8 kHz；
- 5 s viewport；
- 程序化确定性信号：sine、square、saw，以及不同幅值或周期的波形。

这些数据应按 sample index 确定性生成，同一个 frame index 每次得到相同值，便于观察是否发生 lane 串线、时间错位、range 映射错误或滚动残留。

Basic Live 主要用于直观看：

- 启动时数据从左向右填充；
- viewport 未填满时右侧保持空白，而不是把已有少量 sample 拉伸到整个宽度；
- 一屏填满后 cursor 固定右边，viewport 开始向左滚动；
- channel index / lane 顺序正确；
- 相邻 channel 颜色可区分；
- `value_min..value_max` 映射正确；
- 低密度时 Polyline、高密度时 Envelope 的表示切换符合完整 viewport 密度规则。

---

## Basic Replay 与固定测试资产

Replay 数据必须预先生成并提交进仓库，Gallery 运行时只读取，不现场生成。这样每次 Replay demo 使用完全相同的数据，同时真正走“文件/asset → source adapter → Waveform”的路径。

Gallery 自有资产放在：

```text
gallery/src/assets/waveform/basic_replay.wfrm
```

并通过：

```text
gallery/src/assets.rs
GalleryAssetPlugin
```

完成 embedded 注册与语义访问。不得把 Gallery 自有测试 asset 下沉到 Widgetry library asset crate。

### `.wfrm` 格式

使用简单、自定义、版本化、little-endian、Planar `f32` 格式，避免为了单个 Gallery asset 引入 WAV、npy、bincode 等额外语义和依赖：

```text
Header
- magic:       "WFRM"
- version:     u32
- sample_rate: u32
- channels:    u32
- frame_count: u64

Data
- ch0[frame_count]: f32 LE
- ch1[frame_count]: f32 LE
- ...
```

文件只描述“采到了什么数据”。各 channel 的 `value_range` 仍由 Gallery 中的 `WaveformConfig` 提供，不写入 `.wfrm`，避免把 UI 解释语义混进原始测试数据格式。

建议 Basic Replay 数据：

```text
8 kHz
4 channels
30 s
≈ 3.84 MB raw f32 payload
```

生成工具放在：

```text
gallery/tools/generate_waveform_data.rs
```

生成结果提交仓库。生成工具只在需要更新测试资产时手动运行，不属于 Gallery runtime。

Replay driver 按时间推进 cursor；到末尾后可以重新从 0 开始，以便持续观察 loop/restart。Loop 只是 Gallery 外部 driver 行为，不进入 Waveform 本体。

---

## Stress Live

Stress Live 固定使用第一版性能目标场景：

```text
64 kHz
64 channels
10 s viewport
```

这里不在 `WaveformSource::read()` 内实时计算大量昂贵三角函数，否则 FPS 下降时无法区分到底是 Waveform 还是 synthetic signal generation 在消耗 CPU。

使用独立 synthetic producer 模拟设备持续采集：

```text
Synthetic producer
    ↓ 持续产生 frames
Planar source ring buffer
    ↓
WaveformSource::read(range)
    ↓
Waveform
```

数据内容使用便宜、确定性的 saw / triangle / 简单周期序列，通过 channel index 改变周期、相位或幅值，使相邻 lane 视觉上并非完全相同，同时避免让大量 `sin()` 成为压力测试主成本。

producer 根据 elapsed time 计算理论上应该产生到的 frame index：

```text
target_frame = elapsed × sample_rate
frames_to_produce = target_frame - produced_frames
```

因此输入速率不依赖 Gallery 当前 FPS。即使某一帧变慢，producer 下一次会生成 catch-up 数据，而不是因为 UI 变慢就自动降低“设备采样率”。这对检测 backlog 很重要。

正常实时更新仍是批量推进，而不是一 sample 一次 update。source 自身维护 Planar ring/buffer，cursor 只推进到已经确认可用的 latest frame；Waveform 按第一方案的 IncrementalRead 只补新增尾部。

---

## Gallery 状态展示

Gallery 只显示少量对理解/性能观测有用的信息，不把页面变成诊断面板。

Stress 区至少可见：

```text
sample rate
channel count
viewport duration
producer frame / displayed frame
FPS / frame time
```

这些状态用于人工确认配置没有偷偷降级，并帮助 BRP/benchmark 记录目标场景。`producer frame - displayed frame` 也可以反映 backlog 是否持续增长。

Basic 页面不需要为每个内部 runtime 字段增加 UI；必要的 cursor、buffered range、renderer state 可在 BRP 验证时通过 ECS/diagnostics 读取，而不是永久塞进 Gallery 视觉布局。

---

## BRP GUI 验证

按项目 `rules/gui-debugging.md` 使用 Widget Gallery + BRP 做任务级真实运行验收。BRP 不是长期 exhaustive regression suite，也不代替前两方案的 unit/integration tests；它用于验证真实窗口中跨多帧发生的 GUI 行为。

### Basic Live / Replay

必须观察动态过程，而不是只在稳定后截一张最终截图：

- 数据从左向右填充；
- 一屏填满后平滑向左滚动；
- Live / Replay 使用同一 Waveform 行为，没有内部 mode 导致的视觉分叉；
- lane 顺序正确；
- `value_max → top / value_min → bottom`；
- 相邻 channel 颜色区分正确；
- 无 gap、无 separator；
- Polyline / Envelope 视觉结果符合密度和 min/max 语义；
- Replay 确实来自 `.wfrm` 固定资产，而不是运行时重新生成；
- Replay loop/restart 不出现错误旧数据残留、整屏闪空、lane 串线或明显 flicker。

截图用于视觉确认。必要时配合 ECS state / diagnostics 检查当前 cursor、buffered range、channel count、representation 和 renderer state，避免仅凭最终图像误判时序问题。

### Stress

持续观察 64 kHz × 64 ch × 10 s：

- 无周期性空白；
- 无明显“长时间卡住后一次性大跳”；
- 无旧数据残留或 channel/lane 串线；
- backlog 不持续增长；
- Gallery 使用正常 ViewportNode/Mesh2d 配置，没有为了测试临时切换到更低质量或低数据率模式；
- producer 输入速率始终由 elapsed time × 64 kHz 决定，不随着 UI FPS 下降而变慢。

BRP 往返耗时和 screenshot 捕获耗时不能当作 Widget latency 指标。

---

## Gallery GUI Benchmark

完整 `ViewportNode + Camera2d + Mesh2d + render` 性能归 Gallery App benchmark，不能用前两阶段的 headless/geometry CPU benchmark替代。

目标 stress viewport 使用固定、可重复记录的尺寸。建议基准宽度 1600 px，并在结果中记录实际 viewport 尺寸、窗口配置和构建配置，避免不同机器/窗口大小的数据被混成同一 baseline。

测量和报告至少包括：

- sustained input throughput；
- update / frame time；
- FPS；
- p95 / p99 frame 或 update latency，作为观测指标；
- data generation → observable display latency；
- backlog；
- dropped sample / frame；
- allocation；
- CPU / GPU 成本，在能够可靠取得时分别记录；
- raw / reduced / mesh working-set memory；
- 长时间运行后的资源增长趋势。

不能把 BRP 请求、remote round-trip 或 screenshot 编码时间计入 Waveform latency。

### 固定目标场景与预算

目标：

```text
64 kHz × 64 channels × 10 s
```

完成条件：

- 稳态 60 FPS；
- 核心 update/render 路径持续满足 16.67 ms frame budget；
- 持续输入能够跟上 64 kHz × 64 ch；
- backlog 不持续增长；
- dropped sample = 0；
- 不通过隐藏丢数据、降低既定 output/reducer 语义、减少 channel/sample rate 或改变 Gallery 正常运行模式取得结果。

前两阶段 benchmark 中的 100 ms burst/catch-up 场景在完整 GUI benchmark 中也应保留一个有意义的观测，确认短暂输入积压不会形成永久 backlog。

新增能力没有历史 baseline，因此本任务建立第一份可追溯 baseline。记录至少包括：源码版本、场景、负载、viewport/window 配置、构建配置、实际指标和预算判断。p95/p99 等指标是观察工具，最终 gate 仍是稳定 60 FPS、无持续 backlog、dropped sample = 0。

如果结果显示 CPU reduction 已满足预算，而标准 Mesh2d rebuild/upload 是主要瓶颈，则回到第二方案允许的 renderer backend 替换边界处理；不能通过破坏 headless contract 或降低数据语义绕过问题。

---

## Gallery 测试边界

Gallery 本身不新增 unit/integration test。它必须：

- 正常编译；
- 使用真实 `@Waveform` BSN widget；
- 通过上述 BRP GUI 动态验证；
- 通过完整 GUI benchmark 建立 baseline。

可以在 Gallery 中保留 deterministic synthetic data 和固定 replay file，以提升复现性，但不要把这些 demo helper 误当成 Waveform crate 的 correctness test。真正的 config/source/ring/reducer/renderer invariant 已分别放在第一、第二方案的测试中。

---

## 完成前全局影响检查

Gallery 与真实验证收口后，对总方案做一次全局一致性检查，只处理第一版真正需要一起完成的关系：

- root workspace member / dependency 已包含 `bevy_widgetry_waveform`；
- 顶层 `bevy_widgetry` facade 的必要 dependency、re-export 和 plugin 装配完成；
- Gallery 依赖新的 Waveform crate，且页面/BSN 使用正式公开 API；
- `docs/architecture.md` 已反映新增 crate 的角色和依赖关系；
- `gallery/src/assets/waveform/basic_replay.wfrm` 已通过 `gallery/src/assets.rs` / `GalleryAssetPlugin` 正确注册；
- Gallery 自有 asset 没有错误下沉到 library asset crate；
- 没有为了 MinMax/Replay 格式引入不必要第三方依赖；
- crate unit/integration tests、workspace checks、BRP GUI、crate benchmark、Gallery GUI benchmark 都有实际结果；
- 自动 Code Review 完成；
- 最终记录实际 benchmark、性能 gate 判断，以及仍然未验证的项目，而不是把计划中的目标写成已经测得的事实。

这个全局检查是影响面确认，不借机重构无关 crate 或扩展新功能。

---

## 第一版最终范围边界

完成 Gallery 并不意味着顺便进入播放器或交互控件阶段。第一版仍明确不做：

- Waveform 内部 Live / Replay mode enum；
- play / pause / seek / playback_rate；
- 鼠标点击 / 拖动 / picking；
- zoom；
- 上一页 / 下一页；
- runtime 修改 visible duration；
- auto-scale；
- 通道折叠 / 分组 / 独立高度；
- lane gap / separator；
- 设备同步 / 丢包重建 / 插值；
- many-waveform 场景的共享 camera / render target 优化；
- 第三方 downsampling crate。

这些能力后续都应在已经验证过的 headless/source/reducer/renderer 边界上增量扩展，不进入本轮 Gallery 验收。
