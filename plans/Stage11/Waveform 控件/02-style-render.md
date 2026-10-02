# 把 Waveform 接入 BSN，并完成 Style / ViewportNode / Mesh2d 渲染

## 目标

在第一份方案已经稳定的 headless contract 上，把 Waveform 做成符合项目 BSN 习惯的真实 UI widget，并完成第一版 Style 和 renderer：等高多通道 lane、固定 value range、channel palette、Polyline/Envelope 的 triangle mesh、`ViewportNode + RenderTarget::Image + Camera2d + Mesh2d` 渲染链路，以及资源复用和对应测试/benchmark。

这一阶段只解决“已经有正确 raw/reduced data 后，如何作为 Bevy UI 中的波形控件稳定显示”，不重新定义 source、cursor、Live/Replay 或播放控制。

## 范围

本方案负责：

- Waveform `SceneComponent` 与 BSN hierarchy；
- persistent runtime component 与一次性 Scene prop 的边界；
- `ViewportNode`、render target、Camera2d、Mesh2d 关联结构；
- 实际 layout width 对 reducer `output_len` 的驱动；
- 64 channel 等高 lane、固定纵轴映射；
- 无 gap / 无 separator 的布局；
- channel palette 与 vertex color；
- `Polyline` / `Envelope` 到 triangle list 的 geometry；
- Mesh/Vec/Image 等 steady-state 资源复用；
- 与 BSN hierarchy、renderer、资源生命周期直接相关的 integration test 和 benchmark。

本方案不创建 Gallery demo、不生成 `.wfrm` 资产、不做 BRP Gallery 验收或完整 GUI 性能 gate；这些属于第三方案。

## 预期产出

完成后，`@Waveform` 能按项目 BSN 方式生成一个真实 UI widget：外层参加正常 UI layout，内部通过 `ViewportNode` 承载一套 Camera2d/Mesh2d 渲染；64 个 channel 合并在同一 render scene/mesh 中，根据实际 viewport 尺寸从第一阶段的 reducer 获得 `Polyline` 或 `Envelope`，并按 style 规则映射成稳定的 triangle geometry。

## 与前后方案的关系

前置依赖是第一方案的 `WaveformConfig`、cursor/range runtime、Planar working set、`ChannelView`、`WaveformReducer` 和 `ReducedChannel` contract。Style/Render 不得为了方便绘制改变这些数据语义，也不得丢 sample 来降低 mesh 成本。

本方案产出的真实 BSN widget 是第三方案 Gallery 的直接对象。Gallery 只负责组合 source/driver/asset 和观测，不再发明第二套 Waveform hierarchy 或 renderer。

---

## BSN 结构与运行时所有权

项目中的 UI entity hierarchy 统一使用 BSN。Waveform 自身具有明确 ECS 身份并承载运行时语义，因此使用 `SceneComponent`，而不是额外设计 `spawn_waveform()` / `build_waveform()` helper 来绕过 BSN。

建议 hierarchy：

```text
@Waveform
└── Root UI Node
    └── ViewportNode
        ↕
      Camera2d
        ↓
      Mesh2d
```

这里要保持 Scene prop 与 runtime state 的边界：

- Scene/template prop 只用于一次性构造；
- 运行时 system 需要持续读取或修改的数据必须落到持久 Component / asset handle 中；
- 不能让 prop 变成 `WaveformConfig`、runtime 或 renderer state 的第二份副本；
- `WaveformSource` 和 reducer 策略仍作为 runtime handle/component 绑定，不塞入 `WaveformConfig`，也不要求配置对象因为 trait object 人为实现 `Clone`。

BSN 负责声明：

- widget root；
- UI hierarchy；
- `ViewportNode` 结构；
- renderer entity / marker 的组合关系。

运行时 system 负责：

- 创建与维护 Image render target；
- 创建与更新 Mesh asset；
- 根据真实 layout 尺寸更新 reducer/renderer runtime；
- 通过正常 ECS component/asset 机制维护状态。

第一版默认安装 `MinMaxReducer`；未来可以通过替换 reducer runtime handle 使用其他算法，只要仍输出既有 `ReducedChannel` 表示，就不要求修改 BSN hierarchy 或 headless contract。

---

## ViewportNode 渲染路径

第一版按 Bevy 0.19.1 官方 `ViewportNode` 思路建立真实 baseline：

```text
UI Node
→ ViewportNode
→ RenderTarget::Image
→ Camera2d
→ Mesh2d
```

选择这条路径的原因是 Waveform 虽然逻辑上仍是 UI widget，但内容绘制已经不适合用大量普通 UI `Node` 拼成线段。目标场景同时有高采样率和 64 channel，如果把每个 sample/span 映射成 UI entity，会把绘制负载转成大量 ECS entity/layout 工作，既不符合控件语义，也不适合作为性能 baseline。

外层 UI 仍负责 width/height、布局、visibility 和作为控件参与页面排版；内部 2D renderer 负责 Polyline/Envelope。64 个 channel 共用这一套 `ViewportNode + Camera2d`，不能为每个 channel 创建独立 viewport/camera。

第一版只针对常见的单个或少量 Waveform 实例建立这套离屏 renderer，不为未来“几十/上百个独立 Waveform 同时显示”提前设计共享 camera/render target。many-waveform 不是当前目标。

---

## Layout width 与 reducer

Reducer 的 `output_len` 必须来自 Waveform 实际 viewport 的横向像素尺寸，而不是固定常量。这样 800 px、1600 px、2400 px 的控件能生成与真实显示密度相匹配的 reduced data。

Layout width 改变属于 reducer 允许 rebuild 的条件，因为 bucket count / sample-to-x 映射发生变化。正常 steady-state cursor 推进仍使用第一方案的增量 MinMax；不能因为 renderer 每帧都要更新 Mesh，就顺带重新全量扫描 10 s raw history。

Polyline / Envelope 选择继续遵守第一方案的完整 viewport 密度规则：

```text
capacity_frames <= output_len
→ Polyline

capacity_frames > output_len
→ Envelope
```

这里不能改成“当前到达的数据量 vs width”，否则启动阶段会因为 buffered data 少而错误改变 representation。

---

## Lane 布局

所有通道共享同一个 viewport / cursor，在一个 Waveform 实例中上下等高排列：

```text
channel 0
channel 1
channel 2
...
channel N-1
```

规则固定为：

- channel index 从上到下；
- 所有 lane 等高；
- 无 gap；
- 无分隔线；
- 高度不足时仍然强制等高压缩，不增加滚动、不引入 minimum lane height；
- 每个 channel 使用 `WaveformConfig.channel_ranges[index]` 的固定 value range；
- `value_max → lane 顶部`；
- `value_min → lane 底部`；
- 第一版不做 auto-scale。

因此 64 channels 即使在高度较小的 viewport 中变得非常密集，也保持同一布局规则。Gallery Stress 的目的之一就是直观看这种高密度显示和渲染成本，而不是通过隐藏通道、滚动或可变高度规避压力。

---

## Channel 颜色

Style 提供一个 channel palette，并按 index 循环：

```text
color = palette[channel_index % palette.len()]
```

目标不是让 64 路全部拥有唯一颜色，而是保证相邻 channel 有明显区分。这样无需维护 64 个专属色值，也能在没有 gap / separator 的情况下帮助肉眼区分 lane。

同一 channel 的 Polyline 和 Envelope 使用同一颜色，reducer representation 不改变颜色语义。颜色通过 Mesh vertex color 进入同一个 mesh，不为每个 channel 创建独立 material/mesh entity。

默认 palette 的具体色值可以在 Gallery 肉眼调整；Style contract 只固定“相邻可区分 + 按 index 稳定映射 + 可配置 palette”。

---

## Mesh2d geometry

64 个 channel 合并在同一个 Waveform render scene 中，优先使用一个或少数稳定 mesh，而不是 64 个 Mesh entity。vertex 自己携带 channel color。

绘制 primitive 统一转换为 triangle list，从而避免依赖不同平台/后端对 line primitive 宽度的实现差异，并让 line width 由 renderer 自己控制。

### Envelope

一个 `WaveformSpan` 表示某个 x bucket 的 `min..max`。renderer 把它映射到所属 lane 的 y range，并生成约 1 px 宽的竖向 quad：

```text
bucket x
  │
  │  min..max
  ↓
窄 quad → 2 triangles
```

这保留 MinMax reducer 找到的尖峰包络，同时让最终几何数量与 viewport width × channel count 同量级，而不是与 40,960,000 raw samples 同量级。

### Polyline

当完整 viewport sample 数不高于 `output_len` 时，使用 `Polyline`。相邻点之间生成有宽度的线段 quad，同样落到 triangle list。默认 line width 约 1 px。

Polyline/Envelope 最终进入同一 Mesh2d 管线，因此 reducer 算法增加时只要输出既有绘制表示，renderer 不需要认识算法名称。

---

## 资源复用与 renderer baseline

第一版要尽量复用 Mesh、Vec capacity 和 render-target 相关资源，避免 steady-state 每帧因为 cursor 前进发生重复分配或 asset/entity 数量增长。

关键约束：

- Mesh entity、Camera、ViewportNode、RenderTarget 不能随每次更新重新 spawn；
- geometry buffer 可以更新内容，但优先保留容量；
- 64 channel 仍共享一套 renderer hierarchy；
- layout width 稳定、representation 不变时，不因同样容量的数据反复触发无意义 reallocation；
- resource 数量在长期运行后保持有界。

第一版先使用标准 Mesh2d 路径建立真实 baseline，不直接跳到自定义 render pipeline。只有当第三方案的 GUI benchmark 证明：headless/reducer CPU 已满足预算，而标准 Mesh2d 的完整 geometry rebuild/upload 成为无法满足 60 FPS 的主瓶颈，才允许替换 renderer backend。

即使替换 backend，也必须保留：

- headless/source/range contract；
- reducer 输出语义；
- Gallery 视觉行为；
- 不隐藏丢 sample；
- 不降低已经约定的 Polyline/Envelope 视觉语义来“通过”预算。

---

## Style / Renderer 测试

这一阶段测试只覆盖新加入的 BSN/renderer contract；第一方案的 config/source/ring/reducer correctness 不在这里重复实现另一套测试。

### Integration tests

放在 `crates/waveform/tests/`，从真实 ECS / BSN 边界验证：

- `@Waveform` 能构造预期的 root UI hierarchy、ViewportNode、renderer marker/runtime；
- Scene prop 只用于构造，运行时配置/状态落到持久 component，不形成两份可漂移 state；
- layout width 被正确转换成 reducer `output_len`；
- width 改变触发需要的 reduced/geometry rebuild，而 steady-state width 不变时不重复做结构性初始化；
- 多 channel lane 按 index 从上到下，全部共享同一 viewport；
- fixed value range 到 lane y 坐标映射符合 `max → top / min → bottom`；
- palette 按 channel index 稳定映射，相邻 channel 能得到不同 style slot；
- `Polyline` 与 `Envelope` 都能生成有效 triangle geometry，并使用同一 channel 颜色；
- headless read failure 时旧 raw/reduced state 不被错误覆盖，renderer 继续消费上一份成功内容，而不是清空或提交半帧；
- 长期更新后 entity、Mesh asset、Image/render target 等资源数量不持续增长；
- Mesh/Vec 容量在稳定负载下不出现无界增长。

涉及复杂 ECS state 的 integration test module 继续按仓库规则在 module-level documentation 中记录 state、stimuli、guards、invariants 与必要 coupling；如果 style/render 测试拆成多个文件，维护轻量 Coverage Map。每个 test function 上方写中文场景/contract 注释。

这一阶段仍不把 Gallery screenshot/BRP 当 integration test；真实视觉动态验收留给第三方案。

---

## Renderer / geometry Benchmark

继续使用 `crates/waveform/benches/`，补上第二阶段才真实存在的生产路径：

- `ReducedChannel::Envelope → triangle geometry`；
- `ReducedChannel::Polyline → triangle geometry`；
- 64 channel 合并 mesh 的 geometry build/update；
- mesh buffer/Vec capacity 复用情况；
- headless update + incremental MinMax + geometry generation 的 CPU path。

负载仍围绕原方案定义的维度，而不机械展开所有组合：

```text
channels:          4 / 16 / 64
sample rate:       8k / 32k / 64k
visible duration:  1s / 5s / 10s
viewport width:    800 / 1600 / 2400 px
input:             steady / meaningful burst
```

固定包含目标：

```text
64 kHz × 64 channels × 10 s
```

并保留约 100 ms 输入一次性到达的 catch-up 场景。

这里的 benchmark 仍主要是 CPU 侧 reducer/geometry baseline；标准 Mesh2d 的真正 render target、GPU upload、frame time 和视觉可观察 latency 必须到第三方案的 Gallery GUI benchmark 中测，不能用 crate benchmark 替代完整渲染结论。

---

## 本阶段明确不做

本阶段不能因为已经进入 renderer 就顺手加入交互或播放器能力：

- 不加入 Waveform 内部 Live / Replay enum；
- 不加入 play / pause / seek / playback_rate；
- 不加入鼠标点击、拖动、picking、zoom；
- 不加入上一页 / 下一页；
- 不允许运行时修改 visible duration；
- 不做 auto-scale；
- 不做通道折叠、分组、独立高度；
- 不加 lane gap / separator；
- 不处理设备同步、丢包、插值；
- 不为 many-waveform 提前设计共享 camera/render target；
- 不引入第三方 downsampling crate。

Style/Render 的职责只是在既定 headless 数据语义上稳定绘制，不把 UI renderer 变成新的数据层或播放控制层。
