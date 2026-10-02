# Waveform 控件第一版：拆分方案总览

## 整体目标

把 Waveform 第一版按唯一执行顺序拆成三份方案：**headless → style/render → gallery**。三份方案合起来保持总方案中的完整设计边界、性能目标、测试要求和第一版非目标；测试不再单列为独立阶段，而是分布在它所验证的方案内。

全局目标场景保持不变：64 kHz sample rate、64 channels、10 s visible duration，稳态 60 FPS（16.67 ms frame budget），持续输入无 backlog、无 dropped sample。Waveform 本身不区分 Live / Replay；两者由外部 source 和 cursor driver 区分。

## 执行顺序

1. [01-headless.md](./01-headless.md) — **建立 Waveform headless 数据契约与高采样率运行时**  
   完成独立 crate、配置与 cursor 语义、source adapter、Planar ring working set、读取提交/失败语义、reducer 抽象与增量 MinMax，并用 unit/integration test 与 crate benchmark 固化这些 contract。

2. [02-style-render.md](./02-style-render.md) — **把 headless Waveform 接入 BSN，并完成 Style / ViewportNode / Mesh2d 渲染**  
   在不改变 headless contract 的前提下建立 BSN hierarchy、lane/value 映射、channel palette、Polyline/Envelope triangle geometry、Mesh/RenderTarget 生命周期和对应的 integration test / renderer benchmark。

3. [03-gallery-validation.md](./03-gallery-validation.md) — **建立 Gallery Live/Replay/Stress 场景并完成真实 GUI 与性能验收**  
   加入 Basic Live、预生成 `.wfrm` 的 Basic Replay、64 kHz × 64 ch × 10 s Stress Live，按项目规则做 BRP 动态验证和 Gallery GUI benchmark，建立第一份可追溯性能 baseline，并完成 workspace/facade/asset/docs 的最终一致性检查。

## 前后关系

Headless 是后两份方案的语义基础：Style/Render 只能消费已经稳定的 raw/reduced data contract，不重新定义 Live/Replay、source 或 cursor；Gallery 则只组合已经完成的控件与外部 driver/source，不把业务数据生成、播放控制或性能补丁塞回 Waveform 本体。

三份方案均遵守第一版范围：不加入 play/pause/seek/playback_rate、鼠标 picking/拖动、zoom、上一页/下一页、runtime 修改 visible duration、auto-scale、通道折叠/分组/独立高度、lane gap/separator、设备同步/丢包重建/插值，也不为未来 many-waveform 场景提前设计共享 camera/render target，不引入第三方 downsampling crate。
