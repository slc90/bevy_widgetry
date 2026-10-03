use crate::virtualization::{ListRuntime, visible_range};
use crate::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewItem,
    WidgetryListViewState,
};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release, Scroll};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition, Selected};
use bevy::ui_widgets::{ActiveDescendant, ScrollArea, ValueChange};
use bevy_widgetry_log::widgetry_error;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;

#[derive(Component, Clone, Copy, Default)]
pub(crate) struct ListNavigation {
    selected_index: Option<usize>,
    active_index: Option<usize>,
    reveal: Option<WidgetryListItemId>,
}

// virtualization 会把同一 physical row 的 index 复用给新 entry。
// press 同时记录 pointer 与 stable id，避免 release 激活已替换的数据。
#[derive(Component)]
struct PressedEntry {
    pointer: PointerId,
    id: WidgetryListItemId,
}

fn resolve<T: Send + Sync + 'static>(
    model: &WidgetryListModel<T>,
    id: Option<WidgetryListItemId>,
    cached: Option<usize>,
) -> Option<usize> {
    let id = id?;
    cached
        .filter(|index| model.id(*index) == Some(id))
        .or_else(|| model.index_of(id))
}

fn viewport(world: &World, root: Entity) -> Option<Entity> {
    world
        .get::<ListRuntime>(root)
        .map(|runtime| runtime.viewport)
        .or_else(|| {
            world
                .get::<Children>(root)?
                .iter()
                .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
        })
}

fn repair<T: Send + Sync + 'static>(
    model: &WidgetryListModel<T>,
    state: &mut WidgetryListViewState,
    navigation: &mut ListNavigation,
) {
    navigation.selected_index = resolve(model, state.selected, navigation.selected_index);
    state.selected = navigation.selected_index.and_then(|index| model.id(index));
    let active = resolve(model, state.active, navigation.active_index);
    navigation.active_index = if state.active.is_some() && active.is_none() && !model.is_empty() {
        navigation
            .active_index
            .map(|index| index.min(model.len() - 1))
    } else {
        active
    };
    state.active = navigation.active_index.and_then(|index| model.id(index));
}

fn reveal_offset(len: usize, height: f32, viewport: f32, offset: f32, index: usize) -> f32 {
    let (offset, _) = visible_range(len, height, viewport, offset);
    let top = index as f32 * height;
    if top >= offset && top + height <= offset + viewport {
        offset
    } else {
        visible_range(len, height, viewport, top).0
    }
}

fn reveal<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    navigation: &mut ListNavigation,
) {
    let Some(id) = navigation.reveal else {
        return;
    };
    let Some(view) = world.get::<WidgetryListView<T>>(root) else {
        return;
    };
    let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
        return;
    };
    let Some(index) = resolve(model, Some(id), navigation.active_index) else {
        navigation.reveal = None;
        return;
    };
    let Some(viewport) = viewport(world, root) else {
        return;
    };
    let Some(computed) = world.get::<ComputedNode>(viewport) else {
        return;
    };
    let size = computed.size().y * computed.inverse_scale_factor();
    if !size.is_finite() || size <= 0.0 {
        return;
    }
    let Some(scroll) = world.get::<ScrollPosition>(viewport) else {
        return;
    };
    let offset = reveal_offset(model.len(), view.item_height(), size, scroll.0.y, index);
    if scroll.0.y != offset
        && let Some(mut scroll) = world.get_mut::<ScrollPosition>(viewport)
    {
        scroll.0.y = offset;
    }
    navigation.reveal = None;
}

fn update_error(root: Entity, reason: &str) -> BevyError {
    widgetry_error!(?root, reason, "ListView state 更新失败");
    BevyError::error(format!("ListView state 更新失败: {reason}"))
}

fn update_source<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
) -> Result<Entity, BevyError> {
    let view = world
        .get::<WidgetryListView<T>>(root)
        .ok_or_else(|| update_error(root, "Widget 不存在或 type 不匹配"))?;
    if world.get::<WidgetryListModel<T>>(view.source()).is_none() {
        return Err(update_error(root, "source 不存在或 Model type 不匹配"));
    }
    if world.get::<WidgetryListViewState>(root).is_none() {
        return Err(update_error(root, "必需 state 缺失"));
    }
    Ok(view.source())
}

pub(crate) fn set_selected<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    index: usize,
) -> Result<(), BevyError> {
    let source = update_source::<T>(world, root)?;
    let id = world
        .get::<WidgetryListModel<T>>(source)
        .and_then(|model| model.id(index))
        .ok_or_else(|| update_error(root, &format!("index {index} 越界")))?;
    let changed = world
        .get::<WidgetryListViewState>(root)
        .is_some_and(|state| state.selected != Some(id));
    let mut navigation = ListNavigation {
        selected_index: Some(index),
        active_index: Some(index),
        reveal: Some(id),
    };
    world.entity_mut(root).insert(WidgetryListViewState {
        selected: Some(id),
        active: Some(id),
    });
    reveal::<T>(world, root, &mut navigation);
    world.entity_mut(root).insert(navigation);
    if changed {
        world.trigger(ValueChange {
            source: root,
            value: Some(id),
            is_final: true,
        });
    }
    Ok(())
}

pub(crate) fn clear_selection<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    update_source::<T>(world, root)?;
    let mut state = world
        .get::<WidgetryListViewState>(root)
        .copied()
        .ok_or_else(|| update_error(root, "必需 state 缺失"))?;
    if state.selected.is_some() {
        state.selected = None;
        world.entity_mut(root).insert(state);
        if let Some(mut navigation) = world.get_mut::<ListNavigation>(root) {
            navigation.selected_index = None;
        }
        world.trigger(ValueChange::<Option<WidgetryListItemId>> {
            source: root,
            value: None,
            is_final: true,
        });
    }
    Ok(())
}

pub(crate) fn set_active<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    index: Option<usize>,
) -> Result<(), BevyError> {
    let source = update_source::<T>(world, root)?;
    let id = match index {
        Some(index) => Some(
            world
                .get::<WidgetryListModel<T>>(source)
                .and_then(|model| model.id(index))
                .ok_or_else(|| update_error(root, &format!("index {index} 越界")))?,
        ),
        None => None,
    };
    let mut state = world
        .get::<WidgetryListViewState>(root)
        .copied()
        .ok_or_else(|| update_error(root, "必需 state 缺失"))?;
    state.active = id;
    let mut navigation = world
        .get::<ListNavigation>(root)
        .copied()
        .unwrap_or_default();
    navigation.active_index = index;
    navigation.reveal = id;
    world.entity_mut(root).insert(state);
    reveal::<T>(world, root, &mut navigation);
    world.entity_mut(root).insert(navigation);
    Ok(())
}

fn navigate(len: usize, active: Option<usize>, key: KeyCode) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match key {
        KeyCode::Home => Some(0),
        KeyCode::End => Some(len - 1),
        KeyCode::ArrowDown => Some(active.map_or(0, |index| (index + 1) % len)),
        KeyCode::ArrowUp => {
            Some(active.map_or(
                len - 1,
                |index| if index == 0 { len - 1 } else { index - 1 },
            ))
        }
        _ => active,
    }
}

fn scroll_page(world: &mut World, root: Entity, len: usize, height: f32, down: bool) {
    let Some(viewport) = viewport(world, root) else {
        return;
    };
    let Some(computed) = world.get::<ComputedNode>(viewport) else {
        return;
    };
    let size = computed.size().y * computed.inverse_scale_factor();
    if !size.is_finite() || size <= 0.0 {
        return;
    }
    if let Some(mut scroll) = world.get_mut::<ScrollPosition>(viewport) {
        let delta = if down { size } else { -size };
        scroll.0.y = visible_range(len, height, size, scroll.0.y + delta).0;
    }
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<WidgetryListView<T>>>,
    focus: Option<Res<InputFocus>>,
    mut commands: Commands,
) {
    let root = event.focused_entity;
    let key = event.input.key_code;
    if !views.contains(root)
        || focus.is_none_or(|focus| focus.get() != Some(root))
        || event.input.state != ButtonState::Pressed
        || !matches!(
            key,
            KeyCode::ArrowDown
                | KeyCode::ArrowUp
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Space
                | KeyCode::Enter
                | KeyCode::PageDown
                | KeyCode::PageUp
        )
    {
        return;
    }
    event.propagate(false);
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some()
            || world
                .get_resource::<InputFocus>()
                .is_none_or(|focus| focus.get() != Some(root))
        {
            return;
        }
        let Some(view) = world.get::<WidgetryListView<T>>(root) else {
            return;
        };
        let source = view.source();
        let height = view.item_height();
        let Some(model) = world.get::<WidgetryListModel<T>>(source) else {
            return;
        };
        let Some(mut state) = world.get::<WidgetryListViewState>(root).copied() else {
            return;
        };
        let mut navigation = world
            .get::<ListNavigation>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        let mut notification = None;
        match key {
            KeyCode::PageDown | KeyCode::PageUp => {
                let len = model.len();
                scroll_page(world, root, len, height, key == KeyCode::PageDown);
                navigation.reveal = None;
            }
            KeyCode::Space | KeyCode::Enter => {
                if let Some(index) = navigation.active_index
                    && model.is_disabled(index) == Some(false)
                    && state.selected != state.active
                {
                    state.selected = state.active;
                    navigation.selected_index = Some(index);
                    notification = state.selected;
                }
            }
            _ => {
                navigation.active_index = navigate(model.len(), navigation.active_index, key);
                state.active = navigation.active_index.and_then(|index| model.id(index));
                navigation.reveal = state.active;
            }
        }
        world.entity_mut(root).insert(state);
        reveal::<T>(world, root, &mut navigation);
        world.entity_mut(root).insert(navigation);
        if let Some(mut visible) = world.get_resource_mut::<InputFocusVisible>() {
            visible.0 = true;
        }
        if let Some(value) = notification {
            world.trigger(ValueChange {
                source: root,
                value: Some(value),
                is_final: true,
            });
        }
    });
}

pub(crate) fn sync_state<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let Some(view) = world.get::<WidgetryListView<T>>(root) else {
            continue;
        };
        let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
            continue;
        };
        let Some(mut state) = world.get::<WidgetryListViewState>(root).copied() else {
            continue;
        };
        let mut navigation = world
            .get::<ListNavigation>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        if world.get::<WidgetryListViewState>(root) != Some(&state) {
            world.entity_mut(root).insert(state);
        }
        reveal::<T>(world, root, &mut navigation);
        world.entity_mut(root).insert(navigation);
    }
}

pub(crate) fn project<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        project_root::<T>(world, root);
    }
}

pub(crate) fn on_scroll<T: Send + Sync + 'static>(
    mut event: On<Pointer<Scroll>>,
    views: Query<(), (With<WidgetryListView<T>>, With<InteractionDisabled>)>,
) {
    if views.contains(event.entity) {
        event.propagate(false);
    }
}

fn project_root<T: Send + Sync + 'static>(world: &mut World, root: Entity) {
    let Some(state) = world.get::<WidgetryListViewState>(root).copied() else {
        return;
    };
    let Some(view) = world.get::<WidgetryListView<T>>(root) else {
        return;
    };
    let source = view.source();
    let root_disabled = world.get::<InteractionDisabled>(root).is_some();
    if let Some(viewport) = viewport(world, root) {
        // 官方 ScrollArea marker 仅启用 wheel/trackpad，未检查 InteractionDisabled。
        // 关闭该用户入口，保留原生 ScrollPosition、layout 与 programmatic reveal。
        if root_disabled {
            if world.get::<ScrollArea>(viewport).is_some() {
                world.entity_mut(viewport).remove::<ScrollArea>();
            }
            if world.get::<InteractionDisabled>(viewport).is_none() {
                world.entity_mut(viewport).insert(InteractionDisabled);
            }
        } else {
            if world.get::<ScrollArea>(viewport).is_none() {
                world.entity_mut(viewport).insert(ScrollArea);
            }
            if world.get::<InteractionDisabled>(viewport).is_some() {
                world.entity_mut(viewport).remove::<InteractionDisabled>();
            }
        }
    }
    let Some(runtime) = world.get::<ListRuntime>(root).cloned() else {
        return;
    };
    let focused = world
        .get_resource::<InputFocus>()
        .is_some_and(|focus| focus.get() == Some(root));
    let mut active = None;
    for row in runtime.rows {
        let Some(item) = world.get::<WidgetryListViewItem>(row).copied() else {
            continue;
        };
        let selected = state.selected == Some(item.id);
        if selected != world.get::<Selected>(row).is_some() {
            if selected {
                world.entity_mut(row).insert(Selected);
            } else {
                world.entity_mut(row).remove::<Selected>();
            }
        }
        if focused && state.active == Some(item.id) {
            active = Some(row);
        }
        let disabled = root_disabled
            || world
                .get::<WidgetryListModel<T>>(source)
                .and_then(|model| model.is_disabled(item.index))
                .unwrap_or(false);
        if disabled != world.get::<InteractionDisabled>(row).is_some() {
            if disabled {
                world.entity_mut(row).insert(InteractionDisabled);
            } else {
                world.entity_mut(row).remove::<InteractionDisabled>();
            }
        }
        if disabled
            || world
                .get::<PressedEntry>(row)
                .is_some_and(|pressed| pressed.id != item.id)
        {
            world.entity_mut(row).remove::<(Pressed, PressedEntry)>();
        }
    }
    if world
        .get::<ActiveDescendant>(root)
        .is_none_or(|previous| previous.0 != active)
    {
        world.entity_mut(root).insert(ActiveDescendant(active));
    }
}

pub(crate) fn on_disabled_added<T: Send + Sync + 'static>(
    event: On<Add, InteractionDisabled>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if views.contains(event.entity) {
        let root = event.entity;
        commands.queue(move |world: &mut World| project_root::<T>(world, root));
    }
}

pub(crate) fn on_disabled_removed<T: Send + Sync + 'static>(
    event: On<Remove, InteractionDisabled>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if views.contains(event.entity) {
        let root = event.entity;
        commands.queue(move |world: &mut World| project_root::<T>(world, root));
    }
}

pub(crate) fn on_press<T: Send + Sync + 'static>(
    mut event: On<Pointer<Press>>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer_id;
    event.propagate(false);
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some() {
            return;
        }
        let Some(row) = clicked_row(world, root, target) else {
            return;
        };
        let Some(item) = world.get::<WidgetryListViewItem>(row).copied() else {
            return;
        };
        let Some(view) = world.get::<WidgetryListView<T>>(root) else {
            return;
        };
        let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
            return;
        };
        let Some(index) = resolve(model, Some(item.id), Some(item.index)) else {
            return;
        };
        if model.is_disabled(index) == Some(false) {
            world.entity_mut(row).insert((
                Pressed,
                PressedEntry {
                    pointer,
                    id: item.id,
                },
            ));
        }
    });
}

fn clear_pressed(world: &mut World, root: Entity, pointer: PointerId) {
    let Some(runtime) = world.get::<ListRuntime>(root).cloned() else {
        return;
    };
    for row in runtime.rows {
        if world
            .get::<PressedEntry>(row)
            .is_none_or(|pressed| pressed.pointer == pointer)
        {
            world.entity_mut(row).remove::<(Pressed, PressedEntry)>();
        }
    }
}

fn clear_pointer_presses<T: Send + Sync + 'static>(world: &mut World, pointer: PointerId) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        clear_pressed(world, root, pointer);
    }
}

// pointer 离开全部 hovered entity 时 Bevy 不派发目标 Cancel/Release。
// 同时消费原始 pointer input，避免旧 row 的 Pressed 永久残留。
pub(crate) fn clear_ended_presses<T: Send + Sync + 'static>(
    mut input: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let mut ended = Vec::new();
    for input in input.read() {
        match input.action {
            PointerAction::Cancel | PointerAction::Release(PointerButton::Primary) => {
                if !ended.contains(&input.pointer_id) {
                    ended.push(input.pointer_id);
                }
            }
            PointerAction::Press(PointerButton::Primary) => {
                ended.retain(|pointer| *pointer != input.pointer_id)
            }
            _ => {}
        }
    }
    for pointer in ended {
        commands.queue(move |world: &mut World| clear_pointer_presses::<T>(world, pointer));
    }
}

pub(crate) fn on_release<T: Send + Sync + 'static>(
    mut event: On<Pointer<Release>>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if event.button == PointerButton::Primary {
        let pointer = event.pointer_id;
        if event.entity == event.original_event_target() {
            commands.queue(move |world: &mut World| clear_pointer_presses::<T>(world, pointer));
        }
        if views.contains(event.entity) {
            event.propagate(false);
        }
    }
}

pub(crate) fn on_cancel<T: Send + Sync + 'static>(
    mut event: On<Pointer<Cancel>>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if event.entity == event.original_event_target() {
        let pointer = event.pointer_id;
        commands.queue(move |world: &mut World| clear_pointer_presses::<T>(world, pointer));
    }
    if views.contains(event.entity) {
        event.propagate(false);
    }
}

pub(crate) fn on_drag_end<T: Send + Sync + 'static>(
    mut event: On<Pointer<DragEnd>>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if event.button == PointerButton::Primary && views.contains(event.entity) {
        let root = event.entity;
        let pointer = event.pointer_id;
        event.propagate(false);
        commands.queue(move |world: &mut World| clear_pressed(world, root, pointer));
    }
}

fn clicked_row(world: &World, root: Entity, target: Entity) -> Option<Entity> {
    let mut entity = target;
    let mut row = None;
    loop {
        if entity == root {
            return row;
        }
        if world.get::<WidgetryListViewState>(entity).is_some() {
            return None;
        }
        if row.is_none() && world.get::<WidgetryListViewItem>(entity).is_some() {
            row = Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

pub(crate) fn on_click<T: Send + Sync + 'static>(
    mut event: On<Pointer<Click>>,
    views: Query<(), With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    event.propagate(false);
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some() {
            return;
        }
        let Some(row) = clicked_row(world, root, target) else {
            return;
        };
        let Some(item) = world.get::<WidgetryListViewItem>(row).copied() else {
            return;
        };
        let Some(view) = world.get::<WidgetryListView<T>>(root) else {
            return;
        };
        let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
            return;
        };
        let Some(index) = resolve(model, Some(item.id), Some(item.index)) else {
            return;
        };
        let disabled = model.is_disabled(index) == Some(true);
        let Some(mut state) = world.get::<WidgetryListViewState>(root).copied() else {
            return;
        };
        let mut navigation = world
            .get::<ListNavigation>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        let changed = !disabled && state.selected != Some(item.id);
        state.active = Some(item.id);
        navigation.active_index = Some(index);
        navigation.reveal = None;
        if !disabled {
            state.selected = Some(item.id);
            navigation.selected_index = Some(index);
        }
        world.entity_mut(root).insert((state, navigation));
        if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
            focus.set(root, FocusCause::Pressed);
        }
        if let Some(mut visible) = world.get_resource_mut::<InputFocusVisible>() {
            visible.0 = false;
        }
        if changed {
            world.trigger(ValueChange {
                source: root,
                value: Some(item.id),
                is_final: true,
            });
        }
    });
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn navigation_starts_and_wraps() {
        assert_eq!(navigate(0, None, KeyCode::ArrowDown), None);
        for key in [KeyCode::ArrowDown, KeyCode::Home] {
            assert_eq!(navigate(3, None, key), Some(0));
        }
        for key in [KeyCode::ArrowUp, KeyCode::End] {
            assert_eq!(navigate(3, None, key), Some(2));
        }
        assert_eq!(navigate(3, Some(0), KeyCode::ArrowUp), Some(2));
        assert_eq!(navigate(3, Some(2), KeyCode::ArrowDown), Some(0));
    }

    #[test]
    fn reveal_math_top_aligns_and_clamps() {
        assert_eq!(reveal_offset(10, 10.0, 30.0, 10.0, 2), 10.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 1.0, 0), 0.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 1.0, 3), 30.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 0.0, 9), 70.0);
        assert_eq!(reveal_offset(10, 10.0, 5.0, 0.0, 2), 20.0);
    }

    #[test]
    fn cache_repair_follows_identity_and_delete_position() {
        let mut model = WidgetryListModel::default();
        let first = model.push(0_u32).unwrap();
        let selected = model.push(1).unwrap();
        let successor = model.push(2).unwrap();
        let mut state = WidgetryListViewState {
            selected: Some(selected),
            active: Some(selected),
        };
        let mut navigation = ListNavigation {
            selected_index: Some(1),
            active_index: Some(1),
            reveal: None,
        };
        model.move_item(1, 0);
        repair(&model, &mut state, &mut navigation);
        assert_eq!(navigation.active_index, Some(0));
        assert_eq!(state.selected, Some(selected));
        model.remove(0);
        repair(&model, &mut state, &mut navigation);
        assert_eq!(
            state,
            WidgetryListViewState {
                selected: None,
                active: Some(first)
            }
        );
        model.remove(0);
        repair(&model, &mut state, &mut navigation);
        assert_eq!(state.active, Some(successor));
        model.clear();
        repair(&model, &mut state, &mut navigation);
        assert_eq!(state, WidgetryListViewState::default());
    }
}
