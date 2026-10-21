#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_waveform::*;
use bevy_widgetry_waveform_file::*;

fn metadata(frames: u64) -> WaveformMetadata {
    WaveformMetadata {
        channel_count: 2,
        channel_labels: vec!["A".into(), "B".into()],
        sample_rate: 64_000,
        unit: WaveformUnit::Microvolts,
        kind: WaveformSessionKind::Replay {
            total_frames: frames,
        },
    }
}

fn signal(channel: usize, frame: u64) -> Option<f32> {
    if channel == 1 && (514..781).contains(&frame) {
        None
    } else if frame == 1023 {
        Some(999.0)
    } else {
        Some((frame % 113) as f32 - 50.0)
    }
}

#[test]
fn disk_minmax_matches_raw_extrema_preserves_gaps_and_bounds_io() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.widx");
    build_recording(&path, metadata(200_000), |channel, range, out| {
        out.extend(range.map(|frame| signal(channel, frame)));
        Ok(())
    })
    .unwrap();
    let mut recording = Recording::open(&path).unwrap();
    assert_eq!(recording.metadata(), &metadata(200_000));
    assert!(
        recording
            .query(0..200_001, 10, WaveformAlgorithm::MinMax)
            .is_err()
    );
    assert!(
        recording
            .query(0..100, 0, WaveformAlgorithm::MinMax)
            .is_err()
    );
    let selected = recording
        .query_channels(500..1100, 2, WaveformAlgorithm::MinMax, 1..2)
        .unwrap();
    assert_eq!(selected.channels.len(), 1);
    for (range, width) in [
        (500..1100, 2),
        (500..1100, 1000),
        (0..200_000, 80),
        (65_500..68_000, 8),
    ] {
        let result = recording
            .query(range.clone(), width, WaveformAlgorithm::MinMax)
            .unwrap();
        assert!(result.stats.bytes_read < 100_000);
        for (channel, reduced) in result.channels.iter().enumerate() {
            match reduced {
                ReducedChannel::Envelope(spans) => {
                    for span in spans {
                        let raw: Vec<_> = (span.samples.start..span.samples.end)
                            .map(|frame| signal(channel, frame).unwrap())
                            .collect();
                        assert_eq!(span.min, raw.iter().copied().fold(f32::INFINITY, f32::min));
                        assert_eq!(
                            span.max,
                            raw.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                        );
                        assert!(span.samples.start >= range.start && span.samples.end <= range.end);
                    }
                    let covered: u64 = spans
                        .iter()
                        .map(|span| span.samples.end - span.samples.start)
                        .sum();
                    assert_eq!(
                        covered,
                        range
                            .clone()
                            .filter(|&frame| signal(channel, frame).is_some())
                            .count() as u64
                    );
                    if range.contains(&1023) {
                        assert!(spans.iter().any(|span| span.max == 999.0));
                    }
                }
                ReducedChannel::Polyline(points) => {
                    for point in points {
                        assert_eq!(Some(point.value), signal(channel, point.sample));
                    }
                    assert_eq!(
                        points.len(),
                        range
                            .clone()
                            .filter(|&frame| signal(channel, frame).is_some())
                            .count()
                    );
                    if channel == 1 {
                        assert!(
                            points
                                .iter()
                                .find(|point| point.sample == 781)
                                .unwrap()
                                .break_before
                        );
                    }
                }
            }
        }
    }
    let larger = dir.path().join("long.widx");
    build_recording(&larger, metadata(2_000_000), |channel, range, out| {
        out.extend(range.map(|frame| signal(channel, frame)));
        Ok(())
    })
    .unwrap();
    let small = recording
        .query(500..150_000, 200, WaveformAlgorithm::MinMax)
        .unwrap();
    let large = Recording::open(&larger)
        .unwrap()
        .query(500..150_000, 200, WaveformAlgorithm::MinMax)
        .unwrap();
    assert_eq!(small.channels, large.channels);
    assert_eq!(small.stats.bytes_read, large.stats.bytes_read);
    assert_eq!(small.stats.peak_buffer_bytes, large.stats.peak_buffer_bytes);
    let small = recording
        .query(500..150_000, 200, WaveformAlgorithm::Lttb)
        .unwrap();
    let large = Recording::open(&larger)
        .unwrap()
        .query(500..150_000, 200, WaveformAlgorithm::Lttb)
        .unwrap();
    assert_eq!(small.channels, large.channels);
    assert_eq!(small.stats.bytes_read, large.stats.bytes_read);
    assert_eq!(small.stats.peak_buffer_bytes, large.stats.peak_buffer_bytes);
    for start in [0, 1, 255, 256, 257, 511, 65_535] {
        for length in [1, 255, 256, 257, 65_536] {
            for width in [1, 1200] {
                let range = start..start + length;
                let result = recording
                    .query(range.clone(), width, WaveformAlgorithm::MinMax)
                    .unwrap();
                for (channel, reduced) in result.channels.iter().enumerate() {
                    let covered = match reduced {
                        ReducedChannel::Polyline(points) => points.len() as u64,
                        ReducedChannel::Envelope(spans) => {
                            for span in spans {
                                assert!(
                                    span.samples.end - span.samples.start
                                        <= length.div_ceil(width as u64)
                                );
                                assert!(
                                    span.samples.start >= range.start
                                        && span.samples.end <= range.end
                                );
                                assert!(
                                    (span.samples.start..span.samples.end)
                                        .all(|frame| signal(channel, frame).is_some())
                                );
                            }
                            spans
                                .iter()
                                .map(|span| span.samples.end - span.samples.start)
                                .sum()
                        }
                    };
                    assert_eq!(
                        covered,
                        range
                            .clone()
                            .filter(|&frame| signal(channel, frame).is_some())
                            .count() as u64
                    );
                }
            }
        }
    }
}

#[test]
fn rejects_invalid_ranges_corruption_and_failed_builds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("failed.widx");
    assert!(
        build_recording(&path, metadata(1000), |_, _, _| Err(WaveformReadError(
            "input failed".into()
        )))
        .is_err()
    );
    assert!(!path.exists());
    std::fs::write(&path, b"bad").unwrap();
    assert!(Recording::open(&path).is_err());
}
