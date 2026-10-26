# Waveform File

`bevy_widgetry_waveform_file` 是 Waveform 之外的磁盘数据源适配器。`Recording` 提供离线构建与同步查询，`DiskWaveformSource` 将查询放在独立 I/O 线程中，以 Pending 和完整请求身份接入 Widget。此 crate 不经 Facade 导出。

## 记录与索引

`build_recording(path, metadata, producer)` 接受 Replay 元信息；producer 按通道、最多 65536 样本的块提供声明单位下的 `Option<f32>`，None 表示缺口。有限有效样本、通道标签、采样率及单位写入同一记录文件，避免分离 sidecar 与原记录错配。目标已存在时拒绝覆盖；失败清理临时文件，全部写完后发布。

内部 WFRIDX02 格式包含逐通道原始 f32/有效性、MinMax 和 LTTB 索引及 JSON 元信息尾部。MinMax 桶从 256 样本开始倍增，至单桶或 65536 样本为止；层级只合并下层极值和连续有效区间，保留逐通道缺口。桶内各有效段独立保存，不跨缺口合并。记录在打开期间应保持不可变；此格式不承担旧生成文件兼容，旧文件需要重新生成。

LTTB 将每个通道分成 65536 样本的块，每块按 16/64/256/1024/4096/16384 样本密度独立调用原始样本的 LttbReducer。每层重新读取原始块及左邻有效性，既不读取上一层代表点，也不读取 MinMax 桶。记录真实 sample/value/break_before，连续有效的块边界可以连接，逐通道缺口必须断开。查询按窗口/输出密度选层，只读相交块，过滤到目标范围，边界最多补读两个原始样本以保留有效端点；更细密度按块读取原始样本运行相同 LTTB。块局部代表点不保证等于对整个查询窗口一次降采样，预览与最终都保持折线算法；LTTB 不保证保留尖峰。

`Recording::open` 校验头尾、元信息及索引表范围，仅读入元信息。`query(range, output_len, MinMax)` 按像素密度选层，批量读取相交完整桶的索引表及连续有效段；边界逐层细化到少量原始样本，确保极值只来自请求内。`query_channels` 可指定连续通道子范围。稀疏窗口直接给真实采样时刻折线，密集窗口给包络。文件读取量随可见区间、像素密度和有效段数量变化，不加载整份记录。最粗桶固定有界，极长窗口仍可能读取较多桶；极密集缺口的输出和构建工作量随有效段数增长。

`DiskQuery::stats` 给出该次查询实际读取字节、局部缓冲及结果容量估计、主要桶大小。它不代表 Windows 文件缓存命中率、设备物理读量或进程完整工作集。

## 后台请求

`DiskWaveformSource::open(path, &manager)` 同步打开并校验元信息，应用应在后台加载记录，并传入 App 的共享 JobManager。线程收割需要已初始化 IoTaskPool，通常由宿主 Bevy 任务池插件提供。构造后所有 Waveform 数据查询使用一个长期受管 Thread Job；UI 线程只更新请求和取走结果。`job_id()` 提供该服务的 Job 身份，每次查询仍属于业务请求，不单独注册 Job。每个 reader 最多保留最新请求，新目标替换排队目标，旧结果不会写入新目标。共享 Source 的 reader 隔离；会话替换、清空和 Runtime 销毁取消该 reader 的队列和结果。

Source 销毁申请取消、设置停止信号并通知 Condvar，不在 UI 线程等待磁盘 I/O 或 Join。全局取消通过有界 Condvar 等待被空闲服务观察；正在执行的同步读取不能被强行中断，保持 Stopping，完成读取、清理请求并明确确认取消后，真实退出与线程收割才得到 Canceled。停止后的存活 Source 返回明确读取错误，不永久 Pending。单次查询错误仍通过原请求通道反馈，过期错误被丢弃；服务本身的锁错误或 panic 进入 Job Failed 并可由 Source 读取反馈。Manager 保留终态记录，宿主可按 JobId 显式 remove，不能在服务运行时移除。

真正读取失败通过 Source 错误通道向宿主传播，Replay Runtime 暂停并保留已提交画面。尚未完成的读取只返回 Pending。请求身份包含算法和密度，快速算法切换不会提交旧算法结果。

## 验证与测量

`cargo test -p bevy_widgetry_waveform_file` 验证真实文件随机读取、跨块边界、短尖峰、逐通道缺口、独立 LTTB 层正确性及构建失败。`cargo bench -p bevy_widgetry_waveform_file --bench random_read` 通过既有 Harness 测量两种算法的 4 通道、64kHz、10s 窗口、1200 输出密度，在 20s 与 200s 记录中的查询。文件预先构建一次，各场景重新打开文件，计时只包含真实定位、读取及结果生成。Windows 页面缓存会影响耗时，CPU/I/O 测量不代表 GUI 帧时间。

## 本地生成与读取观测

`cargo run -p bevy_widgetry_waveform_file --example generate_recording -- --output <path> --seconds 60 --rate 8000 --channels 8 --seed 42` 流式生成可重复记录及两算法各自的多级索引。默认不是 GB 级别；扩大时长/通道数需明确传参，已有文件不会覆盖。Gallery 使用方式见 [Gallery](../gallery.md)。

`DiskWaveformSource::stats()` 提供只读有界快照：实际 worker 查询次数、已失效/取消查询数、排队 reader 数，以及最近一次成功查询的完整请求身份、总耗时与 `DiskReadStats`。只保留一份观测，不累计请求历史或样本。`DiskReadStats::io_time` 测量 `seek` 与 `read_exact`，包含 OS 缓存命中；总查询减去该值还包括解码、校验、分配和索引计算，不能当作纯降采样时间或冷磁盘耗时。观测不改变 Pending、取消和旧结果丢弃契约。
