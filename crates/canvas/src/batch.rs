use crate::error::CanvasResult as Result;
use crate::{CanvasDocument, CanvasGeometry, CanvasPaint};
use bevy::prelude::*;

use std::collections::{HashMap, HashSet};

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
        update_transforms(
            world,
            document,
            updates,
            |world, document, shape, transform| {
                crate::model::validate_shape(world, document, shape)?;
                let geometry = world
                    .get::<CanvasGeometry>(shape)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::BatchGeometry))?;
                let paint = world
                    .get::<CanvasPaint>(shape)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::BatchPaint))?;
                crate::groups::validate_shape_projection(world, shape, geometry, transform, paint)
            },
        )
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
        remove_items(world, document, shapes, crate::model::validate_shape)
    }
}

pub(crate) fn update_selected_transforms(
    world: &mut World,
    document: Entity,
    updates: &[(Entity, Transform)],
) -> Result {
    update_transforms(
        world,
        document,
        updates,
        |world, document, item, transform| {
            crate::groups::item_parent(world, document, item)?;
            crate::groups::validate_item_transform(world, item, transform)
        },
    )
}

fn update_transforms(
    world: &mut World,
    document: Entity,
    updates: &[(Entity, Transform)],
    validate: impl Fn(&World, Entity, Entity, Transform) -> Result,
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
                validate(world, document, *shape, *transform)?;
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

pub(crate) fn remove_selected_items(
    world: &mut World,
    document: Entity,
    items: &[Entity],
) -> Result {
    remove_items(world, document, items, crate::groups::item_parent)
}

fn remove_items(
    world: &mut World,
    document: Entity,
    shapes: &[Entity],
    membership: impl Fn(&World, Entity, Entity) -> Result<crate::CanvasContainer>,
) -> Result {
    (|| -> Result {
        let shapes = shapes.iter().copied().collect::<HashSet<_>>();
        crate::model::validate_document(world, document)?;
        let mut by_container = HashMap::<_, HashSet<_>>::new();
        for shape in &shapes {
            let container = membership(world, document, *shape)?;
            by_container.entry(container).or_default().insert(*shape);
        }
        for (&container, removed) in &by_container {
            crate::order::container_items_mut(world, document, container)?
                .retain(|shape| !removed.contains(shape));
        }
        for shape in shapes {
            world.despawn(shape);
        }
        for container in by_container.into_keys() {
            if let crate::CanvasContainer::Group(group) = container {
                crate::groups::prune_empty_groups(world, document, group)?;
            }
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
