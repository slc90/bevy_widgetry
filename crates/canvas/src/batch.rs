use crate::CanvasDocument;
use crate::error::CanvasResult as Result;
use bevy::prelude::*;

use std::collections::{HashMap, HashSet};

impl CanvasDocument {
    pub fn update_transforms(
        world: &mut World,
        target: impl Into<crate::CanvasEditTarget>,
        updates: &[(Entity, Transform)],
    ) -> crate::error::PublicResult {
        crate::history::apply(world, target, |world, document| {
            update_selected_transforms(world, document, updates)
        })
        .map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.update_transforms",
            ))
        })
    }

    pub fn remove_shapes(
        world: &mut World,
        target: impl Into<crate::CanvasEditTarget>,
        shapes: &[Entity],
    ) -> crate::error::PublicResult {
        crate::history::apply(world, target, |world, document| {
            Self::_remove_shapes(world, document, shapes)
        })
        .map_err(|error| {
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
                crate::groups::item_parent(world, document, *shape)?;
                crate::groups::validate_item_transform(world, *shape, *transform)?;
            }
            Ok(())
        })();
        result.map_err(|error| match failed_shape {
            Some(shape) => error.at("shape", shape),
            None => error,
        })?;
        crate::history::set_transforms(world, document, updates)
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
    crate::history::apply(world, document, |world, document| {
        remove_items(world, document, items, crate::groups::item_parent)
    })
}

pub(crate) fn remove_items(
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
            crate::history::edit_order(world, document, container, 0, |items| {
                items.retain(|shape| !removed.contains(shape));
            })?;
        }
        for shape in shapes {
            crate::history::detach_item(world, document, shape)?;
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
