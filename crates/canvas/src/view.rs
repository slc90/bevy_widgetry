use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasLayerId};
use bevy::prelude::*;

#[derive(SceneComponent, FromTemplate)]
#[scene(CanvasViewProps)]
#[require(
    crate::renderer::RenderFailure,
    crate::input::DragState,
    crate::input::GroupClick,
    crate::pen::State,
    crate::basic_path::State
)]
pub struct CanvasView;

pub struct CanvasViewProps {
    pub document: Option<Entity>,
    pub center: Vec2,
    pub zoom: f32,
    pub grid: crate::CanvasGrid,
}

impl Default for CanvasViewProps {
    fn default() -> Self {
        Self {
            document: None,
            center: Vec2::ZERO,
            zoom: 1.0,
            grid: default(),
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ViewState {
    pub(crate) document: Option<Entity>,
    pub(crate) active_layer: Option<CanvasLayerId>,
    group_path: Vec<Entity>,
    pub(crate) center: Vec2,
    pub(crate) zoom: f32,
    pub(crate) grid: crate::CanvasGrid,
    pub(crate) selection: Vec<Entity>,
    pub(crate) mode: CanvasMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum CanvasMode {
    #[default]
    Select,
    PathNodeEdit,
    Text,
    Connector(crate::CanvasConnectorStyle),
    Rectangle(crate::CanvasPaint),
    Ellipse(crate::CanvasPaint),
    Pen(crate::CanvasPaint),
    Line(crate::CanvasPaint),
    Polyline(crate::CanvasPaint),
    Polygon(crate::CanvasPaint),
    Flowchart(crate::CanvasFlowchartPreset, crate::CanvasPaint),
    Parametric(crate::CanvasPathTool, crate::CanvasPaint),
}

#[derive(EntityEvent)]
pub struct CanvasModeChanged {
    pub entity: Entity,
    pub mode: CanvasMode,
}

#[derive(EntityEvent)]
pub struct CanvasDocumentChanged {
    pub entity: Entity,
    pub document: Option<Entity>,
}

#[derive(EntityEvent)]
pub struct CanvasActiveLayerChanged {
    pub entity: Entity,
    pub layer: Option<CanvasLayerId>,
}

#[derive(EntityEvent)]
pub struct CanvasGroupPathChanged {
    pub entity: Entity,
    pub path: Vec<Entity>,
}

#[derive(EntityEvent)]
pub struct CanvasCenterChanged {
    pub entity: Entity,
    pub center: Vec2,
}

#[derive(EntityEvent)]
pub struct CanvasZoomChanged {
    pub entity: Entity,
    pub zoom: f32,
}

#[derive(EntityEvent)]
pub struct CanvasSelectionChanged {
    pub entity: Entity,
    pub selection: Vec<Entity>,
}

pub(crate) const MIN_ZOOM: f32 = 0.1;
pub(crate) const MAX_ZOOM: f32 = 10.0;

fn valid_group_path<'a>(world: &World, state: &'a ViewState) -> &'a [Entity] {
    let Some(document) = state.document else {
        return &[];
    };
    let mut parent = None;
    for (index, &group) in state.group_path.iter().enumerate() {
        if world.get::<crate::CanvasGroup>(group).is_none()
            || !crate::selection::selectable(world, Some(document), parent, group)
        {
            return &state.group_path[..index];
        }
        parent = Some(group);
    }
    &state.group_path
}

pub(crate) fn edited_group(world: &World, state: &ViewState) -> Option<Entity> {
    valid_group_path(world, state).last().copied()
}

fn group_layer(world: &World, document: Option<Entity>, group: Entity) -> Option<CanvasLayerId> {
    crate::order::item_placement(world, document?, group).map(|(_, layer)| layer.id())
}

fn current_layer(world: &World, state: &ViewState) -> Option<CanvasLayerId> {
    edited_group(world, state)
        .and_then(|group| group_layer(world, state.document, group))
        .or_else(|| preferred_layer(world, state.document, state.active_layer))
}

pub(crate) fn repair_group_path(world: &mut World, view: Entity) -> Result<bool> {
    let Some(state) = world.get::<ViewState>(view) else {
        return Ok(false);
    };
    let current = valid_group_path(world, state);
    if current.len() == state.group_path.len() {
        return Ok(false);
    }
    let path = current.to_vec();
    let selection = CanvasView::_selection(world, view)?;
    set_group_path(world, view, path, selection)?;
    Ok(true)
}

fn set_group_path(
    world: &mut World,
    view: Entity,
    path: Vec<Entity>,
    selection: Vec<Entity>,
) -> Result {
    let state = world
        .get::<ViewState>(view)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
    let layer = path
        .last()
        .and_then(|&group| group_layer(world, state.document, group))
        .or_else(|| preferred_layer(world, state.document, state.active_layer));
    commit_group_path(world, view, path, layer, selection)
}

fn commit_group_path(
    world: &mut World,
    view: Entity,
    path: Vec<Entity>,
    layer: Option<CanvasLayerId>,
    selection: Vec<Entity>,
) -> Result {
    let mut state = world
        .get_mut::<ViewState>(view)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
    let document = state.document;
    let selection_changed = state.selection != selection;
    let layer_changed = state.active_layer != layer;
    state.selection.clone_from(&selection);
    state.group_path.clone_from(&path);
    state.active_layer = layer;
    crate::input::end(world, view);
    if world
        .get::<ViewState>(view)
        .is_some_and(|state| state.document == document && state.group_path == path)
    {
        world.trigger(CanvasGroupPathChanged { entity: view, path });
    }
    if selection_changed
        && world
            .get::<ViewState>(view)
            .is_some_and(|state| state.document == document && state.selection == selection)
    {
        world.trigger(CanvasSelectionChanged {
            entity: view,
            selection,
        });
    }
    if layer_changed {
        notify_active_layer(world, view, document, layer);
    }
    Ok(())
}

fn preferred_layer(
    world: &World,
    document: Option<Entity>,
    preferred: Option<CanvasLayerId>,
) -> Option<CanvasLayerId> {
    let document = document?;
    world.get::<CanvasDocument>(document)?;
    world
        .get::<crate::CanvasLayerStack>(document)?
        .editable_layer(preferred)
}

pub(crate) fn repair_layers(world: &mut World, document: Entity) -> Result {
    let views = world
        .query::<(Entity, &ViewState)>()
        .iter(world)
        .filter_map(|(view, state)| (state.document == Some(document)).then_some(view))
        .collect::<Vec<_>>();
    for view in views {
        if let Some(state) = world
            .get::<ViewState>(view)
            .filter(|state| state.document == Some(document))
        {
            let layer = current_layer(world, state);
            apply_active_layer(world, view, layer)?;
        }
        if world
            .get::<ViewState>(view)
            .is_some_and(|state| state.document == Some(document))
        {
            crate::selection::repair(world, view)?;
        }
    }
    Ok(())
}

fn apply_active_layer(
    world: &mut World,
    view: Entity,
    layer: Option<CanvasLayerId>,
) -> Result<bool> {
    let state = world
        .get::<ViewState>(view)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
    if let Some(owner) =
        edited_group(world, state).and_then(|group| group_layer(world, state.document, group))
        && Some(owner) != layer
    {
        commit_group_path(world, view, Vec::new(), layer, Vec::new())?;
        return Ok(true);
    }
    let mut state = world
        .get_mut::<ViewState>(view)
        .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
    if state.active_layer == layer {
        return Ok(false);
    }
    let document = state.document;
    state.active_layer = layer;
    crate::input::cancel_drawing_tools(world, view);
    notify_active_layer(world, view, document, layer);
    Ok(true)
}

fn notify_active_layer(
    world: &mut World,
    view: Entity,
    document: Option<Entity>,
    layer: Option<CanvasLayerId>,
) {
    if world
        .get::<ViewState>(view)
        .is_some_and(|state| state.document == document && state.active_layer == layer)
    {
        world.trigger(CanvasActiveLayerChanged {
            entity: view,
            layer,
        });
    }
}

impl CanvasView {
    fn scene(props: CanvasViewProps) -> impl Scene {
        bsn! {
            CanvasView
            template(move |context| {
                let root = context.entity.id();
                let frame = || bevy_widgetry_diagnostics::ErrorFrame::new("canvas.scene").field("view", root);
                if let Some(document) = props.document
                    && context.entity.world().get::<CanvasDocument>(document).is_none()
                {
                    return Err(crate::error::missing(crate::CanvasPart::Document).at("document", document).at("view", root).into_bevy_error(frame()));
                }
                if !props.center.is_finite() {
                    return Err(crate::error::invalid(crate::CanvasIssue::CenterTarget).input(crate::CanvasInput::Point { field: "center", value: props.center }).at("view", root).into_bevy_error(frame()));
                }
                if !(MIN_ZOOM..=MAX_ZOOM).contains(&props.zoom) {
                    return Err(crate::error::invalid(crate::CanvasIssue::ZoomTarget).input(crate::CanvasInput::Float { field: "zoom", value: props.zoom }).at("view", root).into_bevy_error(frame()));
                }
                props.grid.validate().map_err(|error| error.at("view", root).into_bevy_error(frame()))?;
                let active_layer = preferred_layer(context.entity.world(), props.document, None);
                Ok(ViewState { document: props.document, active_layer, group_path: Vec::new(), center: props.center, zoom: props.zoom, grid: props.grid, selection: Vec::new(), mode: CanvasMode::Select })
            })
            Node { min_width: px(0), min_height: px(0), overflow: Overflow::clip() }
            bevy::input_focus::tab_navigation::TabIndex(-1)
            Visibility::default()
        }
    }

    pub fn document(world: &World, view: Entity) -> crate::error::PublicResult<Option<Entity>> {
        Self::_document(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.document",
            ))
        })
    }

    pub(crate) fn _document(world: &World, view: Entity) -> Result<Option<Entity>> {
        world
            .get::<ViewState>(view)
            .map(|state| state.document)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
            .map_err(|error| {
                error.input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
            })
    }

    pub fn set_document_in_world(
        world: &mut World,
        view: Entity,
        document: Option<Entity>,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_document_in_world(world, view, document).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_document_in_world",
            ))
        })
    }

    pub(crate) fn _set_document_in_world(
        world: &mut World,
        view: Entity,
        document: Option<Entity>,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            let Some(previous) = world.get::<ViewState>(view) else {
                return Err(crate::error::invalid(crate::CanvasIssue::DocumentTarget));
            };
            if document.is_some_and(|entity| world.get::<CanvasDocument>(entity).is_none()) {
                return Err(crate::error::invalid(crate::CanvasIssue::DocumentTarget));
            }
            let layer = preferred_layer(world, document, previous.active_layer);
            let layer_changed = previous.active_layer != layer;
            let mut state = world
                .get_mut::<ViewState>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
            if state.document == document {
                return Ok(false);
            }
            let cleared = !state.selection.is_empty();
            let left_group = !state.group_path.is_empty();
            state.selection.clear();
            state.group_path.clear();
            state.document = document;
            state.active_layer = layer;
            crate::pen::cancel(world, view);
            world.trigger(CanvasDocumentChanged {
                entity: view,
                document,
            });
            if left_group
                && world
                    .get::<ViewState>(view)
                    .is_some_and(|state| state.document == document && state.group_path.is_empty())
            {
                world.trigger(CanvasGroupPathChanged {
                    entity: view,
                    path: Vec::new(),
                });
            }
            if cleared
                && world
                    .get::<ViewState>(view)
                    .is_some_and(|state| state.selection.is_empty())
            {
                world.trigger(CanvasSelectionChanged {
                    entity: view,
                    selection: Vec::new(),
                });
            }
            if layer_changed {
                notify_active_layer(world, view, document, layer);
            }
            Ok(true)
        })()
        .map_err(|error| {
            let error = error.at("view", view).input(crate::CanvasInput::Flag {
                field: "has_document",
                value: document.is_some(),
            });
            match document {
                Some(document) => error.at("document", document),
                None => error,
            }
        })
    }

    pub fn set_document(commands: &mut Commands, view: Entity, document: Option<Entity>) {
        commands.queue(move |world: &mut World| {
            Self::_set_document_in_world(world, view, document)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }

    pub fn active_layer(
        world: &World,
        view: Entity,
    ) -> crate::error::PublicResult<Option<CanvasLayerId>> {
        Self::_active_layer(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.active_layer",
            ))
        })
    }

    pub(crate) fn _active_layer(world: &World, view: Entity) -> Result<Option<CanvasLayerId>> {
        let state = world
            .get::<ViewState>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
        Ok(current_layer(world, state))
    }

    pub(crate) fn drawing_target(
        world: &World,
        view: Entity,
    ) -> Result<Option<crate::CanvasContainer>> {
        let state = world
            .get::<ViewState>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
        if !state.group_path.is_empty() {
            // A pending scope repair must not redirect an in-flight drawing into an ancestor.
            return Ok(valid_group_path(world, state)
                .last()
                .copied()
                .filter(|group| state.group_path.last() == Some(group))
                .map(crate::CanvasContainer::Group));
        }
        Ok(preferred_layer(world, state.document, state.active_layer)
            .map(crate::CanvasContainer::Layer))
    }

    pub fn set_active_layer_in_world(
        world: &mut World,
        view: Entity,
        layer: CanvasLayerId,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_active_layer_in_world(world, view, layer).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_active_layer_in_world",
            ))
        })
    }

    fn _set_active_layer_in_world(
        world: &mut World,
        view: Entity,
        layer: CanvasLayerId,
    ) -> Result<bool> {
        let document = Self::_document(world, view)?;
        if preferred_layer(world, document, Some(layer)) != Some(layer) {
            return Err(crate::error::invalid(crate::CanvasIssue::LayerUnavailable)
                .input(crate::CanvasInput::Layer(layer))
                .at("view", view));
        }
        apply_active_layer(world, view, Some(layer))
    }

    pub fn set_active_layer(commands: &mut Commands, view: Entity, layer: CanvasLayerId) {
        commands.queue(move |world: &mut World| {
            Self::_set_active_layer_in_world(world, view, layer)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.CanvasView.set_active_layer",
                    ))
                })
        });
    }

    pub fn group_path(world: &World, view: Entity) -> crate::error::PublicResult<Vec<Entity>> {
        world
            .get::<ViewState>(view)
            .map(|state| valid_group_path(world, state).to_vec())
            .ok_or_else(|| {
                crate::error::missing(crate::CanvasPart::View)
                    .at("view", view)
                    .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.CanvasView.group_path",
                    ))
            })
    }

    pub fn enter_group_in_world(
        world: &mut World,
        view: Entity,
        group: Entity,
    ) -> crate::error::PublicResult {
        Self::_enter_group(world, view, group).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.enter_group_in_world",
            ))
        })
    }

    pub fn enter_group(commands: &mut Commands, view: Entity, group: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_enter_group(world, view, group).map_err(|error| {
                error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                    "canvas.CanvasView.enter_group",
                ))
            })
        });
    }

    pub(crate) fn _enter_group(world: &mut World, view: Entity, group: Entity) -> Result {
        let state = world
            .get::<ViewState>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
        let mut path = valid_group_path(world, state).to_vec();
        if world.get::<crate::CanvasGroup>(group).is_none()
            || !crate::selection::selectable(world, state.document, path.last().copied(), group)
        {
            return Err(crate::error::invalid(crate::CanvasIssue::GroupTarget)
                .at("view", view)
                .at("group", group));
        }
        path.push(group);
        set_group_path(world, view, path, Vec::new())
    }

    pub fn leave_group_in_world(
        world: &mut World,
        view: Entity,
    ) -> crate::error::PublicResult<bool> {
        Self::_leave_group(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.leave_group_in_world",
            ))
        })
    }

    pub fn leave_group(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::_leave_group(world, view)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.CanvasView.leave_group",
                    ))
                })
        });
    }

    pub(crate) fn _leave_group(world: &mut World, view: Entity) -> Result<bool> {
        let state = world
            .get::<ViewState>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View).at("view", view))?;
        let mut path = valid_group_path(world, state).to_vec();
        if path.pop().is_none() {
            return repair_group_path(world, view);
        }
        set_group_path(world, view, path, Vec::new())?;
        Ok(true)
    }

    pub fn center(world: &World, view: Entity) -> crate::error::PublicResult<Vec2> {
        Self::_center(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.center",
            ))
        })
    }

    pub(crate) fn _center(world: &World, view: Entity) -> Result<Vec2> {
        world
            .get::<ViewState>(view)
            .map(|state| state.center)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
            .map_err(|error| {
                error.input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
            })
    }

    pub fn set_center_in_world(
        world: &mut World,
        view: Entity,
        center: Vec2,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_center_in_world(world, view, center).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_center_in_world",
            ))
        })
    }

    pub(crate) fn _set_center_in_world(
        world: &mut World,
        view: Entity,
        center: Vec2,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            if !center.is_finite() || world.get::<ViewState>(view).is_none() {
                return Err(crate::error::invalid(crate::CanvasIssue::CenterTarget));
            }
            let mut state = world
                .get_mut::<ViewState>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
            if state.center == center {
                return Ok(false);
            }
            state.center = center;
            world.trigger(CanvasCenterChanged {
                entity: view,
                center,
            });
            Ok(true)
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
                .input(crate::CanvasInput::Point {
                    field: "center",
                    value: center,
                })
        })
    }

    pub fn set_center(commands: &mut Commands, view: Entity, center: Vec2) {
        commands.queue(move |world: &mut World| {
            Self::_set_center_in_world(world, view, center)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }

    pub fn zoom(world: &World, view: Entity) -> crate::error::PublicResult<f32> {
        Self::_zoom(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.zoom",
            ))
        })
    }

    pub(crate) fn _zoom(world: &World, view: Entity) -> Result<f32> {
        world
            .get::<ViewState>(view)
            .map(|state| state.zoom)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
            .map_err(|error| {
                error.input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
            })
    }

    pub fn set_zoom_in_world(
        world: &mut World,
        view: Entity,
        zoom: f32,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_zoom_in_world(world, view, zoom).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_zoom_in_world",
            ))
        })
    }

    pub(crate) fn _set_zoom_in_world(world: &mut World, view: Entity, zoom: f32) -> Result<bool> {
        (|| -> Result<bool> {
            let center = Self::_center(world, view)?;
            set_zoom_and_center(world, view, zoom, center)
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
                .input(crate::CanvasInput::Float {
                    field: "zoom",
                    value: zoom,
                })
        })
    }

    pub fn set_zoom(commands: &mut Commands, view: Entity, zoom: f32) {
        commands.queue(move |world: &mut World| {
            Self::_set_zoom_in_world(world, view, zoom)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }

    pub fn selection(world: &World, view: Entity) -> crate::error::PublicResult<Vec<Entity>> {
        Self::_selection(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.selection",
            ))
        })
    }

    pub(crate) fn _selection(world: &World, view: Entity) -> Result<Vec<Entity>> {
        world
            .get::<ViewState>(view)
            .map(|state| {
                let scope = edited_group(world, state);
                state
                    .selection
                    .iter()
                    .copied()
                    .filter(|shape| {
                        crate::selection::selectable(world, state.document, scope, *shape)
                    })
                    .collect()
            })
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
            .map_err(|error| {
                error.input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
            })
    }

    pub fn single_selection(
        world: &World,
        view: Entity,
    ) -> crate::error::PublicResult<Option<Entity>> {
        Self::_single_selection(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.single_selection",
            ))
        })
    }

    pub(crate) fn _single_selection(world: &World, view: Entity) -> Result<Option<Entity>> {
        (|| -> Result<Option<Entity>> {
            let selection = Self::_selection(world, view)?;
            Ok(match selection.as_slice() {
                [shape] => Some(*shape),
                _ => None,
            })
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }

    pub fn set_selection_in_world(
        world: &mut World,
        view: Entity,
        selection: impl IntoIterator<Item = Entity>,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_selection_in_world(world, view, selection).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_selection_in_world",
            ))
        })
    }

    pub(crate) fn _set_selection_in_world(
        world: &mut World,
        view: Entity,
        selection: impl IntoIterator<Item = Entity>,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            let mut selection = selection.into_iter().collect::<Vec<_>>();
            let valid = world.get::<ViewState>(view).is_some_and(|state| {
                let scope = edited_group(world, state);
                selection
                    .iter()
                    .all(|shape| crate::selection::selectable(world, state.document, scope, *shape))
            });
            if !valid {
                let mut error = crate::error::invalid(crate::CanvasIssue::SelectionTarget).input(
                    crate::CanvasInput::Entities {
                        field: "selection",
                        values: selection,
                    },
                );
                if let Some(document) = world
                    .get::<ViewState>(view)
                    .and_then(|state| state.document)
                {
                    error = error.at("document", document);
                }
                return Err(error);
            }
            let mut state = world
                .get_mut::<ViewState>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
            selection.sort_unstable();
            selection.dedup();
            if state.selection == selection {
                return Ok(false);
            }
            state.selection = selection.clone();
            world.trigger(CanvasSelectionChanged {
                entity: view,
                selection,
            });
            Ok(true)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }

    pub fn set_selection(
        commands: &mut Commands,
        view: Entity,
        selection: impl IntoIterator<Item = Entity>,
    ) {
        let selection = selection.into_iter().collect::<Vec<_>>();
        commands.queue(move |world: &mut World| {
            Self::_set_selection_in_world(world, view, selection)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }

    pub fn mode(world: &World, view: Entity) -> crate::error::PublicResult<CanvasMode> {
        Self::_mode(world, view).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.mode",
            ))
        })
    }

    pub(crate) fn _mode(world: &World, view: Entity) -> Result<CanvasMode> {
        world
            .get::<ViewState>(view)
            .map(|state| state.mode)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))
            .map_err(|error| {
                error.input(crate::CanvasInput::Entity {
                    field: "view",
                    value: view,
                })
            })
    }

    pub fn set_mode_in_world(
        world: &mut World,
        view: Entity,
        mode: CanvasMode,
    ) -> crate::error::PublicResult<bool> {
        Self::_set_mode_in_world(world, view, mode).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasView.set_mode_in_world",
            ))
        })
    }

    pub(crate) fn _set_mode_in_world(
        world: &mut World,
        view: Entity,
        mode: CanvasMode,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            let validation = (|| {
                if world.get::<ViewState>(view).is_none() {
                    return Err(crate::error::missing(crate::CanvasPart::View));
                }
                match mode {
                    CanvasMode::Select | CanvasMode::Text | CanvasMode::PathNodeEdit => {}
                    CanvasMode::Connector(style) => {
                        let paint = style.paint();
                        crate::model::validate_transform_and_paint(&Transform::IDENTITY, &paint)
                            .map_err(|error| error.input(crate::CanvasInput::Paint(paint)))?;
                    }
                    CanvasMode::Rectangle(paint) => crate::CanvasRectangle {
                        corner_radius: 0.0,
                        size: Vec2::ONE,
                    }
                    ._validate(&Transform::default(), &paint)?,
                    CanvasMode::Ellipse(paint) => crate::CanvasEllipse { radii: Vec2::ONE }
                        ._validate(&Transform::default(), &paint)?,
                    CanvasMode::Parametric(tool, _) if !tool.valid() => {
                        return Err(crate::error::invalid(
                            crate::CanvasIssue::PathToolParameters,
                        ));
                    }
                    CanvasMode::Flowchart(_, paint)
                    | CanvasMode::Parametric(_, paint)
                    | CanvasMode::Pen(paint)
                    | CanvasMode::Line(paint)
                    | CanvasMode::Polyline(paint)
                    | CanvasMode::Polygon(paint) => {
                        crate::model::validate_transform_and_paint(&Transform::default(), &paint)
                            .map_err(|error| error.input(crate::CanvasInput::Paint(paint)))?
                    }
                }
                Ok(())
            })();
            validation?;
            let mut state = world
                .get_mut::<ViewState>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
            if state.mode == mode {
                return Ok(false);
            }
            state.mode = mode;
            crate::input::end(world, view);
            world.trigger(CanvasModeChanged { entity: view, mode });
            Ok(true)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "view",
                value: view,
            })
        })
    }

    pub fn set_mode(commands: &mut Commands, view: Entity, mode: CanvasMode) {
        commands.queue(move |world: &mut World| {
            Self::_set_mode_in_world(world, view, mode)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.command",
                    ))
                })
        });
    }
}

pub(crate) fn set_zoom_and_center(
    world: &mut World,
    view: Entity,
    zoom: f32,
    center: Vec2,
) -> Result<bool> {
    (|| -> Result<bool> {
        if !(MIN_ZOOM..=MAX_ZOOM).contains(&zoom)
            || !center.is_finite()
            || world.get::<ViewState>(view).is_none()
        {
            return Err(crate::error::invalid(crate::CanvasIssue::ZoomTarget));
        }
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
        let zoom_changed = state.zoom != zoom;
        let center_changed = state.center != center;
        if !zoom_changed && !center_changed {
            return Ok(false);
        }
        state.zoom = zoom;
        state.center = center;
        if zoom_changed {
            world.trigger(CanvasZoomChanged { entity: view, zoom });
        }
        // A Zoom observer may change the center again or remove the View before its paired center notice.
        if center_changed
            && world
                .get::<ViewState>(view)
                .is_some_and(|state| state.center == center)
        {
            world.trigger(CanvasCenterChanged {
                entity: view,
                center,
            });
        }
        Ok(true)
    })()
    .map_err(|error| {
        error
            .at("view", view)
            .input(crate::CanvasInput::Float {
                field: "zoom",
                value: zoom,
            })
            .input(crate::CanvasInput::Point {
                field: "center",
                value: center,
            })
    })
}
