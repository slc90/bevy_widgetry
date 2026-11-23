use crate::error::CanvasResult as Result;
use crate::{
    CanvasConnectorRoute, CanvasContainer, CanvasDocument, CanvasGeometry, CanvasMode, CanvasPaint,
    CanvasPath, CanvasView,
};
use bevy::{
    camera::NormalizedRenderTarget,
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, InputFocus},
    picking::{
        PickingSettings,
        events::PointerPress,
        pointer::{PointerId, PointerLocation, PointerMap},
    },
    prelude::*,
    time::Real,
};
use bevy_widgetry_diagnostics::ErrorFrame;
use std::time::Duration;

const CLICK_RADIUS: f32 = 4.0;

#[derive(Component, Default)]
pub(crate) struct State {
    press: Option<Press>,
    draft: Option<Draft>,
}

#[derive(Clone, PartialEq)]
struct Context {
    document: Entity,
    container: CanvasContainer,
    mode: CanvasMode,
    target: NormalizedRenderTarget,
}

struct Press {
    context: Context,
    pointer: PointerId,
    local: Vec2,
    dragged: bool,
}

struct Draft {
    context: Context,
    pointer: PointerId,
    vertices: Vec<Vec2>,
    last_click: Option<(Vec2, Duration)>,
}

fn contour(mode: CanvasMode) -> Option<(CanvasPaint, bool)> {
    match mode {
        CanvasMode::Polyline(paint) => Some((
            CanvasPaint {
                fill: None,
                ..paint
            },
            false,
        )),
        CanvasMode::Polygon(paint) => Some((paint, true)),
        _ => None,
    }
}

fn is_near(a: Vec2, b: Vec2) -> bool {
    a.distance_squared(b) <= CLICK_RADIUS * CLICK_RADIUS
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(enter);
}

pub(crate) fn begin_click(world: &mut World, press: &PointerPress, local: Vec2) -> Result<bool> {
    let view = press.entity;
    let mode = CanvasView::_mode(world, view)?;
    if !matches!(mode, CanvasMode::Text | CanvasMode::Connector(_)) && contour(mode).is_none() {
        return Ok(false);
    }
    let (Some(document), Some(container)) = (
        CanvasView::_document(world, view)?,
        CanvasView::drawing_target(world, view)?,
    ) else {
        return Ok(false);
    };
    let pointer = press.pointer.id;
    let context = Context {
        document,
        container,
        mode,
        target: press.pointer.target.clone(),
    };
    let accepts = world.get::<State>(view).is_some_and(|state| {
        // A finished touch can still have a live Bevy pointer until the end of the frame.
        state
            .press
            .as_ref()
            .is_none_or(|press| press.pointer == pointer)
            && state.draft.as_ref().is_none_or(|draft| {
                draft.context == context
                    && (draft.pointer == pointer
                        || (matches!(draft.pointer, PointerId::Touch(_))
                            && matches!(pointer, PointerId::Touch(_))))
            })
    });
    if accepts && let Some(mut state) = world.get_mut::<State>(view) {
        state.press = Some(Press {
            context,
            pointer,
            local,
            dragged: false,
        });
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn dragged(world: &mut World, view: Entity, local: Vec2) {
    if let Some(mut state) = world.get_mut::<State>(view)
        && let Some(press) = &mut state.press
        && !press.dragged
        && !is_near(local, press.local)
    {
        press.dragged = true;
        if let Some(draft) = &mut state.draft {
            draft.last_click = None;
        }
    }
}

pub(crate) fn interrupt_click(world: &mut World, view: Entity) {
    if let Some(mut state) = world.get_mut::<State>(view) {
        state.press = None;
        if let Some(draft) = &mut state.draft {
            draft.last_click = None;
        }
    }
}

pub(crate) fn press_valid(world: &World, view: Entity) -> bool {
    world
        .get::<State>(view)
        .and_then(|state| state.press.as_ref())
        .is_some_and(|press| valid(world, view, &press.context))
}

pub(crate) fn add_vertex(world: &mut World, view: Entity, screen: Vec2) -> Result {
    let Some(press) = world
        .get_mut::<State>(view)
        .and_then(|mut state| state.press.take())
    else {
        return Ok(());
    };
    if press.dragged || !valid(world, view, &press.context) {
        return Ok(());
    }
    let Some(local) = crate::input::inside_position(world, view, &press.context.target, screen)
        .filter(|local| is_near(*local, press.local))
    else {
        if let Some(mut state) = world.get_mut::<State>(view)
            && let Some(draft) = &mut state.draft
        {
            draft.last_click = None;
        }
        return Ok(());
    };
    let point = CanvasView::_center(world, view)? + local / CanvasView::_zoom(world, view)?;
    if !point.is_finite() {
        return Ok(());
    }
    if press.context.mode == CanvasMode::Text {
        if !crate::history::is_busy(world, press.context.document) {
            let item = CanvasDocument::place_text(
                world,
                press.context.document,
                press.context.container,
                point,
            )?;
            select_created(world, view, &press.context, item)?;
        }
        return Ok(());
    }
    let now = world.resource::<Time<Real>>().elapsed();
    let interval = world.resource::<PickingSettings>().multi_click_interval;
    let Some(mut state) = world.get_mut::<State>(view) else {
        return Ok(());
    };
    let draft = state.draft.get_or_insert_with(|| Draft {
        context: press.context,
        pointer: press.pointer,
        vertices: Vec::new(),
        last_click: None,
    });
    draft.pointer = press.pointer;
    if draft.last_click.is_some_and(|(previous, at)| {
        is_near(local, previous) && now.saturating_sub(at) <= interval
    }) {
        return finish(world, view);
    }
    if draft.vertices.last() != Some(&point) {
        draft.vertices.push(point);
    }
    draft.last_click = Some((local, now));
    Ok(())
}

fn enter(mut event: On<FocusedInput<KeyboardInput>>, views: Query<&State>, mut commands: Commands) {
    let view = event.focused_entity;
    if event.original_event_target() != view
        || event.input.key_code != KeyCode::Enter
        || event.input.state != ButtonState::Pressed
        || event.input.repeat
        || !views.get(view).is_ok_and(|state| state.draft.is_some())
    {
        return;
    }
    event.propagate(false);
    let window = event.input.window;
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        if !crate::input::keyboard_available(world, view, window) {
            return Ok(());
        }
        finish(world, view).map_err(|error| {
            error
                .at("view", view)
                .into_bevy_error(ErrorFrame::new("canvas.input.contour_finish"))
        })
    });
}

fn finish(world: &mut World, view: Entity) -> Result {
    let Some(draft) = world
        .get::<State>(view)
        .and_then(|state| state.draft.as_ref())
    else {
        return Ok(());
    };
    let context = draft.context.clone();
    if !valid(world, view, &context) || crate::history::is_busy(world, context.document) {
        return Ok(());
    }
    if let CanvasMode::Connector(style) = context.mode {
        let Some(route) = CanvasConnectorRoute::from_vertices(&draft.vertices)
            .filter(CanvasConnectorRoute::drawable)
        else {
            return Ok(());
        };
        let item = CanvasDocument::draw_connector(
            world,
            context.document,
            context.container,
            route,
            style,
        )?;
        return select_created(world, view, &context, item);
    }
    let Some((paint, closed)) = contour(context.mode) else {
        return Ok(());
    };
    let vertices = effective_vertices(&draft.vertices, closed);
    if closed && !has_three_vertices(vertices) {
        return Ok(());
    }
    let Ok(commands) = crate::path::contour(vertices, closed) else {
        return Ok(());
    };
    let shape = CanvasDocument::create_drawing(
        world,
        context.document,
        context.container,
        CanvasGeometry::Path(CanvasPath { commands }),
        Transform::default(),
        paint,
    )?;
    select_created(world, view, &context, shape)
}

fn select_created(world: &mut World, view: Entity, context: &Context, shape: Entity) -> Result {
    if world
        .get::<crate::view::ViewState>(view)
        .is_some_and(|state| state.document == Some(context.document) && state.mode == context.mode)
    {
        CanvasView::_set_mode_in_world(world, view, CanvasMode::Select)?;
        crate::selection::select_committed_shape(
            world,
            view,
            context.document,
            CanvasMode::Select,
            shape,
        )?;
    }
    Ok(())
}

fn effective_vertices(vertices: &[Vec2], closed: bool) -> &[Vec2] {
    if closed && vertices.len() > 1 && vertices.first() == vertices.last() {
        &vertices[..vertices.len() - 1]
    } else {
        vertices
    }
}

fn has_three_vertices(vertices: &[Vec2]) -> bool {
    let Some(first) = vertices.first() else {
        return false;
    };
    vertices
        .iter()
        .find(|point| *point != first)
        .is_some_and(|second| {
            vertices
                .iter()
                .any(|point| point != first && point != second)
        })
}

fn valid(world: &World, view: Entity, context: &Context) -> bool {
    crate::input::available(world, view)
        && crate::input::focused(world, &context.target)
        && world.resource::<InputFocus>().get() == Some(view)
        && world
            .get::<crate::view::ViewState>(view)
            .is_some_and(|state| {
                state.document == Some(context.document) && state.mode == context.mode
            })
        && CanvasView::drawing_target(world, view)
            .is_ok_and(|target| target == Some(context.container))
}

pub(crate) fn cancel(world: &mut World, view: Entity) {
    if let Some(mut state) = world.get_mut::<State>(view) {
        state.press = None;
        state.draft = None;
    }
}

pub(crate) fn cancel_pointer(
    world: &mut World,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
) {
    for mut state in world.query::<&mut State>().iter_mut(world) {
        let cancelled_press = state
            .press
            .as_ref()
            .is_some_and(|press| press.pointer == pointer && &press.context.target == target);
        if cancelled_press {
            state.press = None;
        }
        if cancelled_press
            || state
                .draft
                .as_ref()
                .is_some_and(|draft| draft.pointer == pointer && &draft.context.target == target)
        {
            state.draft = None;
        }
    }
}

pub(crate) fn cancel_unavailable(world: &mut World) {
    let cancelled = world
        .query::<(Entity, &State)>()
        .iter(world)
        .filter_map(|(view, state)| {
            let cancel_draft = state.draft.as_ref().is_some_and(|draft| {
                !valid(world, view, &draft.context)
                    || (!matches!(draft.pointer, PointerId::Touch(_))
                        && pointer_position(world, draft.pointer, &draft.context.target).is_none())
            });
            let cancel_press = state.press.as_ref().is_some_and(|press| {
                !valid(world, view, &press.context)
                    || pointer_position(world, press.pointer, &press.context.target).is_none()
            });
            (cancel_draft || cancel_press).then_some((view, cancel_draft))
        })
        .collect::<Vec<_>>();
    for (view, cancel_draft) in cancelled {
        if cancel_draft {
            cancel(world, view);
        } else if let Some(mut state) = world.get_mut::<State>(view) {
            state.press = None;
        }
    }
}

fn pointer_position(
    world: &World,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
) -> Option<Vec2> {
    let pointer = world.resource::<PointerMap>().get_entity(pointer)?;
    let location = world.get::<PointerLocation>(pointer)?.location.as_ref()?;
    (&location.target == target && location.position.is_finite()).then_some(location.position)
}

pub(crate) fn preview(world: &World, view: Entity) -> Option<crate::pen::Preview> {
    let state = world.get::<State>(view)?;
    let draft = state.draft.as_ref()?;
    let pointer = state
        .press
        .as_ref()
        .map_or(draft.pointer, |press| press.pointer);
    let mut vertices = draft.vertices.clone();
    if let Some(point) = pointer_position(world, pointer, &draft.context.target)
        && let Some(local) =
            crate::input::inside_position(world, view, &draft.context.target, point)
    {
        let point =
            CanvasView::_center(world, view).ok()? + local / CanvasView::_zoom(world, view).ok()?;
        if point.is_finite() && vertices.last() != Some(&point) {
            vertices.push(point);
        }
    }
    if let CanvasMode::Connector(style) = draft.context.mode {
        let (path, arrows) = CanvasConnectorRoute::from_vertices(&vertices)
            .filter(CanvasConnectorRoute::drawable)
            .map_or_else(
                || {
                    (
                        CanvasPath {
                            commands: Vec::new(),
                        },
                        CanvasPath {
                            commands: Vec::new(),
                        },
                    )
                },
                |route| route.paths(style),
            );
        return Some(crate::pen::Preview {
            name: "CanvasConnectorPreview",
            path,
            arrows: Some(arrows),
            points: draft.vertices.clone(),
            paint: style.paint(),
        });
    }
    let (paint, closed) = contour(draft.context.mode)?;
    let vertices = effective_vertices(&vertices, closed);
    let closed = closed && has_three_vertices(vertices);
    let commands = crate::path::contour(vertices, closed).unwrap_or_default();
    Some(crate::pen::Preview {
        name: "CanvasContourPreview",
        path: CanvasPath { commands },
        arrows: None,
        points: draft.vertices.clone(),
        paint: if closed {
            paint
        } else {
            CanvasPaint {
                fill: None,
                ..paint
            }
        },
    })
}
