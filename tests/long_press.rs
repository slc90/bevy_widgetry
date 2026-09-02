use std::time::Duration;

use bevy::{
    app::App,
    camera::NormalizedRenderTarget,
    ecs::{entity::Entity, observer::On, resource::Resource, system::ResMut},
    math::Vec2,
    picking::{
        backend::HitData,
        events::{Cancel, DragEnd, Pointer, Press, Release},
        pointer::{Location, PointerButton, PointerId},
    },
    time::{TimePlugin, TimeUpdateStrategy},
    ui::InteractionDisabled,
};

use bevy_widgetry::{LongPressButton, LongPressEvent, LongPressPlugin};

#[derive(Resource, Default)]
struct LongPressCount(usize);

fn record_long_press(_: On<LongPressEvent>, mut count: ResMut<LongPressCount>) {
    count.0 += 1;
}

fn setup_app(frame_time: Duration) -> App {
    let mut app = App::new();

    app.add_plugins(TimePlugin);
    app.add_plugins(LongPressPlugin);

    app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_time));
    app.init_resource::<LongPressCount>();
    app.add_observer(record_long_press);
    app
}

pub fn press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
}

fn primary_press(entity: Entity) -> Pointer<Press> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        },
        entity,
    )
}

pub fn release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

fn primary_release(entity: Entity) -> Pointer<Release> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Release {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        entity,
    )
}

pub fn cancel(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_cancel(entity));
    app.world_mut().flush();
}

fn primary_cancel(entity: Entity) -> Pointer<Cancel> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Cancel {
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        entity,
    )
}

pub fn drag_end(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_drag_end(entity));
    app.world_mut().flush();
}

fn primary_drag_end(entity: Entity) -> Pointer<DragEnd> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        },
        entity,
    )
}

#[test]
fn long_press_does_not_fire_before_threshold() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time，不算作长按时间
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..4 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 0);
    }

    app.update();

    assert_eq!(app.world().resource::<LongPressCount>().0, 1);

    for _ in 0..5 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 1);
    }
}

#[test]
fn release_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    release(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

#[test]
fn cancel_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    cancel(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

#[test]
fn drag_end_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    drag_end(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

#[test]
fn disabled_button_does_not_fire_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app
        .world_mut()
        .spawn((LongPressButton::default(), InteractionDisabled))
        .id();

    press(&mut app, button);

    for _ in 0..10 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

#[test]
fn custom_press_duration_changes_trigger_time() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app
        .world_mut()
        .spawn(LongPressButton {
            press_duration: 300,
        })
        .id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 0);
    }

    app.update();

    assert_eq!(app.world().resource::<LongPressCount>().0, 1);
}
