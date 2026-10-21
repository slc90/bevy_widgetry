#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::{ErrorCapture, scene_app};
use bevy_widgetry_waveform::*;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};

#[derive(Resource, Default)]
struct RunningChanges(Vec<bool>);

struct Source {
    mode: AtomicU8,
    requests: Mutex<Vec<WaveformReadRequest>>,
    saved: Mutex<Option<WaveformReadRequest>>,
    kind: WaveformSessionKind,
}

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Volts,
            kind: self.kind,
        })
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.requests.lock().unwrap().push(request.clone());
        let mode = self.mode.load(Ordering::Relaxed);
        if mode == 1 {
            return Ok(WaveformReadStatus::Pending);
        }
        if mode == 3 {
            return Err(WaveformReadError("record unreadable".into()));
        }
        out.channels_mut()[0].extend(request.range.clone().map(|frame| frame as f32));
        if mode == 2 {
            return Ok(WaveformReadStatus::Ready(
                self.saved.lock().unwrap().clone().unwrap(),
            ));
        }
        *self.saved.lock().unwrap() = Some(request.clone());
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

#[test]
fn pending_and_stale_results_leave_the_committed_snapshot_unchanged() {
    let source = Arc::new(Source {
        mode: AtomicU8::new(0),
        requests: Mutex::default(),
        saved: Mutex::new(None),
        kind: WaveformSessionKind::Live,
    });
    let mut runtime = WaveformRuntime::new(WaveformConfig::default(), source.clone()).unwrap();
    runtime.update(Duration::from_secs(1), 100).unwrap();
    let output = runtime.reduced_channels().to_vec();
    let viewport = runtime.viewport_range();
    let revision = runtime.revision();
    for mode in [1, 2] {
        source.mode.store(mode, Ordering::Relaxed);
        let result = runtime.update(Duration::from_secs(4), 100).unwrap();
        assert_eq!(result.kind, WaveformUpdateKind::Pending);
        assert_eq!(runtime.reduced_channels(), output);
        assert_eq!(runtime.viewport_range(), viewport);
        assert_eq!(runtime.revision(), revision);
        assert!(runtime.last_error().is_none());
    }
    source.mode.store(0, Ordering::Relaxed);
    runtime.update(Duration::from_secs(5), 100).unwrap();
    assert_eq!(runtime.buffered_range(), 4000..5000);
    assert_eq!(runtime.viewport_range(), 4000..5000);
    let requests = source.requests.lock().unwrap();
    assert_eq!(requests[1], requests[2]);
    assert_ne!(requests[2].id, requests[3].id);
    drop(requests);
    source.mode.store(2, Ordering::Relaxed);
    let mut other = WaveformRuntime::new(WaveformConfig::default(), source.clone()).unwrap();
    assert_eq!(other.session_generation(), runtime.session_generation());
    assert_eq!(
        other.update(Duration::from_secs(5), 100).unwrap().kind,
        WaveformUpdateKind::Pending
    );
    assert_eq!(other.buffered_range(), 0..0);
    let requests = source.requests.lock().unwrap();
    assert_ne!(requests[0].reader_id, requests.last().unwrap().reader_id);
}

#[test]
fn replay_read_failure_stops_without_retries_and_pending_does_not_stop() {
    let errors = ErrorCapture::default();
    errors.run(|| {
        let mut app = scene_app();
        app.init_resource::<RunningChanges>();
        app.add_observer(
            |event: On<WaveformRunningChanged>,
             runtime: Query<&WaveformRuntime>,
             mut changes: ResMut<RunningChanges>| {
                assert_eq!(
                    runtime.get(event.entity).unwrap().is_running(),
                    event.running
                );
                changes.0.push(event.running);
            },
        );
        app.add_plugins(WaveformPlugin);
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.set_error_handler(ErrorCapture::handler());
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @Waveform })
            .unwrap()
            .id();
        let source = Arc::new(Source {
            mode: AtomicU8::new(1),
            requests: Mutex::default(),
            saved: Mutex::new(None),
            kind: WaveformSessionKind::Replay { total_frames: 5000 },
        });
        Waveform::set_session_in_world(app.world_mut(), root, source.clone()).unwrap();
        Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
        app.world_mut()
            .get_mut::<WaveformCursor>(root)
            .unwrap()
            .position = Duration::from_secs(1);
        app.update();
        assert!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .is_running()
        );
        source.mode.store(3, Ordering::Relaxed);
        app.update();
        assert!(
            !app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .is_running()
        );
        let calls = source.requests.lock().unwrap().len();
        app.update();
        assert_eq!(source.requests.lock().unwrap().len(), calls);
        assert_eq!(app.world().resource::<RunningChanges>().0, [true, false]);
        assert!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .last_error()
                .unwrap()
                .contains("record unreadable")
        );
        source.mode.store(0, Ordering::Relaxed);
        Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
        app.update();
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .buffered_range(),
            0..1000
        );
        assert!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .last_error()
                .is_none()
        );
    });
    assert_eq!(errors.take().len(), 1);
}

#[test]
fn replacement_and_density_changes_invalidate_pending_request_identity() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    let source = Arc::new(Source {
        mode: AtomicU8::new(0),
        requests: Mutex::default(),
        saved: Mutex::default(),
        kind: WaveformSessionKind::Live,
    });
    Waveform::set_session_in_world(app.world_mut(), root, source.clone()).unwrap();
    app.world_mut()
        .get_mut::<WaveformRuntime>(root)
        .unwrap()
        .update(Duration::from_secs(1), 100)
        .unwrap();
    let old = source.saved.lock().unwrap().clone().unwrap();
    Waveform::set_session_in_world(app.world_mut(), root, source.clone()).unwrap();
    source.mode.store(2, Ordering::Relaxed);
    for width in [100, 200] {
        let mut runtime = app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
        assert_eq!(
            runtime.update(Duration::from_secs(1), width).unwrap().kind,
            WaveformUpdateKind::Pending
        );
        assert_eq!(runtime.buffered_range(), 0..0);
        assert!(runtime.reduced_channels().is_empty());
    }
    let requests = source.requests.lock().unwrap();
    assert_ne!(old.session_generation, requests[1].session_generation);
    assert_ne!(requests[1].id, requests[2].id);
}

#[test]
fn advancing_clock_can_commit_pending_work_without_waiting_for_the_clock_to_stop() {
    struct Deferred(Mutex<Option<WaveformReadRequest>>);
    impl WaveformSource for Deferred {
        fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
            Ok(WaveformMetadata {
                channel_count: 1,
                channel_labels: vec!["A".into()],
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
            let mut pending = self.0.lock().unwrap();
            if let Some(completed) = pending.take() {
                out.channels_mut()[0].extend(completed.range.clone().map(|frame| frame as f32));
                Ok(WaveformReadStatus::Ready(completed))
            } else {
                *pending = Some(request.clone());
                Ok(WaveformReadStatus::Pending)
            }
        }
    }
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let source: Arc<dyn WaveformSource> = Arc::new(Deferred(Mutex::default()));
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform { @source: {Some(source)} } })
        .unwrap()
        .id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(1);
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .stats()
            .kind,
        WaveformUpdateKind::Pending
    );
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_millis(1100);
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .buffered_range(),
        0..1000
    );
    assert_eq!(
        app.world().get::<WaveformCursor>(root).unwrap().position,
        Duration::from_millis(1100)
    );
    for millis in (1200..=1500).step_by(100) {
        app.world_mut()
            .get_mut::<WaveformCursor>(root)
            .unwrap()
            .position = Duration::from_millis(millis);
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .buffered_range(),
        400..1400
    );
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_millis(1600);
    app.update();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .seek(Duration::from_millis(1610));
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .buffered_range(),
        400..1400
    );
}
