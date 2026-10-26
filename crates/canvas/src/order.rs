use crate::CanvasDocument;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Component, Default)]
pub(crate) struct ShapeOrder(pub(crate) Vec<Entity>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasOrder {
    Raise,
    Lower,
    Front,
    Back,
}

impl CanvasDocument {
    pub fn shape_order(world: &mut World, document: Entity) -> Result<Vec<Entity>> {
        let result = (|| {
            crate::model::validate_document(world, document)?;
            let order = world
                .get::<ShapeOrder>(document)
                .ok_or_else(|| BevyError::error("Canvas document order missing"))?;
            let shapes = order
                .0
                .iter()
                .copied()
                .filter(|shape| crate::selection::belongs_to(world, Some(document), *shape))
                .collect::<Vec<_>>();
            if shapes != order.0 {
                world
                    .get_mut::<ShapeOrder>(document)
                    .ok_or_else(|| BevyError::error("Canvas document order missing"))?
                    .0 = shapes.clone();
            }
            Ok(shapes)
        })();
        result.inspect_err(|error| widgetry_error!(?document, %error, "Canvas 图形顺序查询被拒绝"))
    }

    pub fn reorder_shape(
        world: &mut World,
        document: Entity,
        shape: Entity,
        operation: CanvasOrder,
    ) -> Result<bool> {
        crate::model::validate_shape(world, document, shape).inspect_err(
            |error| widgetry_error!(?document, ?shape, %error, "Canvas 图形重排被拒绝"),
        )?;
        let mut shapes = Self::shape_order(world, document)?;
        let Some(index) = shapes.iter().position(|entity| *entity == shape) else {
            widgetry_error!(?document, ?shape, "Canvas 重排图形不在 Document 顺序中");
            return Err(BevyError::error("Canvas shape is not in document order"));
        };
        let target = match operation {
            CanvasOrder::Raise => (index + 1).min(shapes.len() - 1),
            CanvasOrder::Lower => index.saturating_sub(1),
            CanvasOrder::Front => shapes.len() - 1,
            CanvasOrder::Back => 0,
        };
        if index == target {
            return Ok(false);
        }
        shapes.remove(index);
        shapes.insert(target, shape);
        world
            .get_mut::<ShapeOrder>(document)
            .ok_or_else(|| BevyError::error("Canvas document order missing"))?
            .0 = shapes;
        Ok(true)
    }
}
