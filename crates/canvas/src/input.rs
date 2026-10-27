use crate::{CanvasDocument, CanvasDocumentChanged, CanvasView};
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::message::MessageCursor,
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, InputFocus},
    picking::{
        PickingSystems,
        backend::HitData,
        events::{
            PointerCancel, PointerDrag, PointerDragEnd, PointerPress, PointerRelease, PointerScroll,
        },
        hover::{HoverMap, PointerCaptureMap},
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    ui::{ComputedUiTargetCamera, InteractionDisabled, UiGlobalTransform, UiSystems},
};
use std::collections::HashSet;

#[derive(Component, Default)]
pub(crate) struct DragState(Option<Drag>);

#[derive(Resource, Default)]
struct FrameInputs(Vec<PointerInput>);

#[derive(Clone)]
struct Drag {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    local: Vec2,
    hit: HitData,
    raw_driven: bool,
    action: DragAction,
}

#[derive(Clone, Copy)]
enum DragAction {
    Pan,
    Scale(crate::scale::Scale),
    Rotate(crate::rotate::Rotate),
    Move {
        document: Entity,
        shape: Entity,
        offset: Vec2,
    },
    Draw {
        document: Entity,
        start: Vec2,
        current: Vec2,
        paint: crate::CanvasPaint,
    },
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PointerCaptureMap>()
        .init_resource::<FrameInputs>()
        .add_observer(press)
        .add_observer(moved)
        .add_observer(released)
        .add_observer(cancelled)
        .add_observer(drag_ended)
        .add_observer(document_changed)
        .add_observer(scrolled)
        .add_observer(removed)
        .add_observer(delete_selected)
        .add_systems(
            PreUpdate,
            process_input
                .after(bevy::picking::hover::generate_hovermap)
                .before(bevy::picking::hover::update_interactions)
                .in_set(PickingSystems::Hover),
        )
        .add_systems(PreUpdate, cancel_unavailable.after(PickingSystems::Last))
        .add_systems(PreUpdate, escape.after(PickingSystems::Last))
        .add_systems(
            PreUpdate,
            refresh_rotation
                .after(PickingSystems::Last)
                .after(cancel_unavailable),
        )
        .add_systems(
            PostUpdate,
            cancel_unavailable
                .after(UiSystems::Layout)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                .before(bevy_widgetry_core::offscreen::OffscreenUpdate),
        );
}

fn available(world: &World, entity: Entity) -> bool {
    if world
        .get::<crate::view::ViewState>(entity)
        .is_none_or(|state| {
            state
                .document
                .is_none_or(|document| world.get::<CanvasDocument>(document).is_none())
        })
        || world
            .get::<ComputedNode>(entity)
            .is_none_or(|node| node.size().min_element() <= 0.0)
    {
        return false;
    }
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

fn focused(world: &World, target: &NormalizedRenderTarget) -> bool {
    match target {
        NormalizedRenderTarget::Window(window) => world
            .get::<Window>(window.entity())
            .is_some_and(|window| window.focused),
        _ => true,
    }
}

fn position(world: &World, entity: Entity, point: Vec2) -> Option<Vec2> {
    let node = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    let camera = world.get::<ComputedUiTargetCamera>(entity)?.get()?;
    let camera = world.get::<Camera>(camera)?;
    let physical =
        point * camera.target_scaling_factor()? - camera.physical_viewport_rect()?.min.as_vec2();
    let local = transform.try_inverse()?.transform_point2(physical) * node.inverse_scale_factor();
    (point.is_finite() && local.is_finite()).then_some(local)
}

fn press(mut event: On<PointerPress>, views: Query<(), With<CanvasView>>, mut commands: Commands) {
    if event.original_event_target() != event.entity
        || !views.contains(event.entity)
        || event.button != PointerButton::Primary
    {
        return;
    }
    let press = event.event().clone();
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        if world
            .get::<DragState>(press.entity)
            .and_then(|state| state.0.as_ref())
            .is_some_and(|drag| {
                drag.pointer == press.pointer.id && drag.target == press.pointer.target
            })
        {
            // Bevy can send the preceding release to an older hover target without sending DragEnd.
            // A new Primary press must start at its own position, even when the old gesture survived that release.
            if world
                .get::<DragState>(press.entity)
                .and_then(|state| state.0.as_ref())
                .is_some_and(|drag| matches!(drag.action, DragAction::Draw { .. }))
            {
                complete_drawing(world, press.entity)?;
            } else {
                end(world, press.entity);
            }
        }
        let space = world
            .get_resource::<ButtonInput<KeyCode>>()
            .is_some_and(|keys| keys.pressed(KeyCode::Space));
        let action = if space {
            DragAction::Pan
        } else {
            let Some(local) = inside_position(
                world,
                press.entity,
                &press.pointer.target,
                press.pointer.position,
            ) else {
                return Ok(());
            };
            let zoom = CanvasView::zoom(world, press.entity)?;
            let point = CanvasView::center(world, press.entity)? + local / zoom;
            if !point.is_finite() {
                return Ok(());
            }
            if let crate::CanvasMode::Rectangle(paint) = CanvasView::mode(world, press.entity)? {
                let Some(document) = CanvasView::document(world, press.entity)? else {
                    return Ok(());
                };
                DragAction::Draw {
                    document,
                    start: point,
                    current: point,
                    paint,
                }
            } else {
                let previous = CanvasView::selection(world, press.entity)?;
                let rotation = if let Some(shape) = previous {
                    crate::rotate::hit(world, shape, point, zoom)?.then_some(shape)
                } else {
                    None
                };
                if let Some(shape) = rotation {
                    let Some(document) = CanvasView::document(world, press.entity)? else {
                        return Ok(());
                    };
                    let Some(rotate) = crate::rotate::Rotate::begin(world, document, shape, point)?
                    else {
                        return Ok(());
                    };
                    DragAction::Rotate(rotate)
                } else {
                    let handle = if let Some(shape) = previous {
                        crate::scale::hit(world, shape, point, zoom)?.map(|handle| (shape, handle))
                    } else {
                        None
                    };
                    if let Some((shape, handle)) = handle {
                        let Some(document) = CanvasView::document(world, press.entity)? else {
                            return Ok(());
                        };
                        DragAction::Scale(crate::scale::Scale::begin(
                            world, document, shape, point, handle,
                        )?)
                    } else {
                        let selection = crate::selection::hit_test(world, press.entity, point)?;
                        CanvasView::set_selection_in_world(world, press.entity, selection)?;
                        let Some(shape) = selection.filter(|_| previous == selection) else {
                            return Ok(());
                        };
                        let Some(document) = CanvasView::document(world, press.entity)? else {
                            return Ok(());
                        };
                        let Some(transform) = world.get::<Transform>(shape) else {
                            return Ok(());
                        };
                        DragAction::Move {
                            document,
                            shape,
                            offset: point - transform.translation.truncate(),
                        }
                    }
                }
            }
        };
        if !available(world, press.entity)
            || !focused(world, &press.pointer.target)
            || world
                .get::<DragState>(press.entity)
                .is_none_or(|state| state.0.is_some())
            || world
                .resource::<PointerCaptureMap>()
                .is_captured(&press.pointer.id)
        {
            return Ok(());
        }
        let Some(local) = position(world, press.entity, press.pointer.position) else {
            return Ok(());
        };
        if let Some(mut state) = world.get_mut::<DragState>(press.entity) {
            state.0 = Some(Drag {
                pointer: press.pointer.id,
                target: press.pointer.target.clone(),
                local,
                hit: press.hit.clone(),
                raw_driven: false,
                action,
            });
        }
        world.resource_mut::<PointerCaptureMap>().capture(
            press.pointer.id,
            press.entity,
            press.hit,
        );
        Ok(())
    });
}

fn moved(event: On<PointerDrag>, views: Query<(), With<CanvasView>>, mut commands: Commands) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> Result {
        let Some(drag) = world
            .get::<DragState>(event.entity)
            .and_then(|state| state.0.as_ref())
        else {
            return Ok(());
        };
        if drag.raw_driven
            || drag.pointer != event.pointer.id
            || drag.target != event.pointer.target
        {
            return Ok(());
        }
        if !world
            .resource::<PointerCaptureMap>()
            .get(&drag.pointer)
            .is_some_and(|(owner, _)| owner == event.entity)
        {
            end(world, event.entity);
            return Ok(());
        }
        advance(world, event.entity, event.pointer.position)
    });
}

fn advance(world: &mut World, entity: Entity, point: Vec2) -> Result {
    let Some(drag) = world
        .get::<DragState>(entity)
        .and_then(|state| state.0.clone())
    else {
        return Ok(());
    };
    if !continuing(world, entity, &drag) {
        end(world, entity);
        return Ok(());
    }
    let Some(local) = position(world, entity, point) else {
        end(world, entity);
        return Ok(());
    };
    let zoom = CanvasView::zoom(world, entity)?;
    if let Some(mut state) = world.get_mut::<DragState>(entity)
        && let Some(drag) = &mut state.0
    {
        drag.local = local;
    }
    match drag.action {
        DragAction::Pan => {
            let center = CanvasView::center(world, entity)? - (local - drag.local) / zoom;
            if !center.is_finite() {
                end(world, entity);
                return Ok(());
            }
            CanvasView::set_center_in_world(world, entity, center)?;
        }
        DragAction::Move {
            document,
            shape,
            offset,
        } => {
            let point = CanvasView::center(world, entity)? + local / zoom - offset;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            let Some(mut transform) = world.get::<Transform>(shape).copied() else {
                end(world, entity);
                return Ok(());
            };
            transform.translation.x = point.x;
            transform.translation.y = point.y;
            CanvasDocument::update_transform(world, document, shape, transform)?;
        }
        DragAction::Scale(scale) => {
            let point = CanvasView::center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            scale.apply(world, point)?;
        }
        DragAction::Rotate(rotate) => {
            let point = CanvasView::center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            let snap = world
                .get_resource::<ButtonInput<KeyCode>>()
                .is_some_and(|keys| {
                    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
                });
            rotate.apply(world, point, snap)?;
        }
        DragAction::Draw { .. } => {
            let point = CanvasView::center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            if let Some(mut state) = world.get_mut::<DragState>(entity)
                && let Some(Drag {
                    action: DragAction::Draw { current, .. },
                    ..
                }) = &mut state.0
            {
                *current = point;
            }
        }
    }
    if !continuing(world, entity, &drag) {
        end(world, entity);
    }
    Ok(())
}

fn scrolled(mut event: On<PointerScroll>, mut world: bevy::ecs::world::DeferredWorld) {
    if event.original_event_target() != event.entity
        || world.get::<CanvasView>(event.entity).is_none()
        || !event.y.is_finite()
        || event.y == 0.0
        || inside_position(
            &world,
            event.entity,
            &event.pointer.target,
            event.pointer.position,
        )
        .is_none()
    {
        return;
    }
    let event_data = event.event().clone();
    event.propagate(false);
    world.commands().queue(move |world: &mut World| {
        zoom_by_scroll(
            world,
            event_data.entity,
            &event_data.pointer.target,
            event_data.pointer.position,
            event_data.unit,
            event_data.y,
        )
    });
}

fn inside_position(
    world: &World,
    entity: Entity,
    target: &NormalizedRenderTarget,
    point: Vec2,
) -> Option<Vec2> {
    if !available(world, entity) || !focused(world, target) {
        return None;
    }
    let camera = world.get::<ComputedUiTargetCamera>(entity)?.get()?;
    let render_target = world.get::<bevy::camera::RenderTarget>(camera)?;
    let matches = match render_target {
        bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Primary) => {
            matches!(target, NormalizedRenderTarget::Window(window) if world.get::<bevy::window::PrimaryWindow>(window.entity()).is_some())
        }
        _ => render_target.normalize(None).as_ref() == Some(target),
    };
    if !matches {
        return None;
    }
    let local = position(world, entity, point)?;
    let node = world.get::<ComputedNode>(entity)?;
    (!local
        .abs()
        .cmpgt(node.size() * node.inverse_scale_factor() * 0.5)
        .any())
    .then_some(local)
}

fn zoom_by_scroll(
    world: &mut World,
    entity: Entity,
    target: &NormalizedRenderTarget,
    point: Vec2,
    unit: bevy::input::mouse::MouseScrollUnit,
    y: f32,
) -> Result {
    if !y.is_finite() || y == 0.0 {
        return Ok(());
    }
    let Some(local) = inside_position(world, entity, target, point) else {
        return Ok(());
    };
    let lines = match unit {
        bevy::input::mouse::MouseScrollUnit::Line => y,
        bevy::input::mouse::MouseScrollUnit::Pixel => y / 100.0,
    };
    let old = CanvasView::zoom(world, entity)?;
    let zoom = (old.ln() + lines * 0.1)
        .clamp(crate::view::MIN_ZOOM.ln(), crate::view::MAX_ZOOM.ln())
        .exp()
        .clamp(crate::view::MIN_ZOOM, crate::view::MAX_ZOOM);
    if old == zoom {
        return Ok(());
    }
    let center = CanvasView::center(world, entity)? + local * (old.recip() - zoom.recip());
    if !center.is_finite() {
        return Ok(());
    }
    crate::view::set_zoom_and_center(world, entity, zoom, center)?;
    Ok(())
}

fn continuing(world: &World, entity: Entity, drag: &Drag) -> bool {
    world
        .get::<DragState>(entity)
        .and_then(|state| state.0.as_ref())
        .is_some_and(|current| current.pointer == drag.pointer && current.target == drag.target)
        && available(world, entity)
        && focused(world, &drag.target)
        && world.resource::<InputFocus>().get() == Some(entity)
        && world
            .resource::<PointerCaptureMap>()
            .get(&drag.pointer)
            .is_some_and(|(owner, _)| owner == entity)
        && match drag.action {
            DragAction::Pan => true,
            DragAction::Move {
                document, shape, ..
            }
            | DragAction::Scale(crate::scale::Scale {
                document, shape, ..
            })
            | DragAction::Rotate(crate::rotate::Rotate {
                document, shape, ..
            }) => {
                world
                    .get::<crate::view::ViewState>(entity)
                    .is_some_and(|state| {
                        state.document == Some(document) && state.selection == Some(shape)
                    })
                    && crate::selection::belongs_to(world, Some(document), shape)
            }
            DragAction::Draw {
                document, paint, ..
            } => world
                .get::<crate::view::ViewState>(entity)
                .is_some_and(|state| {
                    state.document == Some(document)
                        && state.mode == crate::CanvasMode::Rectangle(paint)
                }),
        }
}

fn process_input(world: &mut World, mut cursor: Local<MessageCursor<PointerInput>>) -> Result {
    let mut mutations = cursor.clone();
    let inputs = cursor
        .read_with_id(world.resource::<Messages<PointerInput>>())
        .map(|(input, id)| (id.id, input.clone()))
        .collect::<Vec<_>>();
    world.resource_mut::<FrameInputs>().0 = inputs.iter().map(|(_, input)| input.clone()).collect();
    let mut consumed = HashSet::new();
    let active = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(entity, state)| state.0.clone().map(|drag| (entity, drag)))
        .collect::<Vec<_>>();
    for (entity, drag) in active {
        let matching = inputs
            .iter()
            .filter(|(_, input)| {
                input.pointer_id == drag.pointer && input.location.target == drag.target
            })
            .collect::<Vec<_>>();
        let released = matching.iter().any(|(_, input)| {
            matches!(
                input.action,
                PointerAction::Release(_) | PointerAction::Cancel
            )
        });
        let capture = world
            .resource::<PointerCaptureMap>()
            .get(&drag.pointer)
            .map(|(owner, _)| owner);
        if !available(world, entity)
            || !focused(world, &drag.target)
            || world.resource::<InputFocus>().get() != Some(entity)
            || capture.is_some_and(|owner| owner != entity)
            || (capture.is_none() && !released)
        {
            end(world, entity);
            continue;
        }
        if capture.is_none() {
            // Recover only the capture Bevy pre-cleared for this frame's terminal input.
            // Later consumer changes to ownership must be checked before every remaining action.
            world.resource_mut::<PointerCaptureMap>().capture(
                drag.pointer,
                entity,
                drag.hit.clone(),
            );
        }
        if let Some(mut state) = world.get_mut::<DragState>(entity)
            && let Some(drag) = &mut state.0
        {
            drag.raw_driven = true;
        }
        // Consume the captured gesture in input order before terminal-frame picking can change focus.
        // Presses after its Primary release retain the backend's real hover route.
        for (message_id, input) in matching {
            if !continuing(world, entity, &drag) {
                end(world, entity);
                break;
            }
            match input.action {
                PointerAction::Press(button) if button != PointerButton::Primary => {
                    consumed.insert(*message_id);
                }
                PointerAction::Move { .. } => advance(world, entity, input.location.position)?,
                PointerAction::Scroll { unit, y, .. } => {
                    zoom_by_scroll(
                        world,
                        entity,
                        &input.location.target,
                        input.location.position,
                        unit,
                        y,
                    )?;
                    consumed.insert(*message_id);
                }
                PointerAction::Release(PointerButton::Primary) => {
                    finish(world, entity, input.location.position)?;
                    break;
                }
                PointerAction::Cancel => {
                    end(world, entity);
                    break;
                }
                _ => {}
            }
        }
        if !continuing(world, entity, &drag) {
            end(world, entity);
        } else if capture.is_none() {
            // Bevy pre-clears capture for secondary releases too, while this Primary gesture remains active.
            if let Some(mut hover) = world.get_resource_mut::<HoverMap>() {
                let hits = hover.entry(drag.pointer).or_default();
                hits.clear();
                hits.insert(entity, drag.hit);
            }
        }
    }
    if !consumed.is_empty() {
        let mut messages = world.resource_mut::<Messages<PointerInput>>();
        for (input, id) in mutations.read_mut_with_id(&mut messages) {
            if consumed.contains(&id.id) {
                // A zero-delta move is Bevy's no-op action and preserves the stream's message IDs and order.
                // Secondary presses during an exclusive Primary gesture must not start another widget gesture.
                input.action = PointerAction::Move { delta: Vec2::ZERO };
            }
        }
    }
    Ok(())
}

fn release_capture(capture: &mut PointerCaptureMap, entity: Entity, drag: &Drag) {
    if capture
        .get(&drag.pointer)
        .is_some_and(|(owner, _)| owner == entity)
    {
        capture.release(drag.pointer);
    }
}

pub(crate) fn end(world: &mut World, entity: Entity) {
    if let Some(drag) = world
        .get_mut::<DragState>(entity)
        .and_then(|mut state| state.0.take())
    {
        release_capture(
            &mut world.resource_mut::<PointerCaptureMap>(),
            entity,
            &drag,
        );
        bevy_widgetry_core::pointer::refresh_hover(world, drag.pointer);
    }
}

pub(crate) fn preview(world: &World, entity: Entity) -> Option<(Vec2, Vec2, crate::CanvasPaint)> {
    let drag = world.get::<DragState>(entity)?.0.as_ref()?;
    match drag.action {
        DragAction::Draw {
            start,
            current,
            paint,
            ..
        } => Some((start, current, paint)),
        _ => None,
    }
}

fn finish(world: &mut World, entity: Entity, point: Vec2) -> Result {
    let Some(drag) = world
        .get::<DragState>(entity)
        .and_then(|state| state.0.clone())
    else {
        return Ok(());
    };
    if !matches!(drag.action, DragAction::Draw { .. }) || !continuing(world, entity, &drag) {
        end(world, entity);
        return Ok(());
    }
    advance(world, entity, point)?;
    complete_drawing(world, entity)
}

fn complete_drawing(world: &mut World, entity: Entity) -> Result {
    let drawing = world
        .get::<DragState>(entity)
        .and_then(|state| state.0.as_ref())
        .filter(|drag| continuing(world, entity, drag))
        .map(|drag| drag.action);
    end(world, entity);
    let Some(DragAction::Draw {
        document,
        start,
        current,
        paint,
    }) = drawing
    else {
        return Ok(());
    };
    CanvasView::set_mode_in_world(world, entity, crate::CanvasMode::Select)?;
    world.flush();
    let size = (current - start).abs();
    if !size.is_finite()
        || size.min_element() < 1.0
        || world
            .get::<crate::view::ViewState>(entity)
            .is_none_or(|state| {
                state.document != Some(document) || state.mode != crate::CanvasMode::Select
            })
        || !available(world, entity)
    {
        return Ok(());
    }
    let center = start + (current - start) * 0.5;
    let shape = CanvasDocument::create_rectangle(
        world,
        document,
        crate::CanvasRectangle { size },
        Transform::from_translation(center.extend(0.0)),
        paint,
    )?;
    CanvasView::set_selection_in_world(world, entity, Some(shape))?;
    Ok(())
}

fn delete_selected(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<CanvasView>>,
    mut commands: Commands,
) {
    if !views.contains(event.focused_entity)
        || event.input.state != ButtonState::Pressed
        || event.input.key_code != KeyCode::Delete
    {
        return;
    }
    let (view, window) = (event.focused_entity, event.input.window);
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        if world.resource::<InputFocus>().get() != Some(view)
            || !available(world, view)
            || !world
                .get::<Window>(window)
                .is_some_and(|window| window.focused)
        {
            return Ok(());
        }
        let Some(shape) = CanvasView::selection(world, view)? else {
            return Ok(());
        };
        let Some(document) = CanvasView::document(world, view)? else {
            return Ok(());
        };
        CanvasDocument::remove_shape(world, document, shape)?;
        let affected = world
            .query::<(Entity, &DragState)>()
            .iter(world)
            .filter_map(|(view, state)| {
                let target = match state.0.as_ref()?.action {
                    DragAction::Move { shape, .. }
                    | DragAction::Scale(crate::scale::Scale { shape, .. })
                    | DragAction::Rotate(crate::rotate::Rotate { shape, .. }) => shape,
                    DragAction::Pan | DragAction::Draw { .. } => return None,
                };
                (target == shape).then_some(view)
            })
            .collect::<Vec<_>>();
        for view in affected {
            end(world, view);
        }
        Ok(())
    });
}

fn escape(world: &mut World) -> Result {
    if !world
        .get_resource::<ButtonInput<KeyCode>>()
        .is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
    {
        return Ok(());
    }
    let Some(entity) = world.resource::<InputFocus>().get() else {
        return Ok(());
    };
    if world
        .get::<crate::view::ViewState>(entity)
        .is_some_and(|state| matches!(state.mode, crate::CanvasMode::Rectangle(_)))
    {
        CanvasView::set_mode_in_world(world, entity, crate::CanvasMode::Select)?;
    }
    Ok(())
}

fn refresh_rotation(world: &mut World) -> Result {
    if !world
        .get_resource::<ButtonInput<KeyCode>>()
        .is_some_and(|keys| {
            [KeyCode::ShiftLeft, KeyCode::ShiftRight]
                .into_iter()
                .any(|key| keys.just_pressed(key) || keys.just_released(key))
        })
    {
        return Ok(());
    }
    let active = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(entity, state)| {
            state
                .0
                .as_ref()
                .filter(|drag| matches!(drag.action, DragAction::Rotate(_)))
                .map(|drag| (entity, drag.clone()))
        })
        .collect::<Vec<_>>();
    for (entity, drag) in active {
        let point = world
            .query::<(&PointerId, &bevy::picking::pointer::PointerLocation)>()
            .iter(world)
            .find(|(id, _)| **id == drag.pointer)
            .and_then(|(_, location)| location.location.as_ref())
            .filter(|location| location.target == drag.target)
            .map(|location| location.position);
        if let Some(point) = point {
            advance(world, entity, point)?;
        }
    }
    Ok(())
}

fn terminal(
    commands: &mut Commands,
    entity: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    completed: Option<Vec2>,
) {
    commands.queue(move |world: &mut World| -> Result {
        if world
            .get::<DragState>(entity)
            .and_then(|state| state.0.as_ref())
            .is_some_and(|drag| drag.pointer == pointer && drag.target == target)
        {
            if let Some(point) = completed {
                finish(world, entity, point)?;
            } else {
                end(world, entity);
            }
        }
        Ok(())
    });
}

fn released(event: On<PointerRelease>, mut commands: Commands) {
    if event.button == PointerButton::Primary {
        terminal(
            &mut commands,
            event.entity,
            event.pointer.id,
            event.pointer.target.clone(),
            Some(event.pointer.position),
        );
    }
}

fn cancelled(event: On<PointerCancel>, mut commands: Commands) {
    terminal(
        &mut commands,
        event.entity,
        event.pointer.id,
        event.pointer.target.clone(),
        None,
    );
}

fn drag_ended(event: On<PointerDragEnd>, mut commands: Commands) {
    if event.button == PointerButton::Primary {
        terminal(
            &mut commands,
            event.entity,
            event.pointer.id,
            event.pointer.target.clone(),
            Some(event.pointer.position),
        );
    }
}

fn document_changed(event: On<CanvasDocumentChanged>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| end(world, entity));
}

fn removed(
    event: On<Remove<DragState>>,
    states: Query<&DragState>,
    mut capture: ResMut<PointerCaptureMap>,
) {
    if let Ok(state) = states.get(event.entity)
        && let Some(drag) = &state.0
    {
        release_capture(&mut capture, event.entity, drag);
    }
}

fn cancel_unavailable(world: &mut World) -> Result {
    let active = world
        .query::<(Entity, &DragState)>()
        .iter(world)
        .filter_map(|(entity, state)| state.0.clone().map(|drag| (entity, drag)))
        .collect::<Vec<_>>();
    for (entity, drag) in active {
        let pointer = world
            .query::<(
                &PointerId,
                &bevy::picking::pointer::PointerLocation,
                &bevy::picking::pointer::PointerPressState,
            )>()
            .iter(world)
            .find(|(id, _, _)| **id == drag.pointer)
            .map(|(_, location, buttons)| {
                (
                    buttons.is_primary_pressed(),
                    location.location.as_ref().is_some_and(|location| {
                        location.target == drag.target && location.position.is_finite()
                    }),
                )
            });
        // A first-frame release can target the previous hover entity without a Canvas terminal event.
        // Distinguish it from Cancel so an invalid click still exits the one-shot drawing mode.
        if pointer == Some((false, true))
            && matches!(drag.action, DragAction::Draw { .. })
            && continuing(world, entity, &drag)
        {
            let terminal = world
                .resource::<FrameInputs>()
                .0
                .iter()
                .rev()
                .find(|input| {
                    input.pointer_id == drag.pointer
                        && input.location.target == drag.target
                        && matches!(
                            input.action,
                            PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel
                        )
                })
                .cloned();
            if let Some(PointerInput {
                action: PointerAction::Release(PointerButton::Primary),
                location,
                ..
            }) = terminal
            {
                finish(world, entity, location.position)?;
                continue;
            }
        }
        if pointer != Some((true, true)) || !continuing(world, entity, &drag) {
            end(world, entity);
        }
    }
    Ok(())
}
