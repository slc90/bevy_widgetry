#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};
use bevy_widgetry_waveform::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Source(WaveformSessionKind, AtomicUsize, AtomicBool);
impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 8,
            channel_labels: (0..8).map(|i| format!("Sensor{i}")).collect(),
            sample_rate: 100,
            unit: WaveformUnit::Volts,
            kind: self.0,
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        if self.2.load(Ordering::Relaxed) {
            return Ok(WaveformReadStatus::Pending);
        }
        for channel in out.channels_mut() {
            channel.resize((request.range.end - request.range.start) as usize, 0.0);
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}
#[derive(Resource, Default)]
struct Changes(Vec<Option<WaveformMeasurement>>);
fn update(app: &mut App) {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}
struct Fixture {
    app: App,
    root: Entity,
    surface: Entity,
    window: Entity,
    source: Arc<Source>,
    at: Location,
    id: PointerId,
    origin: Vec2,
}
fn fixture(kind: WaveformSessionKind, id: PointerId) -> Fixture {
    let mut app = picking_app();
    app.add_plugins((TransformPlugin, WaveformRenderPlugin))
        .init_resource::<Changes>();
    app.set_error_handler(ErrorCapture::handler());
    app.add_observer(
        |event: On<WaveformMeasurementChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                Waveform::measurement(&world, event.entity).unwrap(),
                event.measurement
            );
            world.resource_mut::<Changes>().0.push(event.measurement);
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1000, 800).into(),
                focused: true,
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::new(1000, 800), 1.0);
    let source = Arc::new(Source(kind, AtomicUsize::new(0), AtomicBool::new(false)));
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let root=app.world_mut().spawn_scene(bsn!{
        Name("MeasurementView")
        @Waveform { @source: {Some(adapter)}, @channels_per_screen: {WaveformChannelCount::Count(4)}, @time_ranges_ms: {vec![1000, 2000]} }
        template(move |_|Ok(UiTargetCamera(camera)))
        Node{position_type:PositionType::Absolute,left:px(20),top:px(20),width:px(600),height:px(372)}
    }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(1);
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    for _ in 0..4 {
        update(&mut app);
    }
    let surface = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "WaveformMeasurementSurface")
        .unwrap()
        .0;
    let node = app.world().get::<ComputedNode>(surface).unwrap();
    let transform = app.world().get::<UiGlobalTransform>(surface).unwrap();
    let origin = transform.translation - node.size() * 0.5;
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: origin,
    };
    assert_eq!(
        app.world().get::<Name>(root).unwrap().as_str(),
        "MeasurementView"
    );
    Fixture {
        app,
        root,
        surface,
        window,
        source,
        at,
        id,
        origin,
    }
}
impl Fixture {
    fn input(&mut self, action: PointerAction) {
        queue_pointer(&mut self.app, self.id, self.at.clone(), action);
        update(&mut self.app);
    }
    fn moved(&mut self, point: Vec2) {
        let target = self.origin + point;
        let delta = target - self.at.position;
        self.at.position = target;
        self.input(PointerAction::Move { delta });
    }
    fn press(&mut self) {
        self.input(PointerAction::Press(PointerButton::Primary));
        assert_eq!(
            self.app
                .world()
                .resource::<bevy::input_focus::InputFocus>()
                .get(),
            Some(self.surface)
        );
    }
    fn release(&mut self) {
        self.input(PointerAction::Release(PointerButton::Primary));
    }
    fn draw(&mut self, a: Vec2, b: Vec2) {
        self.moved(a);
        self.press();
        self.moved(b);
        self.release();
    }
    fn measurement(&self) -> WaveformMeasurement {
        Waveform::measurement(self.app.world(), self.root)
            .unwrap()
            .unwrap()
    }
}

#[test]
fn measurement_toolbar_keeps_range_popup_visible_and_pickable() {
    let mut f = fixture(WaveformSessionKind::Live, pointer_ids()[1]);
    Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true).unwrap();
    update(&mut f.app);
    let combo = f
        .app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(f.app.world())
        .find(|(_, name)| name.as_str() == "WaveformTimeRange")
        .unwrap()
        .0;
    let center = f
        .app
        .world()
        .get::<UiGlobalTransform>(combo)
        .unwrap()
        .translation;
    f.moved(center - f.origin);
    f.input(PointerAction::Press(PointerButton::Primary));
    f.release();
    for _ in 0..4 {
        update(&mut f.app);
    }
    let option = f
        .app
        .world_mut()
        .query::<(Entity, &Text, &UiGlobalTransform)>()
        .iter(f.app.world())
        .find(|(_, text, _)| text.0 == "2 s")
        .map(|(entity, _, transform)| (entity, transform.translation))
        .unwrap();
    f.moved(option.1 - f.origin);
    f.input(PointerAction::Press(PointerButton::Primary));
    f.release();
    for _ in 0..3 {
        update(&mut f.app);
    }
    assert_eq!(
        f.app
            .world()
            .get::<WaveformRuntime>(f.root)
            .unwrap()
            .config()
            .visible_duration_ms,
        2000
    );
    assert!(Waveform::measurement_enabled(f.app.world(), f.root).unwrap());
    assert!(
        Waveform::measurement(f.app.world(), f.root)
            .unwrap()
            .is_none()
    );
}

#[test]
fn actual_picking_draws_moves_resizes_and_retains_one_rectangle_without_source_reads() {
    for id in pointer_ids() {
        let mut f = fixture(WaveformSessionKind::Live, id);
        Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true).unwrap();
        update(&mut f.app);
        let reads = f.source.1.load(Ordering::Relaxed);
        f.draw(Vec2::new(40.0, 30.0), Vec2::new(240.0, 180.0));
        let first = f.measurement();
        assert_eq!(
            first.rectangle,
            Rect::from_corners(Vec2::new(40.0, 30.0), Vec2::new(240.0, 180.0))
        );
        assert!((first.delta_seconds - 200.0 / 468.0).abs() < 1e-6);
        assert_eq!(first.delta_volts, 4.0);
        assert!(
            !f.app
                .world()
                .resource::<PointerCaptureMap>()
                .is_captured(&id)
        );
        f.moved(Vec2::new(100.0, 80.0));
        f.press();
        f.moved(Vec2::new(120.0, 100.0));
        f.release();
        let moved = f.measurement();
        assert_eq!(moved.rectangle.min, Vec2::new(60.0, 50.0));
        assert_eq!(moved.delta_seconds, first.delta_seconds);
        assert_eq!(moved.delta_volts, first.delta_volts);
        f.moved(Vec2::new(260.0, 200.0));
        f.press();
        f.moved(Vec2::new(300.0, 220.0));
        f.release();
        assert_eq!(f.measurement().rectangle.max, Vec2::new(300.0, 220.0));
        f.draw(Vec2::new(350.0, 40.0), Vec2::new(351.0, 290.0));
        assert_eq!(f.measurement().rectangle.width(), 1.0);
        assert_eq!(f.source.1.load(Ordering::Relaxed), reads);
        assert!(
            f.app
                .world()
                .get::<WaveformRuntime>(f.root)
                .unwrap()
                .is_running()
        );
        let last = f
            .app
            .world()
            .resource::<Changes>()
            .0
            .last()
            .unwrap()
            .unwrap();
        assert_eq!(last, f.measurement());
    }
}

#[test]
fn committed_scales_density_resize_live_and_replay_clock_do_not_bind_the_rectangle_to_samples() {
    for kind in [
        WaveformSessionKind::Live,
        WaveformSessionKind::Replay {
            total_frames: 10000,
        },
    ] {
        let mut f = fixture(kind, pointer_ids()[1]);
        Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true).unwrap();
        update(&mut f.app);
        f.draw(Vec2::new(100.0, 30.0), Vec2::new(300.0, 180.0));
        let original = f.measurement();
        Waveform::set_running_in_world(f.app.world_mut(), f.root, true).unwrap();
        f.app
            .world_mut()
            .get_mut::<WaveformCursor>(f.root)
            .unwrap()
            .position = Duration::from_secs(2);
        update(&mut f.app);
        assert_eq!(f.measurement(), original);
        Waveform::set_running_in_world(f.app.world_mut(), f.root, false).unwrap();
        update(&mut f.app);
        assert_eq!(f.measurement(), original);
        f.source.2.store(true, Ordering::Relaxed);
        Waveform::set_visible_duration_in_world(f.app.world_mut(), f.root, 2000).unwrap();
        update(&mut f.app);
        assert_eq!(f.measurement().delta_seconds, original.delta_seconds);
        f.source.2.store(false, Ordering::Relaxed);
        update(&mut f.app);
        assert!((f.measurement().delta_seconds - original.delta_seconds * 2.0).abs() < 1e-6);
        Waveform::set_amplitude_in_world(
            f.app.world_mut(),
            f.root,
            WaveformAmplitude {
                value: 500.0,
                unit: WaveformUnit::Millivolts,
            },
        )
        .unwrap();
        update(&mut f.app);
        assert_eq!(f.measurement().delta_volts, original.delta_volts * 0.5);
        assert_eq!(f.measurement().display_unit, WaveformUnit::Millivolts);
        Waveform::set_channels_per_screen_in_world(
            f.app.world_mut(),
            f.root,
            WaveformChannelCount::Count(8),
        )
        .unwrap();
        for _ in 0..3 {
            update(&mut f.app);
        }
        assert_eq!(f.measurement().delta_volts, original.delta_volts);
        assert_eq!(f.measurement().rectangle, original.rectangle);
        f.app.world_mut().get_mut::<Node>(f.root).unwrap().width = px(282);
        for _ in 0..3 {
            update(&mut f.app);
        }
        assert!(f.measurement().rectangle.max.x <= 150.0);
        assert!(
            !f.app
                .world()
                .get::<WaveformRuntime>(f.root)
                .unwrap()
                .is_running()
        );
    }
}

#[test]
fn closing_reopening_session_changes_and_input_cancellation_release_capture() {
    for reason in 0..8 {
        let mut f = fixture(WaveformSessionKind::Live, pointer_ids()[1]);
        Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true).unwrap();
        update(&mut f.app);
        f.moved(Vec2::new(30.0, 30.0));
        f.press();
        f.moved(Vec2::new(100.0, 100.0));
        assert!(
            f.app
                .world()
                .resource::<PointerCaptureMap>()
                .is_captured(&f.id)
        );
        match reason {
            0 => {
                f.input(PointerAction::Cancel);
            }
            1 => {
                f.app
                    .world_mut()
                    .get_mut::<Window>(f.window)
                    .unwrap()
                    .focused = false;
                update(&mut f.app);
            }
            2 => {
                f.app
                    .world_mut()
                    .entity_mut(f.root)
                    .insert(Visibility::Hidden);
                update(&mut f.app);
            }
            3 => {
                Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, false)
                    .unwrap();
                assert!(
                    Waveform::measurement(f.app.world(), f.root)
                        .unwrap()
                        .is_none()
                );
                Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true)
                    .unwrap();
                assert!(
                    Waveform::measurement(f.app.world(), f.root)
                        .unwrap()
                        .is_none()
                );
            }
            4 => {
                let source: Arc<dyn WaveformSource> = f.source.clone();
                Waveform::set_session_in_world(f.app.world_mut(), f.root, source).unwrap();
                assert!(Waveform::measurement_enabled(f.app.world(), f.root).unwrap());
                assert!(
                    Waveform::measurement(f.app.world(), f.root)
                        .unwrap()
                        .is_none()
                );
            }
            5 => {
                f.app.world_mut().despawn(f.root);
            }
            6 => {
                f.app
                    .world_mut()
                    .resource_mut::<bevy::input_focus::InputFocus>()
                    .set(f.window, bevy::input_focus::FocusCause::Navigated);
                update(&mut f.app);
            }
            _ => {
                f.app
                    .world_mut()
                    .entity_mut(f.root)
                    .insert(bevy::ui::InteractionDisabled);
                update(&mut f.app);
            }
        }
        assert!(
            !f.app
                .world()
                .resource::<PointerCaptureMap>()
                .is_captured(&f.id)
        );
    }
}

#[test]
fn ui_scale_keeps_plot_local_units_and_queued_clear_is_silent_when_empty() {
    let mut f = fixture(WaveformSessionKind::Live, pointer_ids()[1]);
    f.app.world_mut().resource_mut::<UiScale>().0 = 1.5;
    Waveform::set_measurement_enabled(&mut f.app.world_mut().commands(), f.root, true);
    for _ in 0..3 {
        update(&mut f.app);
    }
    let node = f.app.world().get::<ComputedNode>(f.surface).unwrap();
    f.origin = f
        .app
        .world()
        .get::<UiGlobalTransform>(f.surface)
        .unwrap()
        .translation
        - node.size() * 0.5;
    f.draw(Vec2::new(30.0, 45.0), Vec2::new(150.0, 270.0));
    let measure = f.measurement();
    assert!(
        (measure.rectangle.min - Vec2::new(20.0, 30.0))
            .abs()
            .max_element()
            < 0.001
    );
    assert!(
        (measure.rectangle.max - Vec2::new(100.0, 180.0))
            .abs()
            .max_element()
            < 0.001
    );
    assert_eq!(measure.delta_volts, 4.0);
    Waveform::clear_measurement(&mut f.app.world_mut().commands(), f.root);
    f.app.world_mut().flush();
    assert!(
        Waveform::measurement(f.app.world(), f.root)
            .unwrap()
            .is_none()
    );
    let count = f.app.world().resource::<Changes>().0.len();
    assert!(!Waveform::clear_measurement_in_world(f.app.world_mut(), f.root).unwrap());
    assert!(!Waveform::set_measurement_enabled_in_world(f.app.world_mut(), f.root, true).unwrap());
    assert_eq!(f.app.world().resource::<Changes>().0.len(), count);
    let wrong = f.app.world_mut().spawn_empty().id();
    assert!(Waveform::set_measurement_enabled_in_world(f.app.world_mut(), wrong, true).is_err());
    assert!(Waveform::clear_measurement_in_world(f.app.world_mut(), wrong).is_err());
}
