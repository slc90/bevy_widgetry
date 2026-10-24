use crate::CanvasDocument;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(SceneComponent, FromTemplate)]
#[scene(CanvasViewProps)]
#[require(crate::renderer::RenderFailure)]
pub struct CanvasView;

#[derive(Default)]
pub struct CanvasViewProps {
    pub document: Option<Entity>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ViewState {
    pub(crate) document: Option<Entity>,
}

#[derive(EntityEvent)]
pub struct CanvasDocumentChanged {
    pub entity: Entity,
    pub document: Option<Entity>,
}

impl CanvasView {
    fn scene(props: CanvasViewProps) -> impl Scene {
        bsn! {
            CanvasView
            template(move |context| {
                if props.document.is_some_and(|document| context.entity.world().get::<CanvasDocument>(document).is_none()) {
                    widgetry_error!(document = ?props.document, "Canvas View 初始化 Document 无效");
                    return Err(bevy_widgetry_core::scene::logged_error("CanvasDocument missing"));
                }
                Ok(ViewState { document: props.document })
            })
            Node { min_width: px(0), min_height: px(0), overflow: Overflow::clip() }
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
        state.document = document;
        world.trigger(CanvasDocumentChanged {
            entity: view,
            document,
        });
        Ok(true)
    }

    pub fn set_document(commands: &mut Commands, view: Entity, document: Option<Entity>) {
        commands.queue(move |world: &mut World| {
            Self::set_document_in_world(world, view, document).map(|_| ())
        });
    }
}
