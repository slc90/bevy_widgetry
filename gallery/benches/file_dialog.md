# FileDialog 性能采集

先执行 cargo build --workspace --release --locked，等待构建完成，再运行采集。
保持 Gallery 的 desktop_app reactive update mode、DX12 DirectComposition renderer 和默认 window 配置。
工具限定 Windows 与 PresentMon 2.3.1 console binary。

```powershell
python gallery/benches/file_dialog.py --fixture C:/bench/fixture-1000 --entries 1000
python gallery/benches/file_dialog_run.py --output C:/bench/first --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --runs 20 --samples 1 --frames --screenshot
python gallery/benches/file_dialog_run.py --output C:/bench/repeated --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --samples 101 --frames
python gallery/benches/file_dialog_run.py --output C:/bench/resources --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --samples 201 --navigate
```

每个 run 使用独立 Gallery process。重复场景的第一个 dialog 是该 process 的首开，随后100个新建 dialog 单独作为稳态样本。
fixture 支持0、1000、10000、100000等非负规模，包含每20项一个空 folder，其余为 txt/png 文件。
--operation 支持 Gallery 的六个 launcher，--modal 通过真实 CheckBox 输入切换 modality。
--navigate 进入首个 folder，等 Ready 后通过 Back 返回并验证原 path 和 Ready。
--scrolls 在每个 dialog 中交替执行真实 wheel input。
可用 --samples 1 --scrolls 300 覆盖超过30秒的持续交互，开窗、导航与关闭分别使用独立timeout，不把交互持续时间算入关闭超时。

--io-delay-ms 2000 在 I/O worker 内延迟实际 read_directory，仍使用 native filesystem 与完整 rendering。
--quiet-wait-ms 3000 在 launcher 输入后3秒不查询 BRP，用于区分主动 polling 对慢 I/O 等待期间 frame 数的影响。
--io-gate 控制 I/O worker 挂起，采集器在 Loading shell 后 Cancel。完成后释放 gate，并等 backlog、session、snapshot、row 回收。
--control 关闭内容探测，保留同样的 frame 计时，用于观测开销比较。该模式没有开窗 latency 或显示验收结果。
资源采集每20轮记录 thread count，额外保存 process 初始/最终 thread count 和 working set。

trace 先保存在有明确上限的内存 buffer。测量后调用 benchmark/file_dialog_export 输出 events.jsonl。
采集器完成或失败后调用 brp_extras/pointer_control release，并等待 status.phase=inactive，再结束本轮普通输入。
benchmark/file_dialog_trace 仅在采样的150ms窗口之后读取阶段和资源，不通过 state mutation 打开或重置 dialog。
App Pointer Left Release / keyboard input、Activate、BSN scene、正确 native target 的 camera、CPU shell、GPU-ready glyph batch、render frame 分开记录。
首内容帧要求目标 attachment 写入，以及对应 native surface 在 render_system 前已获取、之后已交出。
窗口和 camera 的标识保持与 sample/session/app_frame 对应。

PresentMon 使用 --v1_metrics --qpc_time。依据 PID、唯一 primary baseline swapchain、两窗范围和 render 区间匹配 dialog present。
显示 timestamp 使用 QPCTime 加 msUntilDisplayed，经 QPC 与 App Instant bracket 校准映射。
开始和测量后的校准检查均保留。校准 bracket、QPC tick 与漂移上界计入不确定度。
Dropped、NA、缺列、多重候选、无法对应 window/frame 都保留为未验证原因，不能转换成0ms。
该版本对 DirectComposition 的 composition dependency 存在观测限制，实机无法取得可靠对应时，150ms_target_passed 为 null。

report.json 分开输出 submission、first_batch、final_projection、displayed、frame statistics 与无效样本数量。
steady_frame_statistics 排除 App 启动、开窗构建帧，只统计 dialog 活跃期间的后续 frame。
main_frame 是 Main World First→Last，render_schedule 是 Render schedule，window_acquire 包含 native surface 获取等待。
render_frame 是 render_system 区间，probe_cpu_frame 是 Main World 内容探测。
这些 CPU/wait 区间不等于实际 displayed frame time，不应相加冒充完整 GPU pipeline。
shell.png 只证明指定 camera 的内容，不以截图回调或文件时间计 latency。

如采集失败，保留该 run 的 trace 和日志，先修复原因再新建 output 路径重跑，不覆盖或自动删去慢样本。
