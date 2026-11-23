use crate::error::CanvasResult as Result;
use crate::grid::SnapTarget;
use crate::{CanvasContainer, CanvasDocument, CanvasDocumentChanged, CanvasView};
use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    ecs::message::{MessageCursor, MessageId},
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, InputFocus, InputFocusSystems},
    picking::{
        PickingSystems,
        backend::HitData,
        events::{
            PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerPress, PointerRelease,
            PointerScroll,
        },
        hover::{HoverMap, PointerCaptureMap},
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput, PointerMap},
    },
    prelude::*,
    ui::{ComputedUiTargetCamera, InteractionDisabled, UiGlobalTransform, UiSystems},
    window::{PrimaryWindow, WindowRef},
};
use std::collections::{HashMap, HashSet};

#[derive(Component, Default)]
pub(crate) struct DragState(Option<Drag>);

#[derive(Component, Default)]
pub(crate) struct GroupClick {
    pressed: Option<GroupPress>,
    previous: Option<(Entity, usize)>,
    seen_press: Option<usize>,
}

struct GroupPress {
    group: Entity,
    pointer: PointerId,
    position: Vec2,
    sequence: PressSequence,
}

#[derive(Clone, Copy)]
struct PressSequence {
    id: usize,
    previous: Option<usize>,
}

struct PrimaryPress {
    pointer: PointerId,
    location: Location,
    sequence: PressSequence,
}

#[derive(Resource, Default)]
struct FrameInputs {
    events: Vec<(usize, PointerInput)>,
    presses: Vec<PrimaryPress>,
    last_press: HashMap<PointerId, usize>,
}

#[derive(Resource, Default)]
struct HistoryModifiers {
    control_left: bool,
    control_right: bool,
    shift_left: bool,
    shift_right: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum DrawingKind {
    Rectangle,
    Ellipse,
    Line,
    Parametric(crate::CanvasPathTool),
    Flowchart(crate::CanvasFlowchartPreset),
}

impl DrawingKind {
    pub(crate) fn geometry(self, size: Vec2) -> crate::CanvasGeometry {
        match self {
            Self::Rectangle => crate::CanvasGeometry::Rectangle(crate::CanvasRectangle {
                corner_radius: 0.0,
                size,
            }),
            Self::Ellipse => {
                crate::CanvasGeometry::Ellipse(crate::CanvasEllipse { radii: size * 0.5 })
            }
            Self::Flowchart(preset) => preset.outline(size),
            Self::Parametric(tool) => crate::CanvasGeometry::Path(tool.path(size)),
            Self::Line => crate::CanvasGeometry::Path(crate::CanvasPath {
                commands: vec![
                    crate::CanvasPathCommand::MoveTo(-size * 0.5),
                    crate::CanvasPathCommand::LineTo(size * 0.5),
                ],
            }),
        }
    }

    pub(crate) fn placement(
        self,
        world: &World,
        view: Entity,
        start: Vec2,
        end: Vec2,
    ) -> (Vec2, Vec2) {
        let snapped = crate::grid::point(world, view, SnapTarget::Drawing, end);
        let delta = snapped.unwrap_or(end) - start;
        let midpoint = start + delta * 0.5;
        match self {
            Self::Parametric(tool) if tool.radial() => {
                let snap = snapped.is_none()
                    && world
                        .get_resource::<ButtonInput<KeyCode>>()
                        .is_some_and(|keys| {
                            keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
                        });
                let radius = if snap {
                    let step = crate::rotate::SNAP_ANGLE_RADIANS as f32;
                    Vec2::from_angle((delta.to_angle() / step).round() * step) * delta.length()
                } else {
                    delta
                };
                (start, radius)
            }
            Self::Line | Self::Parametric(_) => (midpoint, delta),
            Self::Rectangle | Self::Ellipse | Self::Flowchart(_) => (midpoint, delta.abs()),
        }
    }

    pub(crate) fn committable(self, size: Vec2) -> bool {
        match self {
            Self::Parametric(tool) => tool.accepts_drag(size),
            Self::Line => size != Vec2::ZERO,
            Self::Rectangle | Self::Ellipse | Self::Flowchart(_) => size.min_element() >= 1.0,
        }
    }

    fn mode(self, paint: crate::CanvasPaint) -> crate::CanvasMode {
        match self {
            Self::Rectangle => crate::CanvasMode::Rectangle(paint),
            Self::Ellipse => crate::CanvasMode::Ellipse(paint),
            Self::Flowchart(preset) => crate::CanvasMode::Flowchart(preset, paint),
            Self::Parametric(tool) => crate::CanvasMode::Parametric(tool, paint),
            Self::Line => crate::CanvasMode::Line(paint),
        }
    }
}

#[derive(Clone)]
struct Drag {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    local: Vec2,
    hit: HitData,
    raw_driven: bool,
    edit: Option<crate::CanvasEditToken>,
    action: DragAction,
}

#[derive(Clone)]
enum DragAction {
    Pan,
    Contour,
    Scale(crate::scale::Scale),
    Rotate(crate::rotate::Rotate),
    Parameters(crate::parameters::Edit),
    Node(crate::nodes::Edit),
    MoveSelection(crate::move_selection::MoveSelection),
    Pen {
        document: Entity,
        container: CanvasContainer,
        paint: crate::CanvasPaint,
        current: Vec2,
    },
    Move {
        document: Entity,
        shape: Entity,
        offset: bevy::math::DVec2,
        parent: bevy::math::Affine3A,
    },
    Draw {
        kind: DrawingKind,
        document: Entity,
        container: CanvasContainer,
        start: Vec2,
        current: Vec2,
        paint: crate::CanvasPaint,
    },
}

impl DragAction {
    fn edit_document(&self) -> Option<Entity> {
        match self {
            Self::Move { document, .. } => Some(*document),
            Self::MoveSelection(edit) => Some(edit.document),
            Self::Rotate(edit) => Some(edit.document),
            Self::Scale(edit) => Some(edit.document),
            Self::Parameters(edit) => Some(edit.document),
            Self::Node(edit) => Some(edit.document),
            Self::Pan | Self::Contour | Self::Draw { .. } | Self::Pen { .. } => None,
        }
    }
}

#[derive(Resource, Default)]
struct EscapeDispatch {
    focused: MessageCursor<KeyboardInput>,
    handled: HashSet<MessageId<KeyboardInput>>,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PointerCaptureMap>()
        .init_resource::<FrameInputs>()
        .init_resource::<EscapeDispatch>()
        .init_resource::<HistoryModifiers>()
        .add_observer(press)
        .add_observer(enter_group_on_click)
        .add_observer(moved)
        .add_observer(released)
        .add_observer(cancelled)
        .add_observer(drag_ended)
        .add_observer(document_changed)
        .add_observer(scrolled)
        .add_observer(removed)
        .add_observer(delete_selected)
        .add_observer(focused_escape)
        .add_observer(edit_history)
        .add_systems(
            PreUpdate,
            history_modifiers.before(bevy::input::InputSystems),
        )
        .add_systems(
            PreUpdate,
            process_input
                .after(bevy::picking::hover::generate_hovermap)
                .before(bevy::picking::hover::update_interactions)
                .in_set(PickingSystems::Hover),
        )
        .add_systems(PreUpdate, cancel_unavailable.after(PickingSystems::Last))
        .add_systems(
            PreUpdate,
            undispatched_escapes
                .after(InputFocusSystems::Dispatch)
                .after(PickingSystems::Last),
        )
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

pub(crate) fn available(world: &World, entity: Entity) -> bool {
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

pub(crate) fn focused(world: &World, target: &NormalizedRenderTarget) -> bool {
    match target {
        NormalizedRenderTarget::Window(window) => world
            .get::<Window>(window.entity())
            .is_some_and(|window| window.focused),
        _ => true,
    }
}

fn view_render_target(world: &World, view: Entity) -> Option<&RenderTarget> {
    let camera = world.get::<ComputedUiTargetCamera>(view)?.get()?;
    world.get::<RenderTarget>(camera)
}

pub(crate) fn keyboard_available(world: &World, view: Entity, window: Entity) -> bool {
    if world.resource::<InputFocus>().get() != Some(view)
        || !available(world, view)
        || !world
            .get::<Window>(window)
            .is_some_and(|window| window.focused)
    {
        return false;
    }
    match view_render_target(world, view) {
        Some(RenderTarget::Window(WindowRef::Primary)) => {
            world.get::<PrimaryWindow>(window).is_some()
        }
        Some(RenderTarget::Window(WindowRef::Entity(owner))) => *owner == window,
        Some(_) => true,
        None => false,
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
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            let sequence = group_press_sequence(world, &press);
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
                } else if world
                    .get::<DragState>(press.entity)
                    .and_then(|s| s.0.as_ref())
                    .is_some_and(|d| matches!(d.action, DragAction::Pen { .. }))
                {
                    complete_pen(world, press.entity)?;
                } else if world
                    .get::<DragState>(press.entity)
                    .and_then(|s| s.0.as_ref())
                    .is_some_and(|d| matches!(d.action, DragAction::Contour))
                {
                    if let Some(point) = preceding_contour_release(world, &press, sequence) {
                        finish(world, press.entity, point)?;
                    } else {
                        release_gesture(world, press.entity);
                        crate::basic_path::interrupt_click(world, press.entity);
                    }
                } else if world
                    .get::<DragState>(press.entity)
                    .and_then(|state| state.0.as_ref())
                    .is_some_and(|drag| matches!(drag.action, DragAction::MoveSelection(_)))
                {
                    complete_selection(world, press.entity)?;
                } else {
                    end(world, press.entity);
                }
            }
            let space = world
                .get_resource::<ButtonInput<KeyCode>>()
                .is_some_and(|keys| keys.pressed(KeyCode::Space));
            let action = if space {
                crate::basic_path::interrupt_click(world, press.entity);
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
                let zoom = CanvasView::_zoom(world, press.entity)?;
                let point = CanvasView::_center(world, press.entity)? + local / zoom;
                if !point.is_finite() {
                    return Ok(());
                }
                if matches!(
                    CanvasView::_mode(world, press.entity)?,
                    crate::CanvasMode::Select | crate::CanvasMode::PathNodeEdit
                ) && world
                    .get_resource::<ButtonInput<KeyCode>>()
                    .is_some_and(|keys| {
                        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
                    })
                {
                    if let Some(shape) = crate::selection::hit_test(world, press.entity, point)? {
                        let mut selection = CanvasView::_selection(world, press.entity)?;
                        if selection.contains(&shape) {
                            selection.retain(|entity| *entity != shape);
                        } else {
                            selection.push(shape);
                        }
                        CanvasView::_set_selection_in_world(world, press.entity, selection)?;
                    }
                    return Ok(());
                }
                let drawing = match CanvasView::_mode(world, press.entity)? {
                    crate::CanvasMode::Select
                    | crate::CanvasMode::PathNodeEdit
                    | crate::CanvasMode::Pen(_)
                    | crate::CanvasMode::Polyline(_)
                    | crate::CanvasMode::Connector(_)
                    | crate::CanvasMode::Text
                    | crate::CanvasMode::Polygon(_) => None,
                    crate::CanvasMode::Rectangle(paint) => Some((DrawingKind::Rectangle, paint)),
                    crate::CanvasMode::Ellipse(paint) => Some((DrawingKind::Ellipse, paint)),
                    crate::CanvasMode::Flowchart(preset, paint) => {
                        Some((DrawingKind::Flowchart(preset), paint))
                    }
                    crate::CanvasMode::Parametric(tool, paint) => {
                        Some((DrawingKind::Parametric(tool), paint))
                    }
                    crate::CanvasMode::Line(paint) => Some((DrawingKind::Line, paint)),
                };
                if matches!(
                    CanvasView::_mode(world, press.entity)?,
                    crate::CanvasMode::Text
                        | crate::CanvasMode::Polyline(_)
                        | crate::CanvasMode::Connector(_)
                        | crate::CanvasMode::Polygon(_)
                ) {
                    DragAction::Contour
                } else if let crate::CanvasMode::Pen(paint) =
                    CanvasView::_mode(world, press.entity)?
                {
                    let (Some(document), Some(container)) = (
                        CanvasView::_document(world, press.entity)?,
                        CanvasView::drawing_target(world, press.entity)?,
                    ) else {
                        return Ok(());
                    };
                    if !crate::pen::accepts(
                        world,
                        press.entity,
                        press.pointer.id,
                        &press.pointer.target,
                    ) {
                        return Ok(());
                    }
                    DragAction::Pen {
                        document,
                        container,
                        paint,
                        current: crate::grid::point(
                            world,
                            press.entity,
                            SnapTarget::Drawing,
                            point,
                        )
                        .unwrap_or(point),
                    }
                } else if let Some((kind, paint)) = drawing {
                    let (Some(document), Some(container)) = (
                        CanvasView::_document(world, press.entity)?,
                        CanvasView::drawing_target(world, press.entity)?,
                    ) else {
                        return Ok(());
                    };
                    DragAction::Draw {
                        kind,
                        document,
                        container,
                        start: crate::grid::point(world, press.entity, SnapTarget::Drawing, point)
                            .unwrap_or(point),
                        current: point,
                        paint,
                    }
                } else if CanvasView::_mode(world, press.entity)? == crate::CanvasMode::PathNodeEdit
                {
                    let selection = CanvasView::_single_selection(world, press.entity)?;
                    if let Some(shape) = selection
                        && let Some(handle) = crate::nodes::hit(world, shape, point, zoom)?
                        && let Some(document) = CanvasView::_document(world, press.entity)?
                    {
                        DragAction::Node(crate::nodes::Edit::begin(
                            world, document, shape, handle, point,
                        )?)
                    } else {
                        let selection = crate::selection::hit_test(world, press.entity, point)?;
                        CanvasView::_set_selection_in_world(world, press.entity, selection)?;
                        return Ok(());
                    }
                } else if let selection = CanvasView::_selection(world, press.entity)?
                    && selection.len() > 1
                    && let Some(shape) = crate::selection::hit_test(world, press.entity, point)?
                        .filter(|shape| selection.contains(shape))
                    && let Some(document) = CanvasView::_document(world, press.entity)?
                {
                    let action = crate::move_selection::MoveSelection::begin(
                        world, document, selection, shape, point,
                    )?;
                    track_group_body_press(world, &press, shape, sequence);
                    DragAction::MoveSelection(action)
                } else {
                    let previous = CanvasView::_single_selection(world, press.entity)?;
                    let rotation = if let Some(shape) = previous {
                        crate::rotate::hit(world, shape, point, zoom)?.then_some(shape)
                    } else {
                        None
                    };
                    if let Some(shape) = rotation {
                        let Some(document) = CanvasView::_document(world, press.entity)? else {
                            return Ok(());
                        };
                        let Some(rotate) =
                            crate::rotate::Rotate::begin(world, document, shape, point)?
                        else {
                            return Ok(());
                        };
                        DragAction::Rotate(rotate)
                    } else {
                        let handle = if let Some(shape) = previous {
                            crate::scale::hit(world, shape, point, zoom)?
                                .map(|(handle, d)| (shape, handle, d))
                        } else {
                            None
                        };
                        let parameter = if let Some(shape) = previous {
                            crate::parameters::hit(world, shape, point, zoom)?
                                .map(|(h, d)| (shape, h, d))
                        } else {
                            None
                        };
                        if let Some((shape, parameter, _)) = parameter
                            .filter(|(_, _, d)| handle.is_none_or(|(_, _, other)| *d < other))
                        {
                            let Some(document) = CanvasView::_document(world, press.entity)? else {
                                return Ok(());
                            };
                            DragAction::Parameters(crate::parameters::Edit::begin(
                                world, document, shape, point, parameter,
                            )?)
                        } else if let Some((shape, handle, _)) = handle {
                            let Some(document) = CanvasView::_document(world, press.entity)? else {
                                return Ok(());
                            };
                            if world.get::<crate::CanvasGroup>(shape).is_some()
                                && crate::selection::hit_test(world, press.entity, point)?
                                    == Some(shape)
                            {
                                track_group_body_press(world, &press, shape, sequence);
                            }
                            DragAction::Scale(crate::scale::Scale::begin(
                                world, document, shape, point, handle,
                            )?)
                        } else {
                            let selection = crate::selection::hit_test(world, press.entity, point)?;
                            if let Some(item) = selection {
                                track_group_body_press(world, &press, item, sequence);
                            }
                            CanvasView::_set_selection_in_world(world, press.entity, selection)?;
                            let Some(shape) = selection.filter(|_| previous == selection) else {
                                return Ok(());
                            };
                            let Some(document) = CanvasView::_document(world, press.entity)? else {
                                return Ok(());
                            };
                            let Some(transform) = world.get::<Transform>(shape) else {
                                return Ok(());
                            };
                            let parent = crate::groups::parent_transform(world, shape)?;
                            DragAction::Move {
                                document,
                                shape,
                                parent,
                                offset: crate::scale::local_point(parent, point)
                                    - transform.translation.truncate().as_dvec2(),
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
            if matches!(action, DragAction::Contour)
                && !crate::basic_path::begin_click(world, &press, local)?
            {
                return Ok(());
            }
            let edit = if let Some(document) = action.edit_document() {
                let Some(edit) = crate::history::begin_gesture(world, document)? else {
                    return Ok(());
                };
                Some(edit)
            } else {
                None
            };
            if let Some(mut state) = world.get_mut::<DragState>(press.entity) {
                state.0 = Some(Drag {
                    pointer: press.pointer.id,
                    target: press.pointer.target.clone(),
                    local,
                    hit: press.hit.clone(),
                    raw_driven: false,
                    edit,
                    action,
                });
            }
            world.resource_mut::<PointerCaptureMap>().capture(
                press.pointer.id,
                press.entity,
                press.hit,
            );
            Ok(())
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new("canvas.command"))
        })
    });
}

fn group_press_sequence(world: &mut World, press: &PointerPress) -> Option<PressSequence> {
    let seen = world.get::<GroupClick>(press.entity)?.seen_press;
    let sequence = world
        .resource::<FrameInputs>()
        .presses
        .iter()
        .find_map(|input| {
            (input.pointer == press.pointer.id
                && input.location.target == press.pointer.target
                && input.location.position == press.pointer.position
                && seen.is_none_or(|seen| input.sequence.id > seen))
            .then_some(input.sequence)
        });
    let mut clicks = world.get_mut::<GroupClick>(press.entity)?;
    clicks.pressed = None;
    if let Some(sequence) = sequence {
        clicks.seen_press = Some(sequence.id);
    }
    sequence
}

fn preceding_contour_release(
    world: &World,
    press: &PointerPress,
    sequence: Option<PressSequence>,
) -> Option<Vec2> {
    let sequence = sequence?;
    let previous = sequence.previous?;
    world
        .resource::<FrameInputs>()
        .events
        .iter()
        .rev()
        .find_map(|(id, input)| {
            (*id > previous
                && *id < sequence.id
                && input.pointer_id == press.pointer.id
                && input.location.target == press.pointer.target
                && matches!(input.action, PointerAction::Release(PointerButton::Primary)))
            .then_some(input.location.position)
        })
}

fn track_group_body_press(
    world: &mut World,
    press: &PointerPress,
    item: Entity,
    sequence: Option<PressSequence>,
) {
    if let Some(sequence) = sequence
        && world.get::<crate::CanvasGroup>(item).is_some()
        && let Some(mut clicks) = world.get_mut::<GroupClick>(press.entity)
    {
        clicks.pressed = Some(GroupPress {
            group: item,
            pointer: press.pointer.id,
            position: press.pointer.position,
            sequence,
        });
    }
}

fn enter_group_on_click(
    click: On<PointerClick>,
    views: Query<(), With<CanvasView>>,
    mut commands: Commands,
) {
    if click.original_event_target() != click.entity
        || !views.contains(click.entity)
        || click.button != PointerButton::Primary
    {
        return;
    }
    let event = click.event().clone();
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            let view = event.entity;
            let Some(mut clicks) = world.get_mut::<GroupClick>(view) else {
                return Ok(());
            };
            let Some(pressed) = clicks.pressed.take() else {
                return Ok(());
            };
            let previous = clicks.previous.take();
            if !available(world, view)
                || !focused(world, &event.pointer.target)
                || world.resource::<InputFocus>().get() != Some(view)
                || CanvasView::_mode(world, view)? != crate::CanvasMode::Select
                || pressed.pointer != event.pointer.id
                || world
                    .get_resource::<ButtonInput<KeyCode>>()
                    .is_some_and(|keys| {
                        [KeyCode::Space, KeyCode::ControlLeft, KeyCode::ControlRight]
                            .into_iter()
                            .any(|key| keys.pressed(key))
                    })
            {
                return Ok(());
            }
            let Some(local) =
                inside_position(world, view, &event.pointer.target, event.pointer.position)
            else {
                return Ok(());
            };
            let point = CanvasView::_center(world, view)? + local / CanvasView::_zoom(world, view)?;
            if point.is_finite()
                && crate::selection::hit_test(world, view, point)? == Some(pressed.group)
            {
                let repeats_previous = pressed
                    .sequence
                    .previous
                    .is_some_and(|id| previous == Some((pressed.group, id)));
                if event.count >= 2 && repeats_previous {
                    CanvasView::_enter_group(world, view, pressed.group)?;
                } else if let Some(mut clicks) = world.get_mut::<GroupClick>(view) {
                    clicks.previous = Some((pressed.group, pressed.sequence.id));
                }
            }
            Ok(())
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.input.enter_group",
            ))
        })
    });
}

fn moved(event: On<PointerDrag>, views: Query<(), With<CanvasView>>, mut commands: Commands) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let event = event.event().clone();
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            cancel_group_click_on_motion(
                world,
                event.entity,
                event.pointer.id,
                event.pointer.position,
            );
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
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new("canvas.command"))
        })
    });
}

fn cancel_group_click_on_motion(
    world: &mut World,
    view: Entity,
    pointer: PointerId,
    position: Vec2,
) {
    if let Some(mut clicks) = world.get_mut::<GroupClick>(view)
        && clicks
            .pressed
            .as_ref()
            .is_some_and(|press| press.pointer == pointer && press.position != position)
    {
        clicks.pressed = None;
    }
}

fn advance(world: &mut World, entity: Entity, point: Vec2) -> Result {
    (|| -> Result {
        let Some(drag) = world
            .get::<DragState>(entity)
            .and_then(|state| state.0.clone())
        else {
            return Ok(());
        };
        cancel_group_click_on_motion(world, entity, drag.pointer, point);
        if !continuing(world, entity, &drag) {
            end(world, entity);
            return Ok(());
        }
        let Some(local) = position(world, entity, point) else {
            end(world, entity);
            return Ok(());
        };
        let zoom = CanvasView::_zoom(world, entity)?;
        if let Some(mut state) = world.get_mut::<DragState>(entity)
            && let Some(drag) = &mut state.0
        {
            drag.local = local;
        }
        if let Some(token) = drag.edit {
            crate::history::apply(world, token, |world, _| {
                advance_action(world, entity, &drag, local, zoom)
            })?;
        } else {
            advance_action(world, entity, &drag, local, zoom)?;
        }
        if !continuing(world, entity, &drag) {
            end(world, entity);
        }
        Ok(())
    })()
    .map_err(|error| error.at("entity", entity))
}

fn advance_action(
    world: &mut World,
    entity: Entity,
    drag: &Drag,
    local: Vec2,
    zoom: f32,
) -> Result {
    match drag.action.clone() {
        DragAction::Contour => crate::basic_path::dragged(world, entity, local),
        DragAction::Pan => {
            let center = CanvasView::_center(world, entity)? - (local - drag.local) / zoom;
            if !center.is_finite() {
                end(world, entity);
                return Ok(());
            }
            CanvasView::_set_center_in_world(world, entity, center)?;
        }
        DragAction::Move {
            document,
            shape,
            offset,
            parent,
        } => {
            let pointer = CanvasView::_center(world, entity)? + local / zoom;
            let position = crate::scale::local_point(parent, pointer) - offset;
            let Some(mut transform) = world.get::<Transform>(shape).copied() else {
                end(world, entity);
                return Ok(());
            };
            let anchor = position + crate::grid::move_anchor_offset(world, shape, transform);
            let correction =
                crate::grid::local_delta(world, entity, SnapTarget::Movement, parent, anchor);
            let point = (position + correction).as_vec2();
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            transform.translation.x = point.x;
            transform.translation.y = point.y;
            crate::groups::update_item_transform(world, document, shape, transform)?;
        }
        DragAction::Scale(scale) => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            scale.apply(world, point)?;
        }
        DragAction::Rotate(rotate) => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
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
        DragAction::Parameters(edit) => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            edit.apply(world, point)?;
        }
        DragAction::Node(edit) => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            edit.apply(world, entity, point)?;
        }
        DragAction::MoveSelection(group) => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            let moved = group.apply(world, entity, point)?;
            if let Some(mut state) = world.get_mut::<DragState>(entity)
                && let Some(Drag {
                    action: DragAction::MoveSelection(group),
                    ..
                }) = &mut state.0
            {
                group.moved |= moved;
            }
        }
        DragAction::Draw { .. } | DragAction::Pen { .. } => {
            let point = CanvasView::_center(world, entity)? + local / zoom;
            if !point.is_finite() {
                end(world, entity);
                return Ok(());
            }
            let point = if matches!(&drag.action, DragAction::Pen { .. }) {
                crate::grid::point(world, entity, SnapTarget::Drawing, point).unwrap_or(point)
            } else {
                point
            };
            if let Some(mut state) = world.get_mut::<DragState>(entity)
                && let Some(Drag {
                    action: DragAction::Draw { current, .. } | DragAction::Pen { current, .. },
                    ..
                }) = &mut state.0
            {
                *current = point;
            }
        }
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
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.input.scroll",
            ))
        })
    });
}

pub(crate) fn inside_position(
    world: &World,
    entity: Entity,
    target: &NormalizedRenderTarget,
    point: Vec2,
) -> Option<Vec2> {
    if !available(world, entity) || !focused(world, target) {
        return None;
    }
    let render_target = view_render_target(world, entity)?;
    let matches = match render_target {
        RenderTarget::Window(WindowRef::Primary) => {
            matches!(target, NormalizedRenderTarget::Window(window) if world.get::<PrimaryWindow>(window.entity()).is_some())
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
    (|| -> Result {
        if !y.is_finite() || y == 0.0 {
            return Ok(());
        }
        let Some(local) = inside_position(world, entity, target, point) else {
            return Ok(());
        };
        let pixels_per_scroll_line = 100.0;
        let log_zoom_per_scroll_line = 0.1;
        let lines = match unit {
            bevy::input::mouse::MouseScrollUnit::Line => y,
            bevy::input::mouse::MouseScrollUnit::Pixel => y / pixels_per_scroll_line,
        };
        let old = CanvasView::_zoom(world, entity)?;
        let zoom = (old.ln() + lines * log_zoom_per_scroll_line)
            .clamp(crate::view::MIN_ZOOM.ln(), crate::view::MAX_ZOOM.ln())
            .exp()
            .clamp(crate::view::MIN_ZOOM, crate::view::MAX_ZOOM);
        if old == zoom {
            return Ok(());
        }
        let center = CanvasView::_center(world, entity)? + local * (old.recip() - zoom.recip());
        if !center.is_finite() {
            return Ok(());
        }
        crate::view::set_zoom_and_center(world, entity, zoom, center)?;
        Ok(())
    })()
    .map_err(|error| error.at("entity", entity))
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
        && match drag.action.clone() {
            DragAction::Pan => true,
            DragAction::Contour => crate::basic_path::press_valid(world, entity),
            DragAction::MoveSelection(group) => group.valid(world, entity),
            DragAction::Node(edit) => {
                world
                    .get::<crate::view::ViewState>(entity)
                    .is_some_and(|s| {
                        s.document == Some(edit.document)
                            && s.selection == [edit.shape]
                            && s.mode == crate::CanvasMode::PathNodeEdit
                            && crate::selection::selectable(
                                world,
                                Some(edit.document),
                                crate::view::edited_group(world, s),
                                edit.shape,
                            )
                    })
                    && edit.valid(world)
            }
            DragAction::Move {
                document,
                shape,
                parent,
                ..
            }
            | DragAction::Scale(crate::scale::Scale {
                document,
                shape,
                parent,
                ..
            })
            | DragAction::Rotate(crate::rotate::Rotate {
                document,
                shape,
                parent,
                ..
            })
            | DragAction::Parameters(crate::parameters::Edit {
                document,
                shape,
                parent,
                ..
            }) => {
                crate::groups::parent_transform(world, shape).is_ok_and(|current| current == parent)
                    && world
                        .get::<crate::view::ViewState>(entity)
                        .is_some_and(|state| {
                            state.document == Some(document)
                                && state.selection == [shape]
                                && crate::selection::selectable(
                                    world,
                                    Some(document),
                                    crate::view::edited_group(world, state),
                                    shape,
                                )
                        })
            }
            DragAction::Draw {
                kind,
                document,
                paint,
                ..
            } => world
                .get::<crate::view::ViewState>(entity)
                .is_some_and(|state| {
                    state.document == Some(document) && state.mode == kind.mode(paint)
                }),
            DragAction::Pen {
                document,
                container,
                paint,
                ..
            } => {
                world
                    .get::<crate::view::ViewState>(entity)
                    .is_some_and(|s| {
                        s.document == Some(document) && s.mode == crate::CanvasMode::Pen(paint)
                    })
                    && CanvasView::drawing_target(world, entity)
                        .is_ok_and(|active| active == Some(container))
            }
        }
}

fn process_input(
    world: &mut World,
    mut cursor: Local<MessageCursor<PointerInput>>,
) -> bevy::prelude::Result {
    (|| -> Result {
        let mut mutations = cursor.clone();
        let inputs = cursor
            .read_with_id(world.resource::<Messages<PointerInput>>())
            .map(|(input, id)| (id.id, input.clone()))
            .collect::<Vec<_>>();
        world.resource_scope(|world, mut frame: Mut<FrameInputs>| {
            let pointers = world.resource::<PointerMap>();
            frame
                .last_press
                .retain(|pointer, _| pointers.get_entity(*pointer).is_some());
            frame.events.clear();
            frame.presses.clear();
            for (id, input) in &inputs {
                frame.events.push((*id, input.clone()));
                match input.action {
                    PointerAction::Press(PointerButton::Primary) => {
                        // Raw presses have one sequence entry even when Picking emits zero or several events.
                        let previous = frame.last_press.insert(input.pointer_id, *id);
                        frame.presses.push(PrimaryPress {
                            pointer: input.pointer_id,
                            location: input.location.clone(),
                            sequence: PressSequence { id: *id, previous },
                        });
                    }
                    PointerAction::Cancel => {
                        frame.last_press.remove(&input.pointer_id);
                    }
                    _ => {}
                }
            }
        });
        for (_, input) in &inputs {
            if matches!(input.action, PointerAction::Cancel) {
                crate::pen::cancel_pointer(world, input.pointer_id, &input.location.target);
                crate::basic_path::cancel_pointer(world, input.pointer_id, &input.location.target);
            }
        }
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
            if world
                .get::<DragState>(entity)
                .is_some_and(|s| s.0.is_some())
                && !continuing(world, entity, &drag)
            {
                end(world, entity);
            } else if capture.is_none() && continuing(world, entity, &drag) {
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
    })()
    .map_err(|error| {
        error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
            "canvas.input.process_input",
        ))
    })
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
    release_gesture(world, entity);
    crate::pen::cancel(world, entity);
    crate::basic_path::cancel(world, entity);
}

pub(crate) fn cancel_drawing_tools(world: &mut World, view: Entity) {
    if world
        .get::<DragState>(view)
        .and_then(|state| state.0.as_ref())
        .is_some_and(|drag| {
            matches!(
                drag.action,
                DragAction::Draw { .. } | DragAction::Pen { .. } | DragAction::Contour
            )
        })
    {
        release_gesture(world, view);
    }
    crate::pen::cancel(world, view);
    crate::basic_path::cancel(world, view);
}

pub(crate) fn release_gesture(world: &mut World, entity: Entity) {
    if let Some(drag) = world
        .get_mut::<DragState>(entity)
        .and_then(|mut state| state.0.take())
    {
        release_capture(
            &mut world.resource_mut::<PointerCaptureMap>(),
            entity,
            &drag,
        );
        if let Some(edit) = drag.edit {
            crate::history::finish_gesture(world, edit);
        }
        bevy_widgetry_core::pointer::refresh_hover(world, drag.pointer);
    }
}

pub(crate) fn preview(
    world: &World,
    entity: Entity,
) -> Option<(DrawingKind, Vec2, Vec2, crate::CanvasPaint)> {
    let drag = world.get::<DragState>(entity)?.0.as_ref()?;
    match drag.action {
        DragAction::Draw {
            kind,
            start,
            current,
            paint,
            ..
        } => Some((kind, start, current, paint)),
        _ => None,
    }
}

fn finish(world: &mut World, entity: Entity, point: Vec2) -> Result {
    (|| -> Result {
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
        match drag.action {
            DragAction::Contour => {
                release_gesture(world, entity);
                crate::basic_path::add_vertex(world, entity, point)
            }
            DragAction::MoveSelection(_) => {
                advance(world, entity, point)?;
                complete_selection(world, entity)
            }
            DragAction::Draw { .. } => {
                advance(world, entity, point)?;
                complete_drawing(world, entity)
            }
            DragAction::Pen { .. } => {
                advance(world, entity, point)?;
                complete_pen(world, entity)
            }
            _ => {
                release_gesture(world, entity);
                Ok(())
            }
        }
    })()
    .map_err(|error| error.at("entity", entity))
}

fn complete_selection(world: &mut World, entity: Entity) -> Result {
    (|| -> Result {
        let group = world
            .get::<DragState>(entity)
            .and_then(|state| state.0.as_ref())
            .and_then(|drag| {
                if let DragAction::MoveSelection(group) = &drag.action {
                    Some(group.clone())
                } else {
                    None
                }
            });
        release_gesture(world, entity);
        if let Some(group) = group {
            group.finish(world, entity)?;
        }
        Ok(())
    })()
    .map_err(|error| error.at("entity", entity))
}

pub(crate) fn pen_point(world: &World, view: Entity) -> Option<(Vec2, crate::CanvasPaint)> {
    let d = world.get::<DragState>(view)?.0.as_ref()?;
    if let DragAction::Pen { current, paint, .. } = d.action {
        Some((current, paint))
    } else {
        None
    }
}

fn complete_pen(world: &mut World, view: Entity) -> Result {
    (|| -> Result {
        let Some(drag) = world
            .get::<DragState>(view)
            .and_then(|s| s.0.clone())
            .filter(|d| continuing(world, view, d))
        else {
            end(world, view);
            return Ok(());
        };
        let DragAction::Pen {
            document,
            container,
            paint,
            current,
        } = drag.action
        else {
            return Ok(());
        };
        release_gesture(world, view);
        crate::pen::append_point(
            world,
            view,
            document,
            container,
            paint,
            drag.pointer,
            drag.target,
            current,
        )
    })()
    .map_err(|error| error.at("view", view))
}

fn complete_drawing(world: &mut World, entity: Entity) -> Result {
    (|| -> Result {
        let drawing = world
            .get::<DragState>(entity)
            .and_then(|state| state.0.as_ref())
            .filter(|drag| continuing(world, entity, drag))
            .map(|drag| drag.action.clone());
        end(world, entity);
        let Some(DragAction::Draw {
            kind,
            document,
            container,
            start,
            current,
            paint,
        }) = drawing
        else {
            return Ok(());
        };
        if crate::history::is_busy(world, document) {
            return Ok(());
        }
        let (center, size) = kind.placement(world, entity, start, current);
        CanvasView::_set_mode_in_world(world, entity, crate::CanvasMode::Select)?;
        world.flush();
        if !center.is_finite()
            || !size.is_finite()
            || !kind.committable(size)
            || world
                .get::<crate::view::ViewState>(entity)
                .is_none_or(|state| {
                    state.document != Some(document) || state.mode != crate::CanvasMode::Select
                })
            || !available(world, entity)
            || CanvasView::drawing_target(world, entity)? != Some(container)
        {
            return Ok(());
        }
        let transform = Transform::from_translation(center.extend(0.0));
        let shape = CanvasDocument::create_drawing(
            world,
            document,
            container,
            kind.geometry(size),
            transform,
            paint,
        )?;
        crate::selection::select_committed_shape(
            world,
            entity,
            document,
            crate::CanvasMode::Select,
            shape,
        )
    })()
    .map_err(|error| error.at("entity", entity))
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
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            if !keyboard_available(world, view, window) {
                return Ok(());
            }
            let shapes = CanvasView::_selection(world, view)?;
            if shapes.is_empty() {
                return Ok(());
            }
            let Some(document) = CanvasView::_document(world, view)? else {
                return Ok(());
            };
            if world
                .get::<DragState>(view)
                .and_then(|state| state.0.as_ref())
                .is_some_and(|drag| drag.edit.is_some())
            {
                end(world, view);
            }
            if crate::history::is_busy(world, document) {
                return Ok(());
            }
            crate::batch::remove_selected_items(world, document, &shapes)?;
            Ok(())
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new("canvas.command"))
        })
    });
}

fn history_modifiers(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut modifiers: ResMut<HistoryModifiers>,
) {
    // InputSystems has not yet applied this frame's KeyboardInput messages.
    *modifiers = keys.map_or_else(HistoryModifiers::default, |keys| HistoryModifiers {
        control_left: keys.pressed(KeyCode::ControlLeft),
        control_right: keys.pressed(KeyCode::ControlRight),
        shift_left: keys.pressed(KeyCode::ShiftLeft),
        shift_right: keys.pressed(KeyCode::ShiftRight),
    });
}

fn edit_history(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<CanvasView>>,
    mut modifiers: ResMut<HistoryModifiers>,
    mut commands: Commands,
) {
    if event.original_event_target() != event.focused_entity {
        return;
    }
    let pressed = event.input.state == ButtonState::Pressed;
    let modifier = match event.input.key_code {
        KeyCode::ControlLeft => Some(&mut modifiers.control_left),
        KeyCode::ControlRight => Some(&mut modifiers.control_right),
        KeyCode::ShiftLeft => Some(&mut modifiers.shift_left),
        KeyCode::ShiftRight => Some(&mut modifiers.shift_right),
        _ => None,
    };
    if let Some(modifier) = modifier {
        *modifier = pressed;
        return;
    }
    if !views.contains(event.focused_entity)
        || !pressed
        || event.input.repeat
        || !(modifiers.control_left || modifiers.control_right)
    {
        return;
    }
    let redo = match event.input.key_code {
        KeyCode::KeyZ => modifiers.shift_left || modifiers.shift_right,
        KeyCode::KeyY => true,
        _ => return,
    };
    let view = event.focused_entity;
    let window = event.input.window;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
            if !keyboard_available(world, view, window) {
                return Ok(());
            }
            let Some(document) = CanvasView::_document(world, view)? else {
                return Ok(());
            };
            if !crate::history::is_busy(world, document) {
                if redo {
                    CanvasDocument::redo(world, document)?;
                } else {
                    CanvasDocument::undo(world, document)?;
                }
            }
            Ok(())
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.input.history",
            ))
        })
    });
}

fn is_escape_press(input: &KeyboardInput) -> bool {
    input.key_code == KeyCode::Escape && input.state == ButtonState::Pressed && !input.repeat
}

fn focused_escape(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<&crate::view::ViewState>,
    messages: Option<Res<Messages<KeyboardInput>>>,
    mut dispatch: ResMut<EscapeDispatch>,
    mut commands: Commands,
) {
    let view = event.focused_entity;
    if event.original_event_target() != view {
        return;
    }

    // FocusedInput omits the message ID. Following each dispatched key preserves stream order.
    let id = messages.as_deref().and_then(|messages| {
        let mut matching = dispatch.focused.clone();
        let id = matching
            .read_with_id(messages)
            .find_map(|(input, id)| (input == &event.input).then_some(id))?;
        dispatch.focused = matching;
        Some(id)
    });
    if event.input.key_code != KeyCode::Escape || event.input.state != ButtonState::Pressed {
        return;
    }
    let state = views.get(view);
    if state.is_ok_and(|state| matches!(state.mode, crate::CanvasMode::Pen(_))) {
        event.propagate(false);
    }
    if !event.input.repeat && state.is_ok() && id.is_none_or(|id| dispatch.handled.insert(id)) {
        let window = event.input.window;
        commands.queue(move |world: &mut World| apply_escape(world, view, window));
    }
}

fn undispatched_escapes(
    world: &mut World,
    mut cursor: Local<MessageCursor<KeyboardInput>>,
    mut primary: Local<QueryState<Entity, With<PrimaryWindow>>>,
) -> bevy::prelude::Result {
    let Some(messages) = world.get_resource::<Messages<KeyboardInput>>() else {
        return Ok(());
    };
    let oldest = messages.oldest_message_count();
    let presses = cursor
        .read_with_id(messages)
        .filter(|(event, _)| is_escape_press(event))
        .map(|(event, id)| (event.window, id))
        .collect::<Vec<_>>();

    let can_dispatch = primary.single(world).is_ok();
    {
        let mut dispatch = world.resource_mut::<EscapeDispatch>();
        dispatch.handled.retain(|id| id.id >= oldest);
        if can_dispatch {
            // Bevy drains its reader with a unique primary window. Widgetry's window dispatcher
            // can also drain keys without emitting a FocusedInput, for example behind a modal.
            dispatch.focused = cursor.clone();
        }
    }
    for (window, id) in presses {
        if world.resource_mut::<EscapeDispatch>().handled.insert(id)
            && let Some(view) = world.resource::<InputFocus>().get()
        {
            apply_escape(world, view, window)?;
        }
    }
    Ok(())
}

fn apply_escape(world: &mut World, view: Entity, window: Entity) -> bevy::prelude::Result {
    (|| -> Result {
        if !keyboard_available(world, view, window) {
            return Ok(());
        }
        match CanvasView::_mode(world, view)? {
            crate::CanvasMode::Rectangle(_)
            | crate::CanvasMode::Ellipse(_)
            | crate::CanvasMode::Flowchart(_, _)
            | crate::CanvasMode::Parametric(_, _)
            | crate::CanvasMode::Line(_)
            | crate::CanvasMode::Polyline(_)
            | crate::CanvasMode::Connector(_)
            | crate::CanvasMode::Text
            | crate::CanvasMode::Polygon(_) => {
                CanvasView::_set_mode_in_world(world, view, crate::CanvasMode::Select)?;
            }
            crate::CanvasMode::Select | crate::CanvasMode::PathNodeEdit => {
                CanvasView::_leave_group(world, view)?;
            }
            crate::CanvasMode::Pen(_) => {
                let placing_point = pen_point(world, view).is_some();
                if !CanvasView::_cancel_pen_in_world(world, view)? && !placing_point {
                    CanvasView::_leave_group(world, view)?;
                }
            }
        }
        Ok(())
    })()
    .map_err(|error| {
        error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
            "canvas.input.escape",
        ))
    })
}

fn refresh_rotation(world: &mut World) -> bevy::prelude::Result {
    (|| -> Result {
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
    })()
    .map_err(|error| {
        error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
            "canvas.input.refresh_rotation",
        ))
    })
}

fn terminal(
    commands: &mut Commands,
    entity: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    completed: Option<Vec2>,
) {
    commands.queue(move |world: &mut World| -> bevy::prelude::Result {
        (|| -> Result {
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
        })()
        .map_err(|error| {
            error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new("canvas.command"))
        })
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
    mut commands: Commands,
) {
    if let Ok(state) = states.get(event.entity)
        && let Some(drag) = &state.0
    {
        release_capture(&mut capture, event.entity, drag);
        if let Some(edit) = drag.edit {
            commands.queue(move |world: &mut World| crate::history::finish_gesture(world, edit));
        }
    }
}

fn cancel_unavailable(world: &mut World) -> bevy::prelude::Result {
    (|| -> Result {
        crate::basic_path::cancel_unavailable(world);
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
            // Distinguish it from Cancel so clicks complete one-shot drawing and multi-selection semantics.
            if pointer == Some((false, true))
                && matches!(
                    drag.action,
                    DragAction::Draw { .. }
                        | DragAction::Pen { .. }
                        | DragAction::Contour
                        | DragAction::MoveSelection(_)
                )
                && continuing(world, entity, &drag)
            {
                let terminal = world
                    .resource::<FrameInputs>()
                    .events
                    .iter()
                    .rev()
                    .map(|(_, input)| input)
                    .find(|input| {
                        input.pointer_id == drag.pointer
                            && input.location.target == drag.target
                            && matches!(
                                input.action,
                                PointerAction::Release(PointerButton::Primary)
                                    | PointerAction::Cancel
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
    })()
    .map_err(|error| {
        error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
            "canvas.input.cancel_unavailable",
        ))
    })
}
