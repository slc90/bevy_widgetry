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
        build_recording(&path, metadata(1000), |_, _, _| Err(
            WaveformReadError::from(std::io::Error::other("input failed"))
        ))
        .is_err()
    );
    assert!(!path.exists());
    std::fs::write(&path, b"bad").unwrap();
    assert!(Recording::open(&path).is_err());
}
#[test]
fn fixed_wire_fixture_matches_the_writer_and_remains_readable() {
    const FORMAT_MAGIC: &[u8; 8] = b"WFRIDX02";
    const INDEX_ENTRY_BYTES: u64 = 16;
    const EXPECTED_MINMAX_SPANS: u64 = 2;
    const EXPECTED_LTTB_POINTS: u64 = 2;
    const MINMAX_PAYLOAD_OFFSET: u64 = 39;
    const RAW_RECORDS: [u8; 15] = [0, 0, 160, 63, 1, 0, 0, 0, 0, 0, 0, 0, 32, 192, 1];
    const MINMAX_SPANS: [u8; 48] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 160, 63, 0, 0, 160, 63, 2, 0, 0, 0,
        0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 192, 0, 0, 32, 192,
    ];
    const LTTB_POINTS: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 160, 63, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 192,
        1, 0, 0, 0,
    ];
    const SAMPLES: [Option<f32>; 3] = [Some(1.25), None, Some(-2.5)];
    const LTTB_TABLES: [u64; 6] = [87, 135, 183, 231, 279, 327];
    let metadata = WaveformMetadata {
        channel_count: 1,
        channel_labels: vec!["fixture".into()],
        sample_rate: 8000,
        unit: WaveformUnit::Millivolts,
        kind: WaveformSessionKind::Replay {
            total_frames: SAMPLES.len() as u64,
        },
    };
    let directory = tempfile::tempdir().unwrap();
    let written = directory.path().join("written.widx");
    build_recording(&written, metadata.clone(), |_, range, out| {
        out.extend_from_slice(&SAMPLES[range.start as usize..range.end as usize]);
        Ok(())
    })
    .unwrap();

    let mut expected_payload = FORMAT_MAGIC.to_vec();
    expected_payload.extend(RAW_RECORDS);
    expected_payload.extend(MINMAX_PAYLOAD_OFFSET.to_le_bytes());
    expected_payload.extend(EXPECTED_MINMAX_SPANS.to_le_bytes());
    expected_payload.extend(MINMAX_SPANS);
    for table in LTTB_TABLES {
        let point_payload_offset = table + INDEX_ENTRY_BYTES;
        expected_payload.extend(point_payload_offset.to_le_bytes());
        expected_payload.extend(EXPECTED_LTTB_POINTS.to_le_bytes());
        expected_payload.extend(LTTB_POINTS);
    }
    let header = serde_json::json!({
        "rate":8000,"frames":3,"unit":1,"labels":["fixture"],
        "levels":[[{"size":256,"table":23,"count":1}]],
        "lttb":[[
            {"stride":16,"table":87},{"stride":64,"table":135},
            {"stride":256,"table":183},{"stride":1024,"table":231},
            {"stride":4096,"table":279},{"stride":16384,"table":327}
        ]]
    });
    let actual = std::fs::read(&written).unwrap();
    let encoded_header_length_bytes = 8;
    let footer_size_bytes = encoded_header_length_bytes + FORMAT_MAGIC.len();
    let magic_bytes = FORMAT_MAGIC.len();
    let header_size = u64::from_le_bytes(
        actual[actual.len() - footer_size_bytes..actual.len() - magic_bytes]
            .try_into()
            .unwrap(),
    ) as usize;
    let payload_end = actual.len() - footer_size_bytes - header_size;
    assert_eq!(&actual[..payload_end], expected_payload);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &actual[payload_end..payload_end + header_size]
        )
        .unwrap(),
        header
    );
    assert_eq!(&actual[actual.len() - magic_bytes..], FORMAT_MAGIC);

    let fixture = directory.path().join("fixed.widx");
    let json = serde_json::to_vec(&header).unwrap();
    expected_payload.extend(&json);
    expected_payload.extend((json.len() as u64).to_le_bytes());
    expected_payload.extend(FORMAT_MAGIC);
    std::fs::write(&fixture, expected_payload).unwrap();
    let mut recording = Recording::open(&fixture).unwrap();
    assert_eq!(recording.metadata(), &metadata);
    for algorithm in [WaveformAlgorithm::MinMax, WaveformAlgorithm::Lttb] {
        let query = recording
            .query(0..SAMPLES.len() as u64, SAMPLES.len(), algorithm)
            .unwrap();
        let ReducedChannel::Polyline(points) = &query.channels[0] else {
            panic!("expected raw points")
        };
        assert_eq!(
            points
                .iter()
                .map(|point| (point.sample, point.value, point.break_before))
                .collect::<Vec<_>>(),
            [(0, 1.25, true), (2, -2.5, true)]
        );
    }
}
