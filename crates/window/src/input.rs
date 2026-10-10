use crate::WidgetryModalWindow;
use crate::window_root::{WindowInitialized, WindowRoot};
use bevy::ecs::schedule::{ScheduleCleanupPolicy, ScheduleError};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{
    FocusCause, FocusedInput, InputFocus, InputFocusSystems, InputFocusVisible,
    dispatch_focused_input,
};
use bevy::prelude::*;
use bevy::text::{EditableText, PreeditCursor, TextEdit, TextReadWriteMode};
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{ImeSystems, TextInput};
use bevy::window::{Ime, PrimaryWindow};
use std::collections::BTreeMap;

#[derive(Resource, Default)]
struct FocusScopes {
    sequence: u64,
    modal: Vec<ModalFocus>,
    remembered: BTreeMap<Entity, Entity>,
}

#[derive(Component, Clone, Copy)]
struct ModalOrder(u64);

#[derive(Clone)]
struct ModalFocus {
    root: Entity,
    parent: Entity,
    previous: Option<Entity>,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<FocusScopes>()
        .init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<KeyboardInput>()
        .add_message::<Ime>()
        .add_observer(order_modal)
        .add_systems(Startup, replace_keyboard_dispatch)
        .add_systems(
            PreUpdate,
            (sync_focus, keyboard)
                .chain()
                .after(bevy::input::InputSystems)
                .in_set(InputFocusSystems::Dispatch),
        )
        .add_systems(
            PreUpdate,
            ime.after(InputFocusSystems::Dispatch)
                .after(bevy::ui::UiSystems::Focus),
        )
        .add_systems(
            PostUpdate,
            sync_focus.after(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        )
        .add_systems(
            PostUpdate,
            ime_position
                .in_set(bevy::ui::UiSystems::PostLayout)
                .after(bevy::ui::widget::update_editable_text_layout),
        )
        .configure_sets(
            PreUpdate,
            (ImeSystems::HandleEvents, ImeSystems::ToggleWindowIMEInput).run_if(no_managed_windows),
        )
        .configure_sets(
            PostUpdate,
            ImeSystems::UpdatePosition.run_if(no_managed_windows),
        );
}

fn no_managed_windows(roots: Query<(), With<WindowRoot>>) -> bool {
    roots.is_empty()
}

fn order_modal(
    event: On<Add<WidgetryModalWindow>>,
    mut scopes: ResMut<FocusScopes>,
    mut commands: Commands,
) {
    scopes.sequence = scopes.sequence.saturating_add(1);
    commands
        .entity(event.entity)
        .insert(ModalOrder(scopes.sequence));
}

fn replace_keyboard_dispatch(world: &mut World) -> Result {
    // 只替换 keyboard 分发，保留 raw messages 和 gamepad/mouse dispatch，供宿主独立消费。
    let result = world.schedule_scope(PreUpdate, |world, schedule| {
        schedule.remove_systems_in_set(
            dispatch_focused_input::<KeyboardInput>,
            world,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        )
    });
    match result {
        Ok(_) | Err(ScheduleError::SetNotFound) => Ok(()),
        Err(error) => Err(bevy_widgetry_core::scene::logged_error(format!(
            "window keyboard dispatch replacement failed: {error}"
        ))),
    }
}

fn within(world: &World, mut entity: Entity, root: Entity) -> bool {
    loop {
        if entity == root {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

fn available(world: &World, mut entity: Entity, require_enabled: bool) -> bool {
    if world.get_entity(entity).is_err() {
        return false;
    }
    loop {
        if require_enabled && world.get::<InteractionDisabled>(entity).is_some()
            || world.get::<Visibility>(entity) == Some(&Visibility::Hidden)
            || world
                .get::<Node>(entity)
                .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return true;
        };
        entity = parent.parent();
    }
}

fn root_for(world: &mut World, target: Entity) -> Option<(Entity, Entity)> {
    let mut entity = target;
    loop {
        if let Some(root) = world.get::<WindowRoot>(entity) {
            return Some((entity, root.target_window));
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            break;
        };
        entity = parent.parent();
    }
    // 原生 Window focus 与 primary embedded UI 仍保留独立的窗口身份。
    if world.get::<Window>(target).is_some() {
        return Some((target, target));
    }
    world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok()
        .map(|native| (native, native))
}

pub(crate) fn active_root(world: &mut World, native: Entity) -> Option<Entity> {
    let roots: Vec<_> = world
        .query_filtered::<(
            Entity,
            &WindowRoot,
            Option<&WidgetryModalWindow>,
            Option<&ModalOrder>,
        ), With<WindowInitialized>>()
        .iter(world)
        .map(|(entity, root, modal, order)| {
            (
                entity,
                root.target_window,
                modal.map(|modal| modal.parent),
                order.map(|order| order.0).unwrap_or(0),
            )
        })
        .collect();
    let mut base = native;
    for _ in 0..=roots.len() {
        let Some((_, _, Some(parent), _)) = roots.iter().find(|(_, window, _, _)| *window == base)
        else {
            break;
        };
        base = *parent;
    }
    let mut active = roots
        .iter()
        .find(|(_, window, _, _)| *window == base)
        .map(|root| root.0)?;
    for _ in 0..roots.len() {
        let Some(window) = roots
            .iter()
            .find(|root| root.0 == active)
            .map(|root| root.1)
        else {
            break;
        };
        let Some(child) = roots
            .iter()
            .filter(|root| root.2 == Some(window))
            .max_by_key(|root| root.3)
        else {
            break;
        };
        active = child.0;
    }
    Some(active)
}

fn tab_stops(world: &World, root: Entity, focus: Option<Entity>) -> Vec<Entity> {
    // Window 是默认 scope。显式 TabGroup 仍保留独立 order 与最近 modal group 的 contract。
    let mut ancestor = focus;
    let mut nearest_group = None;
    while let Some(entity) = ancestor {
        if let Some(group) = world.get::<TabGroup>(entity) {
            nearest_group = Some((entity, *group));
            break;
        }
        if entity == root {
            break;
        }
        ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    let mut groups = Vec::new();
    if let Some((entity, group)) = nearest_group.filter(|(_, group)| group.modal) {
        groups.push((entity, group.order));
    } else {
        let mut pending = vec![root];
        while let Some(entity) = pending.pop() {
            if !available(world, entity, true) {
                continue;
            }
            if let Some(group) = world.get::<TabGroup>(entity) {
                if !group.modal {
                    groups.push((entity, group.order));
                }
            } else if entity == root {
                groups.push((root, 0));
            }
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter().rev());
            }
        }
        groups.sort_by_key(|(_, order)| *order);
    }
    let mut stops = Vec::new();
    for (group, _) in groups {
        let mut group_stops = Vec::new();
        let mut pending: Vec<_> = if world.get::<TabGroup>(group).is_none() {
            vec![group]
        } else {
            world
                .get::<Children>(group)
                .map(|children| children.iter().rev().collect())
                .unwrap_or_default()
        };
        while let Some(entity) = pending.pop() {
            if !available(world, entity, true)
                || entity != group && world.get::<TabGroup>(entity).is_some()
            {
                continue;
            }
            if let Some(index) = world.get::<TabIndex>(entity)
                && index.0 >= 0
            {
                group_stops.push((entity, index.0));
            }
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter().rev());
            }
        }
        group_stops.sort_by_key(|(_, index)| *index);
        stops.extend(group_stops.into_iter().map(|(entity, _)| entity));
    }
    stops
}

fn initial(world: &World, root: Entity) -> Entity {
    let stops = tab_stops(world, root, Some(root));
    stops
        .iter()
        .copied()
        .filter_map(|entity| {
            world
                .get::<crate::WidgetryWindowInitialFocus>(entity)
                .map(|priority| (entity, priority.0))
        })
        .min_by_key(|(_, priority)| *priority)
        .map(|(entity, _)| entity)
        .or_else(|| stops.first().copied())
        .unwrap_or(root)
}

// Disabled 不使现有 focus 失效；隐藏、despawn 和 modal scope 切换仍按原规则处理。
pub(crate) fn sync_focus(world: &mut World) {
    let focus = world.resource::<InputFocus>().get();
    if let Some(focus) = focus
        && available(world, focus, false)
        && let Some((_, native)) = root_for(world, focus)
    {
        world
            .resource_mut::<FocusScopes>()
            .remembered
            .insert(native, focus);
    }
    let live: Vec<_> = world
        .query_filtered::<(Entity, &WidgetryModalWindow, &ModalOrder), With<WindowInitialized>>()
        .iter(world)
        .map(|(root, modal, order)| (root, modal.parent, order.0))
        .collect();
    let old = world.resource::<FocusScopes>().modal.clone();
    for frame in old
        .iter()
        .rev()
        .filter(|frame| !live.iter().any(|item| item.0 == frame.root))
    {
        let current = world.resource::<InputFocus>().get();
        if current.is_none_or(|entity| {
            world.get_entity(entity).is_err() || within(world, entity, frame.root)
        }) {
            let parent_root = active_root(world, frame.parent);
            let restored = frame
                .previous
                .filter(|entity| {
                    available(world, *entity, false)
                        && parent_root.is_some_and(|root| within(world, *entity, root))
                })
                .or_else(|| parent_root.map(|root| initial(world, root)));
            if let Some(restored) = restored {
                world
                    .resource_mut::<InputFocus>()
                    .set(restored, FocusCause::Navigated);
            } else {
                world.resource_mut::<InputFocus>().clear();
            }
        }
    }
    world
        .resource_mut::<FocusScopes>()
        .modal
        .retain(|frame| live.iter().any(|item| item.0 == frame.root));
    let mut added: Vec<_> = live
        .into_iter()
        .filter(|item| !old.iter().any(|frame| frame.root == item.0))
        .collect();
    added.sort_by_key(|item| item.2);
    for (root, parent, _) in added {
        let previous = world.resource::<InputFocus>().get();
        world.resource_mut::<FocusScopes>().modal.push(ModalFocus {
            root,
            parent,
            previous,
        });
        let target = initial(world, root);
        world
            .resource_mut::<InputFocus>()
            .set(target, FocusCause::Navigated);
    }
    if let Some(focus) = world.resource::<InputFocus>().get()
        && let Some((root, native)) = root_for(world, focus)
        && let Some(active) = active_root(world, native)
        && (active != root || !available(world, focus, false))
    {
        let target = initial(world, active);
        world
            .resource_mut::<InputFocus>()
            .set(target, FocusCause::Navigated);
    }
    world.resource_scope(|world, mut scopes: Mut<FocusScopes>| {
        scopes.remembered.retain(|native, focus| {
            world.get::<Window>(*native).is_some() && available(world, *focus, false)
        })
    });
    if world
        .resource::<InputFocus>()
        .get()
        .is_some_and(|entity| world.get_entity(entity).is_err())
    {
        world.resource_mut::<InputFocus>().clear();
    }
}

fn keyboard(mut events: MessageReader<KeyboardInput>, mut commands: Commands) {
    for input in events.read().cloned() {
        commands.queue(move |world: &mut World| -> Result {
            sync_focus(world);
            let native = input.window;
            let scope = active_root(world, native);
            if let Some(scope) = scope
                && world
                    .get::<WindowRoot>(scope)
                    .is_some_and(|root| root.target_window != native)
            {
                return Ok(());
            }
            let current = world.resource::<InputFocus>().get();
            let remembered = world
                .resource::<FocusScopes>()
                .remembered
                .get(&native)
                .copied();
            let target = current
                .filter(|target| {
                    root_for(world, *target).is_some_and(|(_, window)| window == native)
                        && available(world, *target, false)
                })
                .or_else(|| {
                    remembered.filter(|target| {
                        available(world, *target, false)
                            && root_for(world, *target).is_some_and(|(_, window)| window == native)
                    })
                })
                .or_else(|| scope.map(|root| initial(world, root)))
                .unwrap_or(native);
            if world.get::<Window>(native).is_none() {
                return Ok(());
            }
            if current != Some(target) {
                world
                    .resource_mut::<InputFocus>()
                    .set(target, FocusCause::Navigated);
            }
            let composing = world.get::<EditableText>(target).is_some_and(|edit| {
                edit.is_composing()
                    || edit.pending_edits.iter().any(|edit| {
                        matches!(
                            edit,
                            TextEdit::ImeSetCompose { .. } | TextEdit::ImeCommit { .. }
                        )
                    })
            });
            if scope.is_some()
                && input.key_code == KeyCode::Tab
                && input.state == ButtonState::Pressed
                && !input.repeat
                && !composing
            {
                let stops = tab_stops(world, scope.unwrap_or(target), Some(target));
                let backward = world
                    .resource::<ButtonInput<KeyCode>>()
                    .any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
                if !stops.is_empty() {
                    let next = match stops.iter().position(|entity| *entity == target) {
                        Some(index) if backward => (index + stops.len() - 1) % stops.len(),
                        Some(index) => (index + 1) % stops.len(),
                        None if backward => stops.len() - 1,
                        None => 0,
                    };
                    world
                        .resource_mut::<InputFocus>()
                        .set(stops[next], FocusCause::Navigated);
                    world.resource_mut::<InputFocusVisible>().0 = true;
                }
            } else {
                world.trigger(FocusedInput::new(target, input, native));
            }
            world.flush();
            Ok(())
        });
    }
}

fn accepts_ime(world: &World, target: Entity) -> bool {
    available(world, target, true)
        && world.get::<TextInput>(target).is_some()
        && world.get::<EditableText>(target).is_some()
        && world.get::<TextReadWriteMode>(target) == Some(&TextReadWriteMode::Editable)
}

fn ime(
    mut events: MessageReader<Ime>,
    mut commands: Commands,
    roots: Query<(), With<WindowRoot>>,
    mut changed_modes: Query<
        (&mut EditableText, &TextReadWriteMode),
        (With<TextInput>, Changed<TextReadWriteMode>),
    >,
) {
    if roots.is_empty() {
        events.read().for_each(drop);
        return;
    }
    // 受管 window 已停用官方 IME toggle system，mode 改变时仍需取消旧 composition。
    for (mut editor, mode) in &mut changed_modes {
        if *mode != TextReadWriteMode::Editable {
            editor.queue_edit(TextEdit::clear_ime_compose());
        }
    }
    for event in events.read().cloned() {
        commands.queue(move |world: &mut World| {
            sync_focus(world);
            let native = match &event {
                Ime::Preedit { window, .. }
                | Ime::Commit { window, .. }
                | Ime::Enabled { window }
                | Ime::Disabled { window } => *window,
            };
            let Some(target) = world.resource::<InputFocus>().get() else {
                return;
            };
            if root_for(world, target).is_none_or(|(_, window)| window != native)
                || !accepts_ime(world, target)
            {
                return;
            }
            if let Some(mut editor) = world.get_mut::<EditableText>(target) {
                let edit = match event {
                    Ime::Preedit { value, cursor, .. } => TextEdit::ImeSetCompose {
                        value: value.into(),
                        cursor: cursor.map(|(anchor, focus)| PreeditCursor { anchor, focus }),
                    },
                    Ime::Commit { value, .. } => TextEdit::ImeCommit {
                        value: value.into(),
                    },
                    Ime::Enabled { .. } | Ime::Disabled { .. } => TextEdit::clear_ime_compose(),
                };
                editor.queue_edit(edit);
            }
        });
    }
}

fn ime_position(world: &mut World) {
    if world.query::<&WindowRoot>().iter(world).next().is_none() {
        return;
    }
    let focused = world
        .resource::<InputFocus>()
        .get()
        .filter(|entity| accepts_ime(world, *entity));
    let native = focused
        .and_then(|entity| root_for(world, entity))
        .map(|(_, native)| native);
    let position = focused.and_then(|entity| {
        let editor = world.get::<EditableText>(entity)?;
        let area = editor.editor().ime_cursor_area();
        let node = world.get::<ComputedNode>(entity)?;
        let transform = world.get::<UiGlobalTransform>(entity)?;
        let target = world.get::<ComputedUiRenderTargetInfo>(entity)?;
        let scale = world
            .get_resource::<UiScale>()
            .map(|scale| scale.0)
            .unwrap_or(1.0);
        Some(
            transform.affine().transform_point2(
                Vec2::new(area.x0 as f32, area.y1 as f32) + node.content_box().min
                    - editor.viewport.offset,
            ) * scale
                / target.scale_factor(),
        )
    });
    for (entity, mut window) in world.query::<(Entity, &mut Window)>().iter_mut(world) {
        window.ime_enabled = native == Some(entity);
        if native == Some(entity)
            && let Some(position) = position
        {
            window.ime_position = position;
        }
    }
}
