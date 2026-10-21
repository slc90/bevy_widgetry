#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};
use bevy_widgetry_waveform::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

struct Source(WaveformUnit, f32, AtomicUsize);

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 4,
            channel_labels: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            sample_rate: 10,
            unit: self.0,
            kind: WaveformSessionKind::Live,
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.2.fetch_add(1, Ordering::Relaxed);
        for channel in out.channels_mut() {
            channel.extend(
                request
                    .range
                    .clone()
                    .map(|i| [0.0, 3.0, 3.0, -3.0, 0.0][i as usize % 5] * self.1),
            );
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn fixture(unit: WaveformUnit, value: f32) -> (App, Entity, Arc<Source>, Handle<Mesh>) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let source = Arc::new(Source(unit, value, AtomicUsize::new(0)));
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let amplitude = WaveformAmplitude {
        value: 100.0,
        unit: WaveformUnit::Microvolts,
    };
    let root = app.world_mut().spawn_scene(bsn! {
        @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { amplitude_range: amplitude, ..default() }} }
        template(move |_| Ok(UiTargetCamera(camera)))
        Node { width: px(400), height: px(228) }
    }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(1);
    for _ in 0..3 {
        app.update();
    }
    let mesh = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "WaveformMesh")
        .unwrap()
        .1
        .0
        .clone();
    (app, root, source, mesh)
}

fn positions(app: &App, mesh: &Handle<Mesh>) -> Vec<[f32; 3]> {
    let VertexAttributeValues::Float32x3(points) = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(mesh)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
    else {
        panic!("Waveform mesh requires position attributes")
    };
    points.clone()
}

#[test]
fn source_units_match_and_amplitude_reprojects_paused_data_without_read_or_reduction() {
    let mut reference = None;
    for (unit, value) in [
        (WaveformUnit::Volts, 0.0001),
        (WaveformUnit::Millivolts, 0.1),
        (WaveformUnit::Microvolts, 100.0),
    ] {
        let (mut app, root, source, mesh) = fixture(unit, value);
        let points = positions(&app, &mesh);
        assert!(
            points
                .iter()
                .all(|point| point[0].abs() <= 0.500001 && point[1].abs() <= 0.500001)
        );
        if let Some(reference) = &reference {
            let reference: &Vec<[f32; 3]> = reference;
            assert_eq!(reference.len(), points.len());
            for (a, b) in reference.iter().zip(&points) {
                assert!((a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5);
            }
        } else {
            reference = Some(points.clone());
        }
        Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        let revision = runtime.revision();
        let output = runtime.reduced_channels().to_vec();
        let range = runtime.viewport_range();
        let generation = runtime.session_generation();
        let reads = source.2.load(Ordering::Relaxed);
        let amplitude = WaveformAmplitude {
            value: 0.2,
            unit: WaveformUnit::Millivolts,
        };
        Waveform::set_amplitude_in_world(app.world_mut(), root, amplitude).unwrap();
        app.update();
        assert_ne!(positions(&app, &mesh), points);
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.revision(), revision);
        assert_eq!(runtime.reduced_channels(), output);
        assert_eq!(runtime.viewport_range(), range);
        assert_eq!(source.2.load(Ordering::Relaxed), reads);
        assert!(!runtime.is_running());
        assert_eq!(runtime.session_generation(), generation);
        let adapter: Arc<dyn WaveformSource> = source;
        Waveform::set_session_in_world(app.world_mut(), root, adapter).unwrap();
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .config()
                .amplitude_range,
            amplitude
        );
    }
}

#[derive(Resource, Default)]
struct Notices(Vec<WaveformAmplitude>);

#[test]
fn invalid_queued_amplitude_reaches_host_handler_and_logs_once() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            let mut app = scene_app();
            app.set_error_handler(ErrorCapture::handler());
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @Waveform })
                .unwrap()
                .id();
            Waveform::set_amplitude(
                &mut app.world_mut().commands(),
                root,
                WaveformAmplitude {
                    value: 0.0,
                    unit: WaveformUnit::Volts,
                },
            );
            app.world_mut().flush();
            assert_eq!(
                app.world()
                    .get::<WaveformRuntime>(root)
                    .unwrap()
                    .config()
                    .amplitude_range,
                WaveformAmplitude::default()
            );
        })
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        1
    );
}

#[test]
fn amplitude_commits_before_notification_and_validates_without_a_session() {
    let mut app = scene_app();
    app.init_resource::<Notices>();
    app.add_observer(
        |event: On<WaveformAmplitudeChanged>,
         runtimes: Query<&WaveformRuntime>,
         mut notices: ResMut<Notices>| {
            assert_eq!(
                runtimes.get(event.entity).unwrap().config().amplitude_range,
                event.amplitude
            );
            notices.0.push(event.amplitude);
        },
    );
    let uv = WaveformAmplitude {
        value: 100.0,
        unit: WaveformUnit::Microvolts,
    };
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    assert!(Waveform::set_amplitude_in_world(app.world_mut(), root, uv).unwrap());
    assert!(!Waveform::set_amplitude_in_world(app.world_mut(), root, uv).unwrap());
    assert_eq!(app.world().resource::<Notices>().0, vec![uv]);
    Waveform::set_amplitude(
        &mut app.world_mut().commands(),
        root,
        WaveformAmplitude::default(),
    );
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Notices>().0.len(), 2);
    for value in [0.0, f32::INFINITY, f32::NAN, -1.0] {
        let error = Waveform::set_amplitude_in_world(
            app.world_mut(),
            root,
            WaveformAmplitude { value, ..uv },
        )
        .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .config()
                .amplitude_range,
            WaveformAmplitude::default()
        );
    }
    app.world_mut().despawn(root);
    assert!(Waveform::set_amplitude_in_world(app.world_mut(), root, uv).is_err());
    assert_eq!(app.world().resource::<Notices>().0.len(), 2);
}
