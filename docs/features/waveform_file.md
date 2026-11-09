# Waveform File

Waveform File 将离线记录及多级索引存入一个文件，提供同步随机查询和非阻塞 `DiskWaveformSource`。它是独立适配器，不经顶层 Facade 导出；应用直接依赖 `bevy_widgetry_waveform_file`。

```rust
use bevy_widgetry::waveform::{
    WaveformAlgorithm, WaveformMetadata, WaveformSessionKind, WaveformUnit,
};
use bevy_widgetry_waveform_file::{build_recording, Recording, DiskWaveformSource};
```

## 记录构建

### 逐块写入样本与缺口

```rust
let metadata = WaveformMetadata {
    channel_count: 2,
    channel_labels: vec!["CH 1".into(), "CH 2".into()],
    sample_rate: 1000,
    unit: WaveformUnit::Microvolts,
    kind: WaveformSessionKind::Replay { total_frames: 60_000 },
};

build_recording("measurement.widx", metadata, |channel, range, out| {
    out.extend(range.map(|frame| {
        if channel == 1 && (10_000..10_200).contains(&frame) {
            None
        } else {
            let phase = frame as f64 / 1000.0 * std::f64::consts::TAU;
            Some(phase.sin() as f32 * 50.0)
        }
    }));
    Ok(())
}).map_err(|failure| failure.into_bevy_error())?;
```

Producer 按通道和最多 65536 样本的范围被调用，必须提供该范围的完整长度。`None` 表示真实缺口，`Some(0.0)` 是有效零值。数值单位与 Metadata 一致。

构建同时写入原始数据、有效性、MinMax 与 LTTB 索引和元信息。已有目标拒绝覆盖；失败不会把临时文件发布成完整记录。

## 同步随机查询

### 只读取需要的区间

```rust
let mut recording = Recording::open("measurement.widx")
    .map_err(|failure| failure.into_bevy_error())?;
let result = recording.query(10_000..20_000, 1200, WaveformAlgorithm::MinMax)
    .map_err(|failure| failure.into_bevy_error())?;

info!(bytes = result.stats.bytes_read, "查询完成");
```

范围是半开帧区间，`output_len` 表示目标显示密度，不是要求返回恰好多少个点。缺口分段和端点可能增加输出数量。`query_channels` 可进一步限制连续通道范围。

`Recording::open` 只读取必要元信息，不把整份记录加载到内存。查询根据范围与密度选择索引层，在边界处细化，避免范围外的极值污染结果。

### 选择算法

```rust
let line = recording.query(10_000..20_000, 1200, WaveformAlgorithm::Lttb)
    .map_err(|failure| failure.into_bevy_error())?;
```

MinMax 适合极值观察，LTTB 返回真实代表点折线。LTTB 索引直接从原始数据建立，不从 MinMax 包络或更粗代表点再次推导。

## 接入 Waveform

### 使用共享任务管理器打开源

```rust
use bevy_widgetry::job::JobManager;
use bevy_widgetry::waveform::WaveformSource;
use std::sync::Arc;

// manager 来自应用共享的 JobManager；在应用后台加载流程执行 open。
let source: Arc<dyn WaveformSource> = Arc::new(
    DiskWaveformSource::open("measurement.widx", &manager)
        .map_err(|failure| failure.into_bevy_error())?,
);
```

`open` 同步检查文件，适合放在应用后台加载任务中。打开后，波形数据读取由长期受管线程完成，UI 调用只提交目标或取得结果，未完成时返回 Pending。

把 `source` 交给 `Waveform` 的 `@source` 或 `set_session`。应用依然负责回放时钟，文件源不会自动播放记录。

### 共享源与替换会话

不同 View 使用独立 reader 身份。每个 reader 只保留最新目标；换会话、清空或 Runtime 删除会取消相应 reader 的排队请求，不取消其他 View 的工作。

Source 销毁申请停止并唤醒后台线程，不在 UI 线程等待同步 I/O 或 Join。正在执行的读取结束后才能真正退出；停止后的存活 Source 返回明确错误，而不是永久 Pending。

## 查询观测

### 查看真实读取成本

`DiskQuery::stats` 包含本次读取字节、局部缓冲估计、所用桶大小及 I/O 时间。`DiskWaveformSource::stats()` 提供有界的队列与最近一次查询快照。

```rust
let stats = disk_source.stats()
    .map_err(|failure| failure.into_bevy_error())?;
```

`disk_source` 是保留的具体 `DiskWaveformSource`，不是已经擦除为 trait object 的句柄。统计不保存全部查询历史或样本。

读取字节描述代码实际读取量，不等于物理磁盘流量；操作系统缓存仍会影响耗时。查询总时间还包含解码、校验和索引工作，不能全部算成降采样计算。

## 文件与生命周期

记录打开期间保持不可变。文件格式同时绑定数据和索引，不能把另一份记录的索引覆盖过来。格式演进时旧生成文件需要重新构建，不应将内部格式当作长期交换标准。

极长时间范围仍可能访问多个粗桶；碎片化缺口的成本随有效段数增长。索引减少的是不必要的原始读取，不把所有查询变成固定成本。
