use super::WidgetryPointerQuery;
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::{EntityHashMap, EntityHashSet},
    picking::{
        PickingSystems,
        events::{Cancel, DragEnd, Pointer, Press, Release, pointer_events},
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    ui::{InteractionDisabled, Pressed},
};

#[derive(Component, Default, Clone)]
pub struct WidgetryPointerPressed;

#[derive(Component, Clone)]
struct PressOwner {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    button: PointerButton,
}

#[derive(Resource, Default)]
struct PressFrame {
    preexisting: EntityHashSet,
    inputs: Vec<PointerInput>,
    owners: EntityHashMap<PressOwner>,
    ignored: EntityHashSet,
    started: EntityHashSet,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<PressFrame>()
        .add_message::<PointerInput>()
        .add_observer(capture_press)
        .add_observer(forget_removed_press)
        .add_observer(guard_cancel)
        .add_observer(guard_release)
        .add_observer(guard_drag_end)
        .add_systems(
            PreUpdate,
            capture_frame
                .in_set(PickingSystems::Hover)
                .before(pointer_events),
        )
        .add_systems(PreUpdate, finish_presses.after(PickingSystems::Last));
}

fn capture_frame(
    pressed: Query<(Entity, Has<PressOwner>), (With<WidgetryPointerPressed>, With<Pressed>)>,
    owners: Query<(Entity, &PressOwner)>,
    mut inputs: MessageReader<PointerInput>,
    mut frame: ResMut<PressFrame>,
) {
    frame.preexisting.clear();
    frame.preexisting.extend(
        pressed
            .iter()
            .filter_map(|(entity, owned)| (!owned).then_some(entity)),
    );
    frame.inputs.clear();
    frame.inputs.extend(inputs.read().cloned());
    frame.owners.clear();
    frame
        .owners
        .extend(owners.iter().map(|(entity, owner)| (entity, owner.clone())));
    frame.ignored.clear();
    frame.started.clear();
}

fn capture_press(
    event: On<Pointer<Press>>,
    controls: Query<Has<InteractionDisabled>, With<WidgetryPointerPressed>>,
    owners: Query<(), With<PressOwner>>,
    pointers: WidgetryPointerQuery,
    mut frame: ResMut<PressFrame>,
    mut commands: Commands,
) {
    if !controls.get(event.entity).is_ok_and(|disabled| !disabled)
        || owners.contains(event.entity)
        || frame.preexisting.contains(&event.entity)
        || pointers
            .location(event.pointer_id)
            .is_none_or(|location| location.target != event.pointer_location.target)
        || !frame.inputs.iter().any(|input| {
            input.pointer_id == event.pointer_id
                && input.location.target == event.pointer_location.target
                && matches!(input.action, PointerAction::Press(button) if button==event.button)
        })
    {
        return;
    }
    let owner = PressOwner {
        pointer: event.pointer_id,
        target: event.pointer_location.target.clone(),
        button: event.button,
    };
    frame.owners.insert(event.entity, owner.clone());
    frame.started.insert(event.entity);
    commands.entity(event.entity).insert(owner);
}

fn forget_removed_press(
    event: On<Remove, Pressed>,
    owners: Query<(), With<PressOwner>>,
    mut commands: Commands,
) {
    if owners.contains(event.entity) {
        let entity = event.entity;
        commands.queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.remove::<PressOwner>();
            }
        });
    }
}

fn guard_terminal(
    entity: Entity,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
    button: Option<PointerButton>,
    frame: &mut PressFrame,
) {
    if frame.owners.get(&entity).is_some_and(|owner| {
        owner.pointer != pointer
            || owner.target != *target
            || button.is_some_and(|button| button != owner.button)
    }) {
        frame.ignored.insert(entity);
    }
}

fn guard_cancel(event: On<Pointer<Cancel>>, mut frame: ResMut<PressFrame>) {
    guard_terminal(
        event.entity,
        event.pointer_id,
        &event.pointer_location.target,
        None,
        &mut frame,
    );
}

fn guard_release(event: On<Pointer<Release>>, mut frame: ResMut<PressFrame>) {
    guard_terminal(
        event.entity,
        event.pointer_id,
        &event.pointer_location.target,
        Some(event.button),
        &mut frame,
    );
}

fn guard_drag_end(event: On<Pointer<DragEnd>>, mut frame: ResMut<PressFrame>) {
    guard_terminal(
        event.entity,
        event.pointer_id,
        &event.pointer_location.target,
        Some(event.button),
        &mut frame,
    );
}

fn finish_presses(
    pointers: WidgetryPointerQuery,
    frame: Res<PressFrame>,
    controls: Query<
        (Has<Pressed>, Has<InteractionDisabled>, Has<PressOwner>),
        With<WidgetryPointerPressed>,
    >,
    mut commands: Commands,
) {
    for (&entity, owner) in &frame.owners {
        let Ok((pressed, disabled, owned)) = controls.get(entity) else {
            continue;
        };
        let ignored = frame.ignored.contains(&entity);
        if !owned && !ignored {
            continue;
        }
        let invalid = pointers
            .location(owner.pointer)
            .is_none_or(|location| location.target != owner.target);
        let mut ended = false;
        for input in &frame.inputs {
            if input.pointer_id != owner.pointer || input.location.target != owner.target {
                continue;
            }
            match input.action {
                PointerAction::Cancel => ended = true,
                PointerAction::Release(button) if button == owner.button => ended = true,
                PointerAction::Press(button)
                    if button == owner.button && frame.started.contains(&entity) =>
                {
                    ended = false
                }
                _ => {}
            }
        }
        if disabled || invalid || ended {
            commands.entity(entity).remove::<(Pressed, PressOwner)>();
        } else if ignored && !pressed {
            commands.entity(entity).insert((Pressed, owner.clone()));
        } else if !pressed {
            commands.entity(entity).remove::<PressOwner>();
        }
    }
}
