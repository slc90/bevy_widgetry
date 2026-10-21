#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_waveform::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::Duration;

struct Source;
impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Replay {
                total_frames: 100_000,
            },
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        for channel in out.channels_mut() {
            channel.extend(request.range.clone().map(|frame| (frame % 17) as f32));
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

#[test]
fn playback_commits_external_state_and_refreshes_paused_positions() {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @source: {Some(Arc::new(Source) as Arc<dyn WaveformSource>)}, @config: {WaveformConfig { visible_duration_ms: 10_000, ..default() }} } }).unwrap().id();
    let initial = Waveform::playback(app.world(), root).unwrap().unwrap();
    assert_eq!(initial.position, Duration::ZERO);
    assert!(!initial.playing);
    assert_eq!(initial.speed, 1.0);
    app.world_mut()
        .add_observer(move |event: On<WaveformPlaybackChanged>, world: &World| {
            assert_eq!(
                Waveform::playback(world, event.entity).unwrap(),
                Some(event.playback)
            );
        });
    let mut playback = initial;
    playback.position = Duration::from_secs(15);
    playback.speed = 2.0;
    assert!(Waveform::set_playback_in_world(app.world_mut(), root, playback).unwrap());
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 20;
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        5000..15_000
    );
    assert!(
        !app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .is_running()
    );
    assert!(!Waveform::set_playback_in_world(app.world_mut(), root, playback).unwrap());
    playback.playing = true;
    assert!(Waveform::set_playback_in_world(app.world_mut(), root, playback).unwrap());
    app.update();
    assert!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .is_running()
    );
    Waveform::set_session_in_world(app.world_mut(), root, Arc::new(Source)).unwrap();
    let reset = Waveform::playback(app.world(), root).unwrap().unwrap();
    assert_eq!(reset.position, Duration::ZERO);
    assert!(!reset.playing);
    assert_eq!(reset.speed, 2.0);
    assert!(Waveform::set_playback_in_world(app.world_mut(), root, playback).is_err());
}

#[test]
fn invalid_and_stale_playback_updates_leave_committed_state_unchanged() {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @Waveform { @source: {Some(Arc::new(Source) as Arc<dyn WaveformSource>)} } },
        )
        .unwrap()
        .id();
    let original = Waveform::playback(app.world(), root).unwrap().unwrap();
    for state in [
        WaveformPlayback {
            speed: f64::NAN,
            ..original
        },
        WaveformPlayback {
            speed: 3.0,
            ..original
        },
        WaveformPlayback {
            position: Duration::from_secs(101),
            ..original
        },
        WaveformPlayback {
            position: Duration::from_secs(100),
            playing: true,
            ..original
        },
        WaveformPlayback {
            session_generation: original.session_generation + 1,
            ..original
        },
    ] {
        let error = Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            Waveform::playback(app.world(), root).unwrap(),
            Some(original)
        );
    }
    let mut end = original;
    end.position = Duration::from_secs(100);
    Waveform::set_playback(&mut app.world_mut().commands(), root, end);
    assert_eq!(
        Waveform::playback(app.world(), root).unwrap(),
        Some(original)
    );
    app.world_mut().flush();
    assert_eq!(Waveform::playback(app.world(), root).unwrap(), Some(end));
    let mut restart = end;
    restart.position = Duration::ZERO;
    restart.playing = true;
    Waveform::set_playback_in_world(app.world_mut(), root, restart).unwrap();
    assert_eq!(
        Waveform::playback(app.world(), root).unwrap(),
        Some(restart)
    );
    for speeds in [
        vec![],
        vec![0.5],
        vec![1.0, 1.0],
        vec![1.0, -1.0],
        vec![1.0, f64::INFINITY],
    ] {
        assert!(
            app.world_mut()
                .spawn_scene(bsn! { @Waveform { @replay_speeds: {speeds.clone()} } })
                .is_err()
        );
    }
}

struct PreparedSource {
    mode: AtomicU8,
    calls: AtomicU64,
}
impl WaveformSource for PreparedSource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Source.metadata()
    }
    fn provides_reduced(&self) -> bool {
        true
    }
    fn read(
        &self,
        _: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(WaveformReadError("raw forbidden".into()))
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        match self.mode.load(Ordering::Relaxed) {
            1 => Ok(WaveformReducedReadStatus::Pending),
            2 => Err(WaveformReadError("read failed".into())),
            _ => Ok(WaveformReducedReadStatus::Ready {
                request: request.clone(),
                channels: vec![ReducedChannel::Envelope(vec![WaveformSpan {
                    samples: RangeBoundary {
                        start: request.range.start,
                        end: request.range.end,
                    },
                    min: -0.1,
                    max: 0.2,
                }])],
            }),
        }
    }
}

#[test]
fn pending_clock_updates_keep_snapshots_and_real_errors_stop_playback_without_retry() {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let source = Arc::new(PreparedSource {
        mode: AtomicU8::new(0),
        calls: AtomicU64::new(0),
    });
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @source: {Some(source.clone() as Arc<dyn WaveformSource>)}, @config: {WaveformConfig { visible_duration_ms: 10_000, ..default() }} } }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 20;
    let mut state = Waveform::playback(app.world(), root).unwrap().unwrap();
    state.playing = true;
    state.position = Duration::from_secs(6);
    Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        0..10_000
    );
    source.mode.store(1, Ordering::Relaxed);
    for seconds in [15, 16] {
        state.position = Duration::from_secs(seconds);
        Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
        app.update();
        assert!(
            Waveform::playback(app.world(), root)
                .unwrap()
                .unwrap()
                .playing
        );
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .viewport_range(),
            0..10_000
        );
    }
    source.mode.store(0, Ordering::Relaxed);
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        6000..16_000
    );
    source.mode.store(2, Ordering::Relaxed);
    state.position = Duration::from_secs(20);
    Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
    app.update();
    assert!(
        !Waveform::playback(app.world(), root)
            .unwrap()
            .unwrap()
            .playing
    );
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        6000..16_000
    );
    let calls = source.calls.load(Ordering::Relaxed);
    app.update();
    assert_eq!(source.calls.load(Ordering::Relaxed), calls);
    source.mode.store(0, Ordering::Relaxed);
    Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
    app.update();
    state.playing = false;
    Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
    app.update();
    state.playing = true;
    state.position = Duration::from_secs(21);
    Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        11_000..21_000
    );
}
