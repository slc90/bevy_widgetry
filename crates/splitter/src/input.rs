use crate::{
    SplitterError, SplitterFailure,
    error::{failure, missing},
};
use crate::{
    WidgetrySplitter,
    splitter::{Parts, State},
};
use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        PickingSystems,
        events::{PointerCancel, PointerDrag, PointerDragEnd, PointerPress, PointerRelease},
        hover::PointerCaptureMap,
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    ui::{ComputedUiRenderTargetInfo, InteractionDisabled, Pressed, UiGlobalTransform},
};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, WidgetryEcsError},
};
use std::sync::Arc;

#[derive(Component, Default, Clone)]
#[require(bevy::picking::hover::Hovered, DragState)]
pub(crate) struct Handle;

#[derive(Component, Default)]
#[component(on_remove=release_capture)]
struct DragState(Option<Drag>);
#[derive(Clone)]
struct Drag {
    pointer: PointerId,
    target: NormalizedRenderTarget,
}

#[derive(Resource, Default)]
struct ReleasingCaptures(Vec<(PointerId, Entity, NormalizedRenderTarget)>);

fn remember_releasing_captures(
    mut inputs: MessageReader<PointerInput>,
    captures: Res<PointerCaptureMap>,
    mut releasing: ResMut<ReleasingCaptures>,
) {
    releasing.0.clear();
    for input in inputs.read() {
        if matches!(input.action, PointerAction::Release(PointerButton::Primary))
            && let Some((owner, _)) = captures.get(&input.pointer_id)
        {
            releasing
                .0
                .push((input.pointer_id, owner, input.location.target.clone()));
        }
    }
}

fn forget_releasing_captures(mut releasing: ResMut<ReleasingCaptures>) {
    releasing.0.clear();
}

fn release_capture(
    mut world: bevy::ecs::world::DeferredWorld,
    context: bevy::ecs::lifecycle::HookContext,
) {
    if let Some(mut map) = world.get_resource_mut::<PointerCaptureMap>() {
        map.0.retain(|_, (owner, _)| *owner != context.entity);
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PointerCaptureMap>()
        .init_resource::<ReleasingCaptures>()
        .add_message::<PointerInput>()
        .add_observer(press)
        .add_observer(moved)
        .add_observer(released)
        .add_observer(cancelled)
        .add_observer(drag_ended)
        .add_observer(disabled)
        .add_systems(
            PreUpdate,
            remember_releasing_captures
                .after(PickingSystems::Backend)
                .before(PickingSystems::Hover),
        )
        .add_systems(
            PreUpdate,
            (forget_releasing_captures, cancel_unavailable)
                .chain()
                .after(PickingSystems::Last),
        )
        .add_systems(
            PostUpdate,
            cancel_unavailable
                .after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                .before(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
}

fn queue(
    commands: &mut Commands,
    command: impl FnOnce(&mut World) -> Result<(), SplitterFailure> + Send + Sync + 'static,
) {
    commands
        .queue(move |world: &mut World| command(world).map_err(|error| error.into_bevy_error()));
}

fn owner(world: &World, handle: Entity) -> Result<Option<Entity>, SplitterFailure> {
    if world.get::<Handle>(handle).is_none() {
        return Ok(None);
    }
    let root = world
        .get::<ChildOf>(handle)
        .ok_or_else(|| missing(None, handle, "ChildOf"))?
        .parent();
    if world.get::<WidgetrySplitter>(root).is_none() {
        return Err(failure(
            SplitterError::InvalidOwner {
                root: Some(root),
                entity: handle,
                parent: Some(root),
                part: "Handle",
            },
            ErrorFrame::new("splitter.drag_owner"),
        ));
    }
    if world.get::<State>(root).is_none() {
        return Err(missing(Some(root), root, "State"));
    }
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(Some(root), root, "Parts"))?;
    if parts.handle != handle {
        return Err(failure(
            SplitterError::InvalidOwner {
                root: Some(root),
                entity: handle,
                parent: Some(root),
                part: "Parts.handle",
            },
            ErrorFrame::new("splitter.drag_owner").field("expected_handle", parts.handle),
        ));
    }
    Ok(Some(root))
}

fn available(world: &World, handle: Entity, root: Entity) -> bool {
    if world.get::<State>(root).is_none_or(|s| !s.resizable) {
        return false;
    }
    let mut current = handle;
    let mut visible = false;
    loop {
        if world.get::<InteractionDisabled>(current).is_some()
            || world
                .get::<Node>(current)
                .is_some_and(|n| n.display == Display::None)
        {
            return false;
        }
        if !visible {
            match world.get::<Visibility>(current) {
                Some(Visibility::Hidden) => return false,
                Some(Visibility::Visible) => visible = true,
                _ => {}
            }
        }
        match world.get::<ChildOf>(current) {
            Some(p) => current = p.parent(),
            None => return true,
        }
    }
}

fn focused(world: &World, target: &NormalizedRenderTarget) -> bool {
    match target {
        NormalizedRenderTarget::Window(w) => {
            world.get::<Window>(w.entity()).is_some_and(|w| w.focused)
        }
        _ => true,
    }
}

fn delta(world: &World, root: Entity, delta: Vec2) -> Option<f32> {
    if !delta.is_finite() {
        return None;
    }
    let state = world.get::<State>(root)?;
    let transform = world.get::<UiGlobalTransform>(root)?.try_inverse()?;
    let scale = world.get_resource::<UiScale>().map_or(1.0, |s| s.0);
    let target = world.get::<ComputedUiRenderTargetInfo>(root)?;
    let computed = world.get::<ComputedNode>(root)?;
    let local = transform.transform_vector2(delta * target.scale_factor() / scale)
        * computed.inverse_scale_factor;
    let delta = state.orientation.main(local);
    delta.is_finite().then_some(delta)
}

fn geometry_error(handle: Entity, error: SplitterFailure) -> SplitterFailure {
    error.with_context(ErrorFrame::new("splitter.drag_geometry").field("handle", handle))
}

fn press(mut event: On<PointerPress>, handles: Query<(), With<Handle>>, mut commands: Commands) {
    if !handles.contains(event.entity) {
        return;
    }
    event.propagate(false);
    let event = event.event().clone();
    queue(
        &mut commands,
        move |world: &mut World| -> Result<(), SplitterFailure> {
            let Some(root) = owner(world, event.entity)? else {
                return Ok(());
            };
            if event.button != bevy::picking::pointer::PointerButton::Primary
                || !event.pointer.position.is_finite()
                || !available(world, event.entity, root)
                || !focused(world, &event.pointer.target)
            {
                return Ok(());
            }
            let drag = world
                .get::<DragState>(event.entity)
                .ok_or_else(|| missing(Some(root), event.entity, "DragState"))?;
            if drag.0.is_some()
                || world
                    .resource::<PointerCaptureMap>()
                    .is_captured(&event.pointer.id)
            {
                return Ok(());
            }
            if crate::layout::ratio_bounds(world, root)
                .map_err(|e| geometry_error(event.entity, e))?
                .is_none()
                || delta(world, root, Vec2::ZERO).is_none()
            {
                return Ok(());
            }
            world.entity_mut(event.entity).insert((
                DragState(Some(Drag {
                    pointer: event.pointer.id,
                    target: event.pointer.target.clone(),
                })),
                Pressed,
            ));
            world.resource_mut::<PointerCaptureMap>().capture(
                event.pointer.id,
                event.entity,
                event.hit,
            );
            Ok(())
        },
    );
}

fn moved(event: On<PointerDrag>, handles: Query<(), With<Handle>>, mut commands: Commands) {
    if !handles.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        let Some(drag) = world
            .get::<DragState>(event.entity)
            .and_then(|s| s.0.clone())
        else {
            return Ok(());
        };
        if drag.pointer != event.pointer.id || drag.target != event.pointer.target {
            return Ok(());
        }
        let result = (|| -> Result<(), SplitterFailure> {
            let Some(root) = owner(world, event.entity)? else {
                return end(world, event.entity);
            };
            let captured = world
                .resource::<PointerCaptureMap>()
                .get(&drag.pointer)
                .is_some_and(|(owner, _)| owner == event.entity);
            // Bevy releases captures before dispatching the frame's ordered Move/Release events.
            // Drag events still reach the press owner even when the final move ends outside it.
            let releasing = !world
                .resource::<PointerCaptureMap>()
                .is_captured(&drag.pointer)
                && world
                    .resource::<ReleasingCaptures>()
                    .0
                    .iter()
                    .any(|(id, owner, target)| {
                        *id == drag.pointer && *owner == event.entity && *target == drag.target
                    });
            if (!captured && !releasing)
                || !event.pointer.position.is_finite()
                || !available(world, event.entity, root)
                || !focused(world, &drag.target)
            {
                return end(world, event.entity);
            }
            let Some(delta) = delta(world, root, event.delta) else {
                return end(world, event.entity);
            };
            if delta == 0.0 {
                return Ok(());
            }
            let Some(bounds) = crate::layout::ratio_bounds(world, root)
                .map_err(|e| geometry_error(event.entity, e))?
            else {
                return end(world, event.entity);
            };
            let current = WidgetrySplitter::split_ratio(world, root)?;
            let next = (bounds.clamp(current) + delta / bounds.available).clamp(0.0, 1.0);
            crate::ratio::commit(world, root, next, false)?;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let mut failure = WidgetryEcsError::new(error);
                if let Err(cleanup) = end(world, event.entity) {
                    failure.push_shared(Arc::new(cleanup));
                }
                Err(failure.into_bevy_error())
            }
        }
    });
}

fn end(world: &mut World, handle: Entity) -> Result<(), SplitterFailure> {
    let root = world.get::<ChildOf>(handle).map(ChildOf::parent);
    if !world.entities().contains(handle) {
        return Ok(());
    }
    let drag = world
        .get_mut::<DragState>(handle)
        .ok_or_else(|| missing(root, handle, "DragState"))?
        .0
        .take();
    if let Some(drag) = drag {
        world.entity_mut(handle).remove::<Pressed>();
        let mut map = world
            .get_resource_mut::<PointerCaptureMap>()
            .ok_or_else(|| missing(root, handle, "PointerCaptureMap"))?;
        if map
            .get(&drag.pointer)
            .is_some_and(|(owner, _)| owner == handle)
        {
            map.release(drag.pointer);
        }
    }
    Ok(())
}

fn terminal(
    world: &mut World,
    handle: Entity,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
) -> Result<(), SplitterFailure> {
    if world
        .get::<DragState>(handle)
        .and_then(|s| s.0.as_ref())
        .is_some_and(|d| d.pointer == pointer && d.target == *target)
    {
        end(world, handle)?;
    }
    Ok(())
}
fn released(event: On<PointerRelease>, mut commands: Commands) {
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    let (e, id, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    queue(&mut commands, move |world: &mut World| {
        terminal(world, e, id, &target)
    });
}
fn cancelled(event: On<PointerCancel>, mut commands: Commands) {
    let (e, id, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    queue(&mut commands, move |world: &mut World| {
        terminal(world, e, id, &target)
    });
}
fn drag_ended(event: On<PointerDragEnd>, mut commands: Commands) {
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    let (e, id, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    queue(&mut commands, move |world: &mut World| {
        terminal(world, e, id, &target)
    });
}
fn disabled(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    mut commands: Commands,
) {
    let e = event.entity;
    queue(
        &mut commands,
        move |world: &mut World| -> Result<(), SplitterFailure> {
            if let Some(root) = owner(world, e)?
                && !available(world, e, root)
            {
                end(world, e)?;
            }
            Ok(())
        },
    );
}
fn cancel_unavailable(world: &mut World) -> Result {
    let mut batch = DiagnosticBatch::default();
    let active: Vec<_> = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(e, s)| s.0.clone().map(|d| (e, d)))
        .collect();
    for (handle, drag) in active {
        let root = match owner(world, handle) {
            Ok(root) => root,
            Err(error) => {
                batch.push(error);
                if let Err(cleanup) = end(world, handle) {
                    batch.push(cleanup);
                }
                continue;
            }
        };
        let present = world
            .query::<(&PointerId, &bevy::picking::pointer::PointerLocation)>()
            .iter(world)
            .any(|(id, p)| {
                *id == drag.pointer
                    && p.location
                        .as_ref()
                        .is_some_and(|p| p.target == drag.target && p.position.is_finite())
            });
        let captured = world
            .resource::<PointerCaptureMap>()
            .get(&drag.pointer)
            .is_some_and(|(e, _)| e == handle);
        let unavailable = !present
            || !captured
            || !focused(world, &drag.target)
            || root.is_none_or(|root| !available(world, handle, root));
        if unavailable && let Err(error) = end(world, handle) {
            batch.push(error);
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

impl WidgetrySplitter {
    pub fn resizable(world: &World, root: Entity) -> Result<bool, SplitterFailure> {
        Self::split_ratio(world, root)?;
        world
            .get::<State>(root)
            .map(|s| s.resizable)
            .ok_or_else(|| missing(Some(root), root, "State"))
    }
    pub fn set_resizable_in_world(
        world: &mut World,
        root: Entity,
        resizable: bool,
    ) -> Result<bool, SplitterFailure> {
        let current = Self::resizable(world, root)?;
        let handle = world
            .get::<Parts>(root)
            .ok_or_else(|| missing(Some(root), root, "Parts"))?
            .handle;
        if owner(world, handle)? != Some(root) || world.get::<DragState>(handle).is_none() {
            return Err(missing(Some(root), handle, "Handle/DragState"));
        }
        if current == resizable {
            return Ok(false);
        }
        if !resizable {
            end(world, handle)?;
        }
        world
            .get_mut::<State>(root)
            .ok_or_else(|| missing(Some(root), root, "State"))?
            .resizable = resizable;
        Ok(true)
    }
    pub fn set_resizable(commands: &mut Commands, root: Entity, resizable: bool) {
        queue(commands, move |world: &mut World| {
            Self::set_resizable_in_world(world, root, resizable).map(|_| ())
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::picking::{backend::HitData, events::Pointer, pointer::Location};
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    use std::error::Error;

    fn app() -> App {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.init_resource::<PointerCaptureMap>();
        app.set_error_handler(ErrorCapture::handler());
        app
    }

    fn activate(world: &mut World, pointer: PointerId) -> (Entity, Entity, NormalizedRenderTarget) {
        let root = world.spawn_scene(bsn! { @WidgetrySplitter }).unwrap().id();
        let handle = world.get::<Parts>(root).unwrap().handle;
        let target = NormalizedRenderTarget::None {
            width: 1000,
            height: 1000,
        };
        world.entity_mut(handle).insert((
            Pressed,
            DragState(Some(Drag {
                pointer,
                target: target.clone(),
            })),
        ));
        world.resource_mut::<PointerCaptureMap>().capture(
            pointer,
            handle,
            HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        );
        (root, handle, target)
    }

    #[test]
    fn failed_first_owner_does_not_skip_other_capture_and_pressed_cleanup() {
        let mut app = app();
        app.add_systems(Update, cancel_unavailable);
        activate(app.world_mut(), PointerId::Touch(11));
        activate(app.world_mut(), PointerId::Touch(12));
        let active: Vec<_> = app
            .world_mut()
            .query::<(Entity, &DragState)>()
            .iter(app.world())
            .filter_map(|(entity, state)| state.0.as_ref().map(|drag| (entity, drag.pointer)))
            .collect();
        assert_eq!(active.len(), 2);
        // Corrupt the first actual query owner so an early return deterministically skips the second.
        let broken = app.world().get::<ChildOf>(active[0].0).unwrap().parent();
        app.world_mut().entity_mut(broken).remove::<Parts>();
        for (handle, pointer) in &active {
            assert!(app.world().get::<Pressed>(*handle).is_some());
            assert!(
                app.world()
                    .resource::<PointerCaptureMap>()
                    .is_captured(pointer)
            );
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.world_mut().run_schedule(Update));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let failure = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(
            matches!(failure.source().unwrap().downcast_ref::<SplitterError>(), Some(SplitterError::MissingPart { root: Some(root), entity, part: "Parts" }) if *root == broken && *entity == broken)
        );
        for (handle, pointer) in active {
            assert!(app.world().get::<Pressed>(handle).is_none());
            assert!(app.world().get::<DragState>(handle).unwrap().0.is_none());
            assert!(
                !app.world()
                    .resource::<PointerCaptureMap>()
                    .is_captured(&pointer)
            );
        }
    }

    #[test]
    fn moved_preserves_primary_and_cleanup_failures_at_the_command_boundary() {
        let mut app = app();
        app.add_observer(moved);
        let pointer = PointerId::Touch(11);
        let (root, handle, target) = activate(app.world_mut(), pointer);
        app.world_mut().entity_mut(root).remove::<Parts>();
        app.world_mut().remove_resource::<PointerCaptureMap>();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.world_mut().trigger(PointerDrag {
                    entity: handle,
                    pointer: Pointer::new(
                        pointer,
                        Location {
                            target,
                            position: Vec2::splat(10.0),
                        },
                    ),
                    button: PointerButton::Primary,
                    distance: Vec2::X,
                    delta: Vec2::X,
                });
                app.world_mut().flush();
            })
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        assert!(matches!(
            records[0].context,
            bevy::ecs::error::ErrorContext::Command { .. }
        ));
        let failures: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
        assert_eq!(failures.len(), 2);
        assert!(!Arc::ptr_eq(failures[0].origin(), failures[1].origin()));
        assert!(
            matches!(failures[0].source().unwrap().downcast_ref::<SplitterError>(), Some(SplitterError::MissingPart { root: Some(actual), entity, part: "Parts" }) if *actual == root && *entity == root)
        );
        assert!(
            matches!(failures[1].source().unwrap().downcast_ref::<SplitterError>(), Some(SplitterError::MissingPart { root: Some(actual), entity, part: "PointerCaptureMap" }) if *actual == root && *entity == handle)
        );
        for diagnostic in failures {
            let original: &(dyn Error + 'static) = diagnostic;
            let original = original.downcast_ref::<SplitterFailure>().unwrap();
            assert!(Arc::ptr_eq(original.origin(), diagnostic.origin()));
            assert_eq!(
                logs.diagnostic_records(original.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
        assert!(app.world().get::<Pressed>(handle).is_none());
        assert!(app.world().get::<DragState>(handle).unwrap().0.is_none());
    }
}
