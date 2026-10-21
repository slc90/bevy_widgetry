# Waveform 磁盘索引测量

## 2026-10-21 · MinMax 局部随机读取

环境：Windows 10 x64 19045，Ryzen 9 3900X，Rust 1.99.0 MSVC；本地文件，Windows 文件缓存保持正常启用。通过项目既有 Harness 记录独立的 200 次操作样本，初始化与离线构建不计入查询耗时；每次查询包含真实定位、读取、结果生成及释放。

运行：

```powershell
$env:CARGO_PROFILE_DEV_OPT_LEVEL = '3'
cargo bench -p bevy_widgetry_waveform_file --bench random_read --profile dev
Remove-Item Env:CARGO_PROFILE_DEV_OPT_LEVEL
```

使用 dev 目录复用第三方 opt-level=3 依赖，当前 Workspace crate 同样以 opt-level=3 编译，保留 dev 的 debug assertions。默认 bench profile 也可运行该基建，但结果需要注明实际构建配置。

负载：4 通道、64kHz、10s 当前窗口、1200 输出密度，确定性不同位置的随机查询；一个通道每 100000 样本含 100 样本缺口。原始部分体积分别为 25.6MB / 256MB，另有多级索引。

| 记录总长 | Median | P95 | Min–Max |
| --- | --- | --- | --- |
| 20s | 118.600 µs | 135.700 µs | 54.600–163.500 µs |
| 200s | 140.350 µs | 159.400 µs | 69.900–167.900 µs |

本地证据：`target/benchmark/waveform_file_random_read-1792580227439-7948/`，包含 `environment.json`、工作区 Diff、200 次原始样本、Criterion 数据和 `status.json=complete`。测试另行验证固定窗口在记录长 10 倍时结果、读取字节与缓冲容量估计完全一致。

此结果只覆盖 CPU 和文件 API 查询路径，包含正常页面缓存影响，不代表冷盘设备延迟、全进程工作集、64 通道 GUI 帧时间或 GPU 显示完成。后台 Source 避免 UI 线程执行数据 I/O，60 FPS 交互仍须在实际 Gallery 场景测量。

## 2026-10-21 · 独立 LTTB 索引与算法对比

相同硬件、优化配置和负载，使用 WFRIDX02 双算法索引。每个记录预先生成一次，各场景重新打开文件再计时，避免离线构建干扰 Criterion 的采样次数估计。LTTB 查询包含相交块代表点和必要的最多两个原始端点读取。

| 算法 / 记录总长 | Median | P95 | Min–Max |
| --- | --- | --- | --- |
| MinMax / 20s | 125.100 µs | 148.200 µs | 56.100–254.800 µs |
| LTTB / 20s | 102.450 µs | 120.800 µs | 81.900–221.100 µs |
| MinMax / 200s | 140.400 µs | 162.000 µs | 75.600–229.500 µs |
| LTTB / 200s | 91.000 µs | 113.600 µs | 71.300–193.900 µs |

本地证据：`target/benchmark/waveform_file_random_read-1792581161075-10428/`，每场景 200 个独立样本，status=complete。LTTB 是折线代表点而 MinMax 是极值包络，输出内容不同，这里的耗时不是等质量替代或尖峰保留能力的结论。两者均为正常 Windows 缓存下的真实局部文件读取，沿用上节 GUI/设备性能测量边界。


## 2026-10-22 · 大文件 Replay GUI

环境：Windows 11 x64 build 26300，i9-14900HX，RTX 5060 Laptop GPU，Rust 1.99.0 MSVC，release / DX12，1920×1080、scale 1、desktop_app。此环境与前述磁盘微基准不同，不能据此比较性能改善。

独立命令生成 16 通道、8 kHz、1800s、seed 42 的真实本地记录；原始数据 1,152,000,000 bytes，含两套索引共 1,529,963,183 bytes，生成用时 73.376s。测量窗口为 30s，实际绘图区 1578×528；隐藏的 Live 不生产数据。Windows 文件缓存正常启用，结果不代表冷物理磁盘延迟。

```powershell
cargo run --release -p bevy_widgetry_waveform_file --example generate_recording --locked -- --output target/replay15-20261022.widx --seconds 1800 --rate 8000 --channels 16 --seed 42
$env:GALLERY_WAVEFORM_REPLAY = (Resolve-Path target/replay15-20261022.widx).Path
$env:GALLERY_WAVEFORM_BENCH_SCENARIO = 'Replay'
$env:GALLERY_WAVEFORM_BENCH_ALGORITHM = 'MinMax'
cargo bench -p widget_gallery --bench waveform --locked
```

90s 实际 GUI 运行中，预热后先用 MinMax 持续拖动 30s，再通过 ComboBox 切到 LTTB 拖动 30s。共 436 次远距离反向拖动，每次松开检查 Slider 目标、所选算法、preview=false 及显示帧等于目标帧。帧时间统计使用 elapsed=14–74.5s 的实际算法分组，包含输入、快照采集及切换开销。查询按请求 ID 去重，避免暂停期间重复记录最近一次读取而扭曲分布。

| 指标 | MinMax | LTTB |
| --- | --- | --- |
| 帧间隔样本 | 5771 | 4855 |
| 帧间隔 P50 / P95 / P99 / Max (ms) | 4.8234 / 7.6889 / 9.8589 / 89.7085 | 5.6883 / 9.7650 / 13.2880 / 103.3530 |
| 主更新 P95 (ms) | 6.7968 | 8.3901 |
| Render schedule P95 (ms) | 3.4713 | 3.6692 |
| Asset extraction P95 (ms) | 0.4835 | 0.6148 |
| 超过 16.7 / 33.3ms 的帧数 | 4 / 1 | 20 / 5 |
| 实际查询样本 | 4560 | 4161 |
| 总查询 P50 / P95 (ms) | 0.5209 / 1.6135 | 0.3339 / 0.6701 |
| seek/read_exact P95 (ms) | 1.2954 | 0.4630 |
| 查询减 I/O P95 (ms) | 0.3808 | 0.2234 |
| 定位至 GPU 读回上界样本 | 228 | 208 |
| 读回上界 P50 / P95 / Max (ms) | 33.2265 / 47.0231 / 146.3140 | 28.5969 / 41.9110 / 108.5031 |

所有读回样本在实际 Replay plot 检出彩色波形像素，采集前核对请求的会话、定位代次、算法和输出精度。读回计时包含 GPU 读回自身开销，排除 BRP RTT 和 PNG 编码，不等同于 OS 呈现或独立磁盘延迟，也不单独证明每个通道的像素正确性。总查询减 I/O 仍包括解码、校验和分配，不是纯降采样时间。

本次累计完成 8721 次真实 worker 查询，结束时无排队，最终显示帧与目标均为 1,276,523、LTTB、preview=false。View 原始样本缓存始终为零，Reduced 缓存峰值分别为 656,640 / 1,048,576 bytes，未把整条 GB 记录载入 View。此值不是全进程内存或 OS 文件缓存占用。

P95/P99 帧间隔落在 16.7ms 内，但存在 89.7/103.4ms 长帧，不能宣称全程稳定 60 FPS。103.4ms 帧的主更新为 15.4ms、最近后台查询仅 0.73ms；另有主更新 31.2ms 和 render schedule 26.3ms 的长帧。现有观测定位到主更新、渲染及调度/读回阶段仍有波动，不能将全部长帧归因于磁盘 I/O。此次交付真实负载与可重复观测入口，保留上述未达预算事实；跨系统性能优化不在本项范围内，不降低负载或放宽预算。

本地证据：`target/benchmark/waveform-gui-1792631257120-21760/`，含环境、可执行文件/记录 SHA256、窗口配置、原始 CSV、`inputs.json`、`analysis.json` 及 `result.json=complete`；Pointer inactive、无 last_error，正常关闭。截图为 `target/replay15-20261022-bench-{minmax,lttb}.png`。普通 Custom Pointer 验证不代表原生 OS Mouse 验收。


### 同机 Live 对照

相同 release / DX12 窗口条件，默认 Live 模式同时运行 100Hz×4ch Basic 和 64kHz×64ch Stress；Stress 配置 10s、根 1600×640、实际绘图区 1468×568、每屏 16 通道。页面按正常布局展示 Basic 和 Stress，窗口底部裁剪了 Stress 的一部分，未将此结果解释为全部通道同时完整可见。90s 运行、12s 预热后共 9432 帧：帧间隔 P50/P95/P99/Max 为 7.9879/10.4215/14.1311/25.6158ms，16 帧超过 16.7ms，无超过 33.3ms；主更新 P95=8.4990ms，生产 P95=0.1474ms，render schedule P95=5.0348ms，asset extraction P95=1.1871ms。显示积压为零，结束 target=displayed=5,760,003。45s 时保留既有 100ms 生产暂缓场景。

157 个数据生成至 GPU 读回上界样本 P50/P95/Max=30.7658/37.5214/44.0537ms。沿用旧 Live 像素检查，不能作为逐通道像素验收，也不代表 OS 呈现时间。仍有少量超预算帧，不宣称全程稳定 60 FPS。本地证据：`target/benchmark/waveform-gui-1792631559461-11588/`，`result.json=complete`、正常关闭，截图 `target/replay15-20261022-live.png`。


### 同机状态文字修复后复测

修复页面重建时状态文字缓存后，用最终 release 再测相同数据集与窗口条件。可执行文件 SHA256 为 `762c355204a5d22e97b9146dc748469ae4721059ae14ea5fe40e35e0c3f2246d`，本地证据 `target/benchmark/waveform-gui-1792632338389-16732/`，同样保留环境、工作区 Diff、窗口配置、原始 CSV 和 `analysis.json`。此轮未额外调用 BRP PNG 截图；两轮数据保留为实际运行波动，不据此宣称状态文字修复带来性能改善。

| 指标 | MinMax | LTTB |
| --- | --- | --- |
| 帧间隔样本 | 5545 | 5136 |
| 帧间隔 P50 / P95 / P99 / Max (ms) | 5.3169 / 7.1122 / 8.4168 / 23.4523 | 5.6068 / 8.2736 / 10.4548 / 19.1416 |
| 主更新 P95 (ms) | 6.2277 | 7.2401 |
| 总查询 P95 (ms)，按请求去重 | 1.1930 | 0.6168 |
| seek/read_exact P95 (ms) | 0.9976 | 0.4195 |
| 定位至 GPU 读回上界样本 / P95 (ms) | 224 / 42.8411 | 221 / 35.7112 |
| 超过 16.7 / 33.3ms 的帧数 | 1 / 0 | 1 / 0 |

445 次拖动均通过每次松开的目标/算法/preview 检查，445 个读回样本均有波形像素。累计 8901 次真实查询，结束 LTTB、preview=false、target=displayed=12,924,700，无排队。`result.json=complete`，Pointer 释放后正常关闭，无 ERROR/panic。测量边界与上文一致；仍存在超预算帧，保留稳定 60 FPS 的未达结论。
