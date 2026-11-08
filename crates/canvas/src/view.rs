use crate::CanvasDocument;
use crate::error::CanvasResult as Result;
use bevy::prelude::*;

#[derive(SceneComponent, FromTemplate)]
#[scene(CanvasViewProps)]
#[require(
    crate::renderer::RenderFailure,
    crate::input::DragState,
    crate::pen::State
)]
pub struct CanvasView;

pub struct CanvasViewProps {
    pub document: Option<Entity>,
    pub center: Vec2,
    pub zoom: f32,
}

impl Default for CanvasViewProps {
    fn default() -> Self {
        Self {
            document: None,
            center: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ViewState {
    pub(crate) document: Option<Entity>,
    pub(crate) center: Vec2,
    pub(crate) zoom: f32,
    pub(crate) selection: Vec<Entity>,
    pub(crate) mode: CanvasMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum CanvasMode {
    #[default]
    Select,
    PathNodeEdit,
    Rectangle(crate::CanvasPaint),
    Ellipse(crate::CanvasPaint),
    Pen(crate::CanvasPaint),
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
                Ok(ViewState { document: props.document, center: props.center, zoom: props.zoom, selection: Vec::new(), mode: CanvasMode::Select })
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
            if world.get::<ViewState>(view).is_none()
                || document.is_some_and(|entity| world.get::<CanvasDocument>(entity).is_none())
            {
                return Err(crate::error::invalid(crate::CanvasIssue::DocumentTarget));
            }
            let mut state = world
                .get_mut::<ViewState>(view)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::View))?;
            if state.document == document {
                return Ok(false);
            }
            let cleared = !state.selection.is_empty();
            state.selection.clear();
            state.document = document;
            crate::pen::cancel(world, view);
            world.trigger(CanvasDocumentChanged {
                entity: view,
                document,
            });
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
                state
                    .selection
                    .iter()
                    .copied()
                    .filter(|shape| crate::selection::selectable(world, state.document, *shape))
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
                selection
                    .iter()
                    .all(|shape| crate::selection::selectable(world, state.document, *shape))
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
                    CanvasMode::Select | CanvasMode::PathNodeEdit => {}
                    CanvasMode::Rectangle(paint) => crate::CanvasRectangle {
                        corner_radius: 0.0,
                        size: Vec2::ONE,
                    }
                    ._validate(&Transform::default(), &paint)?,
                    CanvasMode::Ellipse(paint) => crate::CanvasEllipse { radii: Vec2::ONE }
                        ._validate(&Transform::default(), &paint)?,
                    CanvasMode::Pen(paint) => {
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
