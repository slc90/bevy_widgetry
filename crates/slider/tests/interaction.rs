#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
    ui::Pressed,
    ui_widgets::ValueChange,
};
use bevy_widgetry_slider::{
    SliderDragStrategy, SliderInteractionStart, SliderInteractionStrategy, SliderPointerInput,
    SliderProjection, WidgetrySlider, WidgetrySliderHeadless, WidgetrySliderInteraction,
    WidgetrySliderInteractionPlugin,
};
use bevy_widgetry_test_utils::{ErrorCapture, picking_app, queue_pointer, spawn_picking_camera};
#[derive(Resource, Default)]
struct Changes(Vec<(f32, bool)>);
fn update(app: &mut App) {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}

#[derive(Component, Default, Clone, Copy)]
enum Mode {
    #[default]
    Normal,
    Decline,
    Wait,
    Cancel,
    Fail,
    NonFinite,
}
struct External;
struct Session {
    offset: f64,
}
impl SliderInteractionStrategy for External {
    fn begin(
        &self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<Option<SliderInteractionStart>, bevy_widgetry_slider::SliderFailure> {
        if matches!(world.get::<Mode>(input.entity), Some(Mode::Decline)) {
            return Ok(None);
        }
        Ok(Some(SliderInteractionStart {
            position: f64::from(input.state.value()),
            drag: Box::new(Session {
                offset: f64::from(input.position.x) - f64::from(input.state.value()),
            }),
        }))
    }
    fn key(
        &self,
        _: &World,
        _: Entity,
        input: &bevy::input::keyboard::KeyboardInput,
    ) -> Result<Option<bevy_widgetry_slider::SliderDirection>, bevy_widgetry_slider::SliderFailure>
    {
        use bevy_widgetry_slider::SliderDirection;
        Ok(match input.key_code {
            KeyCode::KeyW => Some(SliderDirection::Increase),
            KeyCode::KeyS => Some(SliderDirection::Decrease),
            _ => None,
        })
    }
}
impl SliderDragStrategy for Session {
    fn project(
        &mut self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<SliderProjection, bevy_widgetry_slider::SliderFailure> {
        use bevy_widgetry_slider::{SliderError, SliderFailure};
        match world.get::<Mode>(input.entity).copied().unwrap_or_default() {
            Mode::Wait => Ok(SliderProjection::Wait),
            Mode::Cancel => Ok(SliderProjection::Cancel),
            Mode::NonFinite => Ok(SliderProjection::Position(f64::NAN)),
            Mode::Fail => Err(SliderFailure::new(
                SliderError::Strategy {
                    root: input.entity,
                    source: Box::new(std::io::Error::other("external failure")),
                },
                bevy_widgetry_diagnostics::ErrorFrame::new("test.external"),
            )),
            _ => Ok(SliderProjection::Position(
                f64::from(input.position.x) - self.offset,
            )),
        }
    }
}
#[test]
fn external_strategy_uses_library_capture_and_nonfinal_commits_without_default_parts() {
    let mut app = picking_app();
    app.add_plugins(WidgetrySliderInteractionPlugin);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetrySliderHeadless { @value:{Some(25.0)} }
        template(|_|Ok(WidgetrySliderInteraction::new(External)))
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(100)}
        template(move |_|Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    app.init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f32>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.value, event.is_final))
        },
    );
    update(&mut app);
    update(&mut app);
    let id = PointerId::Mouse;
    let mut at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(100.0, 80.0),
    };
    queue_pointer(
        &mut app,
        id,
        at.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    );
    update(&mut app);
    queue_pointer(
        &mut app,
        id,
        at.clone(),
        PointerAction::Press(PointerButton::Primary),
    );
    update(&mut app);
    assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        25.0
    );
    at.position.x += 30.0;
    queue_pointer(
        &mut app,
        id,
        at.clone(),
        PointerAction::Move {
            delta: Vec2::new(30.0, 0.0),
        },
    );
    update(&mut app);
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        55.0
    );
    assert_eq!(app.world().resource::<Changes>().0, [(55.0, false)]);
    queue_pointer(
        &mut app,
        id,
        at,
        PointerAction::Release(PointerButton::Primary),
    );
    update(&mut app);
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    assert!(app.world().get::<Pressed>(root).is_none());
    assert!(app.world().get::<WidgetrySlider>(root).is_none());
    assert!(app.world().get::<Children>(root).is_none());
}

#[derive(Resource, Default)]
struct Notices(
    Vec<(
        Entity,
        bool,
        f32,
        Option<bevy_widgetry_slider::SliderDragOutcome>,
    )>,
);
fn fixture() -> (App, Entity, Entity, Location) {
    use bevy_widgetry_slider::{SliderDragEnded, SliderDragStarted};
    let mut app = picking_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetrySliderInteractionPlugin)
        .init_resource::<Notices>()
        .init_resource::<Changes>();
    app.set_error_handler(ErrorCapture::handler());
    app.add_observer(
        |e: On<SliderDragStarted>, mut world: bevy::ecs::world::DeferredWorld| {
            assert!(world.get::<Pressed>(e.entity).is_some());
            assert_eq!(
                WidgetrySliderHeadless::value(&world, e.entity).unwrap(),
                e.value
            );
            world
                .resource_mut::<Notices>()
                .0
                .push((e.entity, true, e.value, None));
        },
    );
    app.add_observer(
        |e: On<SliderDragEnded>, mut world: bevy::ecs::world::DeferredWorld| {
            assert!(world.get::<Pressed>(e.entity).is_none());
            assert!(
                !world
                    .resource::<PointerCaptureMap>()
                    .0
                    .values()
                    .any(|(owner, _)| *owner == e.entity)
            );
            assert_eq!(
                WidgetrySliderHeadless::value(&world, e.entity).unwrap(),
                e.value
            );
            world
                .resource_mut::<Notices>()
                .0
                .push((e.entity, false, e.value, Some(e.outcome)));
        },
    );
    app.add_observer(
        |e: On<ValueChange<f32>>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetrySliderHeadless::value(&world, e.source).unwrap(),
                e.value
            );
            world
                .resource_mut::<Changes>()
                .0
                .push((e.value, e.is_final));
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetrySliderHeadless { @max:10.0,@step:{Some(3.0)} }
        template(|_|Ok(WidgetrySliderInteraction::new(External)))
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(100)}
        template(move |_|Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    update(&mut app);
    update(&mut app);
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(100.0, 80.0),
    };
    (app, root, window, at)
}
fn action(app: &mut App, at: &Location, action: PointerAction) {
    queue_pointer(app, PointerId::Mouse, at.clone(), action);
    update(app);
}
fn press(app: &mut App, at: &Location) {
    action(app, at, PointerAction::Move { delta: Vec2::ZERO });
    action(app, at, PointerAction::Press(PointerButton::Primary));
}
fn move_to(app: &mut App, at: &mut Location, x: f32) {
    let delta = Vec2::new(x - at.position.x, 0.0);
    at.position.x = x;
    action(app, at, PointerAction::Move { delta });
}
#[test]
fn sessions_are_isolated_waiting_retains_capture_and_reversals_use_library_steps() {
    let (mut app, root, _, mut at) = fixture();
    press(&mut app, &at);
    for (x, value) in [
        (103.0, 3.0),
        (106.0, 6.0),
        (109.0, 9.0),
        (110.0, 10.0),
        (107.0, 7.0),
        (104.0, 4.0),
    ] {
        move_to(&mut app, &mut at, x);
        assert_eq!(
            WidgetrySliderHeadless::value(app.world(), root).unwrap(),
            value
        );
    }
    app.world_mut().entity_mut(root).insert(Mode::Wait);
    move_to(&mut app, &mut at, 109.0);
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        4.0
    );
    assert!(
        app.world()
            .resource::<PointerCaptureMap>()
            .is_captured(&PointerId::Mouse)
    );
    app.world_mut().entity_mut(root).insert(Mode::Normal);
    move_to(&mut app, &mut at, 101.0);
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        1.0
    );
    action(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    // A new instance uses its own offset, rather than the first session's start.
    let other=app.world_mut().spawn_scene(bsn! {
        @WidgetrySliderHeadless { @value:{Some(50.0)} }
        template(|_|Ok(WidgetrySliderInteraction::new(External)))
        Node {position_type:PositionType::Absolute,left:px(40),top:px(180),width:px(200),height:px(100)}
    }).unwrap().id();
    update(&mut app);
    update(&mut app);
    at.position = Vec2::new(150.0, 220.0);
    press(&mut app, &at);
    move_to(&mut app, &mut at, 155.0);
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), other).unwrap(),
        55.0
    );
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        1.0
    );
    action(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(app.world().resource::<Notices>().0.len(), 4);
}
#[test]
fn declined_and_cancelled_strategies_keep_lifecycle_truthful() {
    use bevy_widgetry_slider::SliderDragOutcome;
    let (mut app, root, _, mut at) = fixture();
    app.world_mut().entity_mut(root).insert(Mode::Decline);
    press(&mut app, &at);
    assert!(app.world().resource::<Notices>().0.is_empty());
    assert!(
        !app.world()
            .resource::<PointerCaptureMap>()
            .is_captured(&PointerId::Mouse)
    );
    action(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    app.world_mut().entity_mut(root).insert(Mode::Normal);
    press(&mut app, &at);
    move_to(&mut app, &mut at, 106.0);
    app.world_mut().entity_mut(root).insert(Mode::Cancel);
    move_to(&mut app, &mut at, 107.0);
    action(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        app.world().resource::<Notices>().0,
        [
            (root, true, 0.0, None),
            (root, false, 6.0, Some(SliderDragOutcome::Cancelled))
        ]
    );
    assert_eq!(app.world().resource::<Changes>().0, [(6.0, false)]);
}
#[test]
fn strategy_failures_clear_capture_and_retain_typed_sources_and_diagnostics() {
    use bevy_widgetry_slider::{SliderDragOutcome, SliderError};
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    for mode in [Mode::Fail, Mode::NonFinite] {
        let (mut app, root, _, mut at) = fixture();
        press(&mut app, &at);
        app.world_mut().entity_mut(root).insert(mode);
        at.position.x += 2.0;
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            at.clone(),
            PointerAction::Move {
                delta: Vec2::new(2.0, 0.0),
            },
        );
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        let mut cause: &dyn std::error::Error = diagnostic;
        let failure = loop {
            if let Some(failure) = cause.downcast_ref::<bevy_widgetry_slider::SliderFailure>() {
                break failure;
            }
            cause = cause.source().unwrap();
        };
        match mode {
            Mode::Fail => assert!(
                matches!(failure.kind(),SliderError::Strategy {root:owner,source} if *owner==root && source.downcast_ref::<std::io::Error>().is_some())
            ),
            _ => assert!(
                matches!(failure.kind(),SliderError::InvalidPosition {root:owner,position} if *owner==root&&position.is_nan())
            ),
        }
        assert_eq!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        assert!(
            !app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse)
        );
        assert_eq!(
            app.world().resource::<Notices>().0.last(),
            Some(&(root, false, 0.0, Some(SliderDragOutcome::Cancelled)))
        );
    }
}
#[test]
fn custom_keyboard_mapping_cancels_pointer_and_program_writes_still_work_disabled() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput, NativeKey},
        },
        input_focus::{FocusCause, InputFocus},
    };
    let (mut app, root, window, at) = fixture();
    press(&mut app, &at);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for (code, expected) in [
        (KeyCode::ArrowRight, 0.0),
        (KeyCode::KeyW, 3.0),
        (KeyCode::KeyW, 6.0),
        (KeyCode::KeyS, 3.0),
    ] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: true,
            window,
        });
        update(&mut app);
        assert_eq!(
            WidgetrySliderHeadless::value(app.world(), root).unwrap(),
            expected
        );
    }
    assert_eq!(app.world().resource::<Notices>().0.len(), 2);
    assert!(app.world().get::<Pressed>(root).is_none());
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    assert!(WidgetrySliderHeadless::set_value_in_world(app.world_mut(), root, 10.0).unwrap());
    assert_eq!(
        app.world().resource::<Changes>().0.last(),
        Some(&(10.0, true))
    );
}

#[test]
fn loss_of_numeric_state_in_started_commands_cleans_before_error_without_inventing_end() {
    use bevy_widgetry_slider::{SliderDragStarted, WidgetrySliderState};
    let (mut app, root, _, at) = fixture();
    app.add_observer(|e: On<SliderDragStarted>, mut commands: Commands| {
        commands.entity(e.entity).remove::<WidgetrySliderState>();
    });
    action(&mut app, &at, PointerAction::Move { delta: Vec2::ZERO });
    queue_pointer(
        &mut app,
        PointerId::Mouse,
        at,
        PointerAction::Press(PointerButton::Primary),
    );
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert_eq!(errors.take_records().len(), 1);
    assert!(app.world().get::<Pressed>(root).is_none());
    assert!(
        !app.world()
            .resource::<PointerCaptureMap>()
            .is_captured(&PointerId::Mouse)
    );
    assert_eq!(
        app.world().resource::<Notices>().0,
        [(root, true, 0.0, None)]
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn public_linear_binding_works_on_application_parts_without_default_style() {
    use bevy_widgetry_slider::{
        WidgetrySliderLinear, WidgetrySliderOrientation, WidgetrySliderPlugin,
    };
    let (mut app, _, window, mut at) = fixture();
    app.add_plugins(WidgetrySliderPlugin);
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let color = Color::srgb(0.7, 0.1, 0.5);
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetrySliderHeadless { @value:{Some(50.0)} }
        WidgetrySliderLinear {track:#Track,thumb:#Thumb,orientation:WidgetrySliderOrientation::Horizontal}
        Node {position_type:PositionType::Absolute,left:px(40),top:px(200),width:px(200),height:px(40),align_items:AlignItems::Center,justify_content:JustifyContent::Center}
        template(move |_|Ok(UiTargetCamera(camera)))
        Children [
            #Track Pickable::IGNORE BackgroundColor(color)
            Node {width:px(180),height:px(10)}
            Children [
                #Thumb Pickable::IGNORE BackgroundColor(color)
                Node {position_type:PositionType::Absolute,left:percent(50),width:px(20),height:px(20)}
            ]
        ]
    }).unwrap().id();
    update(&mut app);
    update(&mut app);
    let binding = *app.world().get::<WidgetrySliderLinear>(root).unwrap();
    at.position = Vec2::new(230.0, 220.0);
    press(&mut app, &at);
    assert_eq!(
        WidgetrySliderHeadless::value(app.world(), root).unwrap(),
        100.0
    );
    action(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert!(app.world().get::<WidgetrySlider>(root).is_none());
    for part in [binding.track, binding.thumb] {
        assert_eq!(app.world().get::<BackgroundColor>(part).unwrap().0, color);
    }
}
