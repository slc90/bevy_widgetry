const REQUEST_START_STRIDE: u64 = 7919;
const REQUEST_WINDOW_FRAMES: u64 = 640_000;
const GAP_PERIOD_FRAMES: u64 = 100_000;
const GAP_LENGTH_FRAMES: u64 = 100;
const GAP_CHANNEL: usize = 2;
const SIGNAL_PERIOD_FRAMES: u64 = 197;
use bevy::prelude::*;
use bevy_widgetry_test_utils::benchmark::{Harness, run};
use bevy_widgetry_waveform::*;
use bevy_widgetry_waveform_file::*;
use std::sync::Arc;

struct Fixture {
    directory: Arc<tempfile::TempDir>,
    recording: Recording,
    frames: u64,
}

fn fixture(frames: u64) -> Result<Fixture> {
    let directory = tempfile::tempdir().map_err(|e| BevyError::error(e.to_string()))?;
    let path = directory.path().join("benchmark.widx");
    build_recording(
        &path,
        WaveformMetadata {
            channel_count: 4,
            channel_labels: (0..4).map(|i| format!("E{i}")).collect(),
            sample_rate: 64_000,
            unit: WaveformUnit::Microvolts,
            kind: WaveformSessionKind::Replay {
                total_frames: frames,
            },
        },
        |channel, range, out| {
            out.extend(range.map(|i| {
                if i % GAP_PERIOD_FRAMES < GAP_LENGTH_FRAMES && channel == GAP_CHANNEL {
                    None
                } else {
                    Some((i % SIGNAL_PERIOD_FRAMES) as f32)
                }
            }));
            Ok(())
        },
    )
    .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?;
    let recording =
        Recording::open(&path).map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?;
    Ok(Fixture {
        directory: Arc::new(directory),
        recording,
        frames,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("waveform_file_random_read")?;
    for frames in [1_280_000, 12_800_000] {
        let prepared = fixture(frames)?;
        for algorithm in [WaveformAlgorithm::MinMax, WaveformAlgorithm::Lttb] {
            run(
                &mut harness,
                &format!("{algorithm:?}/random_{frames}"),
                false,
                || {
                    Ok(Fixture {
                        directory: prepared.directory.clone(),
                        recording: Recording::open(
                            &prepared.directory.path().join("benchmark.widx"),
                        )
                        .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?,
                        frames,
                    })
                },
                |fixture, index| {
                    let start = (index as u64 * REQUEST_START_STRIDE)
                        % (fixture.frames - REQUEST_WINDOW_FRAMES + 1);
                    let result = fixture
                        .recording
                        .query(start..start + REQUEST_WINDOW_FRAMES, 1200, algorithm)
                        .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?;
                    std::hint::black_box(result);
                    Ok(())
                },
                |_| Ok(0),
            )?;
        }
    }
    harness.finish()
}
