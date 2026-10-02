# Waveform 性能 baseline

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

## BSN / UI / merged geometry CPU

2026-10-02，源码为 d31b778 + renderer working-tree snapshot，环境同上。artifact 为 target/benchmark/waveform-render-criterion-1790929054407-20680；命令 cargo bench -p bevy_widgetry_waveform --bench render -- --noplot。真实 BSN、UI layout、Waveform source/reducer 与单个 indexed Mesh 更新进入计时；没有窗口、GPU upload 或呈现。每场景记录 200 次原始 latency，预热 20 次；entity 数始终为 145。source 的周期数据 generation 进入 steady/burst 计时。

目标 64 kHz × 64 ch × 10 s、1600 physical pixels：

| 路径 | median ms | P95 ms |
| --- | ---: | ---: |
| steady update + geometry | 6.984 | 7.263 |
| 100 ms burst + geometry | 7.737 | 8.185 |
| Envelope geometry（cursor 不变、style 变化） | 5.981 | 6.209 |
| 同 primitive 数 Polyline geometry | 5.481 | 5.689 |
| NoOp（包含 App/UI） | 0.473 | 0.552 |

800/2400 pixels 的 steady median 分别为 3.992/9.974 ms，burst 为 4.664/10.709 ms。各 4/16/64 channel 和 800/1600/2400 pixels 场景详见 samples.csv；最早的 4-channel 场景与一次测试编译重叠，不作为定量结论依据。64-channel 场景没有其他本任务重负载进程并行。

integration test 保护 renderer entity/asset identity、Mesh Vec capacity、持续 wrap、读取失败后的 resize、隐藏期间 style 修改与多实例 RenderLayers 隔离。这里没有测 allocator 调用次数、GPU 或 observable display latency；16.67 ms 完整 GUI gate 仍留给 Gallery 阶段。

## Gallery 实测与 Envelope 优化

2026-10-02，源码为 c8fda2f + 第三项 working-tree snapshot。环境同前两项，GPU driver 为 32.0.15.9579；release/opt-level=3，正常 desktop_app、DX12、1920×1080 window、scale factor 1，Stress viewport 实测 1600×640 physical pixels。三个公开 BSN 实例同屏，Basic Live 为 100 Hz × 4 ch × 5 s、790×200 px 的 Polyline，Basic Replay 为固定 8 kHz × 4 ch × 30 s 文件、5 s viewport 的 Envelope，Stress 为 64 kHz × 64 ch × 10 s。没有降低 sample rate、channel count、输出 span 或颜色格式。

GUI 命令为 `cargo bench -p widget_gallery --bench waveform`，摘要为 `python gallery/tools/summarize_waveform_bench.py <artifact>`。harness 先构建优化 executable，独立进程启动后通过真实 sidebar move/click 进入页面；预热 12 s、总计约 90 s，在约 45 s 保留一次 100 ms producer 输入并批量追赶。源速率由 elapsed time 决定，read 仅复制已生产的 Planar ring 区间。测量期间没有并行构建或其他本任务 benchmark。

初始标准 geometry 路径 artifact 为 target/benchmark/waveform-gui-1790932442022-1188：稳态 53.114 FPS，main CPU median/P95/P99 为 13.558/17.401/18.888 ms，backlog/drop 为 0，未通过 60 FPS gate。Envelope geometry 是主要 CPU 成本，因此缓存每 lane 投影与公共 x scale，直接生成垂直 rectangle，去除无 texture 时未消费的 UV；保留相同 MinMax span、4 vertices/6 indices 与 RGBA32f。Polyline 几何语义保持原样。

优化后重复同一 64-channel CPU 场景：`cargo bench -p bevy_widgetry_waveform --bench render -- 'waveform_render/ch64/w1600' --noplot`，artifact 为 target/benchmark/waveform-render-criterion-1790933491013-18924。每场景仍为 20 次预热、200 次独立原始采样，entity 数 145。

| CPU 路径 | 原 median ms | 优化后 median ms | 优化后 P95 ms |
| --- | ---: | ---: | ---: |
| steady update + geometry | 6.984 | 2.845 | 3.131 |
| 100 ms burst + geometry | 7.737 | 3.572 | 4.000 |
| Envelope geometry | 5.981 | 1.805 | 2.007 |
| 同 primitive 数 Polyline geometry | 5.481 | 5.474 | 5.640 |
| NoOp | 0.473 | 0.472 | 0.555 |

优化后两次完整 GUI artifact 为 waveform-gui-1790932890287-11488、waveform-gui-1790933215111-5052，稳态分别为 59.930/59.893 FPS，持续输入均无 backlog/drop。后一轮有 4129 个排除 warmup、readback 观测窗口与 burst 的稳态样本：

| 实测边界 | median ms | P95 ms | P99 ms | max ms |
| --- | ---: | ---: | ---: | ---: |
| First → Last main ECS CPU | 8.480 | 12.143 | 12.464 | 14.285 |
| Mesh 等 asset extraction CPU | 2.891 | 4.036 | 4.269 | 16.078 |
| render schedule wall time | 13.213 | 13.934 | 14.253 | 16.224 |
| 完整 frame cadence | 16.602 | 20.523 | 21.217 | 27.896 |

main 含 producer、source/reduction、geometry 与 UI，不含之后的 extract/GPU；render schedule 含正常 native present 等待。main/render world 使用最近完成的异步阶段观测，不强制同步，也不把不同阶段直接相加冒充端到端 GPU latency。常态约 60 Hz、核心阶段 P99 在 16.67 ms 内，完整 frame cadence 仍有 Windows/VSync/调度波动，不能宣称每个实际 frame 间隔都小于 16.67 ms。

后一轮实际吞吐为 63997.78 frames/s（每 frame 含 64 channel），全程 backlog=0、dropped sample=0；100 ms hold 的追赶读取最多 7260 frames，7 个 burst update 的 main CPU 最大 12.078 ms。78 次 screenshot GPU readback 在 Stress crop 的全部 64 lane 都有非背景波形像素；generation → readback observer 上界 median 为 52.604 ms、max 为 80.322 ms。此上界排除 BRP RTT 与 PNG 编码，包含 GPU readback，不能等同 OS present latency；78 次捕获不报告可靠 P95/P99。截图 instrumentation 窗口与随后 4 帧冷却从稳态 CPU 统计排除，完整 FPS 仍含这些窗口。

该轮 raw/reduced 容量为 163840000/3145728 bytes，合并 Mesh 属性与 index 逻辑数据为 14207256–14634296 bytes，Assets<Image>/Assets<Mesh> 为 22/4；GPU allocator 为 3 slabs、22504104 bytes、4 个 allocation，预热后不增长。frame CSV 中旧字段 entities=16384 实际是已分配 entity index slots，不能冒充当前存活 entity 数；最终 harness 改为 entity_indices 并单独每 60 帧记录 count_spawned。资源测量是容量/当前 allocator occupancy，不是 allocator 调用次数，也不是 OS resident memory。

原始 frames.csv、render.csv、observable.csv、summary.json、完整源码 snapshot、环境与 executable hash 均在对应 target/benchmark artifact。GPU diagnostics 保存各 pass 的 query time，未把可能重叠的 pass 简单相加成整帧 GPU 总成本。测量完成后正常 shutdown；第三轮仅关闭阶段出现异步 GPU statistics map 取消 WARN，测量期间无 ERROR。

最终 source 改为预留容量的 VecDeque 后再次完整测量，artifact 为 target/benchmark/waveform-gui-1790934847718-6532：4122 个稳态样本，稳态/全部 FPS 为 59.840/59.891，吞吐 63997.37 frames/s，backlog/drop 全程为 0。main CPU median/P95/P99/max 为 8.881/12.289/13.632/18.741 ms，producer P99 为 0.425 ms，asset extraction P99 为 3.455 ms，render schedule P99/max 为 14.383/21.056 ms；完整 cadence P99 为 21.636 ms。100 ms hold 追赶最多读取 7293 frames，7 次 burst 的 main CPU max 为 8.482 ms。少数 main/render 时间超出单帧预算须保留，不能将稳定约 60 Hz 解读为每一帧硬实时保证。

最终存活 entity 数在稳态始终为 11544，entity index slots 为 16384；raw/reduced/mesh/Image/Mesh/GPU allocator 范围与上一轮一致，未持续增长。78 次全部 64 lane 可见，readback 上界 median/max 为 51.561/131.869 ms（样本数不足可靠 tail quantile）。最终轮也只有 shutdown 阶段 statistics map 取消 WARN，正常关闭且测量期间无 ERROR。当前 baseline 支持目标负载下约 60 FPS、持续输入和突发无积压、无丢失，核心阶段 P99 在预算内；native frame cadence 偶发抖动、OS present latency、allocator 调用次数和其他硬件上的表现未由此次测量证明。

## Startup 与资产复现

同一优化配置使用 `cargo bench -p widget_gallery --bench startup -- --samples 3`。计时从 harness 请求创建进程到 Button 初始页面实际 Text/Icon 像素与导航交互就绪的 ready.txt，5 ms 轮询开销包含在内；构建与正常关闭排除。每个 first 样本使用新的 App state，subsequent 保留该样本 state；OS file cache/GPU driver cache 未控制，不能称为完整 cold start，也不能从各 3 个样本推断可靠 tail latency。

| 版本 | first median / range ms | subsequent median / range ms |
| --- | ---: | ---: |
| c8fda2f，接入 Gallery 前 | 725.679 / 698.420–1207.878 | 775.219 / 707.953–776.312 |
| 初版固定 raw Vec 零初始化 | 798.166 / 786.355–1310.295 | 794.501 / 760.958–809.646 |
| 最终 source VecDeque 预留容量、按输入写入 | 716.865 / 687.582–1237.189 | 705.637 / 703.067–731.563 |

初版在启动时写满约 180 MB producer ring；最终保留相同 704000-frame/channel 上限，预留 capacity 而不写尚未产生的数据，并按 VecDeque 的两段连续 slice 复制。这不改变实际输入或 Widget ring，初始页面不因隐藏 Stress 数据提前清零而付出成本。对照 artifact 为 target/benchmark/startup-waveform-before-1790934030784-17720（独立 c8fda2f worktree 测量后保留）；中间与最终 artifact 为 startup-rust-1790933431971-14044、startup-rust-1790934754413-1124。对照就绪区域为 46，新增导航后为 47，主页面/窗口/backend 相同。所有样本正常 ready/shutdown，当前结果未显示超出此环境波动的 startup regression。

共享 Cargo target 的 worktree 对照可能复用含编译时 CARGO_MANIFEST_DIR 的 test_utils Artifact 路径；本次发现后中止误指向旧 workspace 的未完成运行，清理该 package 的 release artifact 并重新测量。比较新旧 worktree 时应使用独立 target 或重新构建此 package，并检查 environment.json/source snapshot 的 workspace 归属。

固定 Replay asset 为 gallery/src/assets/waveform/basic_replay.wfrm，WFRM version 1、little-endian Planar f32，大小 3840024 bytes，SHA256 为 46965A4887C01E2887B3B14600ACE0469DB5BDE55568AE72E87B2AD9FD99C797。手动运行 `rustc --edition=2024 gallery/tools/generate_waveform_data.rs -O -o target/generate_waveform_data.exe`，再运行 `target/generate_waveform_data.exe <output.wfrm>` 可复现；Gallery runtime 只通过 embedded asset/loader 读取文件。

## BRP 动态验收

最终 release Gallery 经 BRP launch、真实 move/click 进入 Waveform，约 0.5 s 内观察到 Basic/Replay/Stress producer=displayed 为 41/3331/26649 frames，buffered/viewport start 均为 0，实际 width 为 790/790/1600；截图 target/waveform-brp-final-initial-fill.png 确认左侧填充、右侧留白。持续运行后截图 waveform-brp-final-scroll.png 确认三实例滚动、固定 lane 顺序/范围、循环颜色、Polyline/Envelope 及全部 64 lanes。

Replay loop 边界在 frame 238603、loop_count=2 截取 waveform-brp-loop-edge-before.png；约 250 ms 后 frame 1543、loop_count=3，buffered/viewport start 重置为 0，截图 waveform-brp-loop-edge-after.png 确认重新从左填充，无旧尾部残留、lane 串线或整屏空白。此时 Stress producer/displayed=5772344、drop=0，独立 source ring 已多次 wrap。截图是所观察边界的视觉证据，不冒充逐帧 exhaustive flicker 检测。两次最终 BRP 运行均无 ERROR/WARN 并正常 shutdown。
