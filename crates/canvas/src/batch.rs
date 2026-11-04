use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasGeometry, CanvasPaint};
use bevy::prelude::*;

use std::collections::HashSet;

impl CanvasDocument {
    pub fn update_transforms(
        world: &mut World,
        document: Entity,
        updates: &[(Entity, Transform)],
    ) -> crate::error::PublicResult {
        Self::_update_transforms(world, document, updates).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.update_transforms",
            ))
        })
    }

    pub(crate) fn _update_transforms(
        world: &mut World,
        document: Entity,
        updates: &[(Entity, Transform)],
    ) -> Result {
        (|| -> Result {
            let mut failed_shape = None;
            let result: Result = (|| {
                crate::model::validate_document(world, document)?;
                let mut seen = HashSet::new();
                for (shape, transform) in updates {
                    failed_shape = Some(*shape);
                    if !seen.insert(*shape) {
                        return Err(crate::error::invalid(crate::CanvasIssue::DuplicateShape));
                    }
                    crate::model::validate_shape(world, document, *shape)?;
                    let geometry = world
                        .get::<CanvasGeometry>(*shape)
                        .ok_or_else(|| crate::error::missing(crate::CanvasPart::BatchGeometry))?;
                    let paint = world
                        .get::<CanvasPaint>(*shape)
                        .ok_or_else(|| crate::error::missing(crate::CanvasPart::BatchPaint))?;
                    geometry._validate(transform, paint)?;
                }
                Ok(())
            })();
            result.map_err(|error| match failed_shape {
                Some(shape) => error.at("shape", shape),
                None => error,
            })?;
            for (shape, transform) in updates {
                world.entity_mut(*shape).insert(*transform);
            }
            Ok(())
        })()
        .map_err(|error| {
            error.input(crate::CanvasInput::Entity {
                field: "document",
                value: document,
            })
        })
    }
    pub fn remove_shapes(
        world: &mut World,
        document: Entity,
        shapes: &[Entity],
    ) -> crate::error::PublicResult {
        Self::_remove_shapes(world, document, shapes).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.remove_shapes",
            ))
        })
    }

    pub(crate) fn _remove_shapes(world: &mut World, document: Entity, shapes: &[Entity]) -> Result {
        (|| -> Result {
            let shapes = shapes.iter().copied().collect::<HashSet<_>>();
            let result: Result = (|| {
                crate::model::validate_document(world, document)?;
                for shape in &shapes {
                    crate::model::validate_shape(world, document, *shape)?;
                }
                Ok(())
            })();
            result?;
            world
                .get_mut::<crate::order::ShapeOrder>(document)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?
                .0
                .retain(|shape| !shapes.contains(shape));
            for shape in shapes {
                world.despawn(shape);
            }
            Ok(())
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entities {
                    field: "shapes",
                    values: shapes.to_vec(),
                })
        })
    }
}
