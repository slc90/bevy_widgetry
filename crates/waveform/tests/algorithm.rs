#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_waveform::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

struct Source {
    pending: AtomicBool,
    requests: Mutex<Vec<WaveformReadRequest>>,
    kind: WaveformSessionKind,
}

impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 2,
            channel_labels: vec!["A".into(), "B".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Microvolts,
            kind: self.kind,
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.requests.lock().unwrap().push(request.clone());
        if self.pending.load(Ordering::Relaxed) {
            return Ok(WaveformReadStatus::Pending);
        }
        for channel in out.channels_mut() {
            channel.extend(
                request
                    .range
                    .clone()
                    .map(|i| if i % 37 == 0 { 100.0 } else { (i % 17) as f32 }),
            );
        }
        for frame in (request.range.start..request.range.end).filter(|i| (400..450).contains(i)) {
            let relative = (frame - request.range.start) as usize;
            out.missing_mut()[0].push(relative..relative + 1);
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

#[derive(Resource, Default)]
struct Changes(Vec<WaveformAlgorithm>);

fn settle(runtime: &mut WaveformRuntime, position: Duration, width: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if runtime.update(position, width).unwrap().kind != WaveformUpdateKind::Pending {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn paused_background_rebuild_requests_frames_until_the_snapshot_is_committed() {
    let mut app = scene_app();
    app.add_message::<bevy::window::RequestRedraw>();
    app.add_plugins(WaveformPlugin);
    let source = Arc::new(Source {
        pending: AtomicBool::new(false),
        requests: Mutex::default(),
        kind: WaveformSessionKind::Replay {
            total_frames: 500_000,
        },
    });
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @source: {Some(source as Arc<dyn WaveformSource>)}, @config: {WaveformConfig { visible_duration_ms: 200_000, ..default() }} } }).unwrap().id();
    Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
    settle(
        &mut app.world_mut().get_mut::<WaveformRuntime>(root).unwrap(),
        Duration::from_secs(200),
        400,
    );
    Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb).unwrap();
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 400;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        app.world_mut()
            .resource_mut::<Messages<bevy::window::RequestRedraw>>()
            .clear();
        app.update();
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert!(!runtime.is_running());
        assert_eq!(runtime.buffered_range(), 0..200_000);
        if runtime.stats().kind != WaveformUpdateKind::Pending {
            assert!(matches!(
                runtime.reduced_channels()[0],
                ReducedChannel::Polyline(_)
            ));
            assert!(
                app.world()
                    .resource::<Messages<bevy::window::RequestRedraw>>()
                    .is_empty()
            );
            break;
        }
        assert!(
            !app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn background_rebuilds_commit_a_complete_snapshot_and_discard_obsolete_algorithms() {
    let mut app = scene_app();
    let source = Arc::new(Source {
        pending: AtomicBool::new(false),
        requests: Mutex::default(),
        kind: WaveformSessionKind::Live,
    });
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @source: {Some(source as Arc<dyn WaveformSource>)}, @config: {WaveformConfig { visible_duration_ms: 200_000, ..default() }} } }).unwrap().id();
    let position = Duration::from_secs(200);
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    assert_eq!(
        runtime.update(position, 200).unwrap().kind,
        WaveformUpdateKind::Pending
    );
    assert!(runtime.reduced_channels().is_empty());
    assert_eq!(runtime.buffered_range(), 0..0);
    settle(runtime, position, 200);
    let snapshot = runtime.reduced_channels().to_vec();
    let revision = runtime.revision();
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb).unwrap();
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    assert_eq!(
        runtime.update(position, 200).unwrap().kind,
        WaveformUpdateKind::Pending
    );
    assert_eq!(runtime.reduced_channels(), snapshot);
    assert_eq!(runtime.revision(), revision);
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::MinMax).unwrap();
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    settle(runtime, position, 400);
    assert!(matches!(
        runtime.reduced_channels()[0],
        ReducedChannel::Envelope(_)
    ));
    assert_eq!(runtime.viewport_range(), 0..200_000);
    assert_eq!(runtime.buffered_range(), 0..200_000);
    Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb).unwrap();
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    settle(runtime, Duration::from_secs(300), 400);
    assert!(matches!(
        runtime.reduced_channels()[0],
        ReducedChannel::Polyline(_)
    ));
    assert!(!runtime.is_running());
    assert_eq!(runtime.viewport_range(), 0..200_000);
    Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::MinMax).unwrap();
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    settle(runtime, Duration::from_secs(201), 300);
    assert_eq!(runtime.buffered_range(), 1000..201_000);
    assert!(matches!(
        runtime.reduced_channels()[0],
        ReducedChannel::Envelope(_)
    ));
    Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb).unwrap();
    let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    settle(runtime, Duration::from_secs(199), 300);
    assert_eq!(runtime.buffered_range(), 0..199_000);
    assert!(matches!(
        runtime.reduced_channels()[0],
        ReducedChannel::Polyline(_)
    ));
}

#[test]
fn algorithm_switch_preserves_session_and_snapshot_until_new_data_is_ready() {
    for kind in [
        WaveformSessionKind::Live,
        WaveformSessionKind::Replay { total_frames: 5000 },
    ] {
        let source = Arc::new(Source {
            pending: AtomicBool::new(false),
            requests: Mutex::default(),
            kind,
        });
        let mut app = scene_app();
        app.init_resource::<Changes>();
        app.add_observer(
            |event: On<WaveformAlgorithmChanged>,
             runtime: Query<&WaveformRuntime>,
             mut changes: ResMut<Changes>| {
                assert_eq!(
                    runtime.get(event.entity).unwrap().algorithm(),
                    event.algorithm
                );
                changes.0.push(event.algorithm);
            },
        );
        let root = app
            .world_mut()
            .spawn_scene(
                bsn! { @Waveform { @source: {Some(source.clone() as Arc<dyn WaveformSource>)} } },
            )
            .unwrap()
            .id();
        let other = app
            .world_mut()
            .spawn_scene(bsn! { @Waveform })
            .unwrap()
            .id();
        Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
        let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
        runtime.update(Duration::from_secs(1), 20).unwrap();
        let old = runtime.reduced_channels().to_vec();
        let generation = runtime.session_generation();
        source.pending.store(true, Ordering::Relaxed);
        runtime.update(Duration::from_secs(2), 20).unwrap();
        let pending = source.requests.lock().unwrap().last().unwrap().clone();
        assert!(
            Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb)
                .unwrap()
        );
        assert!(
            !Waveform::set_algorithm_in_world(app.world_mut(), root, WaveformAlgorithm::Lttb)
                .unwrap()
        );
        let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.reduced_channels(), old);
        assert_eq!(
            runtime.update(Duration::from_secs(2), 20).unwrap().kind,
            WaveformUpdateKind::Pending
        );
        let new = source.requests.lock().unwrap().last().unwrap().clone();
        assert_ne!(pending.id, new.id);
        assert_eq!(new.algorithm, WaveformAlgorithm::Lttb);
        source.pending.store(false, Ordering::Relaxed);
        runtime.update(Duration::from_secs(2), 20).unwrap();
        assert_eq!(runtime.session_generation(), generation);
        assert!(runtime.is_running());
        assert_eq!(runtime.viewport_range(), 1000..2000);
        assert!(
            runtime
                .reduced_channels()
                .iter()
                .all(|channel| matches!(channel, ReducedChannel::Polyline(_)))
        );
        assert!(runtime.channel(0).unwrap().get(0).unwrap().abs() <= 0.0001);
        assert_eq!(
            app.world().resource::<Changes>().0,
            [WaveformAlgorithm::Lttb]
        );
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(other)
                .unwrap()
                .algorithm(),
            WaveformAlgorithm::MinMax
        );
        Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
        Waveform::set_algorithm(
            &mut app.world_mut().commands(),
            root,
            WaveformAlgorithm::MinMax,
        );
        app.world_mut().flush();
        let runtime = &mut *app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
        runtime.update(Duration::from_secs(5), 20).unwrap();
        assert!(!runtime.is_running());
        assert_eq!(runtime.buffered_range(), 1000..2000);
        assert!(matches!(
            runtime.reduced_channels()[0],
            ReducedChannel::Envelope(_)
        ));
    }
}
