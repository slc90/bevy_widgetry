use crate::source::DataSource;
use crate::{WidgetryListItemId, WidgetryListKey, WidgetryListView};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Component)]
#[component(immutable)]
pub(crate) struct ViewOrder<K: WidgetryListKey> {
    pub(crate) source: Entity,
    pub(crate) ids: Vec<K>,
}

pub struct WidgetryListProjectionPolicy<K: WidgetryListKey = WidgetryListItemId>(
    Arc<dyn Fn(&World, Entity, &mut Vec<K>) + Send + Sync>,
);

impl<K: WidgetryListKey> Clone for WidgetryListProjectionPolicy<K> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: WidgetryListKey> WidgetryListProjectionPolicy<K> {
    pub fn new(policy: impl Fn(&World, Entity, &mut Vec<K>) + Send + Sync + 'static) -> Self {
        Self(Arc::new(policy))
    }
}

#[derive(Component)]
#[component(immutable)]
pub struct WidgetryListProjection<K: WidgetryListKey = WidgetryListItemId> {
    ordered_ids: Vec<K>,
    indices: HashMap<K, usize>,
    source_tick: Tick,
    view_tick: Tick,
    source: Entity,
}

impl<K: WidgetryListKey> WidgetryListProjection<K> {
    pub fn ordered_ids(&self) -> &[K] {
        &self.ordered_ids
    }
    pub fn id(&self, display_index: usize) -> Option<K> {
        self.ordered_ids.get(display_index).copied()
    }
    pub fn index_of(&self, id: K) -> Option<usize> {
        self.indices.get(&id).copied()
    }
    pub fn len(&self) -> usize {
        self.ordered_ids.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ordered_ids.is_empty()
    }
}

pub(crate) fn refresh<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        refresh_root::<T, K>(world, root);
    }
}

pub fn refresh_root<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World, root: Entity) {
    let Some(adapter) = world.get_resource::<DataSource<T, K>>().copied() else {
        return;
    };
    let Ok(entity) = world.get_entity(root) else {
        return;
    };
    let Some(view) = entity.get_ref::<WidgetryListView<T, K>>() else {
        return;
    };
    let source = view.source();
    let Some(source_tick) = (adapter.changed_at)(world, source) else {
        return;
    };
    let view_tick = view.last_changed();
    if world
        .get::<WidgetryListProjection<K>>(root)
        .is_some_and(|projection| {
            projection.source_tick == source_tick
                && projection.view_tick == view_tick
                && projection.source == source
        })
    {
        return;
    }
    let Some(mut ids) = (adapter.ids)(world, source) else {
        return;
    };
    let raw_ids = world.get::<ViewOrder<K>>(root).map(|_| ids.clone());
    if let Some(policy) = view.projection_policy() {
        let valid: HashSet<_> = ids.iter().copied().collect();
        (policy.0)(world, source, &mut ids);
        let mut seen = HashSet::new();
        ids.retain(|id| valid.contains(id) && seen.insert(*id));
    }
    if let (Some(order), Some(raw_ids)) = (world.get::<ViewOrder<K>>(root), raw_ids) {
        if order.source == source {
            let live: HashSet<_> = raw_ids.iter().copied().collect();
            let eligible: HashSet<_> = ids.iter().copied().collect();
            let mut complete = order.ids.clone();
            complete.retain(|id| live.contains(id));
            let mut known: HashSet<_> = complete.iter().copied().collect();
            complete.extend(raw_ids.into_iter().filter(|id| known.insert(*id)));
            ids = complete
                .iter()
                .copied()
                .filter(|id| eligible.contains(id))
                .collect();
            world.entity_mut(root).insert(ViewOrder {
                source,
                ids: complete,
            });
        } else {
            world.entity_mut(root).remove::<ViewOrder<K>>();
        }
    }
    let indices = ids
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect();
    world.entity_mut(root).insert(WidgetryListProjection {
        ordered_ids: ids,
        indices,
        source_tick,
        view_tick,
        source,
    });
}

pub(crate) fn reorder<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    movement: crate::drag::WidgetryListMove<K>,
) -> Result<bool, BevyError> {
    refresh_root::<T, K>(world, root);
    let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
        return Err(BevyError::error("ListView reorder requires a live view"));
    };
    let source = view.source();
    let Some(projection) = world.get::<WidgetryListProjection<K>>(root) else {
        return Err(BevyError::error(
            "ListView reorder requires a live projection",
        ));
    };
    if movement.moving == movement.target {
        return Ok(false);
    }
    let mut ids = if let Some(order) = world.get::<ViewOrder<K>>(root) {
        order.ids.clone()
    } else {
        let adapter = world.resource::<DataSource<T, K>>();
        let Some(mut ids) = (adapter.ids)(world, source) else {
            return Err(BevyError::error("ListView reorder source unavailable"));
        };
        let mut displayed = projection.ordered_ids.iter().copied();
        for id in &mut ids {
            if projection.index_of(*id).is_some()
                && let Some(next) = displayed.next()
            {
                *id = next;
            }
        }
        ids
    };
    let (Some(from), Some(anchor)) = (
        ids.iter().position(|id| *id == movement.moving),
        ids.iter().position(|id| *id == movement.target),
    ) else {
        return Err(BevyError::error("ListView reorder IDs unavailable"));
    };
    let insertion =
        anchor + usize::from(movement.placement == crate::drag::WidgetryListDropPlacement::After);
    let to = insertion - usize::from(from < insertion);
    if from == to {
        return Ok(false);
    }
    let (Some(display_from), Some(display_anchor)) = (
        projection.index_of(movement.moving),
        projection.index_of(movement.target),
    ) else {
        return Err(BevyError::error("ListView reorder IDs are not displayed"));
    };
    let display_insertion = display_anchor
        + usize::from(movement.placement == crate::drag::WidgetryListDropPlacement::After);
    let display_to = display_insertion - usize::from(display_from < display_insertion);
    let Some(mut projection) = world.entity_mut(root).take::<WidgetryListProjection<K>>() else {
        return Err(BevyError::error("ListView reorder projection disappeared"));
    };
    ids.remove(from);
    ids.insert(to, movement.moving);
    // Moving one ID cannot change policy eligibility or the source tick.
    // Re-evaluating the source here would repeat a full data refresh for every drop.
    projection.ordered_ids.remove(display_from);
    projection.ordered_ids.insert(display_to, movement.moving);
    for index in display_from.min(display_to)..=display_from.max(display_to) {
        projection
            .indices
            .insert(projection.ordered_ids[index], index);
    }
    world.entity_mut(root).insert(ViewOrder { source, ids });
    world.entity_mut(root).insert(projection);
    Ok(true)
}

#[cfg(test)]
pub(crate) fn from_ids_for_test<K: WidgetryListKey>(ids: Vec<K>) -> WidgetryListProjection<K> {
    let indices = ids
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect();
    WidgetryListProjection {
        ordered_ids: ids,
        indices,
        source_tick: Tick::new(0),
        view_tick: Tick::new(0),
        source: Entity::PLACEHOLDER,
    }
}
