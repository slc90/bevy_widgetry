#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_waveform::*;
use std::{sync::Arc, time::Duration};

struct Source;

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 2,
            channel_labels: vec!["A".into(), "B".into()],
            sample_rate: 1000,
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
        for (channel, values) in out.channels_mut().iter_mut().enumerate() {
            values.extend(range.clone().map(|frame| {
                if channel == 0 && (4..6).contains(&(frame % 10)) {
                    f32::NAN
                } else {
                    frame as f32
                }
            }));
        }
        for (channel, offset, length) in [(0, 4, 2), (1, 7, 1)] {
            let mut gap_start = range.start / 10 * 10 + offset;
            while gap_start < range.end {
                let start = gap_start.max(range.start);
                let end = (gap_start + length).min(range.end);
                if start < end {
                    out.missing_mut()[channel]
                        .push((start - range.start) as usize..(end - range.start) as usize);
                }
                gap_start += 10;
            }
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

#[test]
fn explicit_channel_gap_keeps_time_and_does_not_remove_real_zero() {
    let mut runtime = WaveformRuntime::new(
        WaveformConfig {
            visible_duration_ms: 10,
            amplitude_range: WaveformAmplitude {
                value: 10.0,
                unit: WaveformUnit::Volts,
            },
        },
        Arc::new(Source),
    )
    .unwrap();
    runtime.update(Duration::from_millis(10), 20).unwrap();
    assert_eq!(runtime.buffered_range(), 0..10);
    assert_eq!(runtime.channel(0).unwrap().get(0), Some(0.0));
    assert_eq!(runtime.channel(0).unwrap().get(4), None);
    assert_eq!(runtime.channel(1).unwrap().get(4), Some(4.0));
    let ReducedChannel::Polyline(points) = &runtime.reduced_channels()[0] else {
        panic!("expected polyline")
    };
    assert_eq!(
        points.iter().map(|point| point.sample).collect::<Vec<_>>(),
        [0, 1, 2, 3, 6, 7, 8, 9]
    );
}

#[test]
fn minmax_spans_do_not_combine_samples_across_missing_intervals() {
    let mut runtime = WaveformRuntime::new(
        WaveformConfig {
            visible_duration_ms: 10,
            amplitude_range: WaveformAmplitude {
                value: 10.0,
                unit: WaveformUnit::Volts,
            },
        },
        Arc::new(Source),
    )
    .unwrap();
    runtime.update(Duration::from_millis(10), 1).unwrap();
    let ReducedChannel::Envelope(spans) = &runtime.reduced_channels()[0] else {
        panic!("expected envelope")
    };
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].samples, RangeBoundary { start: 0, end: 4 });
    assert_eq!(spans[1].samples, RangeBoundary { start: 6, end: 10 });
}

#[test]
fn incremental_gap_buckets_match_full_rebuild_through_wrap_and_eviction() {
    let config = WaveformConfig {
        visible_duration_ms: 10,
        amplitude_range: WaveformAmplitude {
            value: 100.0,
            unit: WaveformUnit::Volts,
        },
    };
    let mut incremental = WaveformRuntime::new(config.clone(), Arc::new(Source)).unwrap();
    for frame in 1..40 {
        incremental
            .update(duration_from_frames(frame, 1000).unwrap(), 3)
            .unwrap();
        let mut reference = WaveformRuntime::new(config.clone(), Arc::new(Source)).unwrap();
        reference
            .update(duration_from_frames(frame, 1000).unwrap(), 3)
            .unwrap();
        assert_eq!(
            incremental.reduced_channels(),
            reference.reduced_channels(),
            "frame {frame}"
        );
        assert!(incremental.stats().reduction.visited_samples <= 10);
    }
}

#[test]
fn malformed_missing_ranges_are_rejected_without_committing_data() {
    struct Invalid;
    impl WaveformSource for Invalid {
        fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
            Source.metadata()
        }
        fn read(
            &self,
            request: &WaveformReadRequest,
            out: &mut PlanarBuffer,
        ) -> Result<WaveformReadStatus, WaveformReadError> {
            for values in out.channels_mut() {
                values.resize((request.range.end - request.range.start) as usize, 0.0);
            }
            out.missing_mut()[0].extend([4..6, 3..7]);
            Ok(WaveformReadStatus::Ready(request.clone()))
        }
    }
    let mut runtime = WaveformRuntime::new(WaveformConfig::default(), Arc::new(Invalid)).unwrap();
    let error = runtime.update(Duration::from_millis(10), 20).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(runtime.buffered_range(), 0..0);
    assert!(runtime.reduced_channels().is_empty());
}
