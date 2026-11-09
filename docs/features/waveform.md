# Waveform

Waveform 显示多通道时间序列，支持实时会话、回放、MinMax / LTTB 降采样、时间与幅值范围调整、通道滚动和屏幕测量。它消费应用提交的时间位置，不自行推进采集或播放时钟。

```rust
use bevy::prelude::*;
use bevy_widgetry::waveform::*;
use std::sync::Arc;

app.add_plugins((WaveformPlugin, WaveformRenderPlugin));
```

只需要读取缓存与降采样而不创建 UI 时，可以单独使用 `WaveformPlugin` 和 `WaveformRuntime`。

## 数据源

### 实现一个即时数据源

下面的单通道源按请求生成正弦信号，采样率 1000Hz，数值单位为 µV。Runtime 在调用 `read` 前准备好通道容器，Source 向各通道填入完整请求范围。

```rust
struct SineSource;

impl WaveformSource for SineSource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["CH 1".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Microvolts,
            kind: WaveformSessionKind::Live,
        })
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        for channel in out.channels_mut() {
            channel.clear();
            channel.extend(request.range.clone().map(|frame| {
                let seconds = frame as f64 / 1000.0;
                (seconds * std::f64::consts::TAU * 5.0).sin() as f32 * 50.0
            }));
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}
```

会话元信息在建立时固定。通道数、标签、采样率、单位或记录总长发生改变时，应建立新会话，而不是让同一会话的 metadata 在每次读取中变化。

### 表达缺口和异步等待

`missing_mut()` 为每个通道保存相对本次请求起点的缺失区间。区间有序、不重叠、不过界；每个通道仍提供完整长度。真实零值是有效样本，不能用零或 NaN 自动推断缺口。

耗时 I/O 使用 `Pending`，不要阻塞 UI 调用线程。完成后返回与结果对应的完整 `WaveformReadRequest`。请求中的 reader、session、target、范围、算法和密度共同标识结果，不能拿当前请求标签包装旧数据。

## 波形视图

### 创建实时显示

```rust
let source: Arc<dyn WaveformSource> = Arc::new(SineSource);
let waveform = world.spawn_scene(bsn! {
    @Waveform {
        @source: {Some(source)},
        @config: WaveformConfig {
            visible_duration_ms: 1000,
            amplitude_range: WaveformAmplitude {
                value: 100.0,
                unit: WaveformUnit::Microvolts,
            },
        },
        @time_ranges_ms: {vec![1000, 2000, 5000]},
    }
    Node { width: px(800), height: px(360) }
})?.id();
```

时间候选必须为互不重复的正数，并包含初始跨度。空候选隐藏选择器，仍可通过程序设置。显示跨度乘采样率必须对应整数帧。

### 提交实时位置

```rust
fn advance_live(
    time: Res<Time<Real>>,
    mut cursors: Query<&mut WaveformCursor>,
) {
    for mut cursor in &mut cursors {
        cursor.position = time.elapsed();
    }
}
```

这个系统适用于与 App 同时起始、由该系统独占推进的 Live 视图。实际采集应用使用自己的会话累计时间；新会话归零后也重置对应时钟，不能继续使用旧会话时间。

时间不足一个显示跨度时，样本从左生长，右侧保留空白；之后窗口跟随时间向前移动。Live 不提供拖动回看历史的内建时间轴。

## 会话控制

### 替换或清空源

```rust
Waveform::set_session(&mut commands, waveform, Arc::new(SineSource));
Waveform::set_running(&mut commands, waveform, false);
```

同步版本为 `set_session_in_world`、`set_running_in_world`。停止保留最后提交的波形和区间；`clear_session` / `clear_session_in_world` 才清空源与画面。

每次成功替换都建立新会话，即使 Source 指针相同，也会推进会话代数并归零 Cursor。显示设置、颜色和布局意图可以保留，旧请求不能提交到新会话。

## 显示范围与布局

### 修改时间跨度、幅值与通道密度

```rust
Waveform::set_visible_duration(&mut commands, waveform, 2000);
Waveform::set_amplitude(&mut commands, waveform, WaveformAmplitude {
    value: 200.0,
    unit: WaveformUnit::Microvolts,
});
Waveform::set_channels_per_screen(
    &mut commands, waveform, WaveformChannelCount::Count(8),
);
Waveform::set_grid_visible(&mut commands, waveform, false);
```

配置了候选集合时，运行期设置必须选其中的合法值。幅值是单侧上限：100µV 表示 ±100µV，不是峰峰值 100µV。相同物理量换用 V / mV / µV 不会改变高度。

通道标签和波形共用垂直滚动位置。`channel_scroll_viewport()` 返回原生滚动视口，`channel_layout()` 给出已应用布局；配置提交并不意味着当场完成了 UI 测量。

超幅波形允许进入相邻通道行，裁剪发生在整体绘图区，不逐通道制造一条贴边平线。

## 降采样算法

### 选择 MinMax 或 LTTB

```rust
Waveform::set_algorithm(&mut commands, waveform, WaveformAlgorithm::Lttb);
```

| 算法 | 适合观察的特征 |
| --- | --- |
| MinMax | 高密度时保留极值包络，适合关注尖峰；稀疏时显示真实采样折线 |
| LTTB | 用真实样本代表点描述形状，始终为折线，不保证保留每个极值 |

两种算法都显式保留缺口。切换算法、跨度或密度会形成新目标；准备期间继续显示旧完整波形及旧时间轴，Ready 后一起替换，而不是先把旧波形贴到新坐标上。

Source 也可以实现 `provides_reduced` 与 `read_reduced`，直接提供对应算法的结果。此时 Runtime 不保留 Raw Ring，也不重复做本地降采样。

## Replay 播放控制

### 承接播放请求

Replay 元信息提供总帧数，新会话默认暂停在 0。按钮、倍率和定位条发出 `WaveformPlaybackRequested`，应用决定是否接受。

```rust
fn playback_requested(event: On<WaveformPlaybackRequested>) {
    match event.action {
        WaveformPlaybackAction::Playing(playing) => {
            info!(playing, "收到播放意图");
        }
        WaveformPlaybackAction::Seek(position) => {
            info!(?position, "收到定位意图");
        }
        _ => {}
    }
}
```

应用更新自己的时钟，再提交真实播放快照：

```rust
if let Some(mut state) = Waveform::playback(world, waveform)
    .map_err(|failure| failure.into_bevy_error())?
{
    state.playing = false;
    state.preview = false;
    Waveform::set_playback_in_world(world, waveform, state)
        .map_err(|failure| failure.into_bevy_error())?;
}
```

快照保留 `session_generation`，禁止把旧会话的状态写进新记录。到达总时长必须暂停；重新播放、从零开始和循环策略由控制器决定。

### 处理定位预览

SeekStarted 表示手势开始，即使 Thumb 没移动也会出现；Seek 是实际位置变化；SeekEnded 携带结束位置与完成／取消结果。控制器保存原播放意图，预览期间提交 `preview=true, playing=false`，结束后按业务恢复。

预览降低输出密度，但不会把 MinMax 临时换成 LTTB。结束后重新请求完整密度，位置未改变也需要更新。

## 屏幕测量

测量工具在可见绘图区上选择矩形，以当前时间和幅值映射计算 Δt / ΔV。它属于独立 UI 覆盖层，不修改 Source、样本、降采样或播放时钟。

换会话清除旧矩形，显示设置变化重新解释当前布局。测量不是基于原始样本执行峰值、频域或统计分析。
