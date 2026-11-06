use crate::WidgetrySliderHeadless;
use crate::{
    SliderDragEnded, SliderDragOutcome, SliderDragStarted, WidgetrySlider,
    WidgetrySliderOrientation,
    gesture::{Gesture, directional},
    value::{WidgetrySliderState, commit},
};
use bevy::{
    camera::NormalizedRenderTarget,
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, InputFocus},
    picking::{
        PickingSystems,
        events::{PointerCancel, PointerDragEnd, PointerMove, PointerPress, PointerRelease},
        hover::PointerCaptureMap,
        pointer::PointerId,
    },
    prelude::*,
    ui::{ComputedUiRenderTargetInfo, InteractionDisabled, Pressed, UiGlobalTransform},
};

#[derive(Component, Default)]
pub(crate) struct DragState(pub(crate) Option<Drag>);

#[derive(Clone)]
pub(crate) struct Drag {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    gesture: Gesture,
    offset: f64,
}

fn broken(entity: Entity, component: &'static str) -> BevyError {
    crate::error::missing(entity, entity, component)
        .with_context(bevy_widgetry_diagnostics::ErrorFrame::new("slider.input"))
        .into_bevy_error()
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PointerCaptureMap>()
        .add_observer(press)
        .add_observer(moved)
        .add_observer(released)
        .add_observer(cancelled)
        .add_observer(drag_ended)
        .add_observer(key)
        .add_observer(disabled)
        .add_systems(PreUpdate, cancel_unavailable.after(PickingSystems::Last))
        .add_systems(
            PostUpdate,
            cancel_unavailable.after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled),
        );
}

fn available(world: &World, entity: Entity) -> bool {
    let mut current = entity;
    let mut visibility_resolved = false;
    loop {
        if world.get::<InteractionDisabled>(current).is_some()
            || world
                .get::<Node>(current)
                .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        if !visibility_resolved {
            match world.get::<Visibility>(current) {
                Some(Visibility::Hidden) => return false,
                Some(Visibility::Visible) => visibility_resolved = true,
                _ => {}
            }
        }
        match world.get::<ChildOf>(current) {
            Some(parent) => current = parent.parent(),
            None => return true,
        }
    }
}

fn target_focused(world: &World, target: &NormalizedRenderTarget) -> bool {
    match target {
        NormalizedRenderTarget::Window(window) => world
            .get::<Window>(window.entity())
            .is_some_and(|window| window.focused),
        _ => true,
    }
}

fn position(
    world: &World,
    entity: Entity,
    point: Vec2,
) -> Result<Option<(f64, Vec2, f32, f32)>, crate::SliderFailure> {
    if !point.is_finite() || world.get::<WidgetrySlider>(entity).is_none() {
        return Ok(None);
    }
    let state = world
        .get::<WidgetrySliderState>(entity)
        .ok_or_else(|| crate::error::missing(entity, entity, "WidgetrySliderState"))?;
    let parts = world
        .get::<crate::style::Parts>(entity)
        .ok_or_else(|| crate::error::missing(entity, entity, "Parts"))?;
    Ok(project_position(
        world,
        entity,
        point,
        state,
        WidgetrySlider::orientation(world, entity)?,
        parts,
    ))
}

fn project_position(
    world: &World,
    entity: Entity,
    point: Vec2,
    state: &WidgetrySliderState,
    orientation: WidgetrySliderOrientation,
    parts: &crate::style::Parts,
) -> Option<(f64, Vec2, f32, f32)> {
    let node = world.get::<ComputedNode>(parts.track)?;
    let transform = world.get::<UiGlobalTransform>(parts.track)?;
    let target = world.get::<ComputedUiRenderTargetInfo>(entity)?;
    let ui_scale = world.get_resource::<UiScale>().map_or(1.0, |scale| scale.0);
    let camera = world
        .get::<bevy::ui::ComputedUiTargetCamera>(entity)?
        .get()?;
    let viewport_origin = world
        .get::<Camera>(camera)?
        .physical_viewport_rect()?
        .min
        .as_vec2();
    let local = transform
        .try_inverse()?
        .transform_point2(point * target.scale_factor() / ui_scale - viewport_origin);
    let thumb = parts.thumb;
    let thumb = world.get::<ComputedNode>(thumb)?;
    let vertical = orientation == WidgetrySliderOrientation::Vertical;
    let size = if vertical {
        thumb.size().y
    } else {
        thumb.size().x
    };
    let axis = if vertical { -local.y } else { local.x };
    let length = if vertical {
        node.size().y
    } else {
        node.size().x
    };
    let travel = length;
    if !point.is_finite() || !local.is_finite() || !travel.is_finite() || travel <= 0.0 {
        return None;
    }
    let normalized = f64::from(axis / travel + 0.5);
    let value_position = if normalized == 0.0 {
        f64::from(state.min)
    } else if normalized == 1.0 {
        f64::from(state.max)
    } else {
        f64::from(state.min) + normalized * (f64::from(state.max) - f64::from(state.min))
    };
    Some((value_position, local, size, travel))
}

fn press(
    mut event: On<PointerPress>,
    controls: Query<(), With<WidgetrySlider>>,
    mut commands: Commands,
) {
    if !controls.contains(event.entity) {
        return;
    }
    event.propagate(false);
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        if event.button != bevy::picking::pointer::PointerButton::Primary
            || !available(world, event.entity)
            || !target_focused(world, &event.pointer.target)
            || world
                .get::<DragState>(event.entity)
                .is_none_or(|drag| drag.0.is_some())
            || world
                .resource::<PointerCaptureMap>()
                .is_captured(&event.pointer.id)
        {
            return Ok(());
        }
        let Some((pointer_position, local, thumb_size, travel)) =
            position(world, event.entity, event.pointer.position)
                .map_err(|error| error.into_bevy_error())?
        else {
            return Ok(());
        };
        let state = *world
            .get::<WidgetrySliderState>(event.entity)
            .ok_or_else(|| broken(event.entity, "WidgetrySliderState"))?;
        let normalized = (state.value - state.min) / (state.max - state.min);
        let vertical = WidgetrySlider::orientation(world, event.entity)
            .map_err(|error| error.into_bevy_error())?
            == WidgetrySliderOrientation::Vertical;
        let center = (normalized - 0.5) * travel;
        let distance = if vertical {
            Vec2::new(local.x, -local.y - center)
        } else {
            Vec2::new(local.x - center, local.y)
        };
        let on_thumb = distance.length_squared() <= (thumb_size * 0.5).powi(2);
        let offset = if on_thumb {
            pointer_position - f64::from(state.value)
        } else {
            0.0
        };
        let pointer_position =
            (pointer_position - offset).clamp(f64::from(state.min), f64::from(state.max));
        let mut gesture = Gesture::new(state.value, f64::from(state.value));
        let next = if on_thumb {
            state.value
        } else {
            gesture.advance(&state, pointer_position)
        };
        world.entity_mut(event.entity).insert((
            DragState(Some(Drag {
                pointer: event.pointer.id,
                target: event.pointer.target.clone(),
                gesture,
                offset,
            })),
            Pressed,
        ));
        world.resource_mut::<PointerCaptureMap>().capture(
            event.pointer.id,
            event.entity,
            event.hit,
        );
        world.trigger(SliderDragStarted {
            entity: event.entity,
            value: state.value,
        });
        // Let consumer commands from Started settle before the initial track value.
        world.commands().queue(move |world: &mut World| -> Result {
            if world.get::<WidgetrySlider>(event.entity).is_none() {
                if world
                    .resource::<PointerCaptureMap>()
                    .get(&event.pointer.id)
                    .is_some_and(|(owner, _)| owner == event.entity)
                {
                    world
                        .resource_mut::<PointerCaptureMap>()
                        .release(event.pointer.id);
                }
                return Ok(());
            }
            if !available(world, event.entity)
                || !target_focused(world, &event.pointer.target)
                || !world
                    .resource::<PointerCaptureMap>()
                    .get(&event.pointer.id)
                    .is_some_and(|(owner, _)| owner == event.entity)
            {
                return end(world, event.entity, SliderDragOutcome::Cancelled);
            }
            if world
                .get::<DragState>(event.entity)
                .is_none_or(|drag| drag.0.is_none())
            {
                return Ok(());
            }
            commit(world, event.entity, next, false).map_err(|error| error.into_bevy_error())?;
            Ok(())
        });
        Ok(())
    });
}

fn moved(
    event: On<PointerMove>,
    controls: Query<(), With<WidgetrySlider>>,
    mut commands: Commands,
) {
    if !controls.contains(event.entity) {
        return;
    }
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        if !available(world, event.entity) || !target_focused(world, &event.pointer.target) {
            end(world, event.entity, SliderDragOutcome::Cancelled)?;
            return Ok(());
        }
        let Some(mut drag) = world
            .get::<DragState>(event.entity)
            .and_then(|drag| drag.0.clone())
        else {
            return Ok(());
        };
        if drag.pointer != event.pointer.id || drag.target != event.pointer.target {
            return Ok(());
        }
        let projected = match position(world, event.entity, event.pointer.position) {
            Ok(projected) => projected,
            Err(error) => {
                return Err(cancel_failed_projection(world, event.entity, error));
            }
        };
        let Some((position, _, _, _)) = projected else {
            end(world, event.entity, SliderDragOutcome::Cancelled)?;
            return Ok(());
        };
        let state = world
            .get::<WidgetrySliderState>(event.entity)
            .ok_or_else(|| broken(event.entity, "WidgetrySliderState"))?;
        let position = (position - drag.offset).clamp(f64::from(state.min), f64::from(state.max));
        let value = drag.gesture.advance(state, position);
        world.entity_mut(event.entity).insert(DragState(Some(drag)));
        commit(world, event.entity, value, false).map_err(|error| error.into_bevy_error())?;
        Ok(())
    });
}

fn clear_drag(world: &mut World, entity: Entity) -> bool {
    let Some(drag) = world
        .get_mut::<DragState>(entity)
        .and_then(|mut state| state.0.take())
    else {
        return false;
    };
    let mut capture = world.resource_mut::<PointerCaptureMap>();
    if capture
        .get(&drag.pointer)
        .is_some_and(|(owner, _)| owner == entity)
    {
        capture.release(drag.pointer);
    }
    world.entity_mut(entity).remove::<Pressed>();
    true
}

fn cancel_failed_projection(
    world: &mut World,
    entity: Entity,
    error: crate::SliderFailure,
) -> BevyError {
    let cancelled = clear_drag(world, entity);
    let value = world
        .get::<WidgetrySliderState>(entity)
        .map(|state| state.value);
    if cancelled && let Some(value) = value {
        world.trigger(SliderDragEnded {
            entity,
            value,
            outcome: SliderDragOutcome::Cancelled,
        });
    }
    // A missing State has no truthful committed value for Ended; propagate its existing diagnosis.
    error
        .with_context(
            bevy_widgetry_diagnostics::ErrorFrame::new("slider.cancel_failed_projection")
                .field("root", entity)
                .field("gesture_cleared", cancelled)
                .field("value_available", value.is_some()),
        )
        .into_bevy_error()
}

fn end(world: &mut World, entity: Entity, outcome: SliderDragOutcome) -> Result {
    if clear_drag(world, entity) {
        let value = WidgetrySliderHeadless::value(world, entity)
            .map_err(|error| error.into_bevy_error())?;
        world.trigger(SliderDragEnded {
            entity,
            value,
            outcome,
        });
    }
    Ok(())
}

fn terminal(
    world: &mut World,
    entity: Entity,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
    outcome: SliderDragOutcome,
) -> Result {
    if world
        .get::<DragState>(entity)
        .and_then(|drag| drag.0.as_ref())
        .is_some_and(|drag| drag.pointer == pointer && drag.target == *target)
    {
        let outcome = if available(world, entity) && target_focused(world, target) {
            outcome
        } else {
            SliderDragOutcome::Cancelled
        };
        end(world, entity, outcome)?;
    }
    Ok(())
}

fn released(event: On<PointerRelease>, mut commands: Commands) {
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    let (entity, pointer, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    commands.queue(move |world: &mut World| {
        terminal(
            world,
            entity,
            pointer,
            &target,
            SliderDragOutcome::Completed,
        )
    });
}
fn cancelled(event: On<PointerCancel>, mut commands: Commands) {
    let (entity, pointer, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    commands.queue(move |world: &mut World| {
        terminal(
            world,
            entity,
            pointer,
            &target,
            SliderDragOutcome::Cancelled,
        )
    });
}
fn drag_ended(event: On<PointerDragEnd>, mut commands: Commands) {
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    let (entity, pointer, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    commands.queue(move |world: &mut World| {
        terminal(
            world,
            entity,
            pointer,
            &target,
            SliderDragOutcome::Completed,
        )
    });
}

fn key(
    mut event: On<FocusedInput<KeyboardInput>>,
    controls: Query<
        (
            &WidgetrySliderState,
            &crate::slider::LinearOrientation,
            Has<InteractionDisabled>,
        ),
        With<WidgetrySlider>,
    >,
    focus: Res<InputFocus>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let entity = event.focused_entity;
    let Ok((_state, orientation, disabled)) = controls.get(entity) else {
        return;
    };
    if disabled
        || focus.get() != Some(entity)
        || event.input.state != ButtonState::Pressed
        || !windows
            .get(event.input.window)
            .is_ok_and(|window| window.focused)
    {
        return;
    }
    let input = event.input.clone();
    let Some(direction) = key_direction(orientation.0, input.key_code) else {
        return;
    };
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        if !available(world, entity)
            || world.resource::<InputFocus>().get() != Some(entity)
            || world
                .get::<Window>(input.window)
                .is_none_or(|window| !window.focused)
        {
            return Ok(());
        }
        let state = world
            .get::<WidgetrySliderState>(entity)
            .ok_or_else(|| broken(entity, "WidgetrySliderState"))?;
        let step = state.step.unwrap_or((state.max - state.min) * 0.01);
        let value = directional(state, f64::from(state.value), direction * f64::from(step));
        end(world, entity, SliderDragOutcome::Cancelled)?;
        if world.get::<WidgetrySlider>(entity).is_none() {
            return Ok(());
        }
        commit(world, entity, value, true).map_err(|error| error.into_bevy_error())?;
        Ok(())
    });
}

fn key_direction(orientation: WidgetrySliderOrientation, key: KeyCode) -> Option<f64> {
    match (orientation, key) {
        (WidgetrySliderOrientation::Horizontal, KeyCode::ArrowRight)
        | (WidgetrySliderOrientation::Vertical, KeyCode::ArrowUp) => Some(1.0),
        (WidgetrySliderOrientation::Horizontal, KeyCode::ArrowLeft)
        | (WidgetrySliderOrientation::Vertical, KeyCode::ArrowDown) => Some(-1.0),
        _ => None,
    }
}

fn disabled(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    mut commands: Commands,
) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        if !available(world, entity) {
            end(world, entity, SliderDragOutcome::Cancelled)?;
        }
        Ok(())
    });
}

fn cancel_unavailable(world: &mut World) -> Result {
    let active: Vec<_> = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(entity, state)| state.0.as_ref().map(|drag| (entity, drag.clone())))
        .collect();
    for (entity, drag) in active {
        let valid = world
            .query::<(&PointerId, &bevy::picking::pointer::PointerLocation)>()
            .iter(world)
            .any(|(id, location)| {
                *id == drag.pointer
                    && location.location.as_ref().is_some_and(|location| {
                        location.target == drag.target && location.position.is_finite()
                    })
            });
        let captured = world
            .resource::<PointerCaptureMap>()
            .get(&drag.pointer)
            .is_some_and(|(owner, _)| owner == entity);
        if !valid || !captured || !available(world, entity) || !target_focused(world, &drag.target)
        {
            end(world, entity, SliderDragOutcome::Cancelled)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::SliderError;
    use bevy_widgetry_core::scene::spawn_scene;

    #[test]
    fn position_separates_layout_waiting_and_cancelled_roots_from_broken_owned_state() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider }).unwrap();
        assert!(position(app.world(), root, Vec2::ZERO).unwrap().is_none());
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetrySliderState>();
        assert!(
            matches!(position(app.world(), root, Vec2::ZERO).unwrap_err().kind(), SliderError::MissingPart { root: owner, entity, component: "WidgetrySliderState" } if *owner == root && *entity == root)
        );
        app.world_mut().despawn(root);
        assert!(position(app.world(), root, Vec2::ZERO).unwrap().is_none());
        let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider }).unwrap();
        app.world_mut()
            .entity_mut(root)
            .remove::<crate::style::Parts>();
        assert!(matches!(
            position(app.world(), root, Vec2::ZERO).unwrap_err().kind(),
            SliderError::MissingPart {
                component: "Parts",
                ..
            }
        ));
    }

    #[derive(Resource, Default)]
    struct Ended(Vec<SliderDragEnded>);

    #[test]
    fn broken_projection_cancels_drag_before_forwarding_the_original_failure() {
        use bevy::picking::{events::Pointer, pointer::Location};
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
        for missing_state in [false, true] {
            let mut app = bevy_widgetry_test_utils::scene_app();
            app.init_resource::<PointerCaptureMap>()
                .init_resource::<Ended>();
            app.add_observer(moved).add_observer(
                |event: On<SliderDragEnded>, mut ended: ResMut<Ended>| ended.0.push(*event),
            );
            app.set_error_handler(ErrorCapture::handler());
            let window = app.world_mut().spawn(Window::default()).id();
            let target =
                bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap();
            let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider }).unwrap();
            let pointer = PointerId::Mouse;
            let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
            app.world_mut().entity_mut(root).insert((
                Pressed,
                DragState(Some(Drag {
                    pointer,
                    target: target.clone(),
                    gesture: Gesture::new(0.0, 0.0),
                    offset: 0.0,
                })),
            ));
            app.world_mut()
                .resource_mut::<PointerCaptureMap>()
                .capture(pointer, root, hit.clone());
            if missing_state {
                app.world_mut()
                    .entity_mut(root)
                    .remove::<WidgetrySliderState>();
            } else {
                app.world_mut()
                    .entity_mut(root)
                    .remove::<crate::style::Parts>();
            }
            let errors = ErrorCapture::default();
            let logs = LogCapture::default();
            logs.run(|| {
                errors.run(|| {
                    app.world_mut().trigger(PointerMove {
                        entity: root,
                        pointer: Pointer::new(
                            pointer,
                            Location {
                                target,
                                position: Vec2::ONE,
                            },
                        ),
                        hit,
                        delta: Vec2::ONE,
                    });
                    app.world_mut().flush();
                })
            });
            assert!(app.world().get::<DragState>(root).unwrap().0.is_none());
            assert!(app.world().get::<Pressed>(root).is_none());
            assert!(
                !app.world()
                    .resource::<PointerCaptureMap>()
                    .is_captured(&pointer)
            );
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(logs.records().len(), 1);
            let ended = &app.world().resource::<Ended>().0;
            if missing_state {
                assert!(ended.is_empty());
            } else {
                assert_eq!(ended.len(), 1);
                assert_eq!(ended[0].outcome, SliderDragOutcome::Cancelled);
                assert_eq!(ended[0].value, 0.0);
            }
        }
    }
}
