#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_waveform::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Source(Mutex<(u8, Option<WaveformReadRequest>)>);

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 64_000,
            unit: WaveformUnit::Microvolts,
            kind: WaveformSessionKind::Live,
        })
    }
    fn provides_reduced(&self) -> bool {
        true
    }
    fn read(
        &self,
        _: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(WaveformReadError("raw reads forbidden".into()))
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        let state = self.0.lock().unwrap();
        if state.0 == 0 {
            return Ok(WaveformReducedReadStatus::Pending);
        }
        if state.0 == 3 {
            return Err(WaveformReadError("disk failed".into()));
        }
        Ok(WaveformReducedReadStatus::Ready {
            request: state.1.clone().unwrap_or_else(|| request.clone()),
            channels: vec![ReducedChannel::Envelope(vec![WaveformSpan {
                samples: RangeBoundary {
                    start: request.range.start,
                    end: request.range.end,
                },
                min: -500.0,
                max: if state.0 == 2 { f32::NAN } else { 800.0 },
            }])],
        })
    }
}

#[test]
fn reduced_source_commits_complete_snapshot_without_allocating_raw_window() {
    let source = Arc::new(Source(Mutex::new((1, None))));
    let mut runtime = WaveformRuntime::new(
        WaveformConfig {
            visible_duration_ms: 10_000,
            ..Default::default()
        },
        source.clone(),
    )
    .unwrap();
    assert_eq!(runtime.working_set_bytes().0, 0);
    runtime.update(Duration::from_secs(10), 1200).unwrap();
    assert_eq!(runtime.buffered_range(), 0..640_000);
    assert_eq!(runtime.viewport_range(), 0..640_000);
    assert!(runtime.channel(0).is_none());
    let ReducedChannel::Envelope(spans) = &runtime.reduced_channels()[0] else {
        panic!("expected envelope")
    };
    assert!((spans[0].max - 0.0008).abs() < 1e-9);
    let previous = runtime.reduced_channels().to_vec();
    source.0.lock().unwrap().0 = 0;
    assert_eq!(
        runtime.update(Duration::from_secs(20), 600).unwrap().kind,
        WaveformUpdateKind::Pending
    );
    assert_eq!(runtime.viewport_range(), 0..640_000);
    assert_eq!(runtime.reduced_channels(), previous);
    source.0.lock().unwrap().0 = 1;
    runtime.update(Duration::from_secs(20), 600).unwrap();
    assert_eq!(runtime.viewport_range(), 640_000..1_280_000);
    assert_eq!(runtime.working_set_bytes().0, 0);
    assert_eq!(
        runtime.update(Duration::from_secs(20), 600).unwrap().kind,
        WaveformUpdateKind::NoOp
    );
}

#[test]
fn stale_invalid_and_failed_reduced_reads_cannot_replace_the_snapshot() {
    let source = Arc::new(Source(Mutex::new((1, None))));
    let mut runtime = WaveformRuntime::new(WaveformConfig::default(), source.clone()).unwrap();
    runtime.update(Duration::from_secs(1), 30).unwrap();
    let previous = runtime.reduced_channels().to_vec();
    source.0.lock().unwrap().1 = Some(WaveformReadRequest {
        id: 0,
        reader_id: 0,
        session_generation: 0,
        target_generation: 0,
        range: 64_000..128_000,
        viewport: 64_000..128_000,
        output_len: 30,
        algorithm: WaveformAlgorithm::MinMax,
    });
    assert_eq!(
        runtime.update(Duration::from_secs(2), 30).unwrap().kind,
        WaveformUpdateKind::Pending
    );
    assert_eq!(runtime.viewport_range(), 0..64_000);
    source.0.lock().unwrap().1 = None;
    for mode in [2, 3] {
        source.0.lock().unwrap().0 = mode;
        let error = runtime.update(Duration::from_secs(2), 30).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(runtime.viewport_range(), 0..64_000);
        assert_eq!(runtime.reduced_channels(), previous);
        assert!(runtime.last_error().is_some());
    }
    source.0.lock().unwrap().0 = 1;
    runtime.update(Duration::from_secs(2), 30).unwrap();
    assert_eq!(runtime.viewport_range(), 64_000..128_000);
    assert!(runtime.last_error().is_none());
}
