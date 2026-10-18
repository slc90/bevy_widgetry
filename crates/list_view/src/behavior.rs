use crate::WidgetryListSelectionState;
use crate::selection::{update_error, validate as update_source};
use crate::source::DataSource;
use crate::virtualization::{ListRuntime, visible_range};
use crate::{WidgetryListItemId, WidgetryListView, WidgetryListViewItem, WidgetryListViewState};
use crate::{WidgetryListKey, WidgetryListProjection};
use bevy::camera::NormalizedRenderTarget;
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDragEnd, PointerPress, PointerRelease, PointerScroll,
};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition, Selected};
use bevy::ui_widgets::ActiveDescendant;
use bevy_widgetry_core::disabled::{WidgetryEffectiveDisabled, set_intrinsic_disabled};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;

#[derive(Component, Clone, Copy)]
pub(crate) struct ListNavigation<K: WidgetryListKey = WidgetryListItemId> {
    active_index: Option<usize>,
    reveal: Option<K>,
}

impl<K: WidgetryListKey> Default for ListNavigation<K> {
    fn default() -> Self {
        Self {
            active_index: None,
            reveal: None,
        }
    }
}

// Virtualization reuses a physical row's index for new entries.
// Record both Pointer and stable ID on press so release cannot activate replacement data.
#[derive(Component)]
struct PressedEntry<K: WidgetryListKey> {
    pointer: PointerId,
    target: NormalizedRenderTarget,
    fresh: bool,
    id: K,
}

fn resolve<K: WidgetryListKey>(
    model: &WidgetryListProjection<K>,
    id: Option<K>,
    cached: Option<usize>,
) -> Option<usize> {
    let id = id?;
    cached
        .filter(|index| model.id(*index) == Some(id))
        .or_else(|| model.index_of(id))
}

fn current_projection<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &World,
    root: Entity,
) -> Option<&WidgetryListProjection<K>> {
    let view = world.get::<WidgetryListView<T, K>>(root)?;
    let adapter = world.get_resource::<DataSource<T, K>>()?;
    (adapter.changed_at)(world, view.source())?;
    world.get::<WidgetryListProjection<K>>(root)
}

pub(crate) fn viewport(world: &World, root: Entity) -> Option<Entity> {
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

fn repair<K: WidgetryListKey>(
    model: &WidgetryListProjection<K>,
    state: &mut WidgetryListViewState<K>,
    navigation: &mut ListNavigation<K>,
) {
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

#[derive(Component)]
pub struct NavigationFocusOwner(pub Entity);

pub(crate) fn is_focused(world: &World, root: Entity) -> bool {
    world
        .get_resource::<InputFocus>()
        .and_then(InputFocus::get)
        .is_some_and(|focus| {
            focus == root
                || world
                    .get::<NavigationFocusOwner>(root)
                    .is_some_and(|owner| owner.0 == focus)
        })
}

fn reveal<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    navigation: &mut ListNavigation<K>,
) {
    let Some(id) = navigation.reveal else {
        return;
    };
    let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
        return;
    };
    let Some(model) = current_projection::<T, K>(world, root) else {
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

pub fn set_active<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    index: Option<usize>,
) -> Result<(), BevyError> {
    update_source::<T, K>(world, root)?;
    crate::projection::refresh_root::<T, K>(world, root);
    let id = match index {
        Some(index) => Some(
            world
                .get::<WidgetryListProjection<K>>(root)
                .and_then(|model| model.id(index))
                .ok_or_else(|| update_error(root, &format!("index {index} 越界")))?,
        ),
        None => None,
    };
    let mut state = world
        .get::<WidgetryListViewState<K>>(root)
        .copied()
        .ok_or_else(|| update_error(root, "必需 state 缺失"))?;
    state.active = id;
    let mut navigation = world
        .get::<ListNavigation<K>>(root)
        .copied()
        .unwrap_or_default();
    navigation.active_index = index;
    navigation.reveal = id;
    world.entity_mut(root).insert(state);
    reveal::<T, K>(world, root, &mut navigation);
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

pub(crate) fn on_key<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    focus: Option<Res<InputFocus>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut commands: Commands,
) {
    let modifiers = crate::selection::SelectionModifiers::from_keys(keys.as_deref());
    let root = event.focused_entity;
    let key = event.input.key_code;
    if !views.contains(root)
        || focus.is_none_or(|focus| focus.get() != Some(root))
        || event.input.state != ButtonState::Pressed
        || (key == KeyCode::KeyA && !modifiers.control)
        || !matches!(
            key,
            KeyCode::KeyA
                | KeyCode::ArrowDown
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
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get::<InteractionDisabled>(root).is_some()
            || world
                .get_resource::<InputFocus>()
                .is_none_or(|focus| focus.get() != Some(root))
        {
            return Ok(());
        }
        crate::projection::refresh_root::<T, K>(world, root);
        crate::selection::repair::<T, K>(world, root);
        let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
            return Ok(());
        };
        let source = view.source();
        let height = view.item_height();
        let Some(model) = current_projection::<T, K>(world, root) else {
            return Ok(());
        };
        let Some(mut state) = world.get::<WidgetryListViewState<K>>(root).copied() else {
            return Ok(());
        };
        let mut navigation = world
            .get::<ListNavigation<K>>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        let mut notification = None;
        match key {
            KeyCode::KeyA => {
                world.entity_mut(root).insert((state, navigation));
                if let Some(mut visible) = world.get_resource_mut::<InputFocusVisible>() {
                    visible.0 = true;
                }
                crate::selection::input_select_all::<T, K>(world, root)?;
                return Ok(());
            }
            KeyCode::PageDown | KeyCode::PageUp => {
                let len = model.len();
                scroll_page(world, root, len, height, key == KeyCode::PageDown);
                navigation.reveal = None;
            }
            KeyCode::Space | KeyCode::Enter => {
                if let Some(index) = navigation.active_index
                    && model
                        .id(index)
                        .and_then(|id| {
                            (world.resource::<DataSource<T, K>>().row)(world, source, id)
                        })
                        .is_some_and(|row| !row.disabled)
                    && !viewport(world, root).is_some_and(|viewport| {
                        world.get::<InteractionDisabled>(viewport).is_some()
                    })
                    && !world.get::<ListRuntime>(root).is_some_and(|runtime| {
                        world.get::<InteractionDisabled>(runtime.content).is_some()
                            || runtime.rows.iter().any(|&row| {
                                world
                                    .get::<WidgetryListViewItem<K>>(row)
                                    .is_some_and(|item| Some(item.id) == state.active)
                                    && world.get::<InteractionDisabled>(row).is_some()
                            })
                    })
                {
                    notification = state.active;
                }
            }
            _ => {
                navigation.active_index = navigate(model.len(), navigation.active_index, key);
                state.active = navigation.active_index.and_then(|index| model.id(index));
                navigation.reveal = state.active;
            }
        }
        world.entity_mut(root).insert(state);
        reveal::<T, K>(world, root, &mut navigation);
        world.entity_mut(root).insert(navigation);
        if let Some(mut visible) = world.get_resource_mut::<InputFocusVisible>() {
            visible.0 = true;
        }
        if let Some(value) = notification {
            crate::selection::input_select::<T, K>(world, root, value, modifiers)?;
        }
        Ok(())
    });
}

pub(crate) fn sync_state<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        crate::projection::refresh_root::<T, K>(world, root);
        crate::selection::repair::<T, K>(world, root);
        let Some(model) = current_projection::<T, K>(world, root) else {
            continue;
        };
        let Some(mut state) = world.get::<WidgetryListViewState<K>>(root).copied() else {
            continue;
        };
        let mut navigation = world
            .get::<ListNavigation<K>>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        if world.get::<WidgetryListViewState<K>>(root) != Some(&state) {
            world.entity_mut(root).insert(state);
        }
        reveal::<T, K>(world, root, &mut navigation);
        world.entity_mut(root).insert(navigation);
    }
}

pub(crate) fn project<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        project_root::<T, K>(world, root);
    }
}

pub(crate) fn on_scroll<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerScroll>,
    views: Query<(), (With<WidgetryListView<T, K>>, With<InteractionDisabled>)>,
) {
    if views.contains(event.entity) {
        event.propagate(false);
    }
}

fn project_root<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World, root: Entity) {
    let Some(state) = world.get::<WidgetryListViewState<K>>(root).copied() else {
        return;
    };
    let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
        return;
    };
    let source = view.source();
    let Some(selection) = world.get::<WidgetryListSelectionState<K>>(root).cloned() else {
        return;
    };
    let root_disabled = world.get::<InteractionDisabled>(root).is_some();
    let Some(runtime) = world.get::<ListRuntime>(root).cloned() else {
        return;
    };
    let focused = is_focused(world, root);
    let mut active = None;
    for row in runtime.rows {
        let Some(item) = world.get::<WidgetryListViewItem<K>>(row).copied() else {
            continue;
        };
        let selected = selection.contains(item.id);
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
        let intrinsic = world
            .get_resource::<DataSource<T, K>>()
            .and_then(|adapter| (adapter.row)(world, source, item.id))
            .map(|row| row.disabled)
            .unwrap_or(false);
        set_intrinsic_disabled(world, row, intrinsic);
        let disabled = root_disabled || world.get::<InteractionDisabled>(row).is_some();
        if world
            .get::<PressedEntry<K>>(row)
            .is_some_and(|pressed| disabled || pressed.id != item.id)
        {
            world.entity_mut(row).remove::<(Pressed, PressedEntry<K>)>();
        }
    }
    if world
        .get::<ActiveDescendant>(root)
        .is_none_or(|previous| previous.0 != active)
    {
        world.entity_mut(root).insert(ActiveDescendant(active));
    }
}

pub(crate) fn install_disabled_press_cleanup<K: WidgetryListKey>(app: &mut App) {
    app.add_observer(cancel_disabled_press::<K>);
}

fn cancel_disabled_press<K: WidgetryListKey>(
    event: On<Insert<WidgetryEffectiveDisabled>>,
    owned: Query<&WidgetryEffectiveDisabled, With<PressedEntry<K>>>,
    mut commands: Commands,
) {
    if owned
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        let row = event.entity;
        commands.queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(row)
                && entity.contains::<PressedEntry<K>>()
            {
                entity.remove::<(Pressed, PressedEntry<K>)>();
            }
        });
    }
}

pub(crate) fn on_disabled_added<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<Add<InteractionDisabled>>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if views.contains(event.entity) {
        let root = event.entity;
        commands.queue(move |world: &mut World| project_root::<T, K>(world, root));
    }
}

pub(crate) fn on_disabled_removed<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<Remove<InteractionDisabled>>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if views.contains(event.entity) {
        let root = event.entity;
        commands.queue(move |world: &mut World| project_root::<T, K>(world, root));
    }
}

pub(crate) fn on_press<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerPress>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let context = event.pointer.target.clone();
    event.propagate(false);
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some() {
            return;
        }
        let Some(row) = clicked_row::<K>(world, root, target) else {
            return;
        };
        let Some(item) = world.get::<WidgetryListViewItem<K>>(row).copied() else {
            return;
        };
        crate::projection::refresh_root::<T, K>(world, root);
        crate::selection::repair::<T, K>(world, root);
        let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
            return;
        };
        let Some(model) = current_projection::<T, K>(world, root) else {
            return;
        };
        let Some(_) = resolve(model, Some(item.id), Some(item.index)) else {
            return;
        };
        if (world.resource::<DataSource<T, K>>().row)(world, view.source(), item.id)
            .is_some_and(|row| !row.disabled)
            && world.get::<InteractionDisabled>(row).is_none()
        {
            world.entity_mut(row).insert((
                Pressed,
                PressedEntry {
                    pointer,
                    target: context,
                    fresh: true,
                    id: item.id,
                },
            ));
        }
    });
}

fn clear_pressed<K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    pointer: PointerId,
    context: &NormalizedRenderTarget,
) {
    let Some(runtime) = world.get::<ListRuntime>(root).cloned() else {
        return;
    };
    for row in runtime.rows {
        if world
            .get::<PressedEntry<K>>(row)
            .is_some_and(|pressed| pressed.pointer == pointer && pressed.target == *context)
        {
            world.entity_mut(row).remove::<(Pressed, PressedEntry<K>)>();
        }
    }
}

fn clear_pointer_presses<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    pointer: PointerId,
    context: &NormalizedRenderTarget,
) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        clear_pressed::<K>(world, root, pointer, context);
    }
}

// Bevy emits no targeted PointerCancel or PointerRelease when a Pointer leaves every hovered entity.
// Also consume raw Pointer input so Pressed cannot remain on the old row indefinitely.
pub(crate) fn install_pointer_cleanup<T: Send + Sync + 'static, K: WidgetryListKey>(app: &mut App) {
    app.add_systems(
        PreUpdate,
        clear_ended_presses::<T, K>.after(bevy::picking::PickingSystems::Last),
    );
}

fn clear_ended_presses<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut input: MessageReader<PointerInput>,
    pointers: WidgetryPointerQuery,
    mut pressed: Query<(Entity, &mut PressedEntry<K>)>,
    parents: Query<&ChildOf>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    let inputs = input.read().collect::<Vec<_>>();
    for (row, mut owner) in &mut pressed {
        if !parents.iter_ancestors(row).any(|root| views.contains(root)) {
            continue;
        }
        let mut ended = pointers
            .location(owner.pointer)
            .is_none_or(|location| location.target != owner.target);
        if !ended {
            for input in &inputs {
                if input.pointer_id != owner.pointer || input.location.target != owner.target {
                    continue;
                }
                match input.action {
                    PointerAction::Cancel | PointerAction::Release(PointerButton::Primary) => {
                        ended = true
                    }
                    PointerAction::Press(PointerButton::Primary) if owner.fresh => ended = false,
                    _ => {}
                }
            }
        }
        if owner.fresh {
            owner.fresh = false;
        }
        if ended {
            commands.entity(row).remove::<(Pressed, PressedEntry<K>)>();
        }
    }
}

pub(crate) fn on_release<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerRelease>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if event.button == PointerButton::Primary {
        let pointer = event.pointer.id;
        let context = event.pointer.target.clone();
        if event.entity == event.original_event_target() {
            commands.queue(move |world: &mut World| {
                clear_pointer_presses::<T, K>(world, pointer, &context)
            });
        }
        if views.contains(event.entity) {
            event.propagate(false);
        }
    }
}

pub(crate) fn on_cancel<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerCancel>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if event.entity == event.original_event_target() {
        let pointer = event.pointer.id;
        let context = event.pointer.target.clone();
        commands.queue(move |world: &mut World| {
            clear_pointer_presses::<T, K>(world, pointer, &context)
        });
    }
    if views.contains(event.entity) {
        event.propagate(false);
    }
}

pub(crate) fn on_drag_end<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerDragEnd>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if event.button == PointerButton::Primary && views.contains(event.entity) {
        let root = event.entity;
        let pointer = event.pointer.id;
        let context = event.pointer.target.clone();
        event.propagate(false);
        commands.queue(move |world: &mut World| clear_pressed::<K>(world, root, pointer, &context));
    }
}

fn clicked_row<K: WidgetryListKey>(world: &World, root: Entity, target: Entity) -> Option<Entity> {
    let mut entity = target;
    let mut row = None;
    loop {
        if entity == root {
            return row;
        }
        if world.get::<WidgetryListViewState<K>>(entity).is_some() {
            return None;
        }
        if row.is_none() && world.get::<WidgetryListViewItem<K>>(entity).is_some() {
            row = Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

pub(crate) fn on_click<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut event: On<PointerClick>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let modifiers = crate::selection::SelectionModifiers::from_keys(keys.as_deref());
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get::<InteractionDisabled>(root).is_some()
            || crate::drag::suppress_click::<K>(world, root, pointer)
        {
            return Ok(());
        }
        let Some(row) = clicked_row::<K>(world, root, target) else {
            return Ok(());
        };
        let Some(item) = world.get::<WidgetryListViewItem<K>>(row).copied() else {
            return Ok(());
        };
        crate::projection::refresh_root::<T, K>(world, root);
        crate::selection::repair::<T, K>(world, root);
        let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
            return Ok(());
        };
        let Some(model) = current_projection::<T, K>(world, root) else {
            return Ok(());
        };
        let Some(index) = resolve(model, Some(item.id), Some(item.index)) else {
            return Ok(());
        };
        let disabled = (world.resource::<DataSource<T, K>>().row)(world, view.source(), item.id)
            .is_none_or(|row| row.disabled)
            || world.get::<InteractionDisabled>(row).is_some();
        let Some(mut state) = world.get::<WidgetryListViewState<K>>(root).copied() else {
            return Ok(());
        };
        let mut navigation = world
            .get::<ListNavigation<K>>(root)
            .copied()
            .unwrap_or_default();
        repair(model, &mut state, &mut navigation);
        state.active = Some(item.id);
        navigation.active_index = Some(index);
        navigation.reveal = None;
        world.entity_mut(root).insert((state, navigation));
        if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
            focus.set(root, FocusCause::Pressed);
        }
        if let Some(mut visible) = world.get_resource_mut::<InputFocusVisible>() {
            visible.0 = false;
        }
        if !disabled {
            crate::selection::input_select::<T, K>(world, root, item.id, modifiers)?;
        }
        Ok(())
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::WidgetryListModel;

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
        let active = model.push(1).unwrap();
        let successor = model.push(2).unwrap();
        let update = |model: &WidgetryListModel<u32>,
                      state: &mut WidgetryListViewState,
                      navigation: &mut ListNavigation| {
            let ids = (0..model.len())
                .filter_map(|index| model.id(index))
                .collect();
            let projection = crate::projection::from_ids_for_test(ids);
            repair(&projection, state, navigation);
        };
        let mut state = WidgetryListViewState {
            active: Some(active),
        };
        let mut navigation = ListNavigation {
            active_index: Some(1),
            reveal: None,
        };
        model.move_item(1, 0);
        update(&model, &mut state, &mut navigation);
        assert_eq!(navigation.active_index, Some(0));
        model.remove(0);
        update(&model, &mut state, &mut navigation);
        assert_eq!(state.active, Some(first));
        model.remove(0);
        update(&model, &mut state, &mut navigation);
        assert_eq!(state.active, Some(successor));
        model.clear();
        update(&model, &mut state, &mut navigation);
        assert_eq!(state, WidgetryListViewState::default());
    }
}
