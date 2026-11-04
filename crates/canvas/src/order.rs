use crate::CanvasDocument;
use crate::error::CanvasResult as Result;
use bevy::prelude::*;

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
    pub fn shape_order(
        world: &mut World,
        document: Entity,
    ) -> crate::error::PublicResult<Vec<Entity>> {
        Self::_shape_order(world, document).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.shape_order",
            ))
        })
    }

    pub(crate) fn _shape_order(world: &mut World, document: Entity) -> Result<Vec<Entity>> {
        (|| -> Result<Vec<Entity>> {
            crate::model::validate_document(world, document)?;
            let order = world
                .get::<ShapeOrder>(document)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?;
            let shapes = order
                .0
                .iter()
                .copied()
                .filter(|shape| crate::selection::belongs_to(world, Some(document), *shape))
                .collect::<Vec<_>>();
            if shapes != order.0 {
                world
                    .get_mut::<ShapeOrder>(document)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?
                    .0 = shapes.clone();
            }
            Ok(shapes)
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "document",
                value: document,
            })
        })
    }

    pub fn reorder_shape(
        world: &mut World,
        document: Entity,
        shape: Entity,
        operation: CanvasOrder,
    ) -> crate::error::PublicResult<bool> {
        Self::_reorder_shape(world, document, shape, operation).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.reorder_shape",
            ))
        })
    }

    pub(crate) fn _reorder_shape(
        world: &mut World,
        document: Entity,
        shape: Entity,
        operation: CanvasOrder,
    ) -> Result<bool> {
        (|| -> Result<bool> {
            crate::model::validate_shape(world, document, shape)?;
            let mut shapes = Self::_shape_order(world, document)?;
            let Some(index) = shapes.iter().position(|entity| *entity == shape) else {
                return Err(crate::error::invalid(crate::CanvasIssue::OrderMembership));
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
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?
                .0 = shapes;
            Ok(true)
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
        })
    }
}
