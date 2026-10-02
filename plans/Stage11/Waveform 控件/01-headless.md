# 建立 Waveform headless 数据契约与高采样率运行时

## 目标

先完成与具体 UI 绘制无关的 Waveform 核心：固定配置、外部 cursor、viewport/buffered range 语义、统一 `WaveformSource` adapter、Planar ring working set、读取更新与失败提交规则，以及可替换的 reducer 抽象和第一版增量 `MinMaxReducer`。

这一阶段必须让 64 kHz × 64 channels × 10 s 的数据规模在 CPU 数据路径上具有可测、可验证的稳定复杂度，避免把完整 10 s 历史在每次 cursor 推进后重新扫描。Style/Render 和 Gallery 只能建立在这里已经稳定的 contract 上。

## 范围

本方案负责：

- 新增 `crates/waveform` 及其 workspace / facade 基础接线；
- `WaveformConfig`、`WaveformCursor`、时间与 sample boundary 规则；
- viewport range 与 buffered data range 的区别；
- `WaveformSource` 的统一 frame-range 读取契约；
- 多通道 Planar ring buffer working set；
- NoOp / IncrementalRead / FullRead 与 read failure commit 语义；
- ring wrap 到 reducer 的 `ChannelView`；
- `WaveformReducer`、`ReducedChannel` 与手写 `MinMaxReducer`；
- reducer 的增量维护策略；
- 与这些 headless contract 直接相关的 unit test、integration test 和 crate benchmark。

本方案不负责 BSN UI hierarchy、`ViewportNode`、`Mesh2d`、lane 视觉、颜色或 Gallery 页面；这些由后续方案完成。

## 预期产出

完成后，Waveform crate 应有一条不依赖 Live/Replay 模式的稳定数据路径：外部 cursor → sample range planner → source 增量读取 → Planar ring working set → reducer 表示。该路径对多通道同步、range 半开区间、读取失败、ring wrap、MinMax bucket 边界和稳定容量都有明确 contract，并有自动化测试和可追溯 CPU benchmark 保护。

## 与前后方案的关系

这是整条执行链的第一份方案，没有前置 Waveform 实现依赖。后续 Style/Render 只能消费本方案暴露的配置、runtime data view 和 `ReducedChannel` 绘制表示，不得为了渲染方便改变 source、cursor、range 或数据完整性语义。Gallery 则通过外部 source / cursor driver 驱动这套 headless contract。

---

## Workspace 与 crate 归属

新增独立 Widget crate：

```text
crates/waveform
package = bevy_widgetry_waveform
```

Waveform 专属的 headless 行为、Component、runtime、source adapter contract、reducer、headless system、unit/integration test 与相应 crate benchmark 都归该 crate。后续 Style/Plugin/renderer 也继续归同一个 crate，不拆出另一套包。

同时按现有项目模式完成基础接线：

- root workspace member / dependency 配置；
- 顶层 `bevy_widgetry` facade 的必要依赖、re-export 与 plugin 装配位置预留；
- `gallery` 对 `bevy_widgetry_waveform` 的消费依赖，为第三阶段接入做好 workspace 层准备；
- 因新增 workspace member、crate 角色和依赖关系而同步 `docs/architecture.md`，使架构文档真实反映 Waveform crate 的位置。

第一版不新增第三方降采样依赖，`MinMaxReducer` 自行实现。依赖面保持最小，后续若换 reducer 算法也优先通过既有 trait 扩展，而不是把第三方算法库耦合进 `WaveformSource` 或核心配置。

---

## 固定配置与数据规格

Waveform 创建后以下参数固定：

```rust
pub struct WaveformConfig {
    pub sample_rate: u32,
    pub visible_duration_ms: u32,
    pub channel_ranges: Vec<RangeInclusive<f32>>,
}
```

`channel_count` 不重复保存，直接由 `channel_ranges.len()` 得到。`WaveformConfig` 是 Waveform 对数据规格的唯一来源；`WaveformSource` 不再重复暴露 sample rate、channel count 等 metadata。其他需要显示采样率或通道数的 UI 也从 `WaveformConfig` 读取，而不是向 source 查询第二份规格。

构造时至少校验：

- `sample_rate > 0`；
- `visible_duration_ms > 0`；
- `channel_ranges` 非空；
- 每个 range 的端点有限，且 `min < max`；
- `sample_rate * visible_duration_ms / 1000` 必须得到整数 frame 数。

非法配置在 Scene / template 构造路径返回错误，不使用 panic。运行时不再重复处理这些不变量，因此只有成功构造的 Waveform 才进入 runtime。

容量使用整数 frame：

```text
capacity_frames = sample_rate * visible_duration_ms / 1000
```

第一版不允许运行时修改 `visible_duration_ms`。这样 Planar ring buffer 的容量从创建起固定，不需要 resize/rebuild 语义，也不会把 zoom 或 runtime visible-duration 调整提前带进第一版。

---

## Cursor、时间与 sample boundary

外部以时间语义驱动 Waveform：

```rust
#[derive(Component)]
pub struct WaveformCursor {
    pub position: Duration,
}
```

之所以 cursor 继续使用 `Duration`，而不是跟 `visible_duration_ms` 一样改成整数毫秒，是因为运行中的时间精度要求更高。例如 64 kHz 下单个 sample period 为 15.625 μs，整数毫秒只能以 64 samples 为最小步长，无法准确表达采样边界。

Waveform 在进入 runtime range planning 时立即把 cursor 转成 `u64` sample boundary，之后主要使用 sample index：

```text
target_end_sample = floor(cursor * sample_rate)
```

Cursor 的语义是“已经显示到的时间边界”，不是最后一个 sample 自身的时间位置。例如 sample rate 为 1 kHz、已经有 3000 个 frame 时，最后一个 sample index 是 2999，而 cursor 位于 3.000 s，对应半开区间结束边界 3000。

实时 source 若从精确 frame count 生成 `Duration`，提供统一 helper，以向上取整到可表示的 `Duration`，避免 frame boundary 在 `frame count → Duration → sample index` 往返时因为 `Duration` 的离散表示少一帧。时间/sample 换算必须集中，不能在不同 source、driver 或 renderer 中各自重复一套 floor/round 规则。

所有 sample range 统一使用半开区间：

```text
[start, end)
```

这使 `end - start` 直接等于 frame count，并消除 cursor 是否“包含最后一个 sample”的 `+1/-1` 歧义。

---

## Viewport range 与 buffered data range

Headless 必须区分两个概念：

- **viewport range**：屏幕代表的完整时间范围；
- **buffered data range**：Waveform 当前真正拥有并成功提交的数据范围。

初始阶段 cursor 尚未走满一屏时：

```text
viewport = 0 .. capacity_frames
buffered = 0 .. target_end_sample
```

因此数据从左侧开始出现，cursor 从左向右移动，右侧尚未到达的数据区域保持空白。这里不能把 viewport 错写成 `[cursor - visible_duration, cursor]`，也不能制造负时间；第一版明确采用“从左侧填充，填满后才滚动”的行为。

当 cursor 超过一屏以后：

```text
viewport = target_end_sample - capacity_frames .. target_end_sample
buffered = 同一时间窗口内已经成功加载的数据
```

此时 cursor 固定在 viewport 的右边界，viewport 持续向前滚动。

Live 与 Replay 完全使用同一套规则。Waveform 内部不增加 `WaveformMode::Live / Replay`，也不根据模式改变 viewport。区别只来自外部：Live driver 用已确认可用的最新 frame 推进 cursor；Replay driver 用 transport 时间推进 cursor。

---

## WaveformSource：只读 adapter contract

业务、设备、文件或业务缓存的原始数据结构不属于 Waveform 的约束。底层可以是 interleaved、Planar、ring buffer、文件映射或其他布局；外部通过 adapter 实现统一只读接口：

```text
设备 / 文件 / interleaved / ring buffer / 业务缓存
                    ↓
             WaveformSource
                    ↓
         Waveform 内部 Planar 数据
```

第一版接口语义：

```rust
pub trait WaveformSource: Send + Sync + 'static {
    fn read(
        &self,
        range: Range<u64>,
        out: &mut PlanarBuffer,
    ) -> Result<(), WaveformReadError>;
}
```

契约必须明确：

- `range` 是 frame range，并使用半开区间 `[start, end)`；
- 一次读取覆盖全部 channel；
- 成功时每个 channel 长度一致，且对应完全相同的 sample index；
- partial channel 或 partial range 不能作为成功结果；
- 多通道同步、设备丢包对齐、插值等问题不由 Waveform 处理，应在更靠近设备/业务的数据层解决；
- 是否内部并行读取由具体 source/adapter 自己决定，不写进 Waveform contract；
- `WaveformSource` 第一版保持同步接口，不把 Bevy task 生命周期、取消或 staging ownership 提前带进 trait；
- Live driver 只把 cursor 推进到 source 已经确认可读取的位置，因此 Waveform 不主动请求未来数据。

Waveform 内部统一使用 Planar 逻辑访问方式，但这不要求外部原始数据本身采用 Planar 存储。adapter 可以从 interleaved、分段 ring 或其他结构读取并写入 Waveform 提供的 Planar buffer。外部 layout 的适配/copy 被封装在 source 层，后续 reducer 不再看到业务数据结构。

---

## Planar raw working set

Waveform 内部维护当前 viewport 的 **Planar raw working set**，它不是完整历史缓存：

```text
ch0: ring buffer<f32>
ch1: ring buffer<f32>
...
chN: ring buffer<f32>
```

所有 channel：

- capacity 相同；
- capacity 固定为 `capacity_frames`；
- 始终对应同一个 logical sample range；
- steady-state 滚动时不通过 `Vec::drain` 等方式搬移整段历史；
- 新数据只淘汰最左侧已经离开 viewport 的 frame，并追加右侧新增 frame。

“所有 channel 始终覆盖同一个 sample range”是强 invariant。Waveform 不允许 ch0 已推进而 ch1 落后几个 frame 的半提交状态；如果 source 无法提供完整多通道 range，此次读取整体失败。

这块 working set technically 是缓存，但职责仅限“当前显示所需的数据工作集”，不承担完整历史存储、回放文件缓存或设备采集缓存策略。完整历史仍由外部 source/业务层负责。

---

## Range 更新分类与提交语义

比较外部 target range 和当前已经成功提交的 buffered range，只分三类：

```text
目标没有变化
→ NoOp

目标连续向前且与当前 range 有重叠
→ IncrementalRead

cursor 倒退 / seek / 下一页 / 跳得太远导致无重叠
→ FullRead
```

虽然第一版不提供 seek、上一页/下一页 UI，但 runtime contract 允许未来这些外部行为通过 cursor 跳转自然落到 FullRead，不需要给 Waveform 增加模式分支。

### IncrementalRead

正常 Live 或 Replay 连续推进时：

```text
current: 1000..2000
target:  1010..2010

保留:   1010..2000
新增:             2000..2010
```

Waveform 只调用 source 读取新增尾部，并在全部 channel 成功后统一提交。初始填充同样自然适用，例如 `0..512 → 0..1024` 只读取 `512..1024`。

### FullRead

倒退或不连续跳转不能依赖旧 ring 内容，直接读取整个目标数据范围到临时 Planar staging buffer：

```text
read 完整目标 range → staging
成功 → swap / commit
失败 → 丢弃 staging
```

FullRead 的完整 staging 只在不连续跳转期间临时存在，不作为 steady-state 常驻双缓冲。这样既保证失败时能保留上一份有效画面，也避免目标负载下长期把约 164 MB raw working set 复制成两份。

### 读取失败

任何 `read()` 失败都不能提交半成品。失败时：

- 当前 raw working set 不变；
- 当前 buffered range 不变；
- 当前“真正显示的位置”继续以最后一次成功提交的数据为准；
- renderer 后续继续显示上一份有效画面，而不是清空、闪烁或让 cursor 与旧数据错位。

外部 cursor 可以继续变化。Waveform 下一次 update 再根据新的 target 尝试追上；只有完整读取成功后，displayed/buffered range 才推进。

NoOp 则不调用 source，也不产生无意义的 reducer/renderer 工作。

---

## Ring buffer 到 reducer 的 ChannelView

Planar ring buffer 绕回物理尾部后，逻辑连续的一条 channel 可能在内存中分成两段。不能为了让 reducer 拿到一个 `&[f32]` 每帧把整屏复制成连续数组，否则会抵消 ring buffer 避免搬移历史数据的意义。

每个 channel 暴露逻辑连续、物理最多两段的只读 view：

```rust
pub struct ChannelView<'a> {
    pub first: &'a [f32],
    pub second: &'a [f32],
}
```

未 wrap 时 `second` 为空；wrap 后 reducer 按 `first + second` 的顺序把它们视作一条逻辑连续数据。

这里的“两段”只来自 Waveform 自己的 ring buffer physical wrap，与外部 `WaveformSource` 的底层布局无关。外部 source 的 interleaved/分段问题已经在 adapter → Planar working set 的边界处理，不泄漏给 reducer。

---

## Reducer 与绘制表示 contract

Reducer 与 source 明确分层：

```text
WaveformSource
    ↓ raw frames
Planar raw working set
    ↓ WaveformReducer
ReducedWaveformData
    ↓ renderer（下一方案）
```

Reducer 只负责“原始 samples → renderer 可理解的绘制表示”，不负责读取 source、时间、viewport 驱动或播放模式。

算法名称和绘制表示分离。输出 contract 先支持：

```rust
pub enum ReducedChannel {
    Polyline(Vec<WaveformPoint>),
    Envelope(Vec<WaveformSpan>),
}
```

示例关系：

```text
MinMax        → Envelope
Average       → Polyline
RMS           → Polyline
LTTB          → Polyline
```

以后增加算法时，只要结果仍能表达成 `Polyline` 或 `Envelope`，renderer 无需增加分支；只有算法产生现有 renderer 无法表达的新绘制语义，才新增 `ReducedChannel` variant。

第一版默认使用 `MinMaxReducer`。reducer 策略不塞进 `WaveformConfig`，也不要求配置对象为了 trait object 人为实现 `Clone`；它作为 runtime handle/component 绑定。第一版不支持运行中切换 reducer，但保留以后替换策略的边界。

---

## 第一版 MinMaxReducer

`MinMaxReducer` 手写，不引入第三方 downsampling crate。

输出密度由 Waveform 实际 viewport 的像素宽度 `output_len` 决定。虽然 headless 阶段尚未创建 UI Node，reducer API 和 benchmark 仍以 `output_len` 作为输入参数；下一方案从真实 layout width 提供它。

Polyline / Envelope 的选择依据必须是**完整 viewport 的 sample 密度**：

```text
capacity_frames <= output_len
→ Polyline

capacity_frames > output_len
→ Envelope
```

不能用“当前 buffered data 有多少 sample”判断。否则高采样率场景刚启动时只有少量数据，会先错误进入 Polyline，并把前几百个 sample 拉伸到整个控件宽度；正确行为是 viewport 从一开始就代表完整 10 s 或配置时长，只是右侧尚无数据。

Envelope 将一个横向 bucket 覆盖的原始 samples 聚合成 `(min, max)`，后续 renderer 在对应 x 位置画 `min → max` 的竖向 span。相比每 N 个 sample 只取一个值，MinMax 能保住短尖峰，不会因为采样抽取恰好跳过峰值而把波形重要特征直接抹掉。

---

## 增量 MinMax：第一版即纳入性能 contract

目标负载为：

```text
64,000 samples/s × 64 channels × 10 s
= 40,960,000 samples
```

按 `f32` 计，完整 raw working set 约 164 MB。这里的 164 MB 是满屏后的常驻工作集规模，并不意味着 Live 每次都读取 164 MB；正常实时/回放从 0 开始只读取每个批次新增的 frames，10 s 后 ring buffer 才达到固定容量。

但 reducer 不能在每次 cursor 前进时重新扫描完整 40,960,000 samples。即使 source 只增量 read，如果 MinMax 每帧全量重算，仍会直接破坏 60 FPS 目标。因此第一版 `MinMaxReducer` 必须支持增量维护：

- bucket 大小由 `capacity_frames / output_len` 推导；
- bucket 使用全局 sample index 对齐，不随每一帧 viewport 起点重新切分；
- 新 sample 到达时只更新当前/新进入的 bucket；
- 已经完成、仍处在 viewport 内部的 bucket 直接复用；
- viewport 左边界落在 bucket 中间时，只重新计算最左侧 partial bucket；
- 右侧正在形成的 partial bucket 随新增 sample 增量更新；
- layout width 改变、FullRead、representation 从 Polyline/Envelope 切换时允许 rebuild。

因此 steady-state reducer 成本应与“新增 sample + 少量边缘 bucket”相关，而不是与完整 10 s 历史数据量相关。

第一版不再进一步为“cursor 只前进 1 sample”的极端情况设计更复杂的更新机制。实际 Live source 通常按设备/采集 batch 推进，Replay 也按 Bevy update cadence 前进；先以真实 batch 下的增量算法建立 baseline，再根据 benchmark 决定是否需要更细粒度优化。

---

## Headless 测试

测试跟随本方案，不另拆测试方案。

### Unit tests

放在对应源码 module 的 `#[cfg(test)]` 中，保护局部 contract：

- `WaveformConfig` 的 sample rate、visible duration、channel ranges 校验；
- `visible_duration_ms → capacity_frames` 的精确整数换算；
- `Duration ↔ sample boundary` 的边界规则及实时 frame-count helper；
- 初始 viewport 与滚动 viewport 的计算；
- NoOp / IncrementalRead / FullRead 分类；
- Planar ring buffer append、eviction、wrap；
- `ChannelView.first + second` 与逻辑顺序一致；
- 所有 channel 共享同一 logical range 的 invariant；
- MinMax bucket 边界不漏 sample、不重复 sample；
- wrap 输入与逻辑连续输入得到相同 MinMax；
- 增量 MinMax 与一次性 reference reduction 结果一致；
- left/right partial bucket；
- `capacity_frames <= output_len` 的 Polyline 与高密度 Envelope 切换；
- fixed capacity、NoOp 不产生无意义工作等可确定表达的性能 invariant。

每个 test function 上方按项目规则写中文注释，说明“这个测试构造什么场景、保护什么 contract/invariant”，不能只把测试函数名翻译成中文。

### Headless integration tests

放在 `crates/waveform/tests/`，在真实 ECS/system 边界验证：

- cursor 首次推进只读取 `0..N`；
- 连续推进只读取新增尾部；
- target range 不变时 source 不被调用；
- cursor 倒退或不连续跳转走 FullRead；
- source read 失败不提交 partial data，旧 buffered range / working set 保持有效；
- 所有 channel 始终共享同一 sample range；
- 长时间连续推进后 raw ring capacity 不持续增长；
- reducer 的 steady-state update 不重新扫描完整历史 working set。

这里不测试 `ViewportNode`、Mesh entity 或视觉 hierarchy，那些属于下一方案的 integration test。

复杂 integration test module 按仓库规则在 module-level documentation 中记录 state、stimuli、guards、invariants 与必要 coupling；若本阶段拆成多个 integration test 文件，则维护轻量 Coverage Map，避免同一 contract 因拆文件后失去测试归属。

---

## Headless / reducer Benchmark

crate benchmark 仍放在：

```text
crates/waveform/benches/
```

本阶段只测已经存在的真实生产 CPU 路径，不用还未实现的 renderer 模拟“完整 GUI 性能”。测量至少包括：

- IncrementalRead + Planar ring append；
- MinMax incremental reduction；
- FullRead + reducer rebuild；
- NoOp；
- headless CPU update path；
- allocation / raw/reduced buffer growth。

Benchmark 负载不机械展开笛卡尔积，而是围绕目标场景一次改变一个有意义的维度：

```text
channels:          4 / 16 / 64
sample rate:       8k / 32k / 64k
visible duration:  1s / 5s / 10s
output_len:        800 / 1600 / 2400
input:             steady / meaningful burst
```

目标负载必须固定包含：

```text
64 kHz × 64 channels × 10 s
```

Burst 至少包含约 100 ms 输入一次性到达的 catch-up 场景，用于观察系统在短暂积压后是否能恢复，而不是只看理想均匀输入。

这一阶段的 benchmark 不能单独宣称“Waveform 已达到 60 FPS”，因为尚未包含 ViewportNode、Mesh2d 和 GPU/render 成本；它负责证明 headless/reducer 路径没有全量历史扫描、无界 backlog 或无界 allocation，并为第二、第三方案提供 CPU baseline。

全局性能 gate 仍保持：64 kHz × 64 ch × 10 s、稳态 60 FPS、持续输入无 backlog、dropped sample = 0。完整 gate 由第三方案的 Gallery GUI benchmark 最终验收。

---

## 本阶段明确不做

以下能力不进入 headless 第一阶段，也不能为了“顺手未来扩展”提前改变 contract：

- Waveform 内部 Live / Replay mode enum；
- play / pause / seek / playback_rate 控制；
- 鼠标点击、拖动、picking；
- zoom；
- 上一页 / 下一页 UI；
- runtime 修改 visible duration；
- auto-scale；
- 通道折叠、分组、独立高度；
- lane gap / separator；
- 设备同步、丢包重建、插值；
- many-waveform 共享 camera/render target；
- 第三方 downsampling crate。

未来这些能力必须在当前 config/source/range/reducer 边界上增量扩展，而不是把第一版 headless 做成播放器、采集同步器或业务缓存层。
