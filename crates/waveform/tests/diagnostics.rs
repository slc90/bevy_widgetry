#![allow(clippy::disallowed_macros)]

use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
use bevy_widgetry_waveform::*;
use std::{io, sync::Arc, time::Duration};

struct FailingSource {
    error: WaveformReadError,
    phase: u8,
}
impl WaveformSource for FailingSource {
    fn provides_reduced(&self) -> bool {
        self.phase == 2
    }
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        if self.phase == 0 {
            return Err(self.error.clone());
        }
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Microvolts,
            kind: WaveformSessionKind::Live,
        })
    }
    fn read(
        &self,
        _: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(self.error.clone())
    }
    fn read_reduced(
        &self,
        _: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        Err(self.error.clone())
    }
}

#[test]
fn source_failures_keep_origin_native_cause_request_context_and_one_error_log() {
    let manager = bevy_widgetry_job::JobManager::default();
    for phase in 0..3 {
        let logs = LogCapture::default();
        let failure = logs.run(|| {
            WaveformReadError::from(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "source denied",
            ))
        });
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let source = Arc::new(FailingSource {
            error: failure,
            phase,
        });
        let error = logs.run(
            || match WaveformRuntime::new(Default::default(), source, &manager) {
                Err(error) => error,
                Ok(mut runtime) => runtime.update(Duration::from_millis(100), 8).unwrap_err(),
            },
        );
        let diagnostic = error.diagnostics().next().unwrap();
        assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
        let mut cause = diagnostic.source();
        let mut native = false;
        while let Some(error) = cause {
            if let Some(error) = error.downcast_ref::<io::Error>() {
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
                native = true;
            }
            cause = error.source();
        }
        assert!(native);
        for name in if phase == 0 {
            vec!["reader_id", "session_generation"]
        } else {
            vec![
                "reader_id",
                "request_id",
                "session_generation",
                "target_generation",
                "range",
                "channels",
                "output_len",
                "algorithm",
            ]
        } {
            assert!(
                diagnostic
                    .contexts()
                    .iter()
                    .any(|frame| frame.fields().iter().any(|(field, _)| field == name)),
                "missing {name}"
            );
        }
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    }
}

#[test]
fn numeric_failures_keep_original_units_and_values() {
    let amplitude = WaveformAmplitude {
        value: f32::NAN,
        unit: WaveformUnit::Microvolts,
    };
    let failure = WaveformConfig {
        amplitude_range: amplitude,
        ..Default::default()
    }
    .capacity_frames(1000, 1)
    .unwrap_err();
    assert!(
        matches!(failure.kind(), WaveformError::Amplitude { amplitude } if amplitude.value.is_nan() && amplitude.unit == WaveformUnit::Microvolts)
    );
    let failure = sample_boundary(Duration::MAX, 1_000_000_000).unwrap_err();
    assert!(matches!(
        failure.kind(),
        WaveformError::SampleBoundary {
            position: Duration::MAX,
            sample_rate: 1_000_000_000
        }
    ));
    let failure = duration_from_frames(1, 0).unwrap_err();
    assert!(matches!(
        failure.kind(),
        WaveformError::SampleRate { sample_rate: 0 }
    ));
    let failure = WaveformConfig {
        visible_duration_ms: 1,
        ..Default::default()
    }
    .capacity_frames(1, 2)
    .unwrap_err();
    assert!(matches!(
        failure.kind(),
        WaveformError::Capacity {
            visible_duration_ms: 1,
            sample_rate: 1,
            channels: 2
        }
    ));
}

#[test]
fn runtime_system_reports_every_root_with_entity_context() {
    use bevy::prelude::*;
    use bevy_widgetry_test_utils::ErrorCapture;
    let errors = ErrorCapture::default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(WaveformPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let manager = app
        .world()
        .resource::<bevy_widgetry_job::JobManager>()
        .clone();
    let mut expected = Vec::new();
    for _ in 0..2 {
        let failure = WaveformReadError::from(io::Error::other("same text, distinct read"));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let runtime = WaveformRuntime::new(
            Default::default(),
            Arc::new(FailingSource {
                error: failure,
                phase: 1,
            }),
            &manager,
        )
        .unwrap();
        let entity = app
            .world_mut()
            .spawn((
                runtime,
                WaveformCursor {
                    position: Duration::from_millis(100),
                    ..Default::default()
                },
                WaveformOutputLength(8),
            ))
            .id();
        expected.push((entity, origin));
    }
    errors.run(|| app.update());
    let captured = errors.take();
    assert_eq!(captured.len(), 1);
    let report = captured[0].downcast_ref::<WidgetryEcsError>().unwrap();
    assert_eq!(report.diagnostics().count(), 2);
    for (entity, origin) in expected {
        let diagnostic = report
            .diagnostics()
            .find(|d| Arc::ptr_eq(d.origin(), &origin))
            .unwrap();
        assert!(diagnostic.contexts().iter().any(|c| {
            c.fields()
                .iter()
                .any(|(k, v)| k == "entity" && v == &entity.to_string())
        }));
    }
}
