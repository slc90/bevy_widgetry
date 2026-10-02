//! 真实 BSN/UI/geometry CPU path，不创建 GPU；完整 render gate 由 Gallery 单独测量。

use bevy::prelude::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, ui_app};
use bevy_widgetry_waveform::*;
use std::ops::Range;
use std::sync::Arc;

/// 便宜确定性 saw，计入组合 update 的 adapter 数据生成/复制成本。
struct Source;

/// 一实例固定 hierarchy，持续推进 cursor 或通过 style mutation 单独重建 geometry。
struct Fixture {
    app: App,
    root: Entity,
    frame: u64,
    rate: u32,
}

impl WaveformSource for Source {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        for channel in out.channels_mut() {
            channel.extend(
                range
                    .clone()
                    .map(|frame| (frame % 400) as f32 / 200.0 - 1.0),
            );
        }
        Ok(())
    }
}

/// 单维度比较 channel/viewport；两个 representation 与 meaningful burst 都使用生产 mesh。
fn main() -> Result {
    let mut harness = Harness::new("waveform-render-criterion")?;
    for (channels, width) in [(4, 1600), (16, 1600), (64, 800), (64, 1600), (64, 2400)] {
        for action in [
            "steady",
            "burst",
            "envelope_geometry",
            "polyline_geometry",
            "noop",
        ] {
            run(
                &mut harness,
                &format!("waveform_render/ch{channels}/w{width}/{action}"),
                false,
                || fixture(channels, width, action),
                |fixture, index| {
                    if matches!(action, "envelope_geometry" | "polyline_geometry") {
                        fixture
                            .app
                            .world_mut()
                            .get_mut::<WaveformStyle>(fixture.root)
                            .ok_or_else(|| missing("style"))?
                            .line_width = if index.is_multiple_of(2) { 1.0 } else { 1.1 };
                    } else if action != "noop" {
                        fixture.frame += if action == "burst" {
                            u64::from(fixture.rate) / 10
                        } else {
                            u64::from(fixture.rate).div_ceil(60)
                        };
                        fixture
                            .app
                            .world_mut()
                            .get_mut::<WaveformCursor>(fixture.root)
                            .ok_or_else(|| missing("cursor"))?
                            .position = duration_from_frames(fixture.frame, fixture.rate)?;
                    }
                    fixture.app.update();
                    Ok(())
                },
                |fixture| {
                    let runtime = fixture
                        .app
                        .world()
                        .get::<WaveformRuntime>(fixture.root)
                        .ok_or_else(|| missing("runtime"))?;
                    if runtime.buffered_range().end != fixture.frame {
                        return Err(missing("committed frame"));
                    }
                    std::hint::black_box(runtime.reduced_channels());
                    Ok(fixture.app.world().entities().count_spawned())
                },
            )?;
        }
    }
    harness.finish()
}

/// 原始数据已满屏后再开始计时；Polyline 使用 width frames 覆盖 1 s，Envelope 固定目标规格。
fn fixture(channels: usize, width: u32, action: &str) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    let polyline = action == "polyline_geometry";
    let rate = if polyline { width } else { 64_000 };
    let duration = if polyline { 1000 } else { 10_000 };
    let adapter: Arc<dyn WaveformSource> = Arc::new(Source);
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @config: {WaveformConfig { sample_rate: rate, visible_duration_ms: duration, channel_ranges: vec![-1.0..=1.0; channels] }}, @source: {Some(adapter)} } Node { width: px(width as f32), height: px(640) } })?.id();
    let frame = u64::from(rate) * u64::from(duration) / 1000;
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .ok_or_else(|| missing("cursor"))?
        .position = duration_from_frames(frame, rate)?;
    app.update();
    app.update();
    Ok(Fixture {
        app,
        root,
        frame,
        rate,
    })
}
