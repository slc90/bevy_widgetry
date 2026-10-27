use crate::{CanvasDocument, CanvasGeometry, CanvasPaint};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::collections::HashSet;

impl CanvasDocument {
    pub fn update_transforms(
        world: &mut World,
        document: Entity,
        updates: &[(Entity, Transform)],
    ) -> Result {
        let mut failed_shape = None;
        let result: Result = (|| {
            crate::model::validate_document(world, document)?;
            let mut seen = HashSet::new();
            for (shape, transform) in updates {
                failed_shape = Some(*shape);
                if !seen.insert(*shape) {
                    return Err(BevyError::error(
                        "Canvas batch transform contains duplicate shapes",
                    ));
                }
                crate::model::validate_shape(world, document, *shape)?;
                let geometry = world
                    .get::<CanvasGeometry>(*shape)
                    .ok_or_else(|| BevyError::error("Canvas batch geometry missing"))?;
                let paint = world
                    .get::<CanvasPaint>(*shape)
                    .ok_or_else(|| BevyError::error("Canvas batch paint missing"))?;
                geometry.validate(transform, paint)?;
            }
            Ok(())
        })();
        result.inspect_err(|error|widgetry_error!(?document,shape=?failed_shape,count=updates.len(),%error,"Canvas 批量变换更新被拒绝"))?;
        for (shape, transform) in updates {
            world.entity_mut(*shape).insert(*transform);
        }
        Ok(())
    }
    pub fn remove_shapes(world: &mut World, document: Entity, shapes: &[Entity]) -> Result {
        let shapes = shapes.iter().copied().collect::<HashSet<_>>();
        let mut failed_shape = None;
        let result: Result = (|| {
            crate::model::validate_document(world, document)?;
            for shape in &shapes {
                failed_shape = Some(*shape);
                crate::model::validate_shape(world, document, *shape)?;
            }
            Ok(())
        })();
        result.inspect_err(
            |error| widgetry_error!(?document,shape=?failed_shape,count=shapes.len(),%error,"Canvas 批量删除被拒绝"),
        )?;
        world
            .get_mut::<crate::order::ShapeOrder>(document)
            .ok_or_else(|| BevyError::error("Canvas document order missing"))?
            .0
            .retain(|shape| !shapes.contains(shape));
        for shape in shapes {
            world.despawn(shape);
        }
        Ok(())
    }
}
