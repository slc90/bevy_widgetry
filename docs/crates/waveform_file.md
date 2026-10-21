# Waveform File

`bevy_widgetry_waveform_file` 是 Waveform 之外的磁盘数据源适配器。`Recording` 提供离线构建与同步查询，`DiskWaveformSource` 将查询放在独立 I/O 线程中，以 Pending 和完整请求身份接入 Widget。此 crate 不经 Facade 导出。

## 记录与索引

`build_recording(path, metadata, producer)` 接受 Replay 元信息；producer 按通道、最多 65536 样本的块提供声明单位下的 `Option<f32>`，None 表示缺口。有限有效样本、通道标签、采样率及单位写入同一记录文件，避免分离 sidecar 与原记录错配。目标已存在时拒绝覆盖；失败清理临时文件，全部写完后发布。

内部 WFRIDX01 格式包含逐通道原始 f32/有效性、MinMax 索引及 JSON 元信息尾部。索引桶从 256 样本开始倍增，至单桶或 65536 样本为止；层级只合并下层极值和连续有效区间，保留逐通道缺口。桶内各有效段独立保存，不跨缺口合并。

`Recording::open` 校验头尾、元信息及索引表范围，仅读入元信息。`query(range, output_len, MinMax)` 按像素密度选层，批量读取相交完整桶的索引表及连续有效段；边界逐层细化到少量原始样本，确保极值只来自请求内。`query_channels` 可指定连续通道子范围。稀疏窗口直接给真实采样时刻折线，密集窗口给包络。文件读取量随可见区间、像素密度和有效段数量变化，不加载整份记录。最粗桶固定有界，极长窗口仍可能读取较多桶；极密集缺口的输出和构建工作量随有效段数增长。

`DiskQuery::stats` 给出该次查询实际读取字节、局部缓冲及结果容量估计、主要桶大小。它不代表 Windows 文件缓存命中率、设备物理读量或进程完整工作集。

## 后台请求

`DiskWaveformSource::open` 同步打开并校验元信息，应用应在后台加载记录。构造后所有 Waveform 数据查询使用专属 I/O 线程；UI 线程只更新请求和取走结果。每个 reader 最多保留最新请求，新目标替换排队目标，旧结果不会写入新目标。共享 Source 的 reader 隔离；会话替换、清空和 Runtime 销毁取消该 reader 的队列和结果。Source 销毁发停止信号，不在 UI 线程等待磁盘 I/O。

真正读取失败通过 Source 错误通道向宿主传播，Replay Runtime 暂停并保留已提交画面。尚未完成的读取只返回 Pending。当前磁盘索引只支持 MinMax，LTTB 请求明确报错。

## 验证与测量

`cargo test -p bevy_widgetry_waveform_file` 验证真实文件随机读取、跨块边界、短尖峰、逐通道缺口及构建失败。`cargo bench -p bevy_widgetry_waveform_file --bench random_read` 通过既有 Harness 测量 4 通道、64kHz、10s 窗口、1200 输出密度，在 20s 与 200s 记录中的查询。计时只包含真实定位、读取及结果生成，不包含离线构建。Windows 页面缓存会影响耗时，CPU/I/O 测量不代表 GUI 帧时间。
