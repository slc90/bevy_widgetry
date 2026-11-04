use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasMode, CanvasPaint, CanvasPath, CanvasPathCommand, CanvasView};
use bevy::{
    camera::NormalizedRenderTarget,
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, InputFocus},
    picking::{PickingSystems, pointer::PointerId},
    prelude::*,
    ui::UiSystems,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CanvasPenSegment {
    #[default]
    Line,
    Quadratic,
    Cubic,
    Arc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPenDraft {
    pub path: CanvasPath,
    pub pending: Vec<Vec2>,
    pub active_subpath: Option<usize>,
}

#[derive(Clone)]
struct Draft {
    data: CanvasPenDraft,
    document: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    paint: CanvasPaint,
}

#[derive(Component, Default)]
pub(crate) struct State {
    segment: CanvasPenSegment,
    draft: Option<Draft>,
}

#[derive(EntityEvent)]
pub struct CanvasPenChanged {
    pub entity: Entity,
}

fn state(world: &World, view: Entity) -> Result<&State> {
    world
        .get::<State>(view)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
        .map_err(|error| error.at("view", view))
}

fn context(world: &World, view: Entity) -> Result<(Entity, CanvasPaint)> {
    (|| -> Result<(Entity, CanvasPaint)> {
        let Some(view_state) = world.get::<crate::view::ViewState>(view) else {
            return Err(crate::error::missing(crate::CanvasPart::View));
        };
        if let CanvasMode::Pen(paint) = view_state.mode
            && let Some(document) = view_state.document
            && world.get::<CanvasDocument>(document).is_some()
        {
            return Ok((document, paint));
        }

        Err(crate::error::invalid(crate::CanvasIssue::PenContext))
    })()
    .map_err(|error| error.at("view", view))
}

fn notify(world: &mut World, view: Entity) {
    world.trigger(CanvasPenChanged { entity: view });
}

pub(crate) fn cancel(world: &mut World, view: Entity) -> bool {
    let changed = world
        .get_mut::<State>(view)
        .is_some_and(|mut s| s.draft.take().is_some());
    if changed {
        notify(world, view);
    }
    changed
}

pub(crate) fn accepts(
    world: &World,
    view: Entity,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
) -> bool {
    world.get::<State>(view).is_some_and(|s| {
        s.draft
            .as_ref()
            .is_none_or(|d| d.pointer == pointer && &d.target == target)
    })
}

pub(crate) fn cancel_pointer(
    world: &mut World,
    pointer: PointerId,
    target: &NormalizedRenderTarget,
) {
    let views = world
        .query::<(Entity, &State)>()
        .iter(world)
        .filter_map(|(v, s)| {
            s.draft
                .as_ref()
                .filter(|d| d.pointer == pointer && &d.target == target)
                .map(|_| v)
        })
        .collect::<Vec<_>>();
    for v in views {
        crate::input::end(world, v);
    }
}

fn advance(data: &mut CanvasPenDraft, segment: CanvasPenSegment, point: Vec2) {
    use CanvasPathCommand::*;
    if data.active_subpath.is_none() {
        data.active_subpath = Some(data.path.commands.len());
        data.path.commands.push(MoveTo(point));
        data.pending.clear();
        return;
    }
    data.pending.push(point);
    let command = match (segment, data.pending.as_slice()) {
        (CanvasPenSegment::Line, [to]) => Some(LineTo(*to)),
        (CanvasPenSegment::Quadratic, [control, to]) => Some(QuadTo {
            control: *control,
            to: *to,
        }),
        (CanvasPenSegment::Cubic, [control1, control2, to]) => Some(CubicTo {
            control1: *control1,
            control2: *control2,
            to: *to,
        }),
        (CanvasPenSegment::Arc, [to]) => {
            let from = data
                .path
                .commands
                .iter()
                .rev()
                .find_map(|c| match *c {
                    MoveTo(p)
                    | LineTo(p)
                    | QuadTo { to: p, .. }
                    | CubicTo { to: p, .. }
                    | ArcTo { to: p, .. } => Some(p),
                    Close => None,
                })
                .unwrap_or(*to);
            Some(ArcTo {
                radii: (*to - from).abs().max(Vec2::ONE),
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: *to,
            })
        }
        _ => None,
    };
    if let Some(command) = command {
        data.path.commands.push(command);
        data.pending.clear();
    }
}

pub(crate) fn append_point(
    world: &mut World,
    view: Entity,
    document: Entity,
    paint: CanvasPaint,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    point: Vec2,
) -> Result {
    (|| -> Result {
        if world
            .get::<crate::view::ViewState>(view)
            .is_none_or(|s| s.document != Some(document) || s.mode != CanvasMode::Pen(paint))
            || !crate::input::available(world, view)
            || !crate::input::focused(world, &target)
            || world.resource::<InputFocus>().get() != Some(view)
            || world.get::<CanvasDocument>(document).is_none()
        {
            return Ok(());
        }
        if !accepts(world, view, pointer, &target) {
            return Ok(());
        }
        let mut draft = state(world, view)?.draft.clone().unwrap_or(Draft {
            data: CanvasPenDraft {
                path: CanvasPath {
                    commands: Vec::new(),
                },
                pending: Vec::new(),
                active_subpath: None,
            },
            document,
            pointer,
            target,
            paint,
        });
        advance(&mut draft.data, state(world, view)?.segment, point);
        draft.data.path._validate(&Transform::default(), &paint)?;
        world
            .get_mut::<State>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenState))?
            .draft = Some(draft);
        notify(world, view);
        Ok(())
    })()
    .map_err(|error| error.at("view", view).at("document", document))
}

fn drawable(path: &CanvasPath) -> bool {
    path.commands
        .iter()
        .any(|c| !matches!(c, CanvasPathCommand::MoveTo(_) | CanvasPathCommand::Close))
}

impl CanvasView {
    pub fn pen_segment(
        world: &World,
        view: Entity,
    ) -> crate::error::PublicResult<CanvasPenSegment> {
        Self::_pen_segment(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.pen_segment",
            ))
        })
    }

    pub(crate) fn _pen_segment(world: &World, view: Entity) -> Result<CanvasPenSegment> {
        (|| -> Result<CanvasPenSegment> { Ok(state(world, view)?.segment) })().map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn pen_draft(
        world: &World,
        view: Entity,
    ) -> crate::error::PublicResult<Option<CanvasPenDraft>> {
        Self::_pen_draft(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.pen_draft",
            ))
        })
    }

    pub(crate) fn _pen_draft(world: &World, view: Entity) -> Result<Option<CanvasPenDraft>> {
        (|| -> Result<Option<CanvasPenDraft>> {
            Ok(state(world, view)?.draft.as_ref().map(|d| d.data.clone()))
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn set_pen_segment_in_world(
        world: &mut World,
        view: Entity,
        segment: CanvasPenSegment,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_pen_segment_in_world(world, view, segment).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_pen_segment_in_world",
            ))
        })
    }

    pub(crate) fn _set_pen_segment_in_world(
        world: &mut World,
        view: Entity,
        segment: CanvasPenSegment,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            if state(world, view)?.segment == segment {
                return Ok(false);
            }
            if world
                .get::<crate::view::ViewState>(view)
                .is_some_and(|s| matches!(s.mode, CanvasMode::Pen(_)))
            {
                crate::input::release_gesture(world, view);
            }
            let mut s = world
                .get_mut::<State>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenState))?;
            s.segment = segment;
            if let Some(d) = &mut s.draft {
                d.data.pending.clear();
            }
            notify(world, view);
            Ok(true)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn set_pen_segment(commands: &mut Commands, view: Entity, segment: CanvasPenSegment) {
        commands.queue(move |world: &mut World| {
            Self::_set_pen_segment_in_world(world, view, segment)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
    pub fn close_pen_subpath_in_world(
        world: &mut World,
        view: Entity,
    ) -> crate::error::PublicResult<bool> {
        Self::_close_pen_subpath_in_world(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.close_pen_subpath_in_world",
            ))
        })
    }

    pub(crate) fn _close_pen_subpath_in_world(world: &mut World, view: Entity) -> Result<bool> {
        (|| -> Result<bool> {
            context(world, view)?;
            let Some(mut draft) = state(world, view)?.draft.clone() else {
                return Ok(false);
            };
            if draft.data.active_subpath.is_none() {
                return Ok(false);
            }
            crate::input::release_gesture(world, view);
            draft.data.path.commands.push(CanvasPathCommand::Close);
            draft.data.pending.clear();
            draft.data.active_subpath = None;
            world
                .get_mut::<State>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenState))?
                .draft = Some(draft);
            notify(world, view);
            Ok(true)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn new_pen_subpath_in_world(
        world: &mut World,
        view: Entity,
    ) -> crate::error::PublicResult<bool> {
        Self::_new_pen_subpath_in_world(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.new_pen_subpath_in_world",
            ))
        })
    }

    pub(crate) fn _new_pen_subpath_in_world(world: &mut World, view: Entity) -> Result<bool> {
        (|| -> Result<bool> {
            context(world, view)?;
            let Some(mut draft) = state(world, view)?.draft.clone() else {
                return Ok(false);
            };
            if draft.data.active_subpath.is_none() && draft.data.pending.is_empty() {
                return Ok(false);
            }
            crate::input::release_gesture(world, view);
            draft.data.active_subpath = None;
            draft.data.pending.clear();
            world
                .get_mut::<State>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenState))?
                .draft = Some(draft);
            notify(world, view);
            Ok(true)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn finish_pen_in_world(
        world: &mut World,
        view: Entity,
    ) -> crate::error::PublicResult<Option<Entity>> {
        Self::_finish_pen_in_world(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.finish_pen_in_world",
            ))
        })
    }

    pub(crate) fn _finish_pen_in_world(world: &mut World, view: Entity) -> Result<Option<Entity>> {
        (|| -> Result<Option<Entity>> {
            let (document, paint) = context(world, view)?;
            let Some(draft) = state(world, view)?.draft.clone() else {
                return Ok(None);
            };
            if !drawable(&draft.data.path) || !draft.data.pending.is_empty() {
                crate::input::end(world, view);
                return Ok(None);
            }
            let shape = CanvasDocument::_create_path(
                world,
                document,
                draft.data.path,
                Transform::default(),
                paint,
            )?;
            crate::input::end(world, view);
            if world
                .get::<crate::view::ViewState>(view)
                .is_some_and(|s| s.document == Some(document) && s.mode == CanvasMode::Pen(paint))
                && world.get::<crate::CanvasShape>(shape).is_some()
            {
                Self::set_selection_in_world(world, view, Some(shape))?;
            }
            Ok(Some(shape))
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn cancel_pen_in_world(
        world: &mut World,
        view: Entity,
    ) -> crate::error::PublicResult<bool> {
        Self::_cancel_pen_in_world(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.cancel_pen_in_world",
            ))
        })
    }

    pub(crate) fn _cancel_pen_in_world(world: &mut World, view: Entity) -> Result<bool> {
        (|| -> Result<bool> {
            let changed = state(world, view)?.draft.is_some();
            crate::input::end(world, view);
            Ok(changed)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }
    pub fn close_pen_subpath(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_close_pen_subpath_in_world(world, view)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
    pub fn new_pen_subpath(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_new_pen_subpath_in_world(world, view)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
    pub fn finish_pen(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_finish_pen_in_world(world, view)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
    pub fn cancel_pen(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_cancel_pen_in_world(world, view)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Preview {
    pub(crate) path: CanvasPath,
    pub(crate) points: Vec<Vec2>,
    pub(crate) paint: CanvasPaint,
}

pub(crate) fn preview(world: &World, view: Entity) -> Option<Preview> {
    let state = world.get::<State>(view)?;
    let mut data = state.draft.as_ref().map(|d| d.data.clone());
    let mut paint = state.draft.as_ref().map(|d| d.paint);
    if let Some((point, p)) = crate::input::pen_point(world, view) {
        let d = data.get_or_insert_with(|| CanvasPenDraft {
            path: CanvasPath {
                commands: Vec::new(),
            },
            pending: Vec::new(),
            active_subpath: None,
        });
        advance(d, state.segment, point);
        paint = Some(p);
    }
    let data = data?;
    let mut points = Vec::new();
    for c in &data.path.commands {
        use CanvasPathCommand::*;
        match *c {
            MoveTo(p) | LineTo(p) | ArcTo { to: p, .. } => points.push(p),
            QuadTo { control, to } => points.extend([control, to]),
            CubicTo {
                control1,
                control2,
                to,
            } => points.extend([control1, control2, to]),
            Close => {}
        }
    }
    points.extend(data.pending);
    Some(Preview {
        path: if drawable(&data.path) {
            data.path
        } else {
            CanvasPath {
                commands: Vec::new(),
            }
        },
        points,
        paint: paint?,
    })
}

fn cancel_unavailable(world: &mut World) {
    let views = world
        .query::<(Entity, &State)>()
        .iter(world)
        .filter_map(|(v, s)| s.draft.clone().map(|d| (v, d)))
        .collect::<Vec<_>>();
    for (v, d) in views {
        let pointer = world
            .query::<(&PointerId, &bevy::picking::pointer::PointerLocation)>()
            .iter(world)
            .find(|(id, _)| **id == d.pointer)
            .and_then(|(_, p)| p.location.as_ref())
            .is_some_and(|p| p.target == d.target && p.position.is_finite());
        if !pointer
            || !crate::input::available(world, v)
            || !crate::input::focused(world, &d.target)
            || world.resource::<InputFocus>().get() != Some(v)
            || world.get::<crate::view::ViewState>(v).is_none_or(|s| {
                s.document != Some(d.document) || s.mode != CanvasMode::Pen(d.paint)
            })
        {
            crate::input::end(world, v);
        }
    }
}

fn key(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<&crate::view::ViewState>,
    mut commands: Commands,
) {
    if event.input.state != ButtonState::Pressed
        || !views
            .get(event.focused_entity)
            .is_ok_and(|s| matches!(s.mode, CanvasMode::Pen(_)))
    {
        return;
    }
    let code = event.input.key_code;
    if !matches!(
        code,
        KeyCode::KeyL
            | KeyCode::KeyQ
            | KeyCode::KeyB
            | KeyCode::KeyA
            | KeyCode::KeyC
            | KeyCode::KeyN
            | KeyCode::Enter
            | KeyCode::Escape
    ) {
        return;
    }
    let view = event.focused_entity;
    let window = event.input.window;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            if world.resource::<InputFocus>().get() != Some(view)
                || !crate::input::available(world, view)
                || !world.get::<Window>(window).is_some_and(|w| w.focused)
                || world
                    .get::<crate::view::ViewState>(view)
                    .is_none_or(|s| !matches!(s.mode, CanvasMode::Pen(_)))
            {
                return Ok(());
            }
            match code {
                KeyCode::KeyL => {
                    CanvasView::_set_pen_segment_in_world(world, view, CanvasPenSegment::Line)?;
                }
                KeyCode::KeyQ => {
                    CanvasView::_set_pen_segment_in_world(
                        world,
                        view,
                        CanvasPenSegment::Quadratic,
                    )?;
                }
                KeyCode::KeyB => {
                    CanvasView::_set_pen_segment_in_world(world, view, CanvasPenSegment::Cubic)?;
                }
                KeyCode::KeyA => {
                    CanvasView::_set_pen_segment_in_world(world, view, CanvasPenSegment::Arc)?;
                }
                KeyCode::KeyC => {
                    CanvasView::_close_pen_subpath_in_world(world, view)?;
                }
                KeyCode::KeyN => {
                    CanvasView::_new_pen_subpath_in_world(world, view)?;
                }
                KeyCode::Enter => {
                    CanvasView::_finish_pen_in_world(world, view)?;
                }
                KeyCode::Escape => {
                    CanvasView::_cancel_pen_in_world(world, view)?;
                }
                _ => {}
            }
            Ok(())
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new("canvas.command"))
        })
    });
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(key)
        .add_systems(PreUpdate, cancel_unavailable.after(PickingSystems::Last))
        .add_systems(
            PostUpdate,
            cancel_unavailable
                .after(UiSystems::Layout)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                .before(bevy_widgetry_core::offscreen::OffscreenUpdate),
        );
}
