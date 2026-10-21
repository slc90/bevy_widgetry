use super::{
    MeasurementState,
    gesture::{self, Edit},
};
use crate::Waveform;
use crate::view::{MeasurementOwner, MeasurementParts};
use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        PickingSystems,
        events::{PointerCancel, PointerDragEnd, PointerMove, PointerPress, PointerRelease},
        hover::PointerCaptureMap,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    ui::{ComputedUiRenderTargetInfo, InteractionDisabled, UiGlobalTransform},
};

#[derive(Clone)]
pub(super) struct Drag {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    surface: Entity,
    edit: Edit,
    original: Rect,
    anchor: Vec2,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<PointerCaptureMap>()
        .add_observer(press)
        .add_observer(moved)
        .add_observer(release)
        .add_observer(cancel)
        .add_observer(drag_end)
        .add_observer(focus_lost)
        .add_observer(removed)
        .add_systems(PreUpdate, cancel_unavailable.after(PickingSystems::Last))
        .add_systems(
            PostUpdate,
            cancel_unavailable.after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled),
        );
}
fn available(world: &World, root: Entity) -> bool {
    if !world.entities().contains(root) {
        return false;
    }
    let mut current = root;
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
fn focused(world: &World, target: &NormalizedRenderTarget) -> bool {
    match target {
        NormalizedRenderTarget::Window(window) => world
            .get::<Window>(window.entity())
            .is_some_and(|window| window.focused),
        _ => true,
    }
}
fn position(world: &World, surface: Entity, point: Vec2) -> Option<Vec2> {
    let node = world.get::<ComputedNode>(surface)?;
    let transform = world.get::<UiGlobalTransform>(surface)?;
    let target = world.get::<ComputedUiRenderTargetInfo>(surface)?;
    let camera = world
        .get::<bevy::ui::ComputedUiTargetCamera>(surface)?
        .get()?;
    let viewport_origin = world
        .get::<Camera>(camera)?
        .physical_viewport_rect()?
        .min
        .as_vec2();
    let ui_scale = world.get_resource::<UiScale>().map_or(1.0, |scale| scale.0);
    let local = transform
        .try_inverse()?
        .transform_point2(point * target.scale_factor() / ui_scale - viewport_origin);
    let local = (local + node.size() * 0.5) * node.inverse_scale_factor();
    (point.is_finite() && local.is_finite()).then_some(local)
}
fn press(
    mut event: On<PointerPress>,
    owners: Query<&MeasurementOwner>,
    parts: Query<&MeasurementParts>,
    mut commands: Commands,
) {
    let Ok(owner) = owners.get(event.entity) else {
        return;
    };
    if parts
        .get(owner.root)
        .is_ok_and(|parts| parts.surface != event.entity)
    {
        return;
    }
    if event.button != PointerButton::Primary {
        return;
    }
    let root = owner.root;
    event.propagate(false);
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        if !available(world, event.entity)
            || !focused(world, &event.pointer.target)
            || !Waveform::measurement_enabled(world, root)?
            || world
                .get::<MeasurementState>(root)
                .is_none_or(|state| state.drag.is_some())
            || world
                .resource::<PointerCaptureMap>()
                .is_captured(&event.pointer.id)
        {
            return Ok(());
        }
        let Some(size) = super::size(world, root) else {
            return Ok(());
        };
        if world
            .get::<crate::WaveformChannelLayout>(root)
            .is_none_or(|layout| layout.row_height <= 0.0)
        {
            return Ok(());
        }
        let Some(anchor) = position(world, event.entity, event.pointer.position) else {
            return Ok(());
        };
        if anchor.cmplt(Vec2::ZERO).any() || anchor.cmpgt(size).any() {
            return Ok(());
        }
        let mut state = world.get_mut::<MeasurementState>(root).ok_or_else(|| {
            bevy_widgetry_core::scene::logged_error("Waveform measurement state missing")
        })?;
        let edit = gesture::hit(state.rectangle, anchor);
        let original = if edit == Edit::Draw {
            Rect::from_corners(anchor, anchor)
        } else {
            state.rectangle.unwrap_or_default()
        };
        state.rectangle = Some(original);
        state.drag = Some(Drag {
            pointer: event.pointer.id,
            target: event.pointer.target.clone(),
            surface: event.entity,
            edit,
            original,
            anchor,
        });
        world.resource_mut::<PointerCaptureMap>().capture(
            event.pointer.id,
            event.entity,
            event.hit,
        );
        world
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(event.entity, bevy::input_focus::FocusCause::Pressed);
        super::publish(world, root)
    });
}
fn moved(event: On<PointerMove>, owners: Query<&MeasurementOwner>, mut commands: Commands) {
    let Ok(owner) = owners.get(event.entity) else {
        return;
    };
    let root = owner.root;
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        let Some(drag) = world
            .get::<MeasurementState>(root)
            .and_then(|state| state.drag.clone())
        else {
            return Ok(());
        };
        if drag.surface != event.entity
            || drag.pointer != event.pointer.id
            || drag.target != event.pointer.target
        {
            return Ok(());
        }
        if !available(world, event.entity)
            || !focused(world, &drag.target)
            || !Waveform::measurement_enabled(world, root)?
        {
            end(world, root);
            return Ok(());
        }
        let Some(point) = position(world, event.entity, event.pointer.position) else {
            end(world, root);
            return Ok(());
        };
        let Some(size) = super::size(world, root) else {
            end(world, root);
            return Ok(());
        };
        let rectangle = gesture::advance(drag.edit, drag.original, drag.anchor, point, size);
        world
            .get_mut::<MeasurementState>(root)
            .ok_or_else(|| {
                bevy_widgetry_core::scene::logged_error("Waveform measurement state missing")
            })?
            .rectangle = Some(rectangle);
        super::publish(world, root)
    });
}
pub(super) fn end(world: &mut World, root: Entity) {
    let drag = world
        .get_mut::<MeasurementState>(root)
        .and_then(|mut state| state.drag.take());
    if let Some(drag) = drag {
        release_capture(&mut world.resource_mut::<PointerCaptureMap>(), &drag);
    }
}
fn release_capture(capture: &mut PointerCaptureMap, drag: &Drag) {
    if capture
        .get(&drag.pointer)
        .is_some_and(|(entity, _)| entity == drag.surface)
    {
        capture.release(drag.pointer);
    }
}
fn terminal(
    commands: &mut Commands,
    root: Entity,
    surface: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
) {
    commands.queue(move |world: &mut World| {
        if world
            .get::<MeasurementState>(root)
            .and_then(|state| state.drag.as_ref())
            .is_some_and(|drag| {
                drag.surface == surface && drag.pointer == pointer && drag.target == target
            })
        {
            end(world, root);
        }
    });
}
fn release(event: On<PointerRelease>, owners: Query<&MeasurementOwner>, mut commands: Commands) {
    if event.button == PointerButton::Primary
        && let Ok(owner) = owners.get(event.entity)
    {
        terminal(
            &mut commands,
            owner.root,
            event.entity,
            event.pointer.id,
            event.pointer.target.clone(),
        );
    }
}
fn cancel(event: On<PointerCancel>, owners: Query<&MeasurementOwner>, mut commands: Commands) {
    if let Ok(owner) = owners.get(event.entity) {
        terminal(
            &mut commands,
            owner.root,
            event.entity,
            event.pointer.id,
            event.pointer.target.clone(),
        );
    }
}
fn drag_end(event: On<PointerDragEnd>, owners: Query<&MeasurementOwner>, mut commands: Commands) {
    if event.button == PointerButton::Primary
        && let Ok(owner) = owners.get(event.entity)
    {
        terminal(
            &mut commands,
            owner.root,
            event.entity,
            event.pointer.id,
            event.pointer.target.clone(),
        );
    }
}
fn focus_lost(
    event: On<bevy::input_focus::FocusLost>,
    owners: Query<&MeasurementOwner>,
    parts: Query<&MeasurementParts>,
    mut commands: Commands,
) {
    if let Ok(owner) = owners.get(event.entity)
        && parts
            .get(owner.root)
            .is_ok_and(|parts| parts.surface == event.entity)
    {
        let root = owner.root;
        let surface = event.entity;
        commands.queue(move |world: &mut World| {
            if world.resource::<bevy::input_focus::InputFocus>().get() != Some(surface) {
                end(world, root);
            }
        });
    }
}
fn removed(
    event: On<Remove<MeasurementState>>,
    states: Query<&MeasurementState>,
    mut capture: ResMut<PointerCaptureMap>,
) {
    if let Ok(state) = states.get(event.entity)
        && let Some(drag) = &state.drag
    {
        release_capture(&mut capture, drag);
    }
}
fn cancel_unavailable(world: &mut World) -> Result {
    let active = world
        .query::<(Entity, &MeasurementState)>()
        .iter(world)
        .filter_map(|(root, state)| state.drag.clone().map(|drag| (root, drag)))
        .collect::<Vec<_>>();
    for (root, drag) in active {
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
            .is_some_and(|(owner, _)| owner == drag.surface);
        if !valid
            || !captured
            || !available(world, drag.surface)
            || !focused(world, &drag.target)
            || !Waveform::measurement_enabled(world, root)?
        {
            end(world, root);
        }
    }
    Ok(())
}
