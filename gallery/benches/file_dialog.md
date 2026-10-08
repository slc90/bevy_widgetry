# FileDialog 性能采集

本文件统一维护 FileDialog 的性能采集命令、测量协议与验收规则。

## 测试环境与 fixture

在仓库根目录使用 PowerShell 7 执行以下命令。
先执行 cargo build --workspace --release --locked，等待构建完成，再运行采集。
保持 Gallery 的 desktop_app reactive update mode、DX12 DirectComposition renderer 和默认 window 配置。
工具限定 Windows 与 PresentMon 2.3.1 console binary。
自动化 runner 使用 Python 3.11 或更高版本的 hashlib.file_digest。Python 脚本使用标准库，不需要额外 Python package。fixture 仍通过实际 filesystem backend 读取，不提前创建窗口。
按 [性能 Benchmark 规则](../../rules/benchmark.md) 记录实际 CPU/GPU、Windows、Rust toolchain、Git 状态、profile/features、测量工具与运行命令，首开单独报告。
runner 的 config.json 保存参数、executable SHA-256、Git HEAD、working-tree 状态、release profile 和 cache 说明，但不代替完整的环境记录。OS/filesystem cache 不会被主动清空。

fixture 支持0、1000、10000、100000等非负规模，包含每20项一个空 folder，其余为 txt/png 文件。生成路径必须尚不存在，切换规模时使用不同路径。

```pwsh
cargo build --workspace --release --locked
python gallery/benches/file_dialog.py --fixture C:/bench/fixture-1000 --entries 1000
```

## 自动化采集

file_dialog_run.py 复用 file_dialog.py 的真实 BRP 输入采集流程，并管理独立 Gallery process、PresentMon、export、分析和资源检查。
--output 必须是新路径，fixture 与 PresentMon binary 必须已存在，BRP port 必须未被占用。

```pwsh
python gallery/benches/file_dialog_run.py --output C:/bench/first --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --runs 20 --samples 1 --frames --screenshot
python gallery/benches/file_dialog_run.py --output C:/bench/repeated --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --samples 101 --frames
python gallery/benches/file_dialog_run.py --output C:/bench/resources --fixture C:/bench/fixture-1000 --presentmon C:/tools/PresentMon-2.3.1-x64.exe --samples 201 --navigate
```

每个 run 使用独立 Gallery process。重复场景的第一个 dialog 是该 process 的首开，随后100个新建 dialog 单独作为稳态样本。
--operation 支持 Gallery 的六个 launcher，--modal 通过真实 CheckBox 输入切换 modality。
runner 默认 --port 15881，并为启动的 Gallery 设置相同 BRP port。

### 资源与持续交互场景

--navigate 进入首个 folder，等 Ready 后通过 Back 返回并验证原 path 和 Ready。
--scrolls 在每个 dialog 中交替执行真实 wheel input。
可用 --samples 1 --scrolls 300 覆盖超过30秒的持续交互，开窗、导航与关闭分别使用独立timeout，不把交互持续时间算入关闭超时。

--io-delay-ms 2000 在 I/O worker 内延迟实际 read_directory，仍使用 native filesystem 与完整 rendering。
--quiet-wait-ms 3000 在 launcher 输入后3秒不查询 BRP，用于区分主动 polling 对慢 I/O 等待期间 frame 数的影响。
--io-gate 控制 I/O worker 挂起，采集器在 Loading shell 后 Cancel。完成后释放 gate，并等 backlog、session、snapshot、row 回收。
--control 关闭内容探测，保留同样的 frame 计时，用于观测开销比较。该模式没有开窗 latency 或显示验收结果。
资源采集每20轮记录 thread count，额外保存 process 初始/最终 thread count 和 working set。
--frames 启用 frame 计时；持续交互和 --control 对照需要 frame 统计时应同时指定。
runner 检查 active_sessions、queued、retained_snapshots、rows 回到0，windows/cameras 回到初始数量，workers 不超过3，且 runtime error 为空。

## 手动启动与采集

file_dialog.py --collect 仍适用于已经启动的 Gallery。自动化 runner 已覆盖顺序打开、等待内容与最终 projection、Cancel 和 owned cleanup，手动流程保留给人工定位或操作后的分析。
在启动 Gallery 前设置输出目录。输出目录必须没有同名 events.jsonl，避免混合两次运行。

```pwsh
python gallery/benches/file_dialog.py --fixture target/file-dialog-fixture-1000 --entries 1000
$env:GALLERY_FILE_DIALOG_BENCH_OUTPUT = "$PWD/target/file-dialog-gui-run"
$env:GALLERY_FILE_DIALOG_BENCH_FIXTURE = "$PWD/target/file-dialog-fixture-1000"
$env:BRP_EXTRAS_PORT = "15702"
./target/release/widget_gallery.exe
```

上面的 executable 使用前述已完成的 release build。正常 Gallery 不设置测量环境变量，测量模式保留 desktop reactive 和透明 rendering。
如需 frame 统计，启动前另设 GALLERY_FILE_DIALOG_BENCH_FRAMES=1。
运行中先真实点击 WindowNav，再定位 OpenFile launcher 的 logical window 坐标。在另一个 PowerShell 7 中执行采集，以下坐标应替换为当前 launcher 的实际位置。

```pwsh
python gallery/benches/file_dialog.py --events target/file-dialog-gui-run/events.jsonl --collect --port 15702 --launcher-position 360 420 --samples 20 --output target/file-dialog-gui-run/report.json
```

采集器等待内容 frame 与最终 projection，再对对应 native Window 真实点击 Cancel，并等待 owned Window 消失。若窗口尺寸发生改变，使用 --cancel-position 指定 Cancel 的 logical 坐标。
选择其他 launcher 时同时指定对应 --operation 和实际坐标。BRP 只触发输入，HTTP 往返和 screenshot 耗时不进入 App 的 latency。
也可以人工操作后先通过 BRP 调用 benchmark/file_dialog_export，再仅用 --events/--output 分析：

```pwsh
python gallery/benches/file_dialog.py --events target/file-dialog-gui-run/events.jsonl --output target/file-dialog-gui-run/report.json
```

## BRP trace 与时间节点

trace 先保存在有明确上限的内存 buffer。测量后调用 benchmark/file_dialog_export 输出 events.jsonl。
当前 buffer 上限为100000项，超出时报告错误。export 保存起始 protocol 和输出时的 clock_check，不重置 trace。
采集器自动 export，人工操作后需先调用该方法，再运行分析器。
采集器完成或失败后调用 brp_extras/pointer_control release，并等待 status.phase=inactive，再结束本轮普通输入。
benchmark/file_dialog_trace 仅在采样的150ms窗口之后读取阶段和资源，不通过 state mutation 打开或重置 dialog。
trace 接收 from 游标，返回 events、next 和 resources。普通采集在 launcher 输入后至少350ms才开始读取；--quiet-wait-ms 可延长此等待。

| 时间节点 | 定义 |
| --- | --- |
| t_input | App 在 Picking input 后首次观察对应 Pointer Left Release，或非 repeat 的 Enter/Space keyboard press。 |
| t_activate | Activate handler 开始处理 launcher 的时间。 |
| t_scene | 业务 BSN 展开且 root/session 已绑定的时间。 |
| camera_ready | camera 活跃、target info 已就绪，且指向对应 native Window。 |
| cpu_content_ready | title、Path label、status 与 Cancel 的 glyphs 已布局，目录区域有效且 Cancel 可用。 |
| first_batch | 业务 state 首次取得目录 snapshot，记录 entry count，不是 rendering glyph batch。 |
| final_projection | 目录不再 Loading 且 projection 不再 pending，记录最终 state 和 entry/visible count。 |
| first_content_frame | 对应 render graph 返回后的 CPU 标记，表示满足内容证据条件的 render submission。 |
| displayed | 外部 presentation adapter 提供的实际显示时间，使用同一 app_monotonic_ns。 |

Mouse 与 Custom 使用相同 Pointer 边界；没有同一 app frame、同一输入窗口的匹配 input 时，程序化 Activate 不产生有效 latency 样本。
App Pointer Left Release / keyboard input、Activate、BSN scene、正确 native target 的 camera、CPU shell、GPU-ready glyph batch、render frame 分开记录。
内容 glyphs 必须被同一 camera extraction，目标 camera 指向对应 native Window，glyph pipeline、batch 和 bind group 已准备完成。
首内容帧要求目标 attachment 写入，以及对应 native surface 在 render_system 前已获取、之后已交出。
first_content_frame 关联 sample/session/root/native_window/camera/app_frame，并记录 render_start_ns、physical_size、scale_factor、window_count 与 hwnd。
分析器要求 t_input ≤ t_activate ≤ t_scene ≤ first_content_frame.ns，camera_ready 与 cpu_content_ready 不晚于该 frame 标记，且 root/session/native_window/camera 一致。

## QPC 校准与 displayed 证据

PresentMon 使用 --v1_metrics --qpc_time。依据 PID、唯一 primary baseline swapchain、两窗范围和 render 区间匹配 dialog present。
adapter 要求内容 frame 恰有两个 native Window 且包含 dialog HWND，在校准不确定度扩展后的 render 区间内只能有一个非 primary present 候选。
显示 timestamp 使用 QPCTime 加 msUntilDisplayed，经 QPC 与 App Instant bracket 校准映射。
开始和测量后的校准检查均保留。校准 bracket、QPC tick 与漂移上界计入不确定度。
起始校准从8次 bracket 中选择最窄的一次，以 before_ns/after_ns 的 midpoint 对应 QPC tick。
映射采用 midpoint + (QPCTime - qpc) × 1000000000 / frequency 的整数结果。初始 uncertainty_ns 为半 bracket 向上取整加一个 QPC tick 向上取整。
输出时校准以预测值到新 bracket 两端的最大距离作为 deviation，加入 uncertainty_ns。deviation 超过100000ns（0.1ms）或校准无效时拒绝映射。
Dropped、NA、缺列、多重候选、无法对应 window/frame 都保留为未验证原因，不能转换成0ms。
该版本对 DirectComposition 的 composition dependency 存在观测限制，实机无法取得可靠对应时，150ms_target_passed 为 null。

自动化 runner 会生成 present.csv，再调用 file_dialog_presentmon.py 生成 presentation.jsonl 与 presentation.mapping.json。
手动采集必须在输入前开始 PresentMon，保留同一 process 的 primary baseline，并使用 --v1_metrics --qpc_time 记录 present.csv。采集和 export 完成后可运行：

```pwsh
python gallery/benches/file_dialog_presentmon.py --events target/file-dialog-gui-run/events.jsonl --csv target/file-dialog-gui-run/present.csv --output target/file-dialog-gui-run/presentation.jsonl
python gallery/benches/file_dialog.py --events target/file-dialog-gui-run/events.jsonl --presentation target/file-dialog-gui-run/presentation.jsonl --output target/file-dialog-gui-run/report.json
```

--presentation 接收 JSONL，每项必须包含 protocol 的 clock_id、sample、root、session、native_window、camera、app_frame、ns、endpoint=displayed、source 和 artifact。
artifact 必须指向实际 evidence 文件，relative path 相对于 presentation.jsonl 所在目录。PresentMon adapter 同时输出 uncertainty_ns，分析器将其计入每项 latency 上界。
ns 必须转换到同一 app_monotonic_ns；sample/root/session/native_window/camera/app_frame 必须匹配 exact content frame。关联、重复记录、artifact 或时间顺序错误会拒绝分析。
displayed 不能早于 cpu_content_ready。阻塞的 presentation call 可能在实际 displayed 之后才返回，因此不要求 displayed 晚于 first_content_frame 的 CPU 标记。
render submission 不能证明 GPU 完成或实际显示，t_presented 默认为 null。不能将 WindowCreated、layout、HTTP、截图返回时间或 submission 填入 ns 冒充 display。

## 统计与150ms目标判定

report.json 分开输出 submission、first_batch、final_projection、displayed、frame statistics 与无效样本数量。
缺少必要时间节点、输入时间无效或已记录的 final_projection state 不是 Ready 时，样本不能作为有效 submission。
没有 display 证据时，分析器输出 display_status=incomplete、150ms_target_passed=null。只有所有样本均有有效 submission 与 display 证据时才判断150ms目标。
每项 input_to_presented_upper_ms 为实际显示 latency 加校准不确定度；所有样本的该上界均不超过150ms时，150ms_target_passed 才为 true。
小样本的 P95 仅为样本描述，不报告可靠 P99。首开与后续新建 dialog 分开报告。
steady_frame_statistics 排除 App 启动、开窗构建帧，只统计 dialog 活跃期间的后续 frame。
main_frame 是 Main World First→Last，render_schedule 是 Render schedule，window_acquire 包含 native surface 获取等待。
render_frame 是 render_system 区间，probe_cpu_frame 是 Main World 内容探测。
这些 CPU/wait 区间不等于实际 displayed frame time，不应相加冒充完整 GPU pipeline。
shell.png 只证明指定 camera 的内容，不以截图回调或文件时间计 latency。

## 无效样本与证据保留

present.csv、presentation.mapping.json、events.jsonl、report.json、资源记录、config.json 与日志共同保留。adapter 的关联失败逐项写入 mapping 的 invalid；schema、baseline 或校准错误可能直接使采集失败。
DirectComposition 无法可靠关联显示帧时，保留内部 submission/projection 结果及失败原因，严格显示验收明确为未验证。
如采集失败，保留该 run 的 trace 和日志，先修复原因再新建 output 路径重跑，不覆盖或自动删去慢样本。
