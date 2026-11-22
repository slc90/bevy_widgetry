use crate::error::{CanvasResult as Result, PublicResult};
use crate::order::{is_item_child, item_container, stack, valid_items};
use crate::{
    CanvasContainer, CanvasDocument, CanvasIssue, CanvasLayer, CanvasLayerId, CanvasLayerStack,
    CanvasPart,
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasLayerRemoval {
    OnlyIfEmpty,
    MoveContentsTo(CanvasLayerId),
    DeleteContents,
}

#[derive(EntityEvent)]
pub struct CanvasLayersChanged {
    pub entity: Entity,
}

impl CanvasLayer {
    pub(crate) fn editable(&self) -> bool {
        self.visible && !self.locked
    }
}

impl CanvasLayerStack {
    pub(crate) fn layer(&self, id: CanvasLayerId) -> Option<&CanvasLayer> {
        self.layers().iter().find(|layer| layer.id() == id)
    }

    pub(crate) fn editable_layer(&self, preferred: Option<CanvasLayerId>) -> Option<CanvasLayerId> {
        preferred
            .filter(|id| self.layer(*id).is_some_and(CanvasLayer::editable))
            .or_else(|| {
                self.layers()
                    .iter()
                    .find(|layer| layer.editable())
                    .map(CanvasLayer::id)
            })
    }
}

impl CanvasDocument {
    pub fn layers(world: &World, document: Entity) -> PublicResult<&[CanvasLayer]> {
        stack(world, document)
            .map(CanvasLayerStack::layers)
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.layers")))
    }

    pub fn default_layer(world: &World, document: Entity) -> PublicResult<CanvasLayerId> {
        stack(world, document)
            .map(|stack| stack.default_layer)
            .map_err(|error| {
                error.into_failure(ErrorFrame::new("canvas.CanvasDocument.default_layer"))
            })
    }

    pub fn create_layer(
        world: &mut World,
        document: Entity,
        index: usize,
        name: impl Into<String>,
    ) -> PublicResult<CanvasLayerId> {
        edit_layers(
            world,
            document,
            "canvas.CanvasDocument.create_layer",
            |world| {
                let mut stack = stack_mut(world, document)?;
                if index > stack.layers.len() {
                    return Err(crate::error::invalid(CanvasIssue::LayerIndex).input(
                        crate::CanvasInput::Index {
                            field: "index",
                            value: index,
                        },
                    ));
                }
                let id = CanvasLayerId {
                    document,
                    local: stack.next_layer_id,
                };
                stack.next_layer_id = stack
                    .next_layer_id
                    .checked_add(1)
                    .ok_or_else(|| crate::error::invalid(CanvasIssue::LayerIdExhausted))?;
                stack.layers.insert(
                    index,
                    CanvasLayer {
                        id,
                        name: name.into(),
                        visible: true,
                        locked: false,
                        items: Vec::new(),
                    },
                );
                Ok((id, true))
            },
        )
    }

    pub fn rename_layer(
        world: &mut World,
        document: Entity,
        layer: CanvasLayerId,
        name: impl Into<String>,
    ) -> PublicResult<bool> {
        set_layer_value(
            world,
            document,
            layer,
            "canvas.CanvasDocument.rename_layer",
            name.into(),
            |layer| &mut layer.name,
        )
    }

    pub fn set_layer_visible(
        world: &mut World,
        document: Entity,
        layer: CanvasLayerId,
        visible: bool,
    ) -> PublicResult<bool> {
        edit_layers(
            world,
            document,
            "canvas.CanvasDocument.set_layer_visible",
            |world| {
                let stack = stack(world, document)?;
                let index = layer_index(stack, layer)?;
                if stack.layers[index].visible == visible {
                    return Ok((false, false));
                }
                if visible {
                    // The renderer skips hidden layers, so their order can still contain removed items.
                    drop(CanvasDocument::shape_order_in(world, document, |entry| {
                        entry.id() == layer
                    })?);
                }
                stack_mut(world, document)?.layers[index].visible = visible;
                Ok((true, true))
            },
        )
    }

    pub fn set_layer_locked(
        world: &mut World,
        document: Entity,
        layer: CanvasLayerId,
        locked: bool,
    ) -> PublicResult<bool> {
        set_layer_value(
            world,
            document,
            layer,
            "canvas.CanvasDocument.set_layer_locked",
            locked,
            |layer| &mut layer.locked,
        )
    }

    pub fn reorder_layer(
        world: &mut World,
        document: Entity,
        layer: CanvasLayerId,
        index: usize,
    ) -> PublicResult<bool> {
        edit_layers(
            world,
            document,
            "canvas.CanvasDocument.reorder_layer",
            |world| {
                let mut stack = stack_mut(world, document)?;
                if index >= stack.layers.len() {
                    return Err(crate::error::invalid(CanvasIssue::LayerIndex).input(
                        crate::CanvasInput::Index {
                            field: "index",
                            value: index,
                        },
                    ));
                }
                let previous = layer_index(&stack, layer)?;
                if previous == index {
                    return Ok((false, false));
                }
                let layer = stack.layers.remove(previous);
                stack.layers.insert(index, layer);
                Ok((true, true))
            },
        )
    }

    pub fn move_item_to_layer(
        world: &mut World,
        document: Entity,
        item: Entity,
        target: CanvasLayerId,
    ) -> PublicResult<bool> {
        edit_layers(
            world,
            document,
            "canvas.CanvasDocument.move_item_to_layer",
            |world| {
                let Some(CanvasContainer::Layer(source)) = item_container(world, document, item)
                else {
                    return Err(crate::error::invalid(CanvasIssue::Ownership).at("item", item));
                };
                if valid_items(world, document, &[item]).next().is_none() {
                    return Err(crate::model::invalid_ownership(world, item).at("item", item));
                }
                let mut stack = stack_mut(world, document)?;
                let source_index = layer_index(&stack, source)?;
                let target_index = layer_index(&stack, target)?;
                if source_index == target_index {
                    return Ok((false, false));
                }
                stack.layers[source_index]
                    .items
                    .retain(|member| *member != item);
                stack.layers[target_index].items.push(item);
                Ok((true, true))
            },
        )
    }

    pub fn remove_layer(
        world: &mut World,
        document: Entity,
        layer: CanvasLayerId,
        removal: CanvasLayerRemoval,
    ) -> PublicResult<()> {
        edit_layers(
            world,
            document,
            "canvas.CanvasDocument.remove_layer",
            |world| {
                let stack = stack(world, document)?;
                let index = layer_index(stack, layer)?;
                if stack.layers.len() == 1 {
                    return Err(crate::error::invalid(CanvasIssue::LastLayer));
                }
                let members =
                    valid_items(world, document, &stack.layers[index].items).collect::<Vec<_>>();
                let target_index = match removal {
                    CanvasLayerRemoval::OnlyIfEmpty if !members.is_empty() => {
                        return Err(crate::error::invalid(CanvasIssue::LayerNotEmpty));
                    }
                    CanvasLayerRemoval::MoveContentsTo(target) => {
                        let index = layer_index(stack, target)?;
                        if target == layer || !stack.layers[index].editable() {
                            return Err(crate::error::invalid(CanvasIssue::LayerUnavailable));
                        }
                        Some(index)
                    }
                    _ => None,
                };
                let mut stack = stack_mut(world, document)?;
                let removed = stack.layers.remove(index);
                if layer == stack.default_layer
                    && let Some(default) = stack.layers.first()
                {
                    stack.default_layer = default.id();
                }
                if let Some(target_index) = target_index {
                    let target_index = target_index - usize::from(index < target_index);
                    stack.layers[target_index].items.extend(members);
                } else if removal == CanvasLayerRemoval::DeleteContents {
                    for item in removed.items {
                        if world
                            .get_entity(item)
                            .is_ok_and(|entry| is_item_child(&entry, document))
                        {
                            world.despawn(item);
                        }
                    }
                }
                Ok(((), true))
            },
        )
    }
}

fn stack_mut(world: &mut World, document: Entity) -> Result<Mut<'_, CanvasLayerStack>> {
    world
        .get_mut::<CanvasLayerStack>(document)
        .ok_or_else(|| crate::error::missing(CanvasPart::DocumentOrder))
}

fn layer_index(stack: &CanvasLayerStack, layer: CanvasLayerId) -> Result<usize> {
    stack
        .layers()
        .iter()
        .position(|entry| entry.id() == layer)
        .ok_or_else(|| {
            crate::error::invalid(CanvasIssue::LayerTarget).input(crate::CanvasInput::Layer(layer))
        })
}

fn set_layer_value<T: PartialEq>(
    world: &mut World,
    document: Entity,
    layer: CanvasLayerId,
    operation: &'static str,
    value: T,
    access: impl Fn(&mut CanvasLayer) -> &mut T,
) -> PublicResult<bool> {
    edit_layers(world, document, operation, |world| {
        let stack = stack_mut(world, document)?;
        let index = layer_index(&stack, layer)?;
        let mut property = stack.map_unchanged(|stack| access(&mut stack.layers[index]));
        let changed = property.set_if_neq(value);
        Ok((changed, changed))
    })
}

fn edit_layers<T>(
    world: &mut World,
    document: Entity,
    operation: &'static str,
    edit: impl FnOnce(&mut World) -> Result<(T, bool)>,
) -> PublicResult<T> {
    (|| -> Result<T> {
        crate::model::validate_document(world, document)?;
        let (value, changed) = edit(world)?;
        if changed {
            crate::view::repair_layers(world, document)?;
            world.trigger(CanvasLayersChanged { entity: document });
        }
        Ok(value)
    })()
    .map_err(|error| {
        error
            .at("document", document)
            .into_failure(ErrorFrame::new(operation))
    })
}
