use crate::source::DataSource;
use crate::{WidgetryListItemId, WidgetryListKey, WidgetryListView};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
    if let Some(policy) = view.projection_policy() {
        let valid: HashSet<_> = ids.iter().copied().collect();
        (policy.0)(world, source, &mut ids);
        let mut seen = HashSet::new();
        ids.retain(|id| valid.contains(id) && seen.insert(*id));
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
