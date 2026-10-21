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
                if i % 100_000 < 100 && channel == 2 {
                    None
                } else {
                    Some((i % 197) as f32)
                }
            }));
            Ok(())
        },
    )
    .map_err(|e| BevyError::error(e.to_string()))?;
    let recording = Recording::open(&path).map_err(|e| BevyError::error(e.to_string()))?;
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
                        .map_err(|e| BevyError::error(e.to_string()))?,
                        frames,
                    })
                },
                |fixture, index| {
                    let start = (index as u64 * 7919) % (fixture.frames - 640_000 + 1);
                    let result = fixture
                        .recording
                        .query(start..start + 640_000, 1200, algorithm)
                        .map_err(|e| BevyError::error(e.to_string()))?;
                    std::hint::black_box(result);
                    Ok(())
                },
                |_| Ok(0),
            )?;
        }
    }
    harness.finish()
}
