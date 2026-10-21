# Waveform

## 功能概述

显示多通道时间序列，提供可复用的数据源、会话元信息、读取缓存、降采样与 Runtime 查询。应用负责采集和回放时钟，Waveform 消费外部会话时间。

## 会话与显示配置

`WaveformSource::metadata()` 在会话建立时返回固定的 `WaveformMetadata`：通道数量、同序非空标签、统一采样率、V / mV / µV 单位，以及 Live 或带总帧数的 Replay 类型。标签数量必须匹配通道数，采样率须在 1..=1_000_000_000，Replay 总帧数必须大于零。`total_duration()` 根据总帧数与采样率给出记录时长；Live 返回 None。标签由数据源提供，不由控件生成兜底名称。

`WaveformConfig` 仅保存显示配置：`visible_duration_ms` 与统一的 `amplitude_range: WaveformAmplitude`。后者保存有限正数 `value` 和 V / mV / µV `unit`，表示对称范围的单侧上限 A；100 µV 代表中心上下 ±100 µV，峰峰值为 200 µV。`volts()` 返回用于投影的 V 值，`label()` 按数值和单位生成文字。显示容量按会话采样率计算且须为整数帧。跨会话保留显示配置和 Style / Palette，不要求不同记录的采样率、通道数相同。

`Waveform` Scene 允许无 Source 构造；可使用 `Waveform::set_session_in_world()` 或对应 Commands 入口在同一根实体建立、重启或替换会话。先校验全部元信息并分配新缓存，再提交替换；失败不会清除原会话和画面。每次成功替换都建立新会话，即使传入相同 Source，也会推进 `session_generation()`、归零 Cursor 并清空旧数据。同步调用提交后发送 `WaveformSessionChanged`，初始化不发送通知。请求同时携带独立 reader 身份和会话代数，用于共享 Source 时隔离不同 View 与旧会话结果。

新 Live 会话允许时间推进，新 Replay 会话默认暂停在 0。`set_running_in_world()` / `set_running()` 通过公开 API 停止或恢复接收外部时间位置，仅在实际改变时发送 `WaveformRunningChanged`。停止保留最后提交的波形和视图区间，不代表清空。会话替换所产生的运行状态由 SessionChanged 一并表达。`clear_session_in_world()` / `clear_session()` 显式清空数据源与画面，已有空会话再次清空为合法 No-op。无会话时设置 Running 或向无效根实体调用 API 返回 Error Severity 的错误。

## 数据与渲染

`WaveformProps::time_ranges_ms` 接收应用提供的正数、互不重复且包含初始跨度的毫秒候选。非空时 Live 与 Replay 顶部组合真正的 ComboBox；空集合隐藏选择器，适用于只由程序设置范围的视图。`WaveformRenderPlugin` 自动注册私有候选类型所需的 ComboBox / ListView 能力；`WaveformPlugin` 仍可独立用于无 UI 场景。

`Waveform::set_visible_duration_in_world()` 与 Commands `set_visible_duration()` 共用时长校验；配置了候选时只能选其中之一。合法同值不通知，实际变化提交 `config().visible_duration_ms` 后发送 `WaveformDurationChanged`，之后 ComboBox 投影选中项。修改不重建根实体、不归零 Cursor、不替换会话，也不改变幅值和配色。

`WaveformProps::amplitude_ranges` 接收应用提供的幅值预设，须包含初始值；物理量等值的重复项（如 1000 µV / 1 mV）会被拒绝。空集合隐藏幅值 ComboBox，允许只使用程序化 API。`set_amplitude_in_world()` / Commands `set_amplitude()` 校验候选和有限正值，先提交 `config().amplitude_range` 再发送 `WaveformAmplitudeChanged`；同值不通知。选择只重建投影几何，不重新读取源、不改会话时间、运行状态或降采样输出，暂停时也立即生效。

全部通道共享幅值上限 A，零点位于通道中心；行高为 h 时竖直偏移量为 `v / (2A) * h`。有效样本已统一为 V，相同物理量在不同源单位下位置相同。不逐通道钳制，超幅波形可进入相邻行，沿用原通道颜色；折线、独立点和 MinMax 包络只裁剪整体绘图区。折线按实际交点裁剪，完全位于视图区外的片段不生成沿边缘的虚假平线。

`WaveformProps::channels_per_screen` 用 `WaveformChannelCount::Count(P)` / `All` 初始化布局密度，默认 All；`channel_counts` 配置顶部 ComboBox 候选，非空时须互不重复且包含初始值。候选显示为数值 `ch` 或表示全部的 `∞ ch`。`set_channels_per_screen_in_world()` / Commands `set_channels_per_screen()` 更新，`channels_per_screen()` 查询；先提交状态，再发送 `WaveformChannelCountChanged`，同值不通知，Count(0) 或未配置候选返回错误。

绘图区高度为 H、通道数为 N 时，行高为 `max(H / min(P,N), min_channel_height)`；All 以 N 为目标。默认最小行高为 24 逻辑像素，标签列宽 `label_width` 默认为 120 逻辑像素。所有标签严格来自当前会话元信息，与波形共用连续垂直滚动，首尾允许局部通道。标签列独立裁剪，超幅彩色波形不会覆盖标签；时间标尺只占右侧绘图区宽度。无会话时没有通道行，行高为零。

`channel_scroll_viewport()` 返回原生 ScrollArea 视口，可按 Bevy 官方契约读写其 `ScrollPosition`；Wheel、键盘和滚动条操作共用这份状态。密度或尺寸变化保持顶部通道索引与行内比例，再按新合法范围裁剪；更换会话保留密度与滚动意图并按新通道数裁剪。布局使用实际测量行高，波形消费本帧已经应用的滚动偏移，与标签同步。垂直滚动不读取数据或重建降采样，纹理始终只按可见绘图区大小分配。

`WaveformChannelLayout` 是库托管的不可变 Component，可直接只读 Query，或通过 `channel_layout()` 取得快照；`row_height`、`scroll_offset`、`content_height`、`viewport_height` 均为逻辑像素。它描述已应用布局，首次 Layout 前为零。配置提交与布局投影并不等价；尺寸变化后锚点的原生 ScrollPosition 会在布局测量后修正，并请求下一帧应用，过程中标签与波形始终共享本帧的实际偏移。不得替换该 Component 绕过布局管理。

采集时刻为 t、跨度为 T：t<T 时提交视图区间为 `[0,T]`，样本从左生长，未采集的右侧空白；t≥T 时为 `[t−T,t]`，坐标始终使用会话累计时间。当前范围通过帧索引 `viewport_range()` 查询，秒数由采样率换算。Live 不提供横向拖动回看或滚轮时间缩放。

热切换重新读取所需区间并按实际绘图区宽度降采样。目标配置与通知可以先变化，Pending / 失败期间提交的旧视图区间与波形仍保持一致；Ready 后一起替换。暂停时更改跨度使用最后提交的采样时刻重算，保持暂停；新会话仍从零开始，并保留所选跨度。外部源负责配置范围所需历史，不支持时返回明确读取错误。库只分配当前窗口工作集，切换事务中短暂同时保留新旧工作集，不复制固定最大候选历史。

Source 通过 `read(&WaveformReadRequest, &mut PlanarBuffer)` 响应半开 Frame Range。每个 Channel 仍提供完整的请求长度；`missing_mut()` 声明相对此次读取起点的、按序不重叠且不越界的缺失区间。有效位置必须有限，真实零值仍有效；无效位置的占位数值不参与计算，也不通过 NaN 猜测缺口。Runtime 在提交前将源的物理量统一换算为 V。

会话 Source 可以固定声明 `provides_reduced() == true`，改用 `read_reduced` 返回完整请求范围的 `WaveformReducedReadStatus`。此路径不分配 Raw Ring、不调用本地 Reducer，`channel()` 返回 None；`buffered_range()` 表示已提交显示覆盖的读取范围。结果保留绝对样本坐标、有效段与算法类型，单位仍按元信息换算成 V；非法/过期结果不得覆盖当前快照。像素密度、算法或窗口变化需要重新请求，Pending 保留旧波形和旧时间轴。`cancel_reads(reader_id)` 在会话替换、清空和 Runtime 销毁时释放该 View 的后台请求。文件格式及索引由外部 [waveform_file](waveform_file.md) Source 实现，Widget 本身不理解文件格式。

`ChannelView` 的 `start` 和两段 missing 切片描述绝对帧索引的缺失区间；`get()` 在缺失或越界位置返回 None。直接读取原始 Sample 切片时必须同时遵守有效性元信息。MinMax 每个桶内保留独立有效段，缺口两侧不会合并成一个 Envelope；Polyline 通过 `WaveformPoint::break_before` 显式标记有效段开始，独立有效样本仍可见。稀疏代表点可以跨多个有效样本相连，不以索引间隔推断缺口。Live 时间继续向前，窗口中的缺口保持真实位置。

`WaveformAlgorithm::{MinMax,Lttb}` 在 Live 与 Replay 中均可使用，默认 MinMax。`Waveform::set_algorithm_in_world()` / Commands `set_algorithm()` 提交目标后发送 `WaveformAlgorithmChanged`，合法同值不通知；`WaveformRuntime::algorithm()` 查询目标。控件工具栏不创建算法选择器。切换不改变会话、Cursor、Running、幅值、通道布局、滚动或网格；暂停时也会刷新显示。读取请求携带算法身份，切换使在途请求失效。新结果准备期间保留旧完整快照，算法查询先于实际显示提交变化。

MinMax 在密集时输出极值包络、稀疏时输出原始折线，适合需要保留尖峰的信号。LTTB 始终输出真实样本的代表点折线，不保证保留极值。为稳定 Live 桶边界，以当前窗口跨度 / 输出像素密度计算桶宽并锚定绝对帧索引；每个有效段内使用前一代表点、当前桶候选和下一桶有效点均值的最大三角形面积选点，相等时选择较早样本。每段保留首点与最新尾点，不等待尾桶完整，不跨缺口求均值或连接。分段与端点可能使点数略多于像素密度。

LTTB 缓存每桶有效段的均值、首尾点和上下凸包；增量只扫描新增样本和左边界桶，利用凸包二分查询代表点，不从 MinMax 结果反推。大于等于 262144 个通道样本的重建交给后台计算任务；持续更新仍使用增量缓存。后台结果与原始缓存、降采样和 viewport 一起提交，过期会话、算法、定位、跨度或密度的任务不能覆盖新目标。单实例最多一个在途重建，旧任务结束后处理最新目标。Source 的同步读取仍由调用线程执行，应用需用 Pending 实现非阻塞 I/O。

后台重建在途时每帧请求 `RequestRedraw`，包括 Layout 在 PostUpdate 启动的任务，保证 Reactive 窗口在暂停且无输入时仍能提交完成的快照；完成后停止这些重绘请求。

读取返回 `Ready(completed_request)`、`Pending` 或真实错误。应用侧异步 Source 按 reader_id 管理独立读取者，可合并同一读取者的新目标并返回已完成请求的身份；不阻塞主线程，不使用人工延迟冒充磁盘 I/O。只有完整身份、会话代数、目标代数、范围、视图和密度均匹配的 Ready 才能提交。ECS 连续时钟推进期间复用在途请求，完成后提交该请求的一致快照，下一次更新直接读取最新窗口，避免持续推进使所有异步结果永远过期。应用主动定位时调用外部输入 `WaveformCursor::seek(position)`（或更新 seek_generation）使前一目标立即失效；普通时钟推进只改 position。换会话、主动定位或密度变化建立新 request id，过期结果按 Pending 处理而不覆盖画面。独立 headless `WaveformRuntime::update(position, density)` 将每次位置作为明确目标，不做连续时钟合并。

Pending 不记为错误，不冻结外部播放时钟；波形与时间区间作为一致快照保留。真正读取失败保持最后快照并传播 Error Severity 错误：Live 仍运行，之后读取最新目标；Replay 将 Running 提交为 false，并在 ECS 更新路径发送 `WaveformRunningChanged`，应用据此暂停自己的时钟，显式恢复前不重试。旧请求错误由 Source 按身份丢弃，不应作为当前请求错误返回。`WaveformCursor` 是应用可写的外部会话时间输入，不承担播放时钟。Replay 读取范围限制在元信息总帧数内；暂停时忽略输入推进，应用恢复后继续提供正确时间。

Runtime 根据当前时间和可见时长维护 Raw / Reduced Output；稳定 Sample Boundary 和输出密度为 NoOp，零读取且清空本轮统计。Source 失败保留先前完整数据和时间区间，相同输入可重试。每个实例可通过 `last_error()` 查询最近读取错误，成功更新后清除；暂停不伪装为错误恢复。

绘图区下方的独立时间标尺与主刻度网格消费已提交的 `viewport_range()`，使用相同的相对位置；Pending 期间仍对应旧波形。时间始终是会话累计秒数，主刻度锚定零点，间距为 `1–2–5 × 10^n`。以约 90 个逻辑像素选取初始密度，再根据实际文字宽度调整；次刻度不标数字，间距不足约 20 个逻辑像素时减少或隐藏。缩放或宽度变化时，布局后的文字测量负责拒绝本帧重叠或越界的标签，随后使用新测量结果调整密度。标尺文字处在自己的裁剪区域，不占绘图区。

网格只延伸主刻度，以主题文字色的低透明度画在波形背后，默认开启；`WaveformProps::show_grid` 初始化，`Waveform::grid_visible()` 查询，`set_grid_visible_in_world()` / Commands `set_grid_visible()` 更新。实际改变先提交状态，再发送 `WaveformGridChanged`；同值设置不通知，开关不读源、不改变时间、不重建降采样。网格设置跨会话保留。

Renderer 根据实际绘图区宽度确定输出密度，复用一个 Viewport / Camera / Image、一个波形 Mesh 和一个主网格 Mesh。更换会话使旧几何失效，但不替换这些渲染资源；下一次渲染清空旧画面。Root 销毁时释放其专属资源。Palette 沿用主题及 `WidgetryWaveformColorOverrides`，按源的通道顺序取色。

## 独立屏幕测量

顶部 `Δt / ΔV` 通用 Toggle Button 的 Checked 是测量开关的唯一 Authority。`measurement_toggle` 返回按钮根实体；`measurement_enabled` 查询，`set_measurement_enabled_in_world` / Commands `set_measurement_enabled` 委托按钮 API。关闭即清除矩形并释放 Pointer，重新开启从空白开始；换会话同样清除矩形，但保持开关配置。

测量开启时，绘图区接收独立测量手势：空白处拖动新建并替换唯一矩形，松开保留，内部平移，四边或四角缩放（边界命中容差 5 个逻辑像素）。移动保持尺寸，所有坐标约束在绘图区。绘制层与文字均在绘图区裁剪，不覆盖标签、控制栏或时间轴。开启期间绘图区 Pointer 由测量层接收，通道滚动条仍可独立操作；关闭恢复原绘图区滚轮输入。取消、焦点离开、窗口失焦、隐藏、Disabled、Pointer 丢失与根销毁结束手势并释放捕获。

`measurement` 返回只读 `Option<WaveformMeasurement>`：plot 局部逻辑像素 rectangle、delta_seconds、delta_volts 和显示单位。`clear_measurement_in_world` / Commands `clear_measurement` 清空；实际变化后发根实体的 `WaveformMeasurementChanged`，通知回调可同步读取结果，同值不通知。比例尺或布局更新导致结果变化时同样报告；初始化不产生空通知。

矩形以屏幕位置和尺寸为真值，不关联样本或通道。时间差为 `w/W * T`，T 来自与波形一致的已提交 viewport_range，Pending 保留旧尺度；幅值差为 `r/h * 2A`，h 使用实际通道行高，A 使用统一单侧幅值上限。显示按幅值预设选 V / mV / µV。范围、幅值、通道密度和尺寸变化时重算；窗口缩小时约束矩形，布局投影在后续 UI 帧稳定。Live 与 Replay 运行/暂停不因测量改变，覆盖层不读取样本、不触发降采样，不将 ΔV 解释成真实样本电位差。

## Replay 基础播放控制

Replay 会话在时间标尺下方动态创建 40px 控制栏，包含纯图标播放/暂停 Button、当前时间/总时长及倍率 ComboBox，不附 Tooltip 或“播放速度”标签。Live 与空会话不创建该栏。进度位置目前保留布局槽，尚未组合 Slider。`WaveformProps::replay_speeds` 提供正有限、不重复且包含默认 1× 的候选，默认 0.25/0.5/1/1.5/2/4×。

`Waveform::playback()` 返回 Replay 的 `WaveformPlayback` 快照，包含 session_generation、Duration position、playing 和 f64 speed；Live/空会话返回 None。playing 直接读取 Runtime.running，position 直接读取外部输入 WaveformCursor，不维护重复权威。通过 `set_playback_in_world` 或 Commands `set_playback` 接受外部控制器实际状态，校验当前会话身份、范围和倍率；同值不通知，变更全部提交后发 `WaveformPlaybackChanged`，运行状态变化还发既有 `WaveformRunningChanged`。Replay 控制器统一通过此 API 提交位置、运行及倍率；Live 继续使用直接 Cursor 输入。

用户操作只发送 `WaveformPlaybackRequested`，携带根 Entity、当前 session_generation 和 `Playing(bool)` / `Speed(f64)` 意图。宿主接受请求后提交真实状态；拒绝时保持原播放图标，倍率在状态投影中恢复，不伪装已成功播放。程序化倍率投影不反馈成新的播放意图。

Widget 不推进时钟。外部控制器按真实 delta×speed 推进 position，正常 Pending 时继续推进时间和控制栏，波形/轴成对保留旧快照并可丢弃过时帧；真实读取错误令 Runtime.running=false，快照查询和播放图标同步暂停，显式恢复前不重试。暂停提交位置改变时可执行一次完整目标刷新；与正常持续播放一样使用跟随窗口，6s/10s 为 `[0,10]`，15s/10s 为 `[5,15]`。

新会话归零暂停并保留倍率与显示设置。总时长位置必须提交暂停，超过总时长或“在末尾仍播放”的状态明确报错。到达末尾、再次播放从零重启及不自动循环由外部控制器负责；Gallery 已提供这些行为。会话切换会重建控制栏以结束旧按钮手势及倍率弹窗，并拒绝旧 session_generation 的排队状态提交。矩形测量与回放时钟独立。

## 已知限制

- Source 可以快速返回 Pending 并在外部实现异步 I/O；库不提供通用后台任务调度器。
- 大窗口的跨度切换会触发一次完整范围读取与后台降采样；同步 Raw Source 的读取、校验与物理量转换成本随当前窗口和通道数增长，范围切换不保证 16.7 ms 内完成。Pending 可以避免同步等待源 I/O。
- 当前提供 MinMax 和 LTTB 的窗口内计算、外部 Source 的两种多级磁盘索引及 Replay 基础控制栏；进度 Slider 拖动预览尚未实现。
- 显示配置、网格、配色、通道密度、滚动意图、测量开关与回放倍率跨会话保留；矩形在换会话时清除。
- Headless 正确性测试及 CPU Benchmark 不代表完整 GPU 绘制性能。
- 实时波形、标尺与网格的组合仍可能出现超过 16.7 ms 的长帧，当前不承诺所有帧均满足稳定 60 FPS 预算。
