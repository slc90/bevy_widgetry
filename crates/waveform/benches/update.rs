use bevy::prelude::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use bevy_widgetry_waveform::*;
use std::sync::Arc;

struct Source(Vec<f32>, usize, u32, bool);

struct Fixture {
    app: App,
    root: Entity,
    frames: u64,
    rate: u32,
    capacity: u64,
}

struct ReductionFixture {
    raw: Vec<Vec<f32>>,
    reducer: MinMaxReducer,
    output: Vec<ReducedChannel>,
    end: u64,
}

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: self.1,
            channel_labels: (0..self.1)
                .map(|index| format!("Electrode {index}"))
                .collect(),
            sample_rate: self.2,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Live,
        })
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        let range = request.range.clone();
        for channel in out.channels_mut() {
            let mut start = range.start;
            while start < range.end {
                let offset = start as usize % self.0.len();
                let count = ((range.end - start) as usize).min(self.0.len() - offset);
                channel.extend_from_slice(&self.0[offset..offset + count]);
                start += count as u64;
            }
        }
        if self.3 {
            let period = u64::from(self.2);
            for (channel, gaps) in out.missing_mut().iter_mut().enumerate() {
                let mut gap_start =
                    range.start / period * period + period * channel as u64 / (self.1 as u64 * 2);
                while gap_start < range.end {
                    let start = gap_start.max(range.start);
                    let end = (gap_start + period / 10).min(range.end);
                    if start < end {
                        gaps.push((start - range.start) as usize..(end - range.start) as usize);
                    }
                    gap_start += period;
                }
            }
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn main() -> Result {
    let mut harness = Harness::new("waveform-criterion")?;
    benchmark_session(&mut harness)?;
    benchmark_duration(&mut harness)?;
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
        for action in ["steady", "burst", "gaps"] {
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

fn benchmark_session(harness: &mut Harness) -> Result {
    run(
        harness,
        "waveform_session/ch64/rate64000/ms10000/replace",
        false,
        || {
            let mut app = bevy_widgetry_test_utils::scene_app();
            let source: Arc<dyn WaveformSource> = Arc::new(Source(vec![0.0], 64, 64_000, false));
            let root = app
                .world_mut()
                .spawn_scene(bsn! {
                    @Waveform {
                        @config: {WaveformConfig { visible_duration_ms: 10_000, ..default() }},
                        @source: {Some(source)},
                    }
                })?
                .id();
            Ok((app, root, Arc::new(Source(vec![0.0], 64, 64_000, false))))
        },
        |(app, root, source), _| {
            Waveform::set_session_in_world(app.world_mut(), *root, source.clone())
        },
        |(app, root, _)| {
            let runtime = app
                .world()
                .get::<WaveformRuntime>(*root)
                .ok_or_else(|| missing("runtime"))?;
            if runtime.channel_count() != 64 || !runtime.buffered_range().is_empty() {
                return Err(missing("session reset"));
            }
            Ok(app.world().entities().count_spawned())
        },
    )
}

fn benchmark_duration(harness: &mut Harness) -> Result {
    run(
        harness,
        "waveform_duration/ch64/rate64000/w1600/alternate_1s_10s",
        false,
        || {
            let mut app = bevy_widgetry_test_utils::scene_app();
            app.add_plugins(WaveformPlugin);
            let source: Arc<dyn WaveformSource> =
                Arc::new(Source(vec![0.1; 4096], 64, 64000, false));
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @Waveform { @source: {Some(source)} } })?
                .id();
            app.world_mut()
                .get_mut::<WaveformCursor>(root)
                .ok_or_else(|| missing("cursor"))?
                .position = std::time::Duration::from_secs(35);
            app.world_mut()
                .get_mut::<WaveformOutputLength>(root)
                .ok_or_else(|| missing("density"))?
                .0 = 1600;
            app.update();
            Ok((app, root))
        },
        |(app, root), _| {
            let current = app
                .world()
                .get::<WaveformRuntime>(*root)
                .ok_or_else(|| missing("runtime"))?
                .config()
                .visible_duration_ms;
            Waveform::set_visible_duration_in_world(
                app.world_mut(),
                *root,
                if current == 1000 { 10000 } else { 1000 },
            )?;
            app.update();
            Ok(())
        },
        |(app, root)| {
            let runtime = app
                .world()
                .get::<WaveformRuntime>(*root)
                .ok_or_else(|| missing("runtime"))?;
            if runtime.stats().kind != WaveformUpdateKind::FullRead
                || runtime.buffered_range().end != 35 * 64000
            {
                return Err(missing("duration rebuild at unchanged time"));
            }
            std::hint::black_box(runtime.working_set_bytes());
            Ok(app.world().entities().count_spawned())
        },
    )
}

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
                    ..Default::default()
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
                    ..Default::default()
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
                    visible_duration_ms: duration,
                    amplitude_range: WaveformAmplitude {
                        value: 1.0,
                        unit: WaveformUnit::Volts,
                    },
                },
                Arc::new(Source(
                    (0..4096)
                        .map(|index| (index as f32 / 2048.0) - 1.0)
                        .collect(),
                    channels,
                    rate,
                    action == "gaps",
                )),
            )?;
            let capacity = u64::from(rate) * u64::from(duration) / 1000;
            let root = app
                .world_mut()
                .spawn((
                    runtime,
                    WaveformCursor {
                        position: duration_from_frames(capacity, rate)?,
                        ..Default::default()
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
