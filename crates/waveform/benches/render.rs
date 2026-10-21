use bevy::prelude::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, ui_app};
use bevy_widgetry_waveform::*;
use std::sync::Arc;

struct Source(usize, u32);

struct Fixture {
    app: App,
    root: Entity,
    frame: u64,
    rate: u32,
}

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: self.0,
            channel_labels: (0..self.0)
                .map(|index| format!("Electrode {index}"))
                .collect(),
            sample_rate: self.1,
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
            channel.extend(
                range
                    .clone()
                    .map(|frame| (frame % 400) as f32 / 200.0 - 1.0),
            );
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn main() -> Result {
    let mut harness = Harness::new("waveform-render-criterion")?;
    for (channels, width) in [(4, 1600), (16, 1600), (64, 800), (64, 1600), (64, 2400)] {
        for action in [
            "steady",
            "burst",
            "envelope_geometry",
            "polyline_geometry",
            "palette",
            "amplitude",
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
                    } else if action == "amplitude" {
                        Waveform::set_amplitude_in_world(
                            fixture.app.world_mut(),
                            fixture.root,
                            WaveformAmplitude {
                                value: if index.is_multiple_of(2) { 0.25 } else { 0.5 },
                                unit: WaveformUnit::Volts,
                            },
                        )?;
                    } else if action == "palette" {
                        let mut colors = WidgetryWaveformColorOverrides::default();
                        colors.normal.palette = Some(if index.is_multiple_of(2) {
                            vec![Color::WHITE, Color::BLACK]
                        } else {
                            vec![Color::BLACK, Color::WHITE]
                        });
                        WidgetryWaveformColorOverrides::set_in_world(
                            fixture.app.world_mut(),
                            fixture.root,
                            colors,
                        )?;
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

fn fixture(channels: usize, width: u32, action: &str) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    let polyline = action == "polyline_geometry";
    let rate = if polyline { width } else { 64_000 };
    let duration = if polyline { 1000 } else { 10_000 };
    let adapter: Arc<dyn WaveformSource> = Arc::new(Source(channels, rate));
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @config: {WaveformConfig { visible_duration_ms: duration, amplitude_range: WaveformAmplitude { value: 1.0, unit: WaveformUnit::Volts } }}, @source: {Some(adapter)} } Node { width: px(width as f32), height: px(640) } })?.id();
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
