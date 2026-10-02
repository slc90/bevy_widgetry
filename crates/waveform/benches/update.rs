//! 真实 headless runtime/ECS 的 CPU baseline；read_ring 不含 reduction，绝不等同完整 GUI FPS。

use bevy::prelude::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use bevy_widgetry_waveform::*;
use std::ops::Range;
use std::sync::Arc;

/// 已存在的便宜周期数据，adapter 只负责复制，与设备 generation 成本分离。
struct Source(Vec<f32>);

/// 持续 cursor 不随采样轮次重置，保证每次都读取实际的新 batch。
struct Fixture {
    app: App,
    root: Entity,
    frames: u64,
    rate: u32,
    capacity: u64,
}

/// 周期等于 400 frames，固定 backing ring 可提供任意滚动位置的两段真实 raw view。
struct ReductionFixture {
    raw: Vec<Vec<f32>>,
    reducer: MinMaxReducer,
    output: Vec<ReducedChannel>,
    end: u64,
}

impl WaveformSource for Source {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        for channel in out.channels_mut() {
            let mut start = range.start;
            while start < range.end {
                let offset = start as usize % self.0.len();
                let count = ((range.end - start) as usize).min(self.0.len() - offset);
                channel.extend_from_slice(&self.0[offset..offset + count]);
                start += count as u64;
            }
        }
        Ok(())
    }
}

/// 一次仅改变一个维度；固定包含完整目标、100 ms burst、FullRead 和 NoOp。
fn main() -> Result {
    let mut harness = Harness::new("waveform-criterion")?;
    let configs = [
        (4, 64_000, 10_000, 1600),
        (16, 64_000, 10_000, 1600),
        (64, 8_000, 10_000, 1600),
        (64, 32_000, 10_000, 1600),
        (64, 64_000, 1_000, 1600),
        (64, 64_000, 5_000, 1600),
        (64, 64_000, 10_000, 800),
        (64, 64_000, 10_000, 1600),
        (64, 64_000, 10_000, 2400),
    ];
    for (channels, rate, duration, width) in configs {
        for action in ["steady", "burst"] {
            benchmark(&mut harness, channels, rate, duration, width, action)?;
        }
    }
    for action in ["read_ring", "full_read", "noop"] {
        benchmark(&mut harness, 64, 64_000, 10_000, 1600, action)?;
    }
    for batch in [1067, 6400] {
        benchmark_reducer(&mut harness, batch)?;
    }
    harness.finish()
}

/// 独立测量生产 MinMax kernel，不把 read/ring 成本当作 reduction 耗时。
fn benchmark_reducer(harness: &mut Harness, batch: u64) -> Result {
    run(
        harness,
        &format!("waveform/ch64/rate64000/ms10000/w1600/reducer_batch{batch}"),
        false,
        || {
            let raw: Vec<_> = (0..64)
                .map(|channel| {
                    (0..640_000)
                        .map(|index| ((index + channel * 13) % 400) as f32 / 200.0 - 1.0)
                        .collect::<Vec<_>>()
                })
                .collect();
            let mut fixture = ReductionFixture {
                raw,
                reducer: MinMaxReducer::default(),
                output: Vec::new(),
                end: 640_000,
            };
            let views: Vec<_> = fixture
                .raw
                .iter()
                .map(|data| ChannelView {
                    first: data,
                    second: &[],
                })
                .collect();
            fixture.reducer.reduce(
                ReductionInput {
                    channels: &views,
                    buffered: 0..640_000,
                    viewport: 0..640_000,
                    appended: 0..640_000,
                    output_len: 1600,
                    rebuild: true,
                },
                &mut fixture.output,
            );
            Ok(fixture)
        },
        |fixture, _| {
            let old_end = fixture.end;
            fixture.end += batch;
            let start = fixture.end - 640_000;
            let offset = start as usize % 640_000;
            let views: Vec<_> = fixture
                .raw
                .iter()
                .map(|data| ChannelView {
                    first: &data[offset..],
                    second: &data[..offset],
                })
                .collect();
            std::hint::black_box(fixture.reducer.reduce(
                ReductionInput {
                    channels: &views,
                    buffered: start..fixture.end,
                    viewport: start..fixture.end,
                    appended: old_end..fixture.end,
                    output_len: 1600,
                    rebuild: false,
                },
                &mut fixture.output,
            ));
            Ok(())
        },
        |fixture| {
            std::hint::black_box(&fixture.output);
            Ok(0)
        },
    )
}

/// 生产 plugin 的真实 update；fixture 初始化和 memory 观测排除在计时之外。
fn benchmark(
    harness: &mut Harness,
    channels: usize,
    rate: u32,
    duration: u32,
    width: usize,
    action: &str,
) -> Result {
    let name = format!("waveform/ch{channels}/rate{rate}/ms{duration}/w{width}/{action}");
    run(
        harness,
        &name,
        false,
        || {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, WaveformPlugin));
            let runtime = WaveformRuntime::new(
                WaveformConfig {
                    sample_rate: rate,
                    visible_duration_ms: duration,
                    channel_ranges: vec![-1.0..=1.0; channels],
                },
                Arc::new(Source(
                    (0..4096)
                        .map(|index| (index as f32 / 2048.0) - 1.0)
                        .collect(),
                )),
            )?;
            let capacity = u64::from(rate) * u64::from(duration) / 1000;
            let root = app
                .world_mut()
                .spawn((
                    runtime,
                    WaveformCursor {
                        position: duration_from_frames(capacity, rate)?,
                    },
                    WaveformOutputLength(if action == "read_ring" { 0 } else { width }),
                ))
                .id();
            app.update();
            Ok(Fixture {
                app,
                root,
                frames: capacity,
                rate,
                capacity,
            })
        },
        |fixture, _| {
            let batch = match action {
                "noop" => 0,
                "full_read" => fixture.capacity + 1,
                "burst" => u64::from(fixture.rate) / 10,
                _ => u64::from(fixture.rate).div_ceil(60),
            };
            fixture.frames += batch;
            fixture
                .app
                .world_mut()
                .get_mut::<WaveformCursor>(fixture.root)
                .ok_or_else(|| missing("cursor"))?
                .position = duration_from_frames(fixture.frames, fixture.rate)?;
            fixture.app.update();
            Ok(())
        },
        |fixture| {
            let runtime = fixture
                .app
                .world()
                .get::<WaveformRuntime>(fixture.root)
                .ok_or_else(|| missing("runtime"))?;
            if runtime.buffered_range().end != fixture.frames {
                return Err(missing("committed range"));
            }
            std::hint::black_box((runtime.working_set_bytes(), runtime.stats()));
            Ok(fixture.app.world().entities().count_spawned())
        },
    )
}
