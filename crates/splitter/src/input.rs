use crate::{
    WidgetrySplitter,
    splitter::{Parts, State, invalid},
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

fn broken(handle: Entity, reason: &str) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?handle, reason, "Splitter 拖动内部结构失效");
    bevy_widgetry_core::scene::logged_error(reason)
}

fn owner(world: &World, handle: Entity) -> Result<Option<Entity>, BevyError> {
    if world.get::<Handle>(handle).is_none() {
        return Ok(None);
    }
    let root = world
        .get::<ChildOf>(handle)
        .ok_or_else(|| broken(handle, "Splitter Handle parent is missing"))?
        .parent();
    if world.get::<WidgetrySplitter>(root).is_none()
        || world.get::<State>(root).is_none()
        || world
            .get::<Parts>(root)
            .is_none_or(|parts| parts.handle != handle)
    {
        return Err(broken(handle, "Splitter Handle owner is invalid"));
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

fn geometry_error(handle: Entity, error: BevyError) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?handle,%error,"Splitter 拖动布局失效");
    error
}

fn press(mut event: On<PointerPress>, handles: Query<(), With<Handle>>, mut commands: Commands) {
    if !handles.contains(event.entity) {
        return;
    }
    event.propagate(false);
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
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
            .ok_or_else(|| broken(event.entity, "Splitter Handle drag state is missing"))?;
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
    });
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
        let result = (|| -> Result {
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
        if result.is_err() {
            end(world, event.entity)?;
        }
        result
    });
}

fn end(world: &mut World, handle: Entity) -> Result {
    if !world.entities().contains(handle) {
        return Ok(());
    }
    let drag = world
        .get_mut::<DragState>(handle)
        .ok_or_else(|| broken(handle, "Splitter Handle drag state is missing"))?
        .0
        .take();
    if let Some(drag) = drag {
        world.entity_mut(handle).remove::<Pressed>();
        let mut map = world
            .get_resource_mut::<PointerCaptureMap>()
            .ok_or_else(|| broken(handle, "Splitter Pointer Capture map is missing"))?;
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
) -> Result {
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
    commands.queue(move |world: &mut World| terminal(world, e, id, &target));
}
fn cancelled(event: On<PointerCancel>, mut commands: Commands) {
    let (e, id, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    commands.queue(move |world: &mut World| terminal(world, e, id, &target));
}
fn drag_ended(event: On<PointerDragEnd>, mut commands: Commands) {
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    let (e, id, target) = (event.entity, event.pointer.id, event.pointer.target.clone());
    commands.queue(move |world: &mut World| terminal(world, e, id, &target));
}
fn disabled(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    mut commands: Commands,
) {
    let e = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        if let Some(root) = owner(world, e)?
            && !available(world, e, root)
        {
            end(world, e)?;
        }
        Ok(())
    });
}
fn cancel_unavailable(world: &mut World) -> Result {
    let active: Vec<_> = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(e, s)| s.0.clone().map(|d| (e, d)))
        .collect();
    for (handle, drag) in active {
        let root = match owner(world, handle) {
            Ok(root) => root,
            Err(error) => {
                end(world, handle)?;
                return Err(error);
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
        if !present
            || !captured
            || !focused(world, &drag.target)
            || root.is_none_or(|root| !available(world, handle, root))
        {
            end(world, handle)?;
        }
    }
    Ok(())
}

impl WidgetrySplitter {
    pub fn resizable(world: &World, root: Entity) -> Result<bool, BevyError> {
        Self::split_ratio(world, root)?;
        world
            .get::<State>(root)
            .map(|s| s.resizable)
            .ok_or_else(|| invalid("Splitter runtime state is missing"))
    }
    pub fn set_resizable_in_world(
        world: &mut World,
        root: Entity,
        resizable: bool,
    ) -> Result<bool, BevyError> {
        let current = Self::resizable(world, root)?;
        let handle = world
            .get::<Parts>(root)
            .ok_or_else(|| invalid("Splitter internal parts are missing"))?
            .handle;
        if owner(world, handle)? != Some(root) || world.get::<DragState>(handle).is_none() {
            return Err(invalid("Splitter Handle structure is missing"));
        }
        if current == resizable {
            return Ok(false);
        }
        if !resizable {
            end(world, handle)?;
        }
        world
            .get_mut::<State>(root)
            .ok_or_else(|| invalid("Splitter runtime state is missing"))?
            .resizable = resizable;
        Ok(true)
    }
    pub fn set_resizable(commands: &mut Commands, root: Entity, resizable: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_resizable_in_world(world, root, resizable).map(|_| ())
        });
    }
}
