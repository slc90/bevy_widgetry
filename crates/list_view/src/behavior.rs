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
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;

/// index 只是 stable id 的快路径；reveal 保留尚未取得 viewport layout 的导航请求。
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct ListNavigation {
    selected_index: Option<usize>,
    active_index: Option<usize>,
    reveal: Option<WidgetryListItemId>,
}

/// 为现成 Pressed 记录 press 的 pointer 和 entry id，防止 index 复用把旧 press 带给新 entry。
#[derive(Component)]
struct PressedEntry {
    pointer: PointerId,
    id: WidgetryListItemId,
}

/// 先验证 cached index，只有结构变化使其失效时才做 linear lookup。
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

/// 同一 BSN shell 的 viewport；programmatic API 在首次 runtime bootstrap 前也可使用。
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

/// 删除时 selection 清空，active 独立选择旧位置上的 successor 或末尾 predecessor。
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

/// 完全可见时保持 offset，部分或完全不可见时 top-align 并 clamp。
fn reveal_offset(len: usize, height: f32, viewport: f32, offset: f32, index: usize) -> f32 {
    let (offset, _) = visible_range(len, height, viewport, offset);
    let top = index as f32 * height;
    if top >= offset && top + height <= offset + viewport {
        offset
    } else {
        visible_range(len, height, viewport, top).0
    }
}

/// 等到真实 viewport 尺寸有效才完成 reveal；不需要目标 row 存在。
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

/// programmatic 路径不读取 disabled，也不发送用户通知；首次有效 layout 前保留 reveal。
pub(crate) fn set_selected<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    index: usize,
) {
    let Some(view) = world.get::<WidgetryListView<T>>(root) else {
        return;
    };
    let Some(id) = world
        .get::<WidgetryListModel<T>>(view.source())
        .and_then(|model| model.id(index))
    else {
        return;
    };
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
}

/// active 未初始化时按方向选首尾，disabled item 不参与导航过滤。
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

/// Page 只改变合法 scroll offset；invalid viewport 保持当前位置。
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

/// root focused-input observer 只消费本列表支持的按键，业务 descendant 自行处理其输入。
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
                value,
                is_final: true,
            });
        }
    });
}

/// 同步 logical state 与 index 快路径，然后在 virtualization 前处理 pending reveal。
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

/// 只把 logical state 投影到现存 rows；offscreen 或失去 root focus 不清 logical active。
pub(crate) fn project<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        project_root::<T>(world, root);
    }
}

/// disabled viewport 关闭官方入口后仍消费 wheel，避免穿透滚动外层 ScrollArea。
pub(crate) fn on_scroll<T: Send + Sync + 'static>(
    mut event: On<Pointer<Scroll>>,
    views: Query<(), (With<WidgetryListView<T>>, With<InteractionDisabled>)>,
) {
    if views.contains(event.entity) {
        event.propagate(false);
    }
}

/// projection 不向 model 写回 disabled metadata。
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

/// root disable 生命周期立即投影用户输入面，不等待下一次 app update 才拦截 wheel。
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

/// 恢复 root 时重新按 entry metadata 投影，并恢复官方 wheel 入口。
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

/// primary press 只给当前 enabled entry 挂官方 Pressed，不修改 logical selection。
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

/// 同 pointer 的 release/cancel/drag end 清掉当前 rows 的 Pressed，即使结束目标发生变化。
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

/// Cancel/Release 的当前 hovered target 可以位于任意 UI，按 pointer 清理原来的列表 press。
fn clear_pointer_presses<T: Send + Sync + 'static>(world: &mut World, pointer: PointerId) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        clear_pressed(world, root, pointer);
    }
}

/// 没有 hovered entity 时 Bevy 不派发 Cancel/Release；原始 pointer 输入仍结束该 gesture。
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

/// 非 primary release 不能结束尚未释放的 primary press。
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

/// pointer cancel 无 button，但只清理其所属 pointer 的 press。
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

/// drag end 即使没有后续 click 也结束 primary pressed lifecycle。
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

/// descendant 向上解析最近 row；遇到嵌套的另一 ListView 时不把其 item 当成本列表。
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

/// 用户 click 不要求 renderer direct child 可识别，只有 root 接管最终 selection 语义。
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
                value: item.id,
                is_final: true,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空 active 的起点与首尾 wrap 不依赖物理 row 或 disabled projection。
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

    /// reveal 完全可见不移动，部分露出 top-align，末尾与 oversized row 保持合法 offset。
    #[test]
    fn reveal_math_top_aligns_and_clamps() {
        assert_eq!(reveal_offset(10, 10.0, 30.0, 10.0, 2), 10.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 1.0, 0), 0.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 1.0, 3), 30.0);
        assert_eq!(reveal_offset(10, 10.0, 30.0, 0.0, 9), 70.0);
        assert_eq!(reveal_offset(10, 10.0, 5.0, 0.0, 2), 20.0);
    }

    /// cached index 失效才按 id 修复；active 删除的 successor 与 selection 清空分别应用。
    #[test]
    fn cache_repair_follows_identity_and_delete_position() {
        let mut model = WidgetryListModel::default();
        let first = model.push(0_u32);
        let selected = model.push(1);
        let successor = model.push(2);
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
