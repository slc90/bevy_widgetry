# Gallery FileDialog

Window 页面通过 `bevy_widgetry::file_dialog` 与 BSN 构造自绘的独立窗口。六个 launcher 分别展示单文件、多文件、单目录、多目录、图片过滤和 SaveFile。Save 预设 `output.txt`，Text documents filter 提供默认扩展名，已存在目标通过 MessageBox 确认。

Modal checkbox 使用官方 Checked state，默认关闭。不同 NonModal 演示可以同时打开。同一 launcher 已有窗口时不会重复创建。每项结果绑定实际 root/session，只更新自己的有限路径摘要。页面销毁结束其所有窗口，外部销毁不伪造 Cancelled。

稳定定位包括 `WindowNav`、`GalleryFileDialogs`、`GalleryFileDialogModality`。每项使用 `GalleryFileDialog<Operation>Launcher`、`GalleryFileDialog<Operation>Result` 和 `GalleryFileDialog<Operation>`，Operation 为 OpenFile/OpenFiles/SelectFolder/SelectFolders/OpenImage/SaveFile。库已有 FileDialogPath、FileDialogEntries、FileDialogFilter、FileDialogSort、FileDialogFilename、FileDialogConfirm、FileDialogCancel 可供 scoped 查询。

GUI 检查从真实 launcher 打开，观察 Loading 与最终目录；通过真实 pointer 选择、filter/sort Popup、Cancel 与 title bar X 决议，确认结果和 owned cleanup。打开两个不同 NonModal 实例，分别完成或关闭，确认 route 隔离。开启 Modal 后观察父窗口阻断与嵌套覆盖确认。重新打开同一演示，检查最近目录、filter/sort、显示偏好和 pinned folders。secondary keyboard、实际 OS IME 与 presentation 性能需各自的后续验收证据。

## GUI 计时入口

在 PowerShell 7 中生成确定性目录，然后在启动 Gallery 前设置输出目录。输出目录必须没有同名 events.jsonl，避免混合两次运行。

```pwsh
python gallery/benches/file_dialog.py --fixture target/file-dialog-fixture-1000 --entries 1000
$env:GALLERY_FILE_DIALOG_BENCH_OUTPUT = "$PWD/target/file-dialog-gui-run"
$env:GALLERY_FILE_DIALOG_BENCH_FIXTURE = "$PWD/target/file-dialog-fixture-1000"
cargo run -p widget_gallery --release
```

正常 Gallery 不设置这些环境变量。测量模式保留 desktop reactive 和透明 rendering，fixture 仍通过实际 filesystem backend 读取，不提前创建窗口。0/1k/10k/100k fixture 可通过 entries 参数分别生成。按 benchmark 规则记录实际 CPU/GPU、Windows、toolchain、Git 状态、profile/features 与运行命令，首开单独报告。可重复的跨进程采集、受控 I/O 和 PresentMon adapter 命令见 [性能采集说明](../gallery/benches/file_dialog.md)。

运行中先真实点击 WindowNav，再定位 OpenFile launcher 的 logical window 坐标。例如以下坐标应替换为当前 launcher 的实际位置。BRP 只触发输入，HTTP 往返和 screenshot 耗时不进入 App 的 latency。

```pwsh
python gallery/benches/file_dialog.py --events target/file-dialog-gui-run/events.jsonl --collect --port 15702 --launcher-position 360 420 --samples 20 --output target/file-dialog-gui-run/report.json
```

采集器等待内容 frame 与最终 projection，再对对应 native Window 真实点击 Cancel，并等待 owned Window 消失。若窗口尺寸发生改变，使用 cancel-position 指定 Cancel 的 logical 坐标。也可以人工操作后仅用 events/output 参数分析。

协议记录 App 首次观察对应 raw release/input 的 `t_input`、Activate handler 的 `t_activate`、业务 BSN 展开的 `t_scene`、`camera_ready`、`cpu_content_ready`、`first_batch`、`final_projection`。内容证据要求 title、Path label、status 与 Cancel 的 glyphs 已布局并被同一 camera extraction，目录区域有效且 Cancel 可用。目标 camera 必须指向对应 native Window，glyph pipeline、batch 和 bind group 已准备完成，目标 attachment 已写入且实际 surface 已交出。`first_content_frame` 关联 sample/session/root/native Window/camera/app frame，时间点为对应 render graph 返回后的 CPU 标记。

trace 在关键路径只写入有上限的内存缓存，运行后的 `benchmark/file_dialog_export` 输出 events.jsonl。采集器自动 export，人工操作后需先通过 BRP 调用该方法，再运行分析器。`benchmark/file_dialog_trace` 提供游标读取和资源状态，不会打开或重置 dialog。QPC 与 App Instant 的起始 bracket 校准和输出时漂移检查一起保存。

该 render submission 标记不能证明 GPU 完成或实际显示。`t_presented` 默认为 null，分析器输出 display_status=incomplete、150ms_target_passed=null。实际 presentation adapter 必须将显示证据转换到同一 app_monotonic_ns，并通过 `--presentation` 提供 JSONL。每项须含 protocol 的 clock_id、sample、root、session、native_window、camera、app_frame、ns、endpoint=displayed、source 和实际 evidence artifact 路径。关联或时间顺序错误会拒绝分析。不能将 WindowCreated、layout、HTTP、截图返回时间或 submission 填入 ns 冒充 display。

只有所有样本具有有效 display 证据时，分析器才判断150ms目标，校准不确定度计入每项上界。小样本的 P95 仅为样本描述，不报告可靠 P99。现有 PresentMon2.3.1 adapter 固定 schema 并拒绝 Dropped、缺失显示时间及无法唯一关联的 frame。DirectComposition 子窗无法取得可靠对应时，内部结果保留，严格显示验收明确为未验证。
