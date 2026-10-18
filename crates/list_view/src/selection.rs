use crate::source::DataSource;
use crate::{
    WidgetryListItemId, WidgetryListKey, WidgetryListModel, WidgetryListProjection,
    WidgetryListView, WidgetryListViewState,
};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::{collections::HashSet, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryListSelectionMode {
    Single,
    #[default]
    Multiple,
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

pub(crate) fn update_error(root: Entity, reason: &str) -> BevyError {
    widgetry_error!(?root, reason, "ListView state 更新失败");
    BevyError::error(format!("ListView state 更新失败: {reason}"))
}

pub(crate) fn validate<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &World,
    root: Entity,
) -> Result<Entity, BevyError> {
    let view = world
        .get::<WidgetryListView<T, K>>(root)
        .ok_or_else(|| update_error(root, "Widget 不存在或 type 不匹配"))?;
    if world
        .get_resource::<DataSource<T, K>>()
        .is_none_or(|adapter| (adapter.changed_at)(world, view.source()).is_none())
    {
        return Err(update_error(root, "source 不存在或数据源 type 不匹配"));
    }
    if world.get::<WidgetryListSelectionState<K>>(root).is_none()
        || world.get::<WidgetryListViewState<K>>(root).is_none()
    {
        return Err(update_error(root, "必需 state 缺失"));
    }
    Ok(view.source())
}

pub(crate) fn commit<K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    selected: HashSet<K>,
    anchor: Option<K>,
    notify: bool,
) -> Result<bool, BevyError> {
    let previous = world
        .get::<WidgetryListSelectionState<K>>(root)
        .ok_or_else(|| update_error(root, "必需 Selection state 缺失"))?;
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
) -> Result<bool, BevyError> {
    let source = validate::<T, K>(world, root)?;
    let view = world
        .get::<WidgetryListView<T, K>>(root)
        .ok_or_else(|| update_error(root, "Widget 缺失"))?;
    let state = world
        .get::<WidgetryListSelectionState<K>>(root)
        .ok_or_else(|| update_error(root, "Selection state 缺失"))?;
    if view.selection_mode() == WidgetryListSelectionMode::Single {
        return commit(world, root, HashSet::from([id]), Some(id), true);
    }
    if modifiers.shift {
        let projection = world
            .get::<WidgetryListProjection<K>>(root)
            .ok_or_else(|| update_error(root, "Projection 缺失"))?;
        let target = projection
            .index_of(id)
            .ok_or_else(|| update_error(root, "选择目标不在 Projection"))?;
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
) -> Result<bool, BevyError> {
    let source = validate::<T, K>(world, root)?;
    if world
        .get::<WidgetryListView<T, K>>(root)
        .is_none_or(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
    {
        return Ok(false);
    }
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
    let adapter = world.resource::<DataSource<T, K>>();
    let ids =
        (adapter.ids)(world, source).ok_or_else(|| update_error(root, "数据源无法枚举 ID"))?;
    let disabled = disabled_rows::<K>(world, root);
    let selected = ids
        .into_iter()
        .filter(|id| {
            !disabled.contains(id)
                && (adapter.row)(world, source, *id).is_some_and(|row| !row.disabled)
        })
        .collect();
    commit(world, root, selected, None, true)
}

pub(crate) fn select<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    id: K,
    notify: bool,
) -> Result<bool, BevyError> {
    let source = validate::<T, K>(world, root)?;
    if !(world.resource::<DataSource<T, K>>().contains)(world, source, id) {
        return Err(update_error(root, "Item ID 不存在或已失效"));
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
) -> Result<bool, BevyError> {
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
    pub fn replace_selection_in_world(
        world: &mut World,
        root: Entity,
        ids: impl IntoIterator<Item = K>,
    ) -> Result<bool, BevyError> {
        let source = validate::<T, K>(world, root)?;
        let selected: HashSet<_> = ids.into_iter().collect();
        let adapter = world.resource::<DataSource<T, K>>();
        if selected
            .iter()
            .any(|id| !(adapter.contains)(world, source, *id))
        {
            return Err(update_error(root, "选择集合包含失效 ID"));
        }
        if selected.len() > 1
            && world
                .get::<Self>(root)
                .is_some_and(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
        {
            return Err(update_error(root, "单选模式不能选择多个 ID"));
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
            Self::replace_selection_in_world(world, root, ids).map(|_| ())
        });
    }
    pub fn add_id_in_world(world: &mut World, root: Entity, id: K) -> Result<bool, BevyError> {
        let source = validate::<T, K>(world, root)?;
        let adapter = world.resource::<DataSource<T, K>>();
        if !(adapter.contains)(world, source, id) {
            return Err(update_error(root, "Item ID 不存在或已失效"));
        }
        let mut ids = Self::selection(world, root)?.selected_ids().clone();
        ids.retain(|id| (adapter.contains)(world, source, *id));
        ids.insert(id);
        if ids.len() > 1
            && world
                .get::<Self>(root)
                .is_some_and(|view| view.selection_mode() == WidgetryListSelectionMode::Single)
        {
            return Err(update_error(root, "单选模式不能选择多个 ID"));
        }
        repair::<T, K>(world, root);
        Self::replace_selection_in_world(world, root, ids)
    }
    pub fn add_id(commands: &mut Commands, root: Entity, id: K) {
        commands.queue(move |world: &mut World| Self::add_id_in_world(world, root, id).map(|_| ()));
    }
    pub fn selection(
        world: &World,
        root: Entity,
    ) -> Result<&WidgetryListSelectionState<K>, BevyError> {
        world
            .get::<Self>(root)
            .ok_or_else(|| update_error(root, "Widget 不存在或 type 不匹配"))?;
        world
            .get::<WidgetryListSelectionState<K>>(root)
            .ok_or_else(|| update_error(root, "必需 Selection state 缺失"))
    }
    pub fn select_id_in_world(world: &mut World, root: Entity, id: K) -> Result<bool, BevyError> {
        select::<T, K>(world, root, id, true)
    }
    pub fn select_id(commands: &mut Commands, root: Entity, id: K) {
        commands
            .queue(move |world: &mut World| Self::select_id_in_world(world, root, id).map(|_| ()));
    }
    pub fn deselect_id_in_world(world: &mut World, root: Entity, id: K) -> Result<bool, BevyError> {
        let source = validate::<T, K>(world, root)?;
        if !(world.resource::<DataSource<T, K>>().contains)(world, source, id) {
            return Err(update_error(root, "Item ID 不存在或已失效"));
        }
        let state = Self::selection(world, root)?;
        let mut selected = state.selected_ids().clone();
        selected.remove(&id);
        let anchor = state.anchor.filter(|anchor| *anchor != id);
        commit(world, root, selected, anchor, true)
    }
    pub fn deselect_id(commands: &mut Commands, root: Entity, id: K) {
        commands.queue(move |world: &mut World| {
            Self::deselect_id_in_world(world, root, id).map(|_| ())
        });
    }
    pub fn clear_selection_in_world(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        validate::<T, K>(world, root)?;
        commit::<K>(world, root, HashSet::new(), None, true)
    }
    pub fn select_display_index_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, BevyError> {
        validate::<T, K>(world, root)?;
        crate::projection::refresh_root::<T, K>(world, root);
        let id = world
            .get::<WidgetryListProjection<K>>(root)
            .and_then(|projection| projection.id(index))
            .ok_or_else(|| update_error(root, "显示 index 越界"))?;
        Self::select_id_in_world(world, root, id)
    }
    pub fn select_display_index(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::select_display_index_in_world(world, root, index).map(|_| ())
        });
    }
}

impl<T: Send + Sync + 'static> WidgetryListView<T> {
    pub fn select_index_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, BevyError> {
        let source = validate::<T, WidgetryListItemId>(world, root)?;
        let id = world
            .get::<WidgetryListModel<T>>(source)
            .and_then(|model| model.id(index))
            .ok_or_else(|| update_error(root, "原始 Model index 越界或缺少 Model"))?;
        Self::select_id_in_world(world, root, id)
    }
    pub fn select_index(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::select_index_in_world(world, root, index).map(|_| ())
        });
    }
}
