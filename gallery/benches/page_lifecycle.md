# Gallery 页面 lifecycle 测量

在 Windows 64 位、PowerShell 7 中构建 Gallery，并通过 BRP 启动，将进程环境变量 `GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT` 设置为全新的绝对输出目录。
此变量只启用测量，不改变 desktop_app update mode、DX12、1920×1080 window 或普通页面包装。

```pwsh
cargo build -p widget_gallery
```

使用 `brp_launch` 启动 `widget_gallery`，传入 `env={"GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT":"<全新绝对输出目录>"}`。
Gallery 会创建目录和 `switches.csv`；等待 Splash 结束、Button 页面完成布局后，在 PowerShell 7 中运行采集器。
`--output` 必须与启动时的目录一致，`--executable` 必须指向正在运行的构建产物。默认 BRP 端口为 15702，非默认端口通过 `--port` 指定。

```pwsh
python gallery/benches/page_lifecycle.py --output <同一输出目录> --executable target/debug/widget_gallery.exe --profile 'dev opt-level=1; dependencies opt-level=3' --pages ListView --warmup 3 --cycles 20
```

测量场景为真实 Sidebar pointer 输入驱动 Button→目标页→同页→Button。
采集器按稳定的 `ButtonNav`、`ListViewNav`、`TreeNav`、`TableNav`、`WaveformNav` 等 `Name` 查询按钮，
通过 `ComputedNode`、`UiGlobalTransform` 和 `Sidebar` 的 ScrollArea viewport 取得实际几何位置与可视区域，并换算为逻辑窗口坐标。
目标中心不在可视区域时，先将 Custom Pointer 移入侧栏并发送真实滚轮输入，等待布局更新后重新查询，再点击可见按钮。
导航不依赖页面排列或固定坐标，不直接修改 ECS 页面状态；每次点击后等待对应 `*Page` 节点确认实际切换结果。
同页点击必须保持 root identity；退出后页面 root、该页 Resource 和拥有的 Model/Tree roots 不再存活，Header identity 保持不变。
页面集合沿用现有场景，`--pages` 不指定时为 `ListView`，也可一次指定多个原有目标，例如 `--pages ComboBox ListView Tree Table Waveform`。
ListView 保留原有 4/10000/20/20 数据及当前页面已有的共享示例，Tree 使用原有 10000 File，Table 使用原有 2000×40 虚拟化数据；采集器不缩减或替换数据。

switches.csv 记录切页所在 frame 从 First 到 Last 中测量 system 的主 App CPU 时间，包含 StateTransition、UI/Data 进入退出、Update 和 PostUpdate layout。
计时结束后采集 active_entities（实际存活 Entity）和 waveform_sources（当前是否持有 producer Resource），脚本检查后者只在 Waveform 页为 true。
不包含 CSV 写入、独立 render thread/GPU、Winit idle 或 BRP 往返，不等于输入到实际显示的 latency。
单次离散同步导航的 CPU 预算为250ms；该预算不替代持续 GUI 交互的16.7ms frame budget。
预热3次后重复20次，报告 min/mean/max，不从少量样本宣称 P95/P99。
第一次访问保留在原始结果中；资源与 asset cache 不主动清空。

scenario.json 保存 executable SHA-256、Git state、Rust、CPU/GPU/Windows、profile 和场景。
report.json 保存逐次结果与预算判定，complete 代表测量完成，budget_passed 才代表该预算通过。
脚本退出前 release Custom Pointer 并等待 inactive；随后调用 `brp_shutdown` 正常关闭 Gallery，确认结果为 `clean_shutdown`。
每轮使用新的输出目录；验收应覆盖可见按钮以及需向下、向上滚动才能点击的按钮，确认生命周期断言通过、CSV 有效且预算未改变。
