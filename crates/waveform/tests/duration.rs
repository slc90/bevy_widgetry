#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_waveform::*;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Source {
    mode: AtomicUsize,
    reads: Mutex<Vec<WaveformReadRequest>>,
}
impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 100,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Live,
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.reads.lock().unwrap().push(request.clone());
        match self.mode.load(Ordering::Relaxed) {
            1 => return Ok(WaveformReadStatus::Pending),
            2 => return Err(WaveformReadError("requested history unavailable".into())),
            _ => (),
        }
        out.channels_mut()[0].extend(request.range.clone().map(|sample| sample as f32));
        if request.range.start <= 550 && request.range.end >= 560 {
            out.missing_mut()[0]
                .push((550 - request.range.start) as usize..(560 - request.range.start) as usize);
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}
fn fixture() -> (App, Entity, Arc<Source>) {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let source = Arc::new(Source::default());
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let root = app.world_mut().spawn_scene(bsn! {
        @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { visible_duration_ms: 10000, ..default() }},
            @time_ranges_ms: {vec![1000, 10000, 30000]} }
    }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 100;
    (app, root, source)
}
fn tick(app: &mut App, root: Entity, seconds: u64) {
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(seconds);
    app.update();
}
#[test]
fn live_grows_then_scrolls_and_hot_duration_keeps_session_time_and_gaps() {
    let (mut app, root, source) = fixture();
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    tick(&mut app, root, 8);
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.viewport_range(), 0..3000);
    assert_eq!(runtime.buffered_range(), 0..800);
    assert_eq!(runtime.channel(0).unwrap().get(550), None);
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 10000).unwrap();
    tick(&mut app, root, 35);
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        2500..3500
    );
    for (duration, expected) in [(30000, 500..3500), (1000, 3400..3500)] {
        Waveform::set_visible_duration_in_world(app.world_mut(), root, duration).unwrap();
        app.update();
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.viewport_range(), expected);
        assert_eq!(runtime.buffered_range(), expected);
        assert_eq!(runtime.session_generation(), 1);
        assert_eq!(runtime.config().amplitude_range, -1.0..=1.0);
        assert_eq!(
            app.world().get::<WaveformCursor>(root).unwrap().position,
            Duration::from_secs(35)
        );
    }
    assert_eq!(
        source.reads.lock().unwrap().last().unwrap().range,
        3400..3500
    );
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert!(
        runtime.working_set_bytes().0 < 5000,
        "no maximum-window raw cache"
    );
    assert_eq!(runtime.stats().read_frames, 100);
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .stats()
            .read_frames,
        0
    );
}
#[test]
fn changed_window_commits_only_ready_and_paused_refresh_uses_frozen_time() {
    let (mut app, root, source) = fixture();
    tick(&mut app, root, 8);
    Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
    let snapshot = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .reduced_channels()
        .to_vec();
    source.mode.store(1, Ordering::Relaxed);
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    tick(&mut app, root, 100);
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.viewport_range(), 0..1000);
    assert_eq!(runtime.reduced_channels(), snapshot);
    assert_eq!(runtime.stats().kind, WaveformUpdateKind::Pending);
    source.mode.store(0, Ordering::Relaxed);
    app.update();
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.viewport_range(), 0..3000);
    assert_eq!(runtime.buffered_range(), 0..800);
    assert!(!runtime.is_running());
    Waveform::set_session_in_world(app.world_mut(), root, source).unwrap();
    assert_eq!(
        app.world().get::<WaveformCursor>(root).unwrap().position,
        Duration::ZERO
    );
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        0..3000
    );
}
#[derive(Resource, Default)]
struct Notices(Vec<u32>);
#[test]
fn queued_and_sync_setters_share_validation_and_notify_only_after_commit() {
    let (mut app, root, _) = fixture();
    app.init_resource::<Notices>();
    app.add_observer(
        |event: On<WaveformDurationChanged>,
         runtimes: Query<&WaveformRuntime>,
         mut notices: ResMut<Notices>| {
            assert_eq!(
                runtimes
                    .get(event.entity)
                    .unwrap()
                    .config()
                    .visible_duration_ms,
                event.visible_duration_ms
            );
            notices.0.push(event.visible_duration_ms);
        },
    );
    for invalid in [0, 5000] {
        assert!(Waveform::set_visible_duration_in_world(app.world_mut(), root, invalid).is_err());
    }
    let other = app.world_mut().spawn_empty().id();
    assert!(Waveform::set_visible_duration_in_world(app.world_mut(), other, 1000).is_err());
    Waveform::set_visible_duration(&mut app.world_mut().commands(), root, 30000);
    assert!(app.world().resource::<Notices>().0.is_empty());
    app.world_mut().flush();
    assert!(!Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap());
    assert_eq!(app.world().resource::<Notices>().0, [30000]);
}

#[test]
fn unsupported_history_is_an_error_and_cannot_replace_the_snapshot() {
    let (mut app, root, source) = fixture();
    tick(&mut app, root, 35);
    let old = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .reduced_channels()
        .to_vec();
    source.mode.store(2, Ordering::Relaxed);
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    let mut runtime = app.world_mut().get_mut::<WaveformRuntime>(root).unwrap();
    assert!(runtime.update(Duration::from_secs(35), 100).is_err());
    assert_eq!(runtime.viewport_range(), 2500..3500);
    assert_eq!(runtime.reduced_channels(), old);
    assert!(
        runtime
            .last_error()
            .unwrap()
            .contains("history unavailable")
    );
    source.mode.store(0, Ordering::Relaxed);
    runtime.update(Duration::from_secs(35), 100).unwrap();
    assert_eq!(runtime.viewport_range(), 500..3500);
    assert_eq!(runtime.channel(0).unwrap().get(50), None);
    assert!(runtime.last_error().is_none());
}

#[test]
fn paused_empty_replay_changes_window_without_reading_or_starting_playback() {
    struct Replay;
    impl WaveformSource for Replay {
        fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
            Ok(WaveformMetadata {
                kind: WaveformSessionKind::Replay { total_frames: 5000 },
                ..Source::default().metadata()?
            })
        }
        fn read(
            &self,
            _: &WaveformReadRequest,
            _: &mut PlanarBuffer,
        ) -> Result<WaveformReadStatus, WaveformReadError> {
            Err(WaveformReadError(
                "empty paused replay must not read".into(),
            ))
        }
    }
    let (mut app, root, _) = fixture();
    Waveform::set_session_in_world(app.world_mut(), root, Arc::new(Replay)).unwrap();
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    app.update();
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert!(!runtime.is_running());
    assert_eq!(runtime.viewport_range(), 0..3000);
    assert_eq!(runtime.buffered_range(), 0..0);
    assert!(runtime.last_error().is_none());
}

#[test]
fn construction_rejects_ambiguous_choices_and_no_selector_allows_programmatic_ranges() {
    let mut app = scene_app();
    for choices in [vec![0, 1000], vec![1000, 1000], vec![5000]] {
        assert!(
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! {
                    @Waveform { @time_ranges_ms: {choices} }
                }
            )
            .is_err()
        );
    }
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform })
        .unwrap()
        .id();
    assert!(Waveform::set_visible_duration_in_world(app.world_mut(), root, 7000).unwrap());
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .config()
            .visible_duration_ms,
        7000
    );
}

#[test]
fn cancelled_large_pending_window_cannot_donate_its_allocation_to_a_short_window() {
    let (mut app, root, source) = fixture();
    tick(&mut app, root, 35);
    source.mode.store(1, Ordering::Relaxed);
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .stats()
            .kind,
        WaveformUpdateKind::Pending
    );
    assert!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .working_set_bytes()
            .1
            >= 12000
    );
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 1000).unwrap();
    source.mode.store(0, Ordering::Relaxed);
    app.update();
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.viewport_range(), 3400..3500);
    assert_eq!(runtime.channel(0).unwrap().get(0), Some(3400.0));
    let (raw, staging, _) = runtime.working_set_bytes();
    assert!(
        raw + staging < 1000,
        "cancelled work leaked into the small-window cache: {raw} + {staging}"
    );
}
