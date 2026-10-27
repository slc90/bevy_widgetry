use crate::CanvasDocument;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

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
    pub(crate) selection: Option<Entity>,
    pub(crate) mode: CanvasMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum CanvasMode {
    #[default]
    Select,
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
    pub selection: Option<Entity>,
}

pub(crate) const MIN_ZOOM: f32 = 0.1;
pub(crate) const MAX_ZOOM: f32 = 10.0;

impl CanvasView {
    fn scene(props: CanvasViewProps) -> impl Scene {
        bsn! {
            CanvasView
            template(move |context| {
                if props.document.is_some_and(|document| context.entity.world().get::<CanvasDocument>(document).is_none()) {
                    widgetry_error!(document = ?props.document, "Canvas View 初始化 Document 无效");
                    return Err(bevy_widgetry_core::scene::logged_error("CanvasDocument missing"));
                }
                if !props.center.is_finite() {
                    widgetry_error!(center = ?props.center, "Canvas View 初始观察中心无效");
                    return Err(bevy_widgetry_core::scene::logged_error("Canvas center must be finite"));
                }
                if !(MIN_ZOOM..=MAX_ZOOM).contains(&props.zoom) {
                    widgetry_error!(zoom = props.zoom, "Canvas View 初始倍率无效");
                    return Err(bevy_widgetry_core::scene::logged_error("Canvas zoom must be in [0.1, 10]"));
                }
                Ok(ViewState { document: props.document, center: props.center, zoom: props.zoom, selection: None, mode: CanvasMode::Select })
            })
            Node { min_width: px(0), min_height: px(0), overflow: Overflow::clip() }
            bevy::input_focus::tab_navigation::TabIndex(0)
            Visibility::default()
        }
    }

    pub fn document(world: &World, view: Entity) -> Result<Option<Entity>> {
        world
            .get::<ViewState>(view)
            .map(|state| state.document)
            .ok_or_else(|| {
                widgetry_error!(?view, "Canvas View 查询目标无效");
                BevyError::error("CanvasView missing")
            })
    }

    pub fn set_document_in_world(
        world: &mut World,
        view: Entity,
        document: Option<Entity>,
    ) -> Result<bool> {
        if world.get::<ViewState>(view).is_none()
            || document.is_some_and(|entity| world.get::<CanvasDocument>(entity).is_none())
        {
            widgetry_error!(?view, ?document, "Canvas View Document 更新被拒绝");
            return Err(BevyError::error("CanvasView or CanvasDocument missing"));
        }
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| BevyError::error("CanvasView missing"))?;
        if state.document == document {
            return Ok(false);
        }
        let cleared = state.selection.take().is_some();
        state.document = document;
        crate::pen::cancel(world, view);
        world.trigger(CanvasDocumentChanged {
            entity: view,
            document,
        });
        if cleared
            && world
                .get::<ViewState>(view)
                .is_some_and(|state| state.selection.is_none())
        {
            world.trigger(CanvasSelectionChanged {
                entity: view,
                selection: None,
            });
        }
        Ok(true)
    }

    pub fn set_document(commands: &mut Commands, view: Entity, document: Option<Entity>) {
        commands.queue(move |world: &mut World| {
            Self::set_document_in_world(world, view, document).map(|_| ())
        });
    }

    pub fn center(world: &World, view: Entity) -> Result<Vec2> {
        world
            .get::<ViewState>(view)
            .map(|state| state.center)
            .ok_or_else(|| {
                widgetry_error!(?view, "Canvas View 查询目标无效");
                BevyError::error("CanvasView missing")
            })
    }

    pub fn set_center_in_world(world: &mut World, view: Entity, center: Vec2) -> Result<bool> {
        if !center.is_finite() || world.get::<ViewState>(view).is_none() {
            widgetry_error!(?view, ?center, "Canvas View 观察中心更新被拒绝");
            return Err(BevyError::error("CanvasView missing or center not finite"));
        }
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| BevyError::error("CanvasView missing"))?;
        if state.center == center {
            return Ok(false);
        }
        state.center = center;
        world.trigger(CanvasCenterChanged {
            entity: view,
            center,
        });
        Ok(true)
    }

    pub fn set_center(commands: &mut Commands, view: Entity, center: Vec2) {
        commands.queue(move |world: &mut World| {
            Self::set_center_in_world(world, view, center).map(|_| ())
        });
    }

    pub fn zoom(world: &World, view: Entity) -> Result<f32> {
        world
            .get::<ViewState>(view)
            .map(|state| state.zoom)
            .ok_or_else(|| {
                widgetry_error!(?view, "Canvas View 查询目标无效");
                BevyError::error("CanvasView missing")
            })
    }

    pub fn set_zoom_in_world(world: &mut World, view: Entity, zoom: f32) -> Result<bool> {
        let center = Self::center(world, view)?;
        set_zoom_and_center(world, view, zoom, center)
    }

    pub fn set_zoom(commands: &mut Commands, view: Entity, zoom: f32) {
        commands
            .queue(move |world: &mut World| Self::set_zoom_in_world(world, view, zoom).map(|_| ()));
    }

    pub fn selection(world: &World, view: Entity) -> Result<Option<Entity>> {
        world
            .get::<ViewState>(view)
            .map(|state| {
                state
                    .selection
                    .filter(|shape| crate::selection::selectable(world, state.document, *shape))
            })
            .ok_or_else(|| {
                widgetry_error!(?view, "Canvas View 查询目标无效");
                BevyError::error("CanvasView missing")
            })
    }

    pub fn set_selection_in_world(
        world: &mut World,
        view: Entity,
        selection: Option<Entity>,
    ) -> Result<bool> {
        let valid = world.get::<ViewState>(view).is_some_and(|state| {
            selection.is_none_or(|shape| crate::selection::selectable(world, state.document, shape))
        });
        if !valid {
            widgetry_error!(?view, ?selection, "Canvas View 选择更新被拒绝");
            return Err(BevyError::error(
                "CanvasView missing or selection does not belong to its document",
            ));
        }
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| BevyError::error("CanvasView missing"))?;
        if state.selection == selection {
            return Ok(false);
        }
        state.selection = selection;
        world.trigger(CanvasSelectionChanged {
            entity: view,
            selection,
        });
        Ok(true)
    }

    pub fn set_selection(commands: &mut Commands, view: Entity, selection: Option<Entity>) {
        commands.queue(move |world: &mut World| {
            Self::set_selection_in_world(world, view, selection).map(|_| ())
        });
    }

    pub fn mode(world: &World, view: Entity) -> Result<CanvasMode> {
        world
            .get::<ViewState>(view)
            .map(|state| state.mode)
            .ok_or_else(|| {
                widgetry_error!(?view, "Canvas View 模式查询目标无效");
                BevyError::error("CanvasView missing")
            })
    }

    pub fn set_mode_in_world(world: &mut World, view: Entity, mode: CanvasMode) -> Result<bool> {
        let validation = (|| {
            if world.get::<ViewState>(view).is_none() {
                return Err(BevyError::error("CanvasView missing"));
            }
            match mode {
                CanvasMode::Select => {}
                CanvasMode::Rectangle(paint) => crate::CanvasRectangle {
                    corner_radius: 0.0,
                    size: Vec2::ONE,
                }
                .validate(&Transform::default(), &paint)?,
                CanvasMode::Ellipse(paint) => crate::CanvasEllipse { radii: Vec2::ONE }
                    .validate(&Transform::default(), &paint)?,
                CanvasMode::Pen(paint) => {
                    crate::model::validate_transform_and_paint(&Transform::default(), &paint)?
                }
            }
            Ok(())
        })();
        validation.inspect_err(
            |error| widgetry_error!(?view, ?mode, %error, "Canvas View 模式更新被拒绝"),
        )?;
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| BevyError::error("CanvasView missing"))?;
        if state.mode == mode {
            return Ok(false);
        }
        state.mode = mode;
        crate::input::end(world, view);
        world.trigger(CanvasModeChanged { entity: view, mode });
        Ok(true)
    }

    pub fn set_mode(commands: &mut Commands, view: Entity, mode: CanvasMode) {
        commands
            .queue(move |world: &mut World| Self::set_mode_in_world(world, view, mode).map(|_| ()));
    }
}

pub(crate) fn set_zoom_and_center(
    world: &mut World,
    view: Entity,
    zoom: f32,
    center: Vec2,
) -> Result<bool> {
    if !(MIN_ZOOM..=MAX_ZOOM).contains(&zoom)
        || !center.is_finite()
        || world.get::<ViewState>(view).is_none()
    {
        widgetry_error!(?view, ?zoom, ?center, "Canvas View 倍率更新被拒绝");
        return Err(BevyError::error(
            "CanvasView missing, zoom outside [0.1, 10], or center not finite",
        ));
    }
    let mut state = world
        .get_mut::<ViewState>(view)
        .ok_or_else(|| BevyError::error("CanvasView missing"))?;
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
}
