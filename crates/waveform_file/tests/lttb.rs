#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_waveform::*;
use bevy_widgetry_waveform_file::*;

fn sample(frame: u64) -> Option<f32> {
    if (32_760..32_790).contains(&frame) || (65_530..65_550).contains(&frame) {
        None
    } else if frame == 1001 {
        Some(10.0)
    } else {
        Some(((frame % 701) as f32 * 0.03).sin())
    }
}

fn metadata(frames: u64) -> WaveformMetadata {
    WaveformMetadata {
        channel_count: 1,
        channel_labels: vec!["E".into()],
        sample_rate: 64_000,
        unit: WaveformUnit::Millivolts,
        kind: WaveformSessionKind::Replay {
            total_frames: frames,
        },
    }
}

#[test]
fn independent_lttb_levels_match_direct_raw_reduction_and_keep_gaps() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lttb.widx");
    build_recording(&path, metadata(196_608), |_, range, out| {
        out.extend(range.map(sample));
        Ok(())
    })
    .unwrap();
    let mut recording = Recording::open(&path).unwrap();
    for stride in [16, 64, 256, 1024, 4096, 16_384] {
        let width = 65_536 / stride;
        let actual = recording
            .query(0..65_536, width, WaveformAlgorithm::Lttb)
            .unwrap();
        assert_eq!(actual.stats.bucket_size, stride as u64);
        let values: Vec<_> = (0..65_536)
            .map(|frame| sample(frame).unwrap_or(0.0))
            .collect();
        let gaps = [32_760..32_790, 65_530..65_536];
        let views = [ChannelView {
            first: &values,
            missing_first: &gaps,
            ..Default::default()
        }];
        let mut expected = Vec::new();
        LttbReducer::default().reduce(
            ReductionInput {
                channels: &views,
                buffered: 0..65_536,
                viewport: 0..65_536,
                appended: 0..65_536,
                output_len: width,
                rebuild: true,
            },
            &mut expected,
        );
        assert_eq!(actual.channels, expected);
    }
    for (range, width) in [
        (0..131_072, 200),
        (65_500..65_600, 5),
        (30_000..70_000, 800),
        (100..200, 200),
        (130_900..131_300, 20),
    ] {
        let result = recording
            .query(range.clone(), width, WaveformAlgorithm::Lttb)
            .unwrap();
        let ReducedChannel::Polyline(points) = &result.channels[0] else {
            panic!("LTTB must remain polyline")
        };
        let mut previous = None;
        for point in points {
            assert_eq!(Some(point.value), sample(point.sample));
            assert!(range.contains(&point.sample));
            if let Some(before) = previous {
                assert!(before < point.sample);
                if !point.break_before {
                    assert!((before..=point.sample).all(|frame| sample(frame).is_some()));
                }
            } else {
                assert!(point.break_before);
            }
            previous = Some(point.sample);
        }
        assert!(!points.is_empty());
        if sample(range.start).is_some() {
            assert_eq!(points.first().unwrap().sample, range.start);
        }
        if sample(range.end - 1).is_some() {
            assert_eq!(points.last().unwrap().sample, range.end - 1);
        }
        if range.contains(&131_072) {
            assert!(
                !points
                    .iter()
                    .find(|point| point.sample == 131_072)
                    .unwrap()
                    .break_before
            );
        }
    }
}

#[test]
fn selected_channel_keeps_its_own_samples_and_lttb_need_not_preserve_extrema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("channels.widx");
    let mut meta = metadata(70_000);
    meta.channel_count = 2;
    meta.channel_labels = vec!["A".into(), "B".into()];
    build_recording(&path, meta, |channel, range, out| {
        out.extend(range.map(|frame| {
            if channel == 0 {
                sample(frame)
            } else {
                Some(-1000.0 - (frame % 17) as f32)
            }
        }));
        Ok(())
    })
    .unwrap();
    let selected = Recording::open(&path)
        .unwrap()
        .query_channels(60_000..69_000, 100, WaveformAlgorithm::Lttb, 1..2)
        .unwrap();
    let ReducedChannel::Polyline(points) = &selected.channels[0] else {
        panic!("expected polyline")
    };
    assert!(
        points
            .iter()
            .all(|point| point.value == -1000.0 - (point.sample % 17) as f32)
    );
    assert_eq!(points.iter().filter(|p| p.break_before).count(), 1);

    let path = dir.path().join("spike.widx");
    let raw = [0.0, 100.0, -101.0, 0.0];
    build_recording(&path, metadata(4), |_, range, out| {
        out.extend(range.map(|i| Some(raw[i as usize])));
        Ok(())
    })
    .unwrap();
    let mut recording = Recording::open(&path).unwrap();
    let lttb = recording.query(0..4, 1, WaveformAlgorithm::Lttb).unwrap();
    let ReducedChannel::Polyline(points) = &lttb.channels[0] else {
        panic!("expected polyline")
    };
    assert!(!points.iter().any(|p| p.value == 100.0));
    let minmax = recording.query(0..4, 1, WaveformAlgorithm::MinMax).unwrap();
    let ReducedChannel::Envelope(spans) = &minmax.channels[0] else {
        panic!("expected envelope")
    };
    assert!(spans.iter().any(|p| p.max == 100.0));
}
