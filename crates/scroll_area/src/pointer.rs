use crate::style::ScrollAreaThumb;
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::EntityHashSet,
    picking::{
        PickingSystems,
        events::{PointerCancel, PointerDragEnd, PointerDragStart, pointer_events},
        pointer::{PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    ui_widgets::{Scrollbar, ScrollbarDragState},
};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;

#[derive(Component)]
struct ThumbOwner {
    pointer: PointerId,
    button: bevy::picking::pointer::PointerButton,
    target: NormalizedRenderTarget,
    fresh: bool,
    scrollbar: Entity,
    viewport: Entity,
}

#[derive(Resource, Default)]
struct IgnoredTerminals(EntityHashSet);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<IgnoredTerminals>()
        .add_message::<PointerInput>()
        .add_observer(capture_drag)
        .add_observer(guard_cancel)
        .add_observer(guard_end)
        .add_systems(
            PreUpdate,
            clear_ignored
                .in_set(PickingSystems::Hover)
                .before(pointer_events),
        )
        .add_systems(PreUpdate, finish_drag.after(PickingSystems::Last));
}

fn capture_drag(
    event: On<PointerDragStart>,
    thumbs: Query<&ChildOf, With<ScrollAreaThumb>>,
    bars: Query<&Scrollbar>,
    pointers: WidgetryPointerQuery,
    mut commands: Commands,
) {
    let Ok(parent) = thumbs.get(event.entity) else {
        return;
    };
    let Ok(bar) = bars.get(parent.parent()) else {
        return;
    };
    if pointers
        .location(event.pointer.id)
        .is_none_or(|location| location.target != event.pointer.target)
    {
        return;
    }
    commands.entity(event.entity).insert(ThumbOwner {
        pointer: event.pointer.id,
        button: event.button,
        target: event.pointer.target.clone(),
        fresh: true,
        scrollbar: parent.parent(),
        viewport: bar.target,
    });
}

fn clear_ignored(mut ignored: ResMut<IgnoredTerminals>) {
    ignored.0.clear();
}

pub(crate) fn cancel_bar(world: &mut World, bar: Entity) {
    let children = world
        .get::<Children>(bar)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    for child in children {
        if world.get::<ThumbOwner>(child).is_some() {
            if let Some(mut drag) = world.get_mut::<ScrollbarDragState>(child) {
                drag.dragging = false;
            }
            world.entity_mut(child).remove::<ThumbOwner>();
        }
    }
}

fn guard_cancel(
    event: On<PointerCancel>,
    owners: Query<&ThumbOwner>,
    mut ignored: ResMut<IgnoredTerminals>,
) {
    if owners.get(event.entity).is_ok_and(|owner| {
        owner.pointer != event.pointer.id || owner.target != event.pointer.target
    }) {
        ignored.0.insert(event.entity);
    }
}

fn guard_end(
    event: On<PointerDragEnd>,
    owners: Query<&ThumbOwner>,
    mut ignored: ResMut<IgnoredTerminals>,
) {
    if owners.get(event.entity).is_ok_and(|owner| {
        owner.pointer != event.pointer.id
            || owner.target != event.pointer.target
            || owner.button != event.button
    }) {
        ignored.0.insert(event.entity);
    }
}

fn finish_drag(
    mut input: MessageReader<PointerInput>,
    pointers: WidgetryPointerQuery,
    ignored: Res<IgnoredTerminals>,
    mut thumbs: Query<(Entity, &mut ThumbOwner, &mut ScrollbarDragState)>,
    bars: Query<&Scrollbar>,
    viewports: Query<(), With<bevy::ui::ScrollPosition>>,
    mut commands: Commands,
) {
    let inputs = input.read().collect::<Vec<_>>();
    for (entity, mut owner, mut drag) in &mut thumbs {
        let invalid = pointers
            .location(owner.pointer)
            .is_none_or(|location| location.target != owner.target)
            || !bars
                .get(owner.scrollbar)
                .is_ok_and(|bar| bar.target == owner.viewport)
            || !viewports.contains(owner.viewport);
        let mut ended = false;
        for input in &inputs {
            if input.pointer_id != owner.pointer || input.location.target != owner.target {
                continue;
            }
            match input.action {
                PointerAction::Cancel => ended = true,
                PointerAction::Release(button) if button == owner.button => ended = true,
                PointerAction::Press(button) if button == owner.button && owner.fresh => {
                    ended = false
                }
                _ => {}
            }
        }
        if owner.fresh {
            owner.fresh = false;
        }
        if !invalid && !ended && ignored.0.contains(&entity) {
            if !drag.dragging {
                drag.dragging = true;
            }
        } else if invalid || ended || !drag.dragging {
            if drag.dragging {
                drag.dragging = false;
            }
            commands.entity(entity).remove::<ThumbOwner>();
        }
    }
}
