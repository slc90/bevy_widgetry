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
