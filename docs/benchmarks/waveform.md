# Waveform 初始性能 baseline

## Headless CPU

2026-10-02，源码为 72d4263 + 本次 Waveform working-tree snapshot；完整 patch/untracked snapshot、环境、executable hash、原始 latency、Criterion report 保存在 target/benchmark/waveform-criterion-1790927056579-18944。执行 cargo bench -p bevy_widgetry_waveform --bench update -- --noplot；bench profile、opt-level=3，Rust 1.98.1，Windows 10 19045，Ryzen 9 3900X，RTX 4070 Ti SUPER。此阶段 GPU 不参与测量。

真实 CPU update 经 MinimalPlugins + WaveformPlugin；初始化/cleanup/实体和 memory 观测排除计时。steady batch=1067 frames，burst batch=6400 frames（100 ms）；每个 frame 含全部 64 channel。source 从预存周期数据复制，不含设备采集/generation。Criterion 20 samples；额外连续采样预热 20 次、记录 200 次原始 latency，P95 描述本次样本，不据此宣称可靠 P99。

固定目标为 64 kHz × 64 ch × 10 s、1600 output pixels：

| 路径 | median ms | P95 ms |
| --- | ---: | ---: |
| ECS headless steady update | 0.388 | 0.488 |
| ECS 100 ms burst/catch-up | 0.767 | 1.052 |
| source read + ring，关闭 reduction | 0.138 | 0.201 |
| 独立 MinMax，1067-frame batch | 0.236 | 0.245 |
| 独立 MinMax，6400-frame batch | 0.349 | 0.365 |
| FullRead + rebuild | 77.407 | 88.803 |
| ECS NoOp | 0.077 | 0.089 |

额外单维度场景包含 4/16/64 channel，8/32/64 kHz，1/5/10 s，800/1600/2400 output 与各自 steady/burst；全部结果见 samples.csv。FullRead 是不连续跳转，包含完整约 164 MB 的读取与 rebuild，不属于 steady frame budget。

runtime raw capacity 固定为每 channel 640000 frames，逻辑 raw working set 为 163840000 bytes；staging 只保留尾部 batch，FullRead staging 成功后转为 ring，不常驻完整双缓冲。integration test 保护多次 wrap 后 raw/reduced/staging allocation 容量有界、channel 对齐、NoOp 零读取/零 reduction 和新增 sample + 边缘 bucket 的访问上限。所有 ECS benchmark entity 数保持 14。此处观察 allocation 容量，未测 allocator 调用次数或 OS resident memory。

这是新增能力的第一份 CPU baseline，无历史对照。steady/burst 的 CPU 成本均低于整体 16.67 ms 预算，但尚不包含 geometry、Mesh upload、ViewportNode、GPU、窗口 frame cadence 或 observable display latency，不能据此宣称 60 FPS gate 已通过。完整 GUI gate 由 Gallery benchmark 验证。
