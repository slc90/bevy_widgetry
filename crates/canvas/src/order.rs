use crate::error::CanvasResult as Result;
use crate::{
    CanvasDocument, CanvasGeometry, CanvasIssue, CanvasItem, CanvasPaint, CanvasPart, CanvasShape,
};
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::EntityRef;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct CanvasLayerId(u64);

const DEFAULT_LAYER_ID: CanvasLayerId = CanvasLayerId(0);

#[derive(Clone, Debug, Reflect)]
pub struct CanvasLayer {
    id: CanvasLayerId,
    name: String,
    visible: bool,
    locked: bool,
    items: Vec<Entity>,
}

impl CanvasLayer {
    pub fn id(&self) -> CanvasLayerId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CanvasLayerStack {
    layers: Vec<CanvasLayer>,
    next_layer_id: u64,
}

impl Default for CanvasLayerStack {
    fn default() -> Self {
        Self {
            layers: vec![CanvasLayer {
                id: DEFAULT_LAYER_ID,
                name: "Layer 1".to_owned(),
                visible: true,
                locked: false,
                items: Vec::new(),
            }],
            next_layer_id: DEFAULT_LAYER_ID.0 + 1,
        }
    }
}

impl CanvasLayerStack {
    pub fn layers(&self) -> &[CanvasLayer] {
        &self.layers
    }

    pub(crate) fn default_items_mut(&mut self) -> Result<&mut Vec<Entity>> {
        self.layers
            .iter_mut()
            .find(|layer| layer.id == DEFAULT_LAYER_ID)
            .map(|layer| &mut layer.items)
            .ok_or_else(|| crate::error::missing(CanvasPart::DocumentOrder))
    }
}

#[derive(Component, Default)]
pub(crate) struct CanvasItemOrder(Vec<Entity>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanvasContainer {
    Layer(CanvasLayerId),
    Group(Entity),
}

impl CanvasContainer {
    pub(crate) fn parent(self, document: Entity) -> Entity {
        match self {
            Self::Layer(_) => document,
            Self::Group(group) => group,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasOrder {
    Raise,
    Lower,
    Front,
    Back,
}

pub(crate) fn is_item_child(item: &EntityRef, parent: Entity) -> bool {
    item.contains::<CanvasItem>()
        && item
            .get::<ChildOf>()
            .is_some_and(|child| child.parent() == parent)
}

struct OrderComponents {
    item: Option<ComponentId>,
    shape: Option<[ComponentId; 4]>,
    group: Option<ComponentId>,
}

impl OrderComponents {
    fn new(world: &World) -> Self {
        Self {
            item: world.component_id::<CanvasItem>(),
            shape: (|| {
                Some([
                    world.component_id::<CanvasShape>()?,
                    world.component_id::<CanvasGeometry>()?,
                    world.component_id::<CanvasPaint>()?,
                    world.component_id::<Transform>()?,
                ])
            })(),
            group: world.component_id::<CanvasItemOrder>(),
        }
    }

    fn is_shape(&self, item: &EntityRef) -> bool {
        self.shape
            .is_some_and(|ids| ids.into_iter().all(|id| item.contains_id(id)))
    }

    fn is_group(&self, item: &EntityRef) -> bool {
        self.group.is_some_and(|id| item.contains_id(id))
    }

    fn is_attached(&self, item: &EntityRef, parent: Entity) -> bool {
        self.item.is_some_and(|id| item.contains_id(id))
            && item
                .get::<ChildOf>()
                .is_some_and(|child| child.parent() == parent)
    }

    fn is_valid(&self, item: &EntityRef, parent: Entity) -> bool {
        self.is_attached(item, parent) && (self.is_shape(item) || self.is_group(item))
    }
}

pub(crate) fn item_container(
    world: &World,
    document: Entity,
    item: Entity,
) -> Option<CanvasContainer> {
    let stack = world.get::<CanvasLayerStack>(document)?;
    let mut current = item;
    let mut groups = HashSet::new();
    let mut container = None;
    loop {
        if current == document || world.get::<CanvasItem>(current).is_none() {
            return None;
        }
        let parent = world.get::<ChildOf>(current)?.parent();
        if parent == document {
            return stack
                .layers()
                .iter()
                .find(|layer| layer.items.contains(&current))
                .map(|layer| container.unwrap_or(CanvasContainer::Layer(layer.id)));
        }
        if !groups.insert(parent) || !world.get::<CanvasItemOrder>(parent)?.0.contains(&current) {
            return None;
        }
        container.get_or_insert(CanvasContainer::Group(parent));
        current = parent;
    }
}

pub(crate) fn visit_draw_order<E>(
    world: &World,
    stack: &CanvasLayerStack,
    mut visit: impl FnMut(Entity, CanvasContainer, Option<&EntityRef>) -> std::result::Result<bool, E>,
) -> std::result::Result<(), E> {
    let mut parents = Vec::new();
    for layer in stack.layers() {
        let mut container = CanvasContainer::Layer(layer.id);
        let mut items = layer.items.iter();
        loop {
            if let Some(&item) = items.next() {
                let entity = world.get_entity(item).ok();
                if visit(item, container, entity.as_ref())?
                    && let Some(children) =
                        entity.and_then(|entity| entity.get::<CanvasItemOrder>())
                {
                    parents.push((container, items));
                    container = CanvasContainer::Group(item);
                    items = children.0.iter();
                }
            } else if let Some((ancestor, siblings)) = parents.pop() {
                container = ancestor;
                items = siblings;
            } else {
                break;
            }
        }
    }
    Ok(())
}

fn container_items(
    world: &World,
    document: Entity,
    container: CanvasContainer,
) -> Result<&[Entity]> {
    match container {
        CanvasContainer::Layer(id) => world
            .get::<CanvasLayerStack>(document)
            .and_then(|stack| stack.layers().iter().find(|layer| layer.id == id))
            .map(|layer| layer.items.as_slice()),
        CanvasContainer::Group(group) => world
            .get::<CanvasItemOrder>(group)
            .filter(|_| item_container(world, document, group).is_some())
            .map(|order| order.0.as_slice()),
    }
    .ok_or_else(|| crate::error::invalid(CanvasIssue::OrderMembership))
}

pub(crate) fn container_items_mut(
    world: &mut World,
    document: Entity,
    container: CanvasContainer,
) -> Result<&mut Vec<Entity>> {
    match container {
        CanvasContainer::Layer(id) => world
            .get_mut::<CanvasLayerStack>(document)
            .and_then(|stack| {
                stack
                    .into_inner()
                    .layers
                    .iter_mut()
                    .find(|layer| layer.id == id)
            })
            .map(|layer| &mut layer.items),
        CanvasContainer::Group(group) => world
            .get_mut::<CanvasItemOrder>(group)
            .map(|order| &mut order.into_inner().0),
    }
    .ok_or_else(|| crate::error::missing(CanvasPart::DocumentOrder))
}

impl CanvasDocument {
    pub fn layer_order(
        world: &World,
        document: Entity,
    ) -> crate::error::PublicResult<Vec<CanvasLayerId>> {
        let result = stack(world, document)
            .map(|stack| stack.layers().iter().map(CanvasLayer::id).collect());
        result.map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.layer_order",
            ))
        })
    }

    pub fn item_order(
        world: &World,
        document: Entity,
        container: CanvasContainer,
    ) -> crate::error::PublicResult<Vec<Entity>> {
        let result = (|| {
            crate::model::validate_document(world, document)?;
            let order = container_items(world, document, container)?;
            let parent = container.parent(document);
            let components = OrderComponents::new(world);
            Ok(order
                .iter()
                .copied()
                .filter(|item| {
                    world
                        .get_entity(*item)
                        .is_ok_and(|item| components.is_valid(&item, parent))
                })
                .collect())
        })();
        result.map_err(|error: crate::error::CanvasFault| {
            error
                .at("document", document)
                .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                    "canvas.CanvasDocument.item_order",
                ))
        })
    }

    pub fn draw_order(world: &World, document: Entity) -> crate::error::PublicResult<Vec<Entity>> {
        Self::_draw_order(world, document).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.draw_order",
            ))
        })
    }

    pub(crate) fn _draw_order(world: &World, document: Entity) -> Result<Vec<Entity>> {
        let components = OrderComponents::new(world);
        collect_draw_order(world, document, |_, container, entity| {
            entity.is_some_and(|item| components.is_valid(item, container.parent(document)))
        })
    }

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
        let mut stale = HashMap::<_, HashSet<_>>::new();
        let stack = stack(world, document)?;
        let capacity = stack.layers().iter().map(|layer| layer.items.len()).sum();
        let mut shapes = Vec::with_capacity(capacity);
        let components = OrderComponents::new(world);
        visit_draw_order(world, stack, |item, container, entity| -> Result<bool> {
            if let Some(entry) =
                entity.filter(|entry| components.is_attached(entry, container.parent(document)))
            {
                if components.is_shape(entry) {
                    shapes.push(item);
                    return Ok(false);
                }
                if components.is_group(entry) {
                    return Ok(true);
                }
            }
            stale.entry(container).or_default().insert(item);
            Ok(false)
        })?;
        for (container, removed) in stale {
            container_items_mut(world, document, container)?.retain(|item| !removed.contains(item));
        }
        Ok(shapes)
    }

    pub fn reorder_item(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        item: Entity,
        operation: CanvasOrder,
    ) -> crate::error::PublicResult<bool> {
        reorder(world, document, container, item, operation).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.reorder_item",
            ))
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
        let container = crate::model::validate_shape(world, document, shape)?;
        reorder(world, document, container, shape, operation)
    }
}

fn stack(world: &World, document: Entity) -> Result<&CanvasLayerStack> {
    world
        .get::<CanvasLayerStack>(document)
        .filter(|_| world.get::<CanvasDocument>(document).is_some())
        .ok_or_else(|| crate::error::missing(CanvasPart::Document).at("document", document))
}

fn collect_draw_order(
    world: &World,
    document: Entity,
    mut include: impl FnMut(Entity, CanvasContainer, Option<&EntityRef>) -> bool,
) -> Result<Vec<Entity>> {
    let stack = stack(world, document)?;
    let mut items = Vec::with_capacity(stack.layers().iter().map(|layer| layer.items.len()).sum());
    visit_draw_order(world, stack, |item, container, entity| -> Result<bool> {
        let valid = include(item, container, entity);
        if valid {
            items.push(item);
        }
        Ok(valid)
    })?;
    Ok(items)
}

fn reorder(
    world: &mut World,
    document: Entity,
    container: CanvasContainer,
    item: Entity,
    operation: CanvasOrder,
) -> Result<bool> {
    (|| {
        crate::model::validate_document(world, document)?;
        let order = container_items(world, document, container)?;
        let parent = container.parent(document);
        let components = OrderComponents::new(world);
        let valid = |candidate: &Entity| {
            world
                .get_entity(*candidate)
                .is_ok_and(|entry| components.is_valid(&entry, parent))
        };
        let repaired = (!order.iter().all(valid))
            .then(|| order.iter().copied().filter(valid).collect::<Vec<_>>());
        let items = repaired.as_deref().unwrap_or(order);
        let index = items
            .iter()
            .position(|entity| *entity == item)
            .ok_or_else(|| crate::error::invalid(CanvasIssue::OrderMembership))?;
        let top = items.len() - 1;
        let target = match operation {
            CanvasOrder::Raise => (index + 1).min(top),
            CanvasOrder::Lower => index.saturating_sub(1),
            CanvasOrder::Front => top,
            CanvasOrder::Back => 0,
        };
        let changed = index != target;
        if changed || repaired.is_some() {
            let order = container_items_mut(world, document, container)?;
            if let Some(repaired) = repaired {
                *order = repaired;
            }
            if changed {
                order.remove(index);
                order.insert(target, item);
            }
        }
        Ok(changed)
    })()
    .map_err(|error: crate::error::CanvasFault| error.at("document", document).at("item", item))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_macros, clippy::unwrap_used)]

    use super::*;
    use crate::{CanvasPaint, CanvasRectangle};

    #[test]
    fn traversal_and_mutation_follow_each_registered_container() {
        let mut world = World::new();
        let document = world.spawn(CanvasDocument).id();
        let group = world
            .spawn((CanvasItem, CanvasItemOrder::default(), ChildOf(document)))
            .id();
        let nested = world
            .spawn((CanvasItem, CanvasItemOrder::default(), ChildOf(group)))
            .id();
        let [a, b, c] = std::array::from_fn(|_| {
            CanvasDocument::create_rectangle(
                &mut world,
                document,
                CanvasRectangle {
                    size: Vec2::ONE,
                    corner_radius: 0.0,
                },
                Transform::default(),
                CanvasPaint::default(),
            )
            .unwrap()
        });
        world.entity_mut(a).insert(ChildOf(nested));
        world.entity_mut(b).insert(ChildOf(nested));
        world.get_mut::<CanvasItemOrder>(nested).unwrap().0 = vec![a, b];
        world
            .get_mut::<CanvasItemOrder>(group)
            .unwrap()
            .0
            .push(nested);

        let mut stack = world.get_mut::<CanvasLayerStack>(document).unwrap();
        let second = CanvasLayerId(stack.next_layer_id);
        stack.next_layer_id += 1;
        *stack.default_items_mut().unwrap() = vec![group];
        stack.layers.push(CanvasLayer {
            id: second,
            name: "Layer 2".to_owned(),
            visible: true,
            locked: false,
            items: vec![c],
        });

        assert_eq!(
            CanvasDocument::layer_order(&world, document).unwrap(),
            [DEFAULT_LAYER_ID, second]
        );
        assert_eq!(
            CanvasDocument::draw_order(&world, document).unwrap(),
            [group, nested, a, b, c]
        );
        assert_eq!(
            CanvasDocument::item_order(&world, document, CanvasContainer::Group(group)).unwrap(),
            [nested]
        );
        assert_eq!(
            CanvasDocument::item_order(&world, document, CanvasContainer::Group(nested)).unwrap(),
            [a, b]
        );
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            [a, b, c]
        );
        assert!(
            CanvasDocument::reorder_shape(&mut world, document, a, CanvasOrder::Front).unwrap()
        );
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            [b, a, c]
        );
        CanvasDocument::remove_shape(&mut world, document, b).unwrap();
        assert_eq!(
            CanvasDocument::item_order(&world, document, CanvasContainer::Group(nested)).unwrap(),
            [a]
        );

        world
            .get_mut::<CanvasLayerStack>(document)
            .unwrap()
            .default_items_mut()
            .unwrap()
            .clear();
        assert_eq!(item_container(&world, document, a), None);
        assert_eq!(CanvasDocument::draw_order(&world, document).unwrap(), [c]);
    }
}
