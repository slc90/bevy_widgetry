use crate::{WidgetryListItemId, WidgetryListModel};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use std::{any::TypeId, fmt::Debug, hash::Hash, marker::PhantomData};

pub trait WidgetryListKey: Copy + Eq + Hash + Debug + Send + Sync + 'static {}
impl<K: Copy + Eq + Hash + Debug + Send + Sync + 'static> WidgetryListKey for K {}

pub enum WidgetryListValue<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<T> AsRef<T> for WidgetryListValue<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}

pub struct WidgetryListData<'a, T> {
    pub value: WidgetryListValue<'a, T>,
    pub disabled: bool,
    pub revision: Option<u64>,
}

pub trait WidgetryListDataSource<T: Send + Sync + 'static, K: WidgetryListKey>:
    Send + Sync + 'static
{
    fn changed_at(world: &World, source: Entity) -> Option<Tick>;
    fn ids(world: &World, source: Entity) -> Option<Vec<K>>;
    fn row(world: &World, source: Entity, id: K) -> Option<WidgetryListData<'_, T>>;
    fn contains(world: &World, source: Entity, id: K) -> bool {
        Self::row(world, source, id).is_some()
    }
}

pub struct WidgetryListModelSource<T>(PhantomData<fn() -> T>);

impl<T: Send + Sync + 'static> WidgetryListDataSource<T, WidgetryListItemId>
    for WidgetryListModelSource<T>
{
    fn changed_at(world: &World, source: Entity) -> Option<Tick> {
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<WidgetryListModel<T>>()?
                .last_changed(),
        )
    }

    fn ids(world: &World, source: Entity) -> Option<Vec<WidgetryListItemId>> {
        let model = world.get::<WidgetryListModel<T>>(source)?;
        Some(
            (0..model.len())
                .filter_map(|index| model.id(index))
                .collect(),
        )
    }

    fn contains(world: &World, source: Entity, id: WidgetryListItemId) -> bool {
        world
            .get::<WidgetryListModel<T>>(source)
            .is_some_and(|model| model.index_of(id).is_some())
    }

    fn row(
        world: &World,
        source: Entity,
        id: WidgetryListItemId,
    ) -> Option<WidgetryListData<'_, T>> {
        let model = world.get::<WidgetryListModel<T>>(source)?;
        let index = model.index_of(id)?;
        Some(WidgetryListData {
            value: WidgetryListValue::Borrowed(model.get(index)?),
            disabled: model.is_disabled(index)?,
            revision: model.revision(index),
        })
    }
}

#[derive(Resource)]
pub(crate) struct DataSource<T: Send + Sync + 'static, K: WidgetryListKey> {
    pub(crate) adapter: TypeId,
    pub(crate) changed_at: fn(&World, Entity) -> Option<Tick>,
    pub(crate) ids: fn(&World, Entity) -> Option<Vec<K>>,
    pub(crate) contains: fn(&World, Entity, K) -> bool,
    pub(crate) row: for<'a> fn(&'a World, Entity, K) -> Option<WidgetryListData<'a, T>>,
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> Copy for DataSource<T, K> {}
impl<T: Send + Sync + 'static, K: WidgetryListKey> Clone for DataSource<T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> DataSource<T, K> {
    pub(crate) fn new<D: WidgetryListDataSource<T, K>>() -> Self {
        Self {
            adapter: TypeId::of::<D>(),
            changed_at: D::changed_at,
            ids: D::ids,
            row: D::row,
            contains: D::contains,
        }
    }
}
