#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_waveform::*;
use std::{sync::Arc, time::Duration};

struct Source(WaveformMetadata);

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(self.0.clone())
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        let range = request.range.clone();
        for channel in out.channels_mut() {
            channel.extend(range.clone().map(|_| 500.0));
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn source(channels: usize, rate: u32, kind: WaveformSessionKind) -> Arc<dyn WaveformSource> {
    Arc::new(Source(WaveformMetadata {
        channel_count: channels,
        channel_labels: (0..channels)
            .map(|index| format!("Electrode {index}"))
            .collect(),
        sample_rate: rate,
        unit: WaveformUnit::Microvolts,
        kind,
    }))
}

#[test]
fn replacing_sessions_keeps_root_and_display_settings_but_resets_data() {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @Waveform { @config: {WaveformConfig { visible_duration_ms: 2000, amplitude_range: -0.001..=0.001 }} }
    }).unwrap().id();
    for (count, rate, kind) in [
        (32, 1000, WaveformSessionKind::Live),
        (64, 2000, WaveformSessionKind::Replay { total_frames: 4000 }),
        (8, 1000, WaveformSessionKind::Replay { total_frames: 2000 }),
        (32, 1000, WaveformSessionKind::Live),
    ] {
        Waveform::set_session_in_world(app.world_mut(), root, source(count, rate, kind)).unwrap();
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.metadata().unwrap().channel_count, count);
        assert_eq!(runtime.metadata().unwrap().sample_rate, rate);
        assert_eq!(runtime.config().visible_duration_ms, 2000);
        assert_eq!(runtime.config().amplitude_range, -0.001..=0.001);
        assert_eq!(runtime.buffered_range(), 0..0);
        assert!(runtime.reduced_channels().is_empty());
        assert_eq!(
            app.world().get::<WaveformCursor>(root).unwrap().position,
            Duration::ZERO
        );
        Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
        app.world_mut()
            .get_mut::<WaveformCursor>(root)
            .unwrap()
            .position = Duration::from_secs(1);
        app.world_mut()
            .get_mut::<WaveformOutputLength>(root)
            .unwrap()
            .0 = 100;
        app.update();
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert!((runtime.channel(0).unwrap().get(0).unwrap() - 0.0005).abs() < 1e-9);
        assert_eq!(runtime.reduced_channels().len(), count);
    }
}

#[test]
fn invalid_metadata_cannot_replace_a_committed_session() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    Waveform::set_session_in_world(
        app.world_mut(),
        root,
        source(2, 1000, WaveformSessionKind::Live),
    )
    .unwrap();
    app.world_mut()
        .get_mut::<WaveformRuntime>(root)
        .unwrap()
        .update(Duration::from_secs(1), 100)
        .unwrap();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(1);
    let output = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .reduced_channels()
        .to_vec();
    let generation = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .session_generation();
    for metadata in [
        WaveformMetadata {
            channel_count: 3,
            ..source_metadata()
        },
        WaveformMetadata {
            channel_labels: vec![" ".into(), "B".into()],
            ..source_metadata()
        },
        WaveformMetadata {
            sample_rate: 0,
            ..source_metadata()
        },
        WaveformMetadata {
            kind: WaveformSessionKind::Replay { total_frames: 0 },
            ..source_metadata()
        },
    ] {
        let error =
            Waveform::set_session_in_world(app.world_mut(), root, Arc::new(Source(metadata)))
                .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.session_generation(), generation);
        assert_eq!(runtime.metadata().unwrap().channel_count, 2);
        assert_eq!(runtime.reduced_channels(), output);
        assert_eq!(
            app.world().get::<WaveformCursor>(root).unwrap().position,
            Duration::from_secs(1)
        );
    }
}

fn source_metadata() -> WaveformMetadata {
    WaveformMetadata {
        channel_count: 2,
        channel_labels: vec!["A".into(), "B".into()],
        sample_rate: 1000,
        unit: WaveformUnit::Volts,
        kind: WaveformSessionKind::Live,
    }
}

#[test]
fn stopping_preserves_snapshot_and_restart_clears_it() {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    Waveform::set_session_in_world(
        app.world_mut(),
        root,
        source(2, 1000, WaveformSessionKind::Live),
    )
    .unwrap();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_millis(10);
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 100;
    app.update();
    let before = app.world().get::<WaveformRuntime>(root).unwrap();
    let output = before.reduced_channels().to_vec();
    let viewport = before.viewport_range();
    let revision = before.revision();
    assert!(Waveform::set_running_in_world(app.world_mut(), root, false).unwrap());
    assert!(!Waveform::set_running_in_world(app.world_mut(), root, false).unwrap());
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(5);
    app.update();
    let stopped = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(stopped.reduced_channels(), output);
    assert_eq!(stopped.viewport_range(), viewport);
    assert_eq!(stopped.revision(), revision);
    Waveform::set_session_in_world(
        app.world_mut(),
        root,
        source(1, 2000, WaveformSessionKind::Replay { total_frames: 6000 }),
    )
    .unwrap();
    let replay = app.world().get::<WaveformRuntime>(root).unwrap();
    assert!(!replay.is_running());
    assert_eq!(
        replay.metadata().unwrap().total_duration().unwrap(),
        Some(Duration::from_secs(3))
    );
    assert!(replay.reduced_channels().is_empty());
    assert!(Waveform::clear_session_in_world(app.world_mut(), root).unwrap());
    assert!(!Waveform::clear_session_in_world(app.world_mut(), root).unwrap());
    assert!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .metadata()
            .is_none()
    );
    assert!(Waveform::set_running_in_world(app.world_mut(), root, true).is_err());
}

#[derive(Resource, Default)]
struct Notices(Vec<(u64, usize, Duration)>);

#[derive(Resource, Default)]
struct RunningNotices(Vec<bool>);

#[test]
fn queued_replacement_notifies_after_commit_and_invalid_targets_propagate() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let mut app = scene_app();
    app.init_resource::<Notices>();
    app.init_resource::<RunningNotices>();
    app.set_error_handler(ErrorCapture::handler());
    app.add_observer(
        |event: On<WaveformRunningChanged>,
         runtimes: Query<&WaveformRuntime>,
         mut notices: ResMut<RunningNotices>| {
            assert_eq!(
                runtimes.get(event.entity).unwrap().is_running(),
                event.running
            );
            notices.0.push(event.running);
        },
    );
    app.add_observer(
        |event: On<WaveformSessionChanged>,
         runtimes: Query<(&WaveformRuntime, &WaveformCursor)>,
         mut notices: ResMut<Notices>| {
            let (runtime, cursor) = runtimes.get(event.entity).unwrap();
            notices
                .0
                .push((event.generation, runtime.channel_count(), cursor.position));
        },
    );
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    Waveform::set_session(
        &mut app.world_mut().commands(),
        root,
        source(2, 1000, WaveformSessionKind::Live),
    );
    assert!(app.world().resource::<Notices>().0.is_empty());
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Notices>().0,
        [(1, 2, Duration::ZERO)]
    );
    let invalid = app.world_mut().spawn_empty().id();
    let errors = ErrorCapture::default();
    errors.run(|| {
        Waveform::set_session(
            &mut app.world_mut().commands(),
            invalid,
            source(1, 1000, WaveformSessionKind::Live),
        );
        app.world_mut().flush();
    });
    assert_eq!(errors.take().len(), 1);
    assert_eq!(app.world().resource::<Notices>().0.len(), 1);
    assert!(Waveform::set_running_in_world(app.world_mut(), root, false).unwrap());
    assert!(!Waveform::set_running_in_world(app.world_mut(), root, false).unwrap());
    assert_eq!(app.world().resource::<RunningNotices>().0, [false]);
    assert!(Waveform::clear_session_in_world(app.world_mut(), root).unwrap());
    assert!(!Waveform::clear_session_in_world(app.world_mut(), root).unwrap());
    assert_eq!(
        app.world().resource::<Notices>().0.last(),
        Some(&(2, 0, Duration::ZERO))
    );
    assert_eq!(app.world().resource::<Notices>().0.len(), 2);
}

#[test]
fn read_failure_is_queryable_on_its_instance() {
    struct Broken;
    impl WaveformSource for Broken {
        fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
            Ok(source_metadata())
        }
        fn read(
            &self,
            _: &WaveformReadRequest,
            _: &mut PlanarBuffer,
        ) -> Result<WaveformReadStatus, WaveformReadError> {
            Err(WaveformReadError("disconnected electrode source".into()))
        }
    }
    let mut runtime = WaveformRuntime::new(WaveformConfig::default(), Arc::new(Broken)).unwrap();
    assert!(runtime.update(Duration::from_secs(1), 100).is_err());
    assert!(
        runtime
            .last_error()
            .unwrap()
            .contains("disconnected electrode source")
    );
}

#[test]
fn each_physical_unit_is_converted_once_and_metadata_is_not_polled() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountingSource(WaveformMetadata, Arc<AtomicUsize>);
    impl WaveformSource for CountingSource {
        fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
            self.1.fetch_add(1, Ordering::Relaxed);
            Ok(self.0.clone())
        }
        fn read(
            &self,
            request: &WaveformReadRequest,
            out: &mut PlanarBuffer,
        ) -> Result<WaveformReadStatus, WaveformReadError> {
            Source(self.0.clone()).read(request, out)
        }
    }
    for (unit, expected) in [
        (WaveformUnit::Volts, 500.0),
        (WaveformUnit::Millivolts, 0.5),
        (WaveformUnit::Microvolts, 0.0005),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = WaveformRuntime::new(
            WaveformConfig::default(),
            Arc::new(CountingSource(
                WaveformMetadata {
                    unit,
                    ..source_metadata()
                },
                calls.clone(),
            )),
        )
        .unwrap();
        for frame in [1, 2, 2000] {
            runtime
                .update(duration_from_frames(frame, 1000).unwrap(), 1000)
                .unwrap();
            assert!(
                (runtime.channel(0).unwrap().get(0).unwrap() - expected).abs() <= expected * 1e-6
            );
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
