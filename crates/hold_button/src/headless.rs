use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus};
use bevy::picking::{
    PickingSystems,
    events::PointerPress,
    hover::HoverMap,
    pointer::{PointerAction, PointerButton, PointerId, PointerInput},
};
use bevy::prelude::*;
use bevy::time::{Real, TimeUpdateStrategy};
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;
use std::time::Duration;

#[derive(Component, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component)]
pub(crate) struct ChargeProgress(pub(crate) f32);

#[derive(Component, Default, Clone)]
pub(crate) struct HoldState {
    owner: Option<InputSource>,
    succeeded: bool,
    pub(crate) successes: u64,
    updated_at: Duration,
}

#[derive(SystemParam)]
struct HoldClock<'w> {
    real: Res<'w, Time<Real>>,
    strategy: Res<'w, TimeUpdateStrategy>,
}

impl HoldClock<'_> {
    fn now(&self) -> Duration {
        // Render-channel timestamps may lag input by a frame, including a long pre-press idle gap.
        // Explicit manual clocks retain Bevy's deterministic stepping semantics.
        match *self.strategy {
            TimeUpdateStrategy::Automatic => self.real.startup().elapsed(),
            _ => self.real.elapsed(),
        }
    }
}

type Availability<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ChildOf>,
        Option<&'static Visibility>,
        Option<&'static Node>,
        Has<InteractionDisabled>,
    ),
>;

#[derive(Clone, PartialEq)]
enum InputSource {
    Pointer(PointerId, NormalizedRenderTarget),
    Key(KeyCode, Entity),
}

#[derive(Component, Clone)]
pub(crate) struct Timing {
    pub(crate) charge: Duration,
    pub(crate) decay: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            charge: Duration::from_secs(3),
            decay: Duration::from_millis(1200),
        }
    }
}

impl HoldState {
    fn advance_to(
        &mut self,
        progress: &mut ChargeProgress,
        timing: &Timing,
        now: Duration,
    ) -> bool {
        let dt = now.saturating_sub(self.updated_at);
        self.updated_at = now;
        self.advance(progress, timing, dt.as_secs_f32())
    }
    pub(crate) fn owner_is_active(&self) -> bool {
        self.owner.is_some()
    }
    fn advance(&mut self, progress: &mut ChargeProgress, timing: &Timing, dt: f32) -> bool {
        if self.owner.is_some() {
            if !self.succeeded {
                progress.0 = if timing.charge.is_zero() {
                    1.0
                } else {
                    (progress.0 + dt / timing.charge.as_secs_f32()).min(1.0)
                };
                if progress.0 >= 1.0 - 8.0 * f32::EPSILON {
                    progress.0 = 1.0;
                    self.succeeded = true;
                    self.successes = self.successes.wrapping_add(1);
                    return true;
                }
            }
        } else {
            progress.0 = if timing.decay.is_zero() {
                0.0
            } else {
                (progress.0 - dt / timing.decay.as_secs_f32()).max(0.0)
            };
        }
        false
    }

    fn end(&mut self, progress: &mut ChargeProgress) {
        self.owner = None;
        if self.succeeded {
            progress.0 = 0.0;
        }
        self.succeeded = false;
    }
}

fn finish_attempt(
    entity: Entity,
    state: &mut HoldState,
    progress: &mut ChargeProgress,
    timing: &Timing,
    now: Duration,
    nodes: &Availability,
    commands: &mut Commands,
) {
    if !available(entity, nodes) {
        state.end(progress);
        progress.0 = 0.0;
        commands.entity(entity).remove::<Pressed>();
        return;
    }
    let activated = state.advance_to(progress, timing, now);
    state.end(progress);
    commands.entity(entity).remove::<Pressed>();
    if activated {
        commands.trigger(bevy::ui_widgets::Activate { entity });
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<PointerInput>()
        .add_message::<KeyboardInput>()
        .register_type::<ChargeProgress>()
        .add_observer(pointer_press)
        .add_observer(key_input)
        .add_observer(pointer_out)
        .add_observer(pointer_release)
        .add_observer(focus_lost)
        .add_observer(interrupted)
        .add_systems(
            PreUpdate,
            cancel_inputs
                .after(PickingSystems::Last)
                .after(bevy::input_focus::InputFocusSystems::Dispatch),
        )
        .add_systems(
            PostUpdate,
            tick.after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                .before(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
}

fn interrupted(
    _event: On<
        Insert<(
            Visibility,
            bevy_widgetry_core::disabled::WidgetryEffectiveDisabled,
        )>,
    >,
    mut controls: Query<
        (
            Entity,
            &mut HoldState,
            &mut ChargeProgress,
            Option<&bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>,
        ),
        With<crate::WidgetryHoldButton>,
    >,
    nodes: Availability,
    mut commands: Commands,
) {
    for (entity, mut state, mut progress, disabled) in &mut controls {
        if (state.owner.is_some() || state.succeeded || progress.0 != 0.0)
            && (disabled.is_some_and(|disabled| disabled.is_disabled())
                || !available(entity, &nodes))
        {
            state.end(&mut progress);
            progress.set_if_neq(ChargeProgress(0.0));
            commands.entity(entity).remove::<Pressed>();
        }
    }
}

fn pointer_release(
    event: On<bevy::picking::events::PointerRelease>,
    mut controls: Query<
        (&mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    clock: HoldClock,
    nodes: Availability,
    mut commands: Commands,
) {
    let Ok((mut state, mut progress, timing)) = controls.get_mut(event.entity) else {
        return;
    };
    if event.button == PointerButton::Primary
        && matches!(&state.owner, Some(InputSource::Pointer(id, target)) if *id == event.pointer.id && *target == event.pointer.target)
    {
        finish_attempt(
            event.entity,
            &mut state,
            &mut progress,
            timing,
            clock.now(),
            &nodes,
            &mut commands,
        );
    }
}

fn pointer_out(
    event: On<bevy::picking::events::PointerOut>,
    mut controls: Query<
        (&mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    clock: HoldClock,
    nodes: Availability,
    mut commands: Commands,
) {
    let Ok((mut state, mut progress, timing)) = controls.get_mut(event.entity) else {
        return;
    };
    if matches!(&state.owner, Some(InputSource::Pointer(id, target)) if *id == event.pointer.id && *target == event.pointer.target)
    {
        finish_attempt(
            event.entity,
            &mut state,
            &mut progress,
            timing,
            clock.now(),
            &nodes,
            &mut commands,
        );
    }
}

fn focus_lost(
    event: On<bevy::input_focus::FocusLost>,
    mut controls: Query<
        (&mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    clock: HoldClock,
    nodes: Availability,
    mut commands: Commands,
) {
    let Ok((mut state, mut progress, timing)) = controls.get_mut(event.entity) else {
        return;
    };
    if matches!(state.owner, Some(InputSource::Key(..))) {
        finish_attempt(
            event.entity,
            &mut state,
            &mut progress,
            timing,
            clock.now(),
            &nodes,
            &mut commands,
        );
    }
}

fn available(entity: Entity, nodes: &Availability) -> bool {
    let mut current = entity;
    while let Ok((parent, visibility, node, disabled)) = nodes.get(current) {
        if disabled
            || visibility == Some(&Visibility::Hidden)
            || node.is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        let Some(parent) = parent else {
            break;
        };
        current = parent.parent();
    }
    true
}

fn pointer_press(
    mut event: On<PointerPress>,
    mut controls: Query<
        (&mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    clock: HoldClock,
    nodes: Availability,
    mut commands: Commands,
) {
    let Ok((mut state, mut progress, timing)) = controls.get_mut(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary
        || state.owner.is_some()
        || !available(event.entity, &nodes)
    {
        return;
    }
    state.advance_to(&mut progress, timing, clock.now());
    state.owner = Some(InputSource::Pointer(
        event.pointer.id,
        event.pointer.target.clone(),
    ));
    commands.entity(event.entity).insert(Pressed);
}

fn key_input(
    mut event: On<FocusedInput<KeyboardInput>>,
    focus: Res<InputFocus>,
    mut controls: Query<
        (&mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    clock: HoldClock,
    nodes: Availability,
    mut commands: Commands,
) {
    let entity = event.focused_entity;
    let Ok((mut state, mut progress, timing)) = controls.get_mut(entity) else {
        return;
    };
    let input = &event.input;
    if !matches!(input.key_code, KeyCode::Enter | KeyCode::Space) {
        return;
    }
    let source = InputSource::Key(input.key_code, input.window);
    if input.state == ButtonState::Released && state.owner.as_ref() == Some(&source) {
        finish_attempt(
            entity,
            &mut state,
            &mut progress,
            timing,
            clock.now(),
            &nodes,
            &mut commands,
        );
    } else if input.state == ButtonState::Pressed
        && !input.repeat
        && state.owner.is_none()
        && focus.get() == Some(entity)
        && available(entity, &nodes)
    {
        state.advance_to(&mut progress, timing, clock.now());
        state.owner = Some(source);
        commands.entity(entity).insert(Pressed);
    }
    event.propagate(false);
}

fn cancel_inputs(
    mut controls: Query<
        (Entity, &mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    nodes: Availability,
    focus: Res<InputFocus>,
    hover: Option<Res<HoverMap>>,
    pointers: WidgetryPointerQuery,
    mut pointer_inputs: MessageReader<PointerInput>,
    mut keys: MessageReader<KeyboardInput>,
    mut commands: Commands,
    clock: HoldClock,
) {
    let inputs: Vec<_> = pointer_inputs.read().collect();
    let keys: Vec<_> = keys.read().collect();
    for (entity, mut state, mut progress, timing) in &mut controls {
        if !available(entity, &nodes) {
            if state.owner.is_none() && !state.succeeded && progress.0 == 0.0 {
                continue;
            }
            state.end(&mut progress);
            progress.set_if_neq(ChargeProgress(0.0));
            commands.entity(entity).remove::<Pressed>();
            continue;
        }
        let cancel = match state.owner.as_ref() {
            Some(InputSource::Pointer(id, target)) => {
                pointers
                    .location(*id)
                    .is_none_or(|location| location.target != *target)
                    || hover.as_ref().is_none_or(|map| {
                        map.get(id).is_none_or(|hits| !hits.contains_key(&entity))
                    })
                    || inputs
                        .iter()
                        .rev()
                        .find(|input| {
                            input.pointer_id == *id
                                && input.location.target == *target
                                && matches!(
                                    input.action,
                                    PointerAction::Press(PointerButton::Primary)
                                        | PointerAction::Release(PointerButton::Primary)
                                        | PointerAction::Cancel
                                )
                        })
                        .is_some_and(|input| !matches!(input.action, PointerAction::Press(_)))
            }
            Some(InputSource::Key(code, window)) => {
                focus.get() != Some(entity)
                    || keys
                        .iter()
                        .rev()
                        .find(|key| key.window == *window && key.key_code == *code)
                        .is_some_and(|key| key.state == ButtonState::Released)
            }
            None => false,
        };
        if cancel {
            finish_attempt(
                entity,
                &mut state,
                &mut progress,
                timing,
                clock.now(),
                &nodes,
                &mut commands,
            );
        }
    }
}

fn tick(
    clock: HoldClock,
    focus: Res<InputFocus>,
    mut controls: Query<
        (Entity, &mut HoldState, &mut ChargeProgress, &Timing),
        With<crate::WidgetryHoldButton>,
    >,
    nodes: Availability,
    mut commands: Commands,
) {
    for (entity, mut state, mut progress, timing) in &mut controls {
        if !available(entity, &nodes) {
            if state.owner.is_none() && !state.succeeded && progress.0 == 0.0 {
                continue;
            }
            state.end(&mut progress);
            progress.set_if_neq(ChargeProgress(0.0));
            commands.entity(entity).remove::<Pressed>();
            continue;
        }
        let mut next = *progress;
        if matches!(state.owner, Some(InputSource::Key(..))) && focus.get() != Some(entity) {
            finish_attempt(
                entity,
                &mut state,
                &mut progress,
                timing,
                clock.now(),
                &nodes,
                &mut commands,
            );
            continue;
        }
        let activated = state.advance_to(&mut next, timing, clock.now());
        progress.set_if_neq(next);
        if activated {
            commands.trigger(bevy::ui_widgets::Activate { entity });
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, queue_key, scene_app};

    #[derive(Resource, Default)]
    struct Count(usize);

    fn fixture() -> (App, Entity, Entity) {
        let mut app = scene_app();
        app.init_asset::<bevy::shader::Shader>();
        add_keyboard_dispatch(&mut app);
        app.add_plugins(crate::WidgetryHoldButtonPlugin)
            .init_resource::<Count>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(100),
        ));
        app.add_observer(
            |_: On<bevy::ui_widgets::Activate>, mut count: ResMut<Count>| count.0 += 1,
        );
        let window = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @crate::WidgetryHoldButton })
            .unwrap()
            .id();
        app.update();
        app.update();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(root, bevy::input_focus::FocusCause::Navigated);
        app.update();
        (app, root, window)
    }
    fn key(app: &mut App, window: Entity, code: KeyCode, state: ButtonState, repeat: bool) {
        queue_key(
            app,
            KeyboardInput {
                key_code: code,
                logical_key: bevy::input::keyboard::Key::Enter,
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
    }
    fn frames(app: &mut App, count: usize) {
        for _ in 0..count {
            app.update();
        }
    }
    fn progress(app: &App, root: Entity) -> f32 {
        app.world().get::<ChargeProgress>(root).unwrap().0
    }

    #[test]
    fn fresh_press_excludes_idle_time_and_a_delayed_virtual_delta() {
        let (mut app, root, window) = fixture();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(5),
        ));
        key(
            &mut app,
            window,
            KeyCode::Enter,
            ButtonState::Pressed,
            false,
        );
        assert_eq!(progress(&app, root), 0.0);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(16),
        ));
        app.add_systems(
            PreUpdate,
            (|mut time: ResMut<Time>| time.advance_by(Duration::from_millis(250)))
                .after(bevy::input_focus::InputFocusSystems::Dispatch),
        );
        app.update();
        assert!((progress(&app, root) - 0.016 / 3.0).abs() < 0.00001);
        assert_eq!(app.world().resource::<Count>().0, 0);
    }

    #[test]
    fn release_at_threshold_settles_charge_once_and_preserves_success_feedback() {
        let (mut app, root, window) = fixture();
        key(
            &mut app,
            window,
            KeyCode::Enter,
            ButtonState::Pressed,
            false,
        );
        frames(&mut app, 29);
        assert_eq!(app.world().resource::<Count>().0, 0);
        key(
            &mut app,
            window,
            KeyCode::Enter,
            ButtonState::Released,
            false,
        );
        assert_eq!(app.world().resource::<Count>().0, 1);
        assert_eq!(progress(&app, root), 0.0);
        assert_eq!(app.world().get::<HoldState>(root).unwrap().successes, 1);
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 1);
    }

    #[test]
    fn in_place_hide_or_display_none_wins_over_same_frame_threshold_release() {
        for kind in 0..4 {
            let (mut app, root, window) = fixture();
            let parent = app.world_mut().spawn(Node::default()).id();
            app.world_mut().entity_mut(parent).add_child(root);
            key(
                &mut app,
                window,
                KeyCode::Enter,
                ButtonState::Pressed,
                false,
            );
            frames(&mut app, 29);
            let target = if kind % 2 == 0 { root } else { parent };
            if kind < 2 {
                *app.world_mut().get_mut::<Visibility>(target).unwrap() = Visibility::Hidden;
            } else {
                app.world_mut().get_mut::<Node>(target).unwrap().display = Display::None;
            }
            key(
                &mut app,
                window,
                KeyCode::Enter,
                ButtonState::Released,
                false,
            );
            assert_eq!(app.world().resource::<Count>().0, 0, "kind {kind}");
            assert_eq!(progress(&app, root), 0.0);
            assert!(app.world().get::<Pressed>(root).is_none());
        }
    }

    #[test]
    fn keyboard_success_is_immediate_one_shot_and_release_resets() {
        for code in [KeyCode::Enter, KeyCode::Space] {
            let (mut app, root, window) = fixture();
            key(&mut app, window, code, ButtonState::Pressed, false);
            frames(&mut app, 15);
            assert!((progress(&app, root) - 0.5).abs() < 0.00001);
            key(&mut app, window, code, ButtonState::Pressed, true);
            frames(&mut app, 14);
            assert_eq!(app.world().resource::<Count>().0, 1);
            assert_eq!(progress(&app, root), 1.0);
            frames(&mut app, 35);
            assert_eq!(app.world().resource::<Count>().0, 1);
            key(&mut app, window, code, ButtonState::Released, false);
            assert_eq!(progress(&app, root), 0.0);
            key(&mut app, window, code, ButtonState::Pressed, false);
            frames(&mut app, 30);
            assert_eq!(app.world().resource::<Count>().0, 2);
        }
    }

    #[test]
    fn interleaved_keys_cannot_take_over_and_focus_loss_requires_repress() {
        let (mut app, root, window) = fixture();
        key(
            &mut app,
            window,
            KeyCode::Enter,
            ButtonState::Pressed,
            false,
        );
        frames(&mut app, 10);
        key(
            &mut app,
            window,
            KeyCode::Space,
            ButtonState::Pressed,
            false,
        );
        key(
            &mut app,
            window,
            KeyCode::Enter,
            ButtonState::Released,
            false,
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        let p = progress(&app, root);
        key(&mut app, window, KeyCode::Space, ButtonState::Pressed, true);
        assert!(progress(&app, root) < p);
        key(
            &mut app,
            window,
            KeyCode::Space,
            ButtonState::Released,
            false,
        );
        key(
            &mut app,
            window,
            KeyCode::Space,
            ButtonState::Pressed,
            false,
        );
        assert!(app.world().get::<Pressed>(root).is_some());
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
        assert!(app.world().get::<Pressed>(root).is_none());
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(root, bevy::input_focus::FocusCause::Navigated);
        key(&mut app, window, KeyCode::Space, ButtonState::Pressed, true);
        frames(&mut app, 40);
        assert_eq!(progress(&app, root), 0.0);
        assert_eq!(app.world().resource::<Count>().0, 0);
        key(
            &mut app,
            window,
            KeyCode::Space,
            ButtonState::Released,
            false,
        );
        key(
            &mut app,
            window,
            KeyCode::Space,
            ButtonState::Pressed,
            false,
        );
        frames(&mut app, 30);
        assert_eq!(app.world().resource::<Count>().0, 1);
    }

    #[test]
    fn disabled_hidden_and_display_none_reset_partial_progress() {
        for kind in 0..6 {
            let (mut app, root, window) = fixture();
            let parent = app.world_mut().spawn(Node::default()).id();
            app.world_mut().entity_mut(parent).add_child(root);
            key(
                &mut app,
                window,
                KeyCode::Enter,
                ButtonState::Pressed,
                false,
            );
            frames(&mut app, 12);
            assert!(progress(&app, root) > 0.3);
            let target = if kind % 2 == 0 { root } else { parent };
            match kind / 2 {
                0 => {
                    app.world_mut()
                        .entity_mut(target)
                        .insert(InteractionDisabled);
                }
                1 => {
                    app.world_mut()
                        .entity_mut(target)
                        .insert(Visibility::Hidden);
                }
                _ => {
                    app.world_mut().get_mut::<Node>(target).unwrap().display = Display::None;
                }
            }
            app.update();
            assert_eq!(progress(&app, root), 0.0, "kind {kind}");
            assert!(app.world().get::<Pressed>(root).is_none());
            app.world_mut()
                .entity_mut(target)
                .remove::<InteractionDisabled>()
                .insert(Visibility::Inherited);
            app.world_mut().get_mut::<Node>(target).unwrap().display = Display::Flex;
            key(&mut app, window, KeyCode::Enter, ButtonState::Pressed, true);
            frames(&mut app, 35);
            assert_eq!(progress(&app, root), 0.0);
            assert_eq!(app.world().resource::<Count>().0, 0);
        }
    }

    #[test]
    fn partial_release_decays_at_fixed_rate_and_repress_resumes() {
        let mut s = HoldState {
            owner: Some(InputSource::Key(KeyCode::Enter, Entity::PLACEHOLDER)),
            ..default()
        };
        let mut p = ChargeProgress::default();
        let t = Timing::default();
        assert!(!s.advance(&mut p, &t, 1.8));
        assert!((p.0 - 0.6).abs() < 0.00001);
        s.end(&mut p);
        assert!(!s.advance(&mut p, &t, 0.36));
        assert!((p.0 - 0.3).abs() < 0.00001);
        s.owner = Some(InputSource::Key(KeyCode::Enter, Entity::PLACEHOLDER));
        assert!(s.advance(&mut p, &t, 2.1));
        assert_eq!(p.0, 1.0);
        assert!(!s.advance(&mut p, &t, 20.0));
        assert_eq!(p.0, 1.0);
        s.end(&mut p);
        assert_eq!(p.0, 0.0);
        s.owner = Some(InputSource::Key(KeyCode::Enter, Entity::PLACEHOLDER));
        assert!(s.advance(&mut p, &t, 3.0));
    }

    #[test]
    fn decay_clamps_and_zero_durations_finish_immediately() {
        let mut s = HoldState::default();
        let mut p = ChargeProgress(0.2);
        let t = Timing {
            charge: Duration::ZERO,
            decay: Duration::ZERO,
        };
        s.advance(&mut p, &t, 0.0);
        assert_eq!(p.0, 0.0);
        s.owner = Some(InputSource::Key(KeyCode::Enter, Entity::PLACEHOLDER));
        assert!(s.advance(&mut p, &t, 0.0));
        assert!(!s.advance(&mut p, &t, 0.0));
        assert_eq!(p.0, 1.0);
    }
}
