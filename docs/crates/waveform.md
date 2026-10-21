# Waveform

## 功能概述

显示多通道时间序列，提供可复用的数据源、会话元信息、读取缓存、降采样与 Runtime 查询。应用负责采集和回放时钟，Waveform 消费外部会话时间。

## 会话与显示配置

`WaveformSource::metadata()` 在会话建立时返回固定的 `WaveformMetadata`：通道数量、同序非空标签、统一采样率、V / mV / µV 单位，以及 Live 或带总帧数的 Replay 类型。标签数量必须匹配通道数，采样率须在 1..=1_000_000_000，Replay 总帧数必须大于零。`total_duration()` 根据总帧数与采样率给出记录时长；Live 返回 None。标签由数据源提供，不由控件生成兜底名称。

`WaveformConfig` 仅保存显示配置：`visible_duration_ms` 与统一的 `amplitude_range`（内部单位 V）。显示容量按会话采样率计算且须为整数帧。跨会话保留显示配置和 Style / Palette，不要求不同记录的采样率、通道数相同。

`Waveform` Scene 允许无 Source 构造；可使用 `Waveform::set_session_in_world()` 或对应 Commands 入口在同一根实体建立、重启或替换会话。先校验全部元信息并分配新缓存，再提交替换；失败不会清除原会话和画面。每次成功替换都建立新会话，即使传入相同 Source，也会推进 `session_generation()`、归零 Cursor 并清空旧数据。同步调用提交后发送 `WaveformSessionChanged`，初始化不发送通知。请求同时携带独立 reader 身份和会话代数，用于共享 Source 时隔离不同 View 与旧会话结果。

新 Live 会话允许时间推进，新 Replay 会话默认暂停在 0。`set_running_in_world()` / `set_running()` 通过公开 API 停止或恢复接收外部时间位置，仅在实际改变时发送 `WaveformRunningChanged`。停止保留最后提交的波形和视图区间，不代表清空。会话替换所产生的运行状态由 SessionChanged 一并表达。`clear_session_in_world()` / `clear_session()` 显式清空数据源与画面，已有空会话再次清空为合法 No-op。无会话时设置 Running 或向无效根实体调用 API 返回 Error Severity 的错误。

## 数据与渲染

Source 通过 `read(&WaveformReadRequest, &mut PlanarBuffer)` 响应半开 Frame Range。每个 Channel 仍提供完整的请求长度；`missing_mut()` 声明相对此次读取起点的、按序不重叠且不越界的缺失区间。有效位置必须有限，真实零值仍有效；无效位置的占位数值不参与计算，也不通过 NaN 猜测缺口。Runtime 在提交前将源的物理量统一换算为 V。

`ChannelView` 的 `start` 和两段 missing 切片描述绝对帧索引的缺失区间；`get()` 在缺失或越界位置返回 None。直接读取原始 Sample 切片时必须同时遵守有效性元信息。MinMax 每个桶内保留独立有效段，缺口两侧不会合并成一个 Envelope；Polyline 通过 `WaveformPoint::break_before` 显式标记有效段开始，独立有效样本仍可见。自定义 Reducer 的稀疏代表点可以跨多个有效样本相连，不以索引间隔推断缺口；Reducer 必须保留真实缺口的断点。Live 时间继续向前，窗口中的缺口保持真实位置。

读取返回 `Ready(completed_request)`、`Pending` 或真实错误。应用侧异步 Source 按 reader_id 管理独立读取者，可合并同一读取者的新目标并返回已完成请求的身份；不阻塞主线程，不使用人工延迟冒充磁盘 I/O。只有完整身份、会话代数、目标代数、范围、视图和密度均匹配的 Ready 才能提交。ECS 连续时钟推进期间复用在途请求，完成后提交该请求的一致快照，下一次更新直接读取最新窗口，避免持续推进使所有异步结果永远过期。应用主动定位时调用外部输入 `WaveformCursor::seek(position)`（或更新 seek_generation）使前一目标立即失效；普通时钟推进只改 position。换会话、主动定位或密度变化建立新 request id，过期结果按 Pending 处理而不覆盖画面。独立 headless `WaveformRuntime::update(position, density)` 将每次位置作为明确目标，不做连续时钟合并。

Pending 不记为错误，不冻结外部播放时钟；波形与时间区间作为一致快照保留。真正读取失败保持最后快照并传播 Error Severity 错误：Live 仍运行，之后读取最新目标；Replay 将 Running 提交为 false，并在 ECS 更新路径发送 `WaveformRunningChanged`，应用据此暂停自己的时钟，显式恢复前不重试。旧请求错误由 Source 按身份丢弃，不应作为当前请求错误返回。`WaveformCursor` 是应用可写的外部会话时间输入，不承担播放时钟。Replay 读取范围限制在元信息总帧数内；暂停时忽略输入推进，应用恢复后继续提供正确时间。

Runtime 根据当前时间和可见时长维护 Raw / Reduced Output；稳定 Sample Boundary 和输出密度为 NoOp，零读取且清空本轮统计。Source 失败保留先前完整数据和时间区间，相同输入可重试。每个实例可通过 `last_error()` 查询最近读取错误，成功更新后清除；暂停不伪装为错误恢复。

Renderer 根据实际 UI 宽度确定输出密度，复用一个 Viewport / Camera / Mesh / Image。更换会话使旧几何失效，但不替换这些渲染资源；下一次渲染清空旧画面。Root 销毁时释放其专属资源。Palette 沿用主题及 `WidgetryWaveformColorOverrides`，按源的通道顺序取色。

## 已知限制

- Source 可以快速返回 Pending 并在外部实现异步 I/O；库不提供通用后台任务调度器。
- 当前只提供 MinMax 及自定义 Reducer；时间与幅值控制栏、标签滚动、矩形测量和 Replay 播放控制栏尚未实现。
- 仅现有显示配置与配色跨会话保留；尚未提供的网格、滚动、测量及倍速状态不属于当前 API。
- Headless 正确性测试及 CPU Benchmark 不代表完整 GPU 绘制性能。
