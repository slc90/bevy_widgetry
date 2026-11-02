use crate::source::DataSource;
use crate::{
    ListError, ListFailure, ListIndexSpace, ListKeyRejection, ListKeySnapshot, ListSourceRejection,
    error::{failure, invalid_key, invalid_source, invalid_view, missing},
};
use crate::{
    WidgetryListItemId, WidgetryListKey, WidgetryListModel, WidgetryListProjection,
    WidgetryListView, WidgetryListViewState,
};
use bevy::prelude::*;
use std::{collections::HashSet, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryListSelectionMode {
    Single,
    #[default]
    Multiple,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryListSelectAllScope {
    #[default]
    Source,
    VisibleUnion,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct SelectionModifiers {
    pub(crate) control: bool,
    shift: bool,
}

impl SelectionModifiers {
    pub(crate) fn from_keys(keys: Option<&ButtonInput<KeyCode>>) -> Self {
        Self {
            control: keys.is_some_and(|keys| {
                keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
            }),
            shift: keys
                .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(immutable)]
pub struct WidgetryListSelectionState<K: WidgetryListKey = WidgetryListItemId> {
    selected: Arc<HashSet<K>>,
    anchor: Option<K>,
}

impl<K: WidgetryListKey> Default for WidgetryListSelectionState<K> {
    fn default() -> Self {
        Self {
            selected: Arc::new(HashSet::new()),
            anchor: None,
        }
    }
}

impl<K: WidgetryListKey> WidgetryListSelectionState<K> {
    pub fn selected_ids(&self) -> &HashSet<K> {
        &self.selected
    }
    pub fn contains(&self, id: K) -> bool {
        self.selected.contains(&id)
    }
    pub fn selected_id(&self) -> Option<K> {
        if self.selected.len() == 1 {
            self.selected.iter().next().copied()
        } else {
            None
        }
    }
    pub fn anchor(&self) -> Option<K> {
        self.anchor
    }
}

#[derive(EntityEvent, Clone, Debug)]
pub struct WidgetryListSelectionChanged<K: WidgetryListKey = WidgetryListItemId> {
    #[event_target]
    pub source: Entity,
    pub selected: Arc<HashSet<K>>,
    pub added: Vec<K>,
    pub removed: Vec<K>,
}

pub(crate) fn validate<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &World,
    root: Entity,
) -> Result<Entity, ListFailure> {
    let view = world
        .get::<WidgetryListView<T, K>>(root)
        .ok_or_else(|| invalid_view::<T, K>(root, "list_view.selection"))?;
    let adapter = world.get_resource::<DataSource<T, K>>().ok_or_else(|| {
        invalid_source::<T, K>(
            root,
            view.source(),
            ListSourceRejection::Unregistered,
            "list_view.validate_selection",
        )
    })?;
    if (adapter.changed_at)(world, view.source()).is_none() {
        return Err(invalid_source::<T, K>(
            root,
            view.source(),
            ListSourceRejection::Unavailable,
            "list_view.validate_selection",
        ));
    }
    if world.get::<WidgetryListSelectionState<K>>(root).is_none() {
        return Err(missing(
            root,
            "WidgetryListSelectionState",
            "list_view.validate_selection",
        ));
    }
    if world.get::<WidgetryListViewState<K>>(root).is_none() {
        return Err(missing(
            root,
            "WidgetryListViewState",
            "list_view.validate_selection",
        ));
    }
    Ok(view.source())
}

pub(crate) fn commit<K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    selected: HashSet<K>,
    anchor: Option<K>,
    notify: bool,
) -> Result<bool, ListFailure> {
    let previous = world
        .get::<WidgetryListSelectionState<K>>(root)
        .ok_or_else(|| missing(root, "WidgetryListSelectionState", "list_view.selection"))?;
    let changed = previous.selected.as_ref() != &selected;
    if !changed && previous.anchor == anchor {
        return Ok(false);
    }
    let selected = Arc::new(selected);
    let event = (notify && changed).then(|| WidgetryListSelectionChanged {
        source: root,
        added: selected.difference(&previous.selected).copied().collect(),
        removed: previous.selected.difference(&selected).copied().collect(),
        selected: selected.clone(),
    });
    world
        .entity_mut(root)
        .insert(WidgetryListSelectionState { selected, anchor });
    if let Some(event) = event {
        world.trigger(event);
    }
    Ok(changed)
}

fn disabled_rows<K: WidgetryListKey>(world: &World, root: Entity) -> HashSet<K> {
    world
        .get::<crate::virtualization::ListRuntime>(root)
        .into_iter()
        .flat_map(|runtime| runtime.rows.iter())
        .filter(|row| world.get::<bevy::ui::InteractionDisabled>(**row).is_some())
        .filter_map(|row| {
            world
                .get::<crate::WidgetryListViewItem<K>>(*row)
                .map(|item| item.id)
        })
        .collect()
}

pub(crate) fn input_select<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    id: K,
    modifiers: SelectionModifiers,
) -> Result<bool, ListFailure> {
    let source = validate::<T, K>(world, root)?;
    let view = world
        .get::<WidgetryListView<T, K>>(root)
        .ok_or_else(|| invalid_view::<T, K>(root, "list_view.input_select"))?;
    let state = world
        .get::<WidgetryListSelectionState<K>>(root)
        .ok_or_else(|| missing(root, "WidgetryListSelectionState", "list_view.selection"))?;
    if view.selection_mode() == WidgetryListSelectionMode::Single {
        return commit(world, root, HashSet::from([id]), Some(id), true);
    }
    if modifiers.shift {
        let projection = world
            .get::<WidgetryListProjection<K>>(root)
            .ok_or_else(|| missing(root, "WidgetryListProjection", "list_view.selection"))?;
        let target = projection.index_of(id).ok_or_else(|| {
            invalid_key(
                root,
                source,
                id,
                ListKeyRejection::MissingInProjection,
                "list_view.input_select",
            )
        })?;
        let anchor = state
            .anchor
            .filter(|anchor| projection.index_of(*anchor).is_some())
            .unwrap_or(id);
        let start = projection.index_of(anchor).unwrap_or(target);
        let disabled = disabled_rows::<K>(world, root);
        let adapter = world.resource::<DataSource<T, K>>();
        let selected = projection.ordered_ids()[start.min(target)..=start.max(target)]
            .iter()
            .copied()
            .filter(|id| {
                !disabled.contains(id)
                    && (adapter.row)(world, source, *id).is_some_and(|row| !row.disabled)
            })
            .collect();
        commit(world, root, selected, Some(anchor), true)
    } else if modifiers.control {
        let mut selected = state.selected_ids().clone();
        if !selected.remove(&id) {
            selected.insert(id);
        }
        commit(world, root, selected, Some(id), true)
    } else {
        commit(world, root, HashSet::from([id]), Some(id), true)
    }
}

pub(crate) fn input_select_all<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
) -> Result<bool, ListFailure> {
    let viewport = crate::behavior::viewport(world, root);
    if world.get::<bevy::ui::InteractionDisabled>(root).is_some()
        || viewport.is_some_and(|viewport| {
            world
                .get::<bevy::ui::InteractionDisabled>(viewport)
                .is_some()
                || world.get::<Children>(viewport).is_some_and(|children| {
                    children.iter().any(|child| {
                        world
                            .get::<bevy_widgetry_scroll_area::WidgetryScrollAreaContent>(child)
                            .is_some()
                            && world.get::<bevy::ui::InteractionDisabled>(child).is_some()
                    })
                })
        })
    {
        return Ok(false);
    }
    select_all::<T, K>(world, root, true)
}

fn select_all<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    exclude_disabled_rows: bool,
) -> Result<bool, ListFailure> {
    let source = validate::<T, K>(world, root)?;
    if world
        .get::<WidgetryListView<T, K>>(root)
        .is_none_or(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
    {
        return Ok(false);
    }
    crate::projection::refresh_root::<T, K>(world, root);
    repair::<T, K>(world, root);
    let scope = world
        .get::<WidgetryListSelectAllScope>(root)
        .copied()
        .unwrap_or_default();
    let adapter = world.resource::<DataSource<T, K>>();
    let ids = match scope {
        WidgetryListSelectAllScope::Source => (adapter.ids)(world, source).ok_or_else(|| {
            invalid_source::<T, K>(
                root,
                source,
                ListSourceRejection::CannotEnumerate,
                "list_view.select_all",
            )
        })?,
        WidgetryListSelectAllScope::VisibleUnion => world
            .get::<WidgetryListProjection<K>>(root)
            .ok_or_else(|| missing(root, "WidgetryListProjection", "list_view.selection"))?
            .ordered_ids()
            .to_vec(),
    };
    let disabled = if exclude_disabled_rows {
        disabled_rows::<K>(world, root)
    } else {
        HashSet::new()
    };
    let mut selected = if scope == WidgetryListSelectAllScope::VisibleUnion {
        WidgetryListView::<T, K>::selection(world, root)?
            .selected_ids()
            .clone()
    } else {
        HashSet::new()
    };
    selected.extend(ids.into_iter().filter(|id| {
        !disabled.contains(id) && (adapter.row)(world, source, *id).is_some_and(|row| !row.disabled)
    }));
    commit(world, root, selected, None, true)
}

pub(crate) fn select<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    id: K,
    notify: bool,
) -> Result<bool, ListFailure> {
    let source = validate::<T, K>(world, root)?;
    if !(world.resource::<DataSource<T, K>>().contains)(world, source, id) {
        return Err(invalid_key(
            root,
            source,
            id,
            ListKeyRejection::MissingInSource,
            "list_view.select_id",
        ));
    }
    crate::projection::refresh_root::<T, K>(world, root);
    let display = world
        .get::<WidgetryListProjection<K>>(root)
        .and_then(|projection| projection.index_of(id));
    crate::behavior::set_active::<T, K>(world, root, display)?;
    commit(world, root, HashSet::from([id]), Some(id), notify)
}

pub fn set_selection_id_silent<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    id: Option<K>,
) -> Result<bool, ListFailure> {
    if let Some(id) = id {
        select::<T, K>(world, root, id, false)
    } else {
        validate::<T, K>(world, root)?;
        commit::<K>(world, root, HashSet::new(), None, false)
    }
}

pub(crate) fn repair<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
) {
    let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
        return;
    };
    let Some(adapter) = world.get_resource::<DataSource<T, K>>() else {
        return;
    };
    let source = view.source();
    if (adapter.changed_at)(world, source).is_none() {
        return;
    }
    let Some(state) = world.get::<WidgetryListSelectionState<K>>(root) else {
        return;
    };
    if state
        .selected
        .iter()
        .all(|id| (adapter.contains)(world, source, *id))
        && state
            .anchor
            .is_none_or(|id| (adapter.contains)(world, source, id))
    {
        return;
    }
    let selected = state
        .selected
        .iter()
        .copied()
        .filter(|id| (adapter.contains)(world, source, *id))
        .collect::<HashSet<_>>();
    let anchor = state
        .anchor
        .filter(|id| (adapter.contains)(world, source, *id));
    world.entity_mut(root).insert(WidgetryListSelectionState {
        selected: Arc::new(selected),
        anchor,
    });
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> WidgetryListView<T, K> {
    pub fn select_all_in_world(world: &mut World, root: Entity) -> Result<bool, ListFailure> {
        select_all::<T, K>(world, root, false)
    }

    pub fn select_all(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::select_all_in_world(world, root)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn replace_selection_in_world(
        world: &mut World,
        root: Entity,
        ids: impl IntoIterator<Item = K>,
    ) -> Result<bool, ListFailure> {
        let source = validate::<T, K>(world, root)?;
        let selected: HashSet<_> = ids.into_iter().collect();
        let adapter = world.resource::<DataSource<T, K>>();
        if let Some(id) = selected
            .iter()
            .find(|id| !(adapter.contains)(world, source, **id))
        {
            return Err(invalid_key(
                root,
                source,
                *id,
                ListKeyRejection::MissingInSource,
                "list_view.replace_selection",
            ));
        }
        if selected.len() > 1
            && world
                .get::<Self>(root)
                .is_some_and(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
        {
            return Err(failure(
                ListError::SelectionCardinality {
                    view: root,
                    source,
                    selected: selected.iter().copied().map(ListKeySnapshot::new).collect(),
                },
                "list_view.replace_selection",
            ));
        }
        commit(world, root, selected, None, true)
    }
    pub fn replace_selection(
        commands: &mut Commands,
        root: Entity,
        ids: impl IntoIterator<Item = K>,
    ) {
        let ids = ids.into_iter().collect::<Vec<_>>();
        commands.queue(move |world: &mut World| {
            Self::replace_selection_in_world(world, root, ids)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
    pub fn add_id_in_world(world: &mut World, root: Entity, id: K) -> Result<bool, ListFailure> {
        let source = validate::<T, K>(world, root)?;
        let adapter = world.resource::<DataSource<T, K>>();
        if !(adapter.contains)(world, source, id) {
            return Err(invalid_key(
                root,
                source,
                id,
                ListKeyRejection::MissingInSource,
                "list_view.select_id",
            ));
        }
        let mut ids = Self::selection(world, root)?.selected_ids().clone();
        ids.retain(|id| (adapter.contains)(world, source, *id));
        ids.insert(id);
        if ids.len() > 1
            && world
                .get::<Self>(root)
                .is_some_and(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
        {
            return Err(failure(
                ListError::SelectionCardinality {
                    view: root,
                    source,
                    selected: ids.iter().copied().map(ListKeySnapshot::new).collect(),
                },
                "list_view.add_id",
            ));
        }
        repair::<T, K>(world, root);
        Self::replace_selection_in_world(world, root, ids)
    }
    pub fn add_id(commands: &mut Commands, root: Entity, id: K) {
        commands.queue(move |world: &mut World| {
            Self::add_id_in_world(world, root, id)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
    pub fn selection(
        world: &World,
        root: Entity,
    ) -> Result<&WidgetryListSelectionState<K>, ListFailure> {
        world
            .get::<Self>(root)
            .ok_or_else(|| invalid_view::<T, K>(root, "list_view.selection"))?;
        world
            .get::<WidgetryListSelectionState<K>>(root)
            .ok_or_else(|| missing(root, "WidgetryListSelectionState", "list_view.selection"))
    }
    pub fn select_id_in_world(world: &mut World, root: Entity, id: K) -> Result<bool, ListFailure> {
        select::<T, K>(world, root, id, true)
    }
    pub fn select_id(commands: &mut Commands, root: Entity, id: K) {
        commands.queue(move |world: &mut World| {
            Self::select_id_in_world(world, root, id)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
    pub fn deselect_id_in_world(
        world: &mut World,
        root: Entity,
        id: K,
    ) -> Result<bool, ListFailure> {
        let source = validate::<T, K>(world, root)?;
        if !(world.resource::<DataSource<T, K>>().contains)(world, source, id) {
            return Err(invalid_key(
                root,
                source,
                id,
                ListKeyRejection::MissingInSource,
                "list_view.select_id",
            ));
        }
        let state = Self::selection(world, root)?;
        let mut selected = state.selected_ids().clone();
        selected.remove(&id);
        let anchor = state.anchor.filter(|anchor| *anchor != id);
        commit(world, root, selected, anchor, true)
    }
    pub fn deselect_id(commands: &mut Commands, root: Entity, id: K) {
        commands.queue(move |world: &mut World| {
            Self::deselect_id_in_world(world, root, id)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
    pub fn clear_selection_in_world(world: &mut World, root: Entity) -> Result<bool, ListFailure> {
        validate::<T, K>(world, root)?;
        commit::<K>(world, root, HashSet::new(), None, true)
    }
    pub fn select_display_index_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, ListFailure> {
        let source = validate::<T, K>(world, root)?;
        crate::projection::refresh_root::<T, K>(world, root);
        let projection = world
            .get::<WidgetryListProjection<K>>(root)
            .ok_or_else(|| {
                missing(
                    root,
                    "WidgetryListProjection",
                    "list_view.select_display_index",
                )
            })?;
        let id = projection.id(index).ok_or_else(|| {
            failure(
                ListError::InvalidIndex {
                    view: root,
                    source,
                    index,
                    len: projection.len(),
                    space: ListIndexSpace::Display,
                },
                "list_view.select_display_index",
            )
        })?;
        Self::select_id_in_world(world, root, id)
    }
    pub fn select_display_index(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::select_display_index_in_world(world, root, index)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
}

impl<T: Send + Sync + 'static> WidgetryListView<T> {
    pub fn select_index_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, ListFailure> {
        let source = validate::<T, WidgetryListItemId>(world, root)?;
        let model = world.get::<WidgetryListModel<T>>(source).ok_or_else(|| {
            invalid_source::<T, WidgetryListItemId>(
                root,
                source,
                ListSourceRejection::MissingModel,
                "list_view.select_index",
            )
        })?;
        let id = model.id(index).ok_or_else(|| {
            failure(
                ListError::InvalidIndex {
                    view: root,
                    source,
                    index,
                    len: model.len(),
                    space: ListIndexSpace::Model,
                },
                "list_view.select_index",
            )
        })?;
        Self::select_id_in_world(world, root, id)
    }
    pub fn select_index(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::select_index_in_world(world, root, index)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }
}
