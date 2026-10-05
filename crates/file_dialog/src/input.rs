use crate::model::contract_error;
use crate::style::{Part, PartKind};
use crate::view::EntryRow;
use crate::*;
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::{Cancel, Click, DragEnd, Pointer, Press};
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::{InteractionDisabled, Pressed};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Weak};

#[derive(Component, Clone)]
struct PressIdentity {
    row: EntryRow,
    pointer: PointerId,
    snapshot: Weak<WidgetryFileDialogSnapshot>,
}

#[derive(Resource, Default)]
struct PendingKeys(Vec<KeyRequest>);

struct KeyRequest {
    focus: Entity,
    key: KeyCode,
    control: bool,
    shift: bool,
    deferred_token: Option<WidgetryFileDialogToken>,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingKeys>()
        .add_observer(on_press)
        .add_observer(on_click)
        .add_observer(on_cancel)
        .add_observer(on_drag_end)
        .add_observer(on_key);
}

fn composing(edit: &EditableText) -> bool {
    edit.is_composing()
        || edit.pending_edits.iter().any(|edit| {
            matches!(
                edit,
                TextEdit::ImeSetCompose { .. } | TextEdit::ImeCommit { .. }
            )
        })
}

fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    parts: Query<&Part>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryFileDialog>>,
    editors: Query<&EditableText>,
    focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let target = event.focused_entity;
    let key = event.input.key_code;
    if event.input.state != ButtonState::Pressed
        || focus.get() != Some(target)
        || editors.get(target).is_ok_and(composing)
    {
        return;
    }
    let relevant = parts.get(target).is_ok_and(|part| match part.kind {
        PartKind::Entries => matches!(
            key,
            KeyCode::ArrowDown
                | KeyCode::ArrowUp
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageDown
                | KeyCode::PageUp
                | KeyCode::Space
                | KeyCode::Enter
                | KeyCode::Escape
                | KeyCode::KeyA
        ),
        _ => matches!(key, KeyCode::Enter | KeyCode::Escape),
    });
    let mut ancestor = target;
    while !roots.contains(ancestor) {
        if parts
            .get(ancestor)
            .is_ok_and(|part| matches!(part.kind, PartKind::Filter | PartKind::Sort))
        {
            return;
        }
        let Ok(parent) = parents.get(ancestor) else {
            return;
        };
        ancestor = parent.parent();
    }
    if !relevant && key != KeyCode::Escape {
        return;
    }
    event.propagate(false);
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    commands.queue(move |world: &mut World| -> Result {
        let mut pending = world.resource_mut::<PendingKeys>();
        if pending.0.len() >= 128 {
            return Err(contract_error(
                "FileDialog keyboard queue capacity exceeded",
            ));
        }
        pending.0.push(KeyRequest {
            focus: target,
            key,
            control,
            shift,
            deferred_token: None,
        });
        Ok(())
    });
}

pub(crate) fn keyboard_jobs(world: &mut World) -> Result {
    let jobs = std::mem::take(&mut world.resource_mut::<PendingKeys>().0);
    let mut blocked = BTreeSet::new();
    let mut deferred = Vec::new();
    for mut job in jobs {
        if world.resource::<InputFocus>().get() != Some(job.focus) {
            continue;
        }
        if let Some(part) = world.get::<Part>(job.focus)
            && part.kind == PartKind::Entries
            && let Some(state) = world.get::<WidgetryFileDialogState>(part.root)
        {
            if job
                .deferred_token
                .is_some_and(|token| !state.matches_query(token))
                || state.session_state() != WidgetryFileDialogSessionState::Open
                || world.get::<InteractionDisabled>(part.root).is_some()
            {
                continue;
            }
            if job.key != KeyCode::Escape
                && (blocked.contains(&part.root)
                    || matches!(job.key, KeyCode::Space | KeyCode::Enter)
                        && state.selection_pending())
            {
                blocked.insert(part.root);
                job.deferred_token = Some(state.token());
                deferred.push(job);
                continue;
            }
        }
        if let Err(error) = keyboard_job(world, job) {
            world
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
    }
    world.resource_mut::<PendingKeys>().0 = deferred;
    Ok(())
}

fn keyboard_job(world: &mut World, job: KeyRequest) -> Result {
    if world.resource::<InputFocus>().get() != Some(job.focus)
        || world.get::<EditableText>(job.focus).is_some_and(composing)
    {
        return Ok(());
    }
    let part = world.get::<Part>(job.focus).copied();
    let mut root = part.map(|part| part.root).unwrap_or(job.focus);
    while world.get::<WidgetryFileDialog>(root).is_none() {
        let Some(parent) = world.get::<ChildOf>(root) else {
            break;
        };
        root = parent.parent();
    }
    let Some(state) = world.get::<WidgetryFileDialogState>(root) else {
        return Ok(());
    };
    if state.session_state() != WidgetryFileDialogSessionState::Open
        || world.get::<InteractionDisabled>(root).is_some()
    {
        return Ok(());
    }
    if job.key == KeyCode::Escape {
        if !crate::controls::escape(world, root)? {
            crate::style::apply_input(world, root, WidgetryFileDialogAction::Cancel)?;
        }
        return Ok(());
    }
    let Some(part) = part else {
        return Ok(());
    };
    let action = if part.kind == PartKind::Entries {
        match job.key {
            KeyCode::KeyA if job.control && state.mode().is_multiple() => {
                Some(WidgetryFileDialogAction::SelectAll)
            }
            KeyCode::Enter => state.active().map(|id| WidgetryFileDialogAction::Activate {
                id,
                token: state.token(),
            }),
            KeyCode::Space => state
                .active()
                .filter(|id| {
                    state
                        .snapshot()
                        .and_then(|snapshot| snapshot.entry(*id))
                        .is_some_and(|entry| state.mode().accepts(entry.kind()))
                })
                .map(|id| WidgetryFileDialogAction::Select {
                    id,
                    token: state.token(),
                    operation: if !state.mode().is_multiple() {
                        WidgetryFileDialogSelection::Replace
                    } else if job.shift {
                        WidgetryFileDialogSelection::Range
                    } else {
                        WidgetryFileDialogSelection::Toggle
                    },
                }),
            KeyCode::ArrowDown => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::Next,
            )),
            KeyCode::ArrowUp => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::Previous,
            )),
            KeyCode::Home => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::First,
            )),
            KeyCode::End => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::Last,
            )),
            KeyCode::PageDown => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::PageDown(crate::view::page_size(world, job.focus)),
            )),
            KeyCode::PageUp => Some(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::PageUp(crate::view::page_size(world, job.focus)),
            )),
            _ => None,
        }
    } else if job.key == KeyCode::Enter {
        let text = world
            .get::<EditableText>(job.focus)
            .map(|edit| edit.value().to_string())
            .unwrap_or_default();
        match part.kind {
            PartKind::Path => {
                let path = state
                    .current_path()
                    .filter(|path| path.to_string_lossy() == text)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| PathBuf::from(text));
                Some(WidgetryFileDialogAction::Navigate(
                    WidgetryFileDialogNavigation::Path(path),
                ))
            }
            PartKind::Filename => {
                if text != state.filename().to_string_lossy() {
                    crate::style::apply_input(
                        world,
                        root,
                        WidgetryFileDialogAction::Filename(text.into()),
                    )?;
                }
                Some(WidgetryFileDialogAction::Confirm)
            }
            PartKind::Search => Some(WidgetryFileDialogAction::Search(text)),
            PartKind::Folder => {
                crate::controls::create_folder(world, root, text.into())?;
                None
            }
            _ => None,
        }
    } else {
        None
    };
    if let Some(action) = action {
        crate::style::apply_input(world, root, action)?;
    }
    Ok(())
}

pub(crate) fn editor_updates(world: &mut World) -> Result {
    let edits: Vec<_> = world
        .query_filtered::<(&Part, &EditableText), Changed<EditableText>>()
        .iter(world)
        .filter(|(part, edit)| {
            matches!(part.kind, PartKind::Filename | PartKind::Search) && !composing(edit)
        })
        .map(|(part, edit)| (*part, edit.value().to_string()))
        .collect();
    for (part, text) in edits {
        let Some(state) = world.get::<WidgetryFileDialogState>(part.root) else {
            continue;
        };
        if state.session_state() != WidgetryFileDialogSessionState::Open {
            continue;
        }
        let action = match part.kind {
            PartKind::Search if text != state.search() => {
                Some(WidgetryFileDialogAction::Search(text))
            }
            PartKind::Filename if text != state.filename().to_string_lossy() => {
                Some(WidgetryFileDialogAction::Filename(text.into()))
            }
            _ => None,
        };
        if let Some(action) = action
            && let Err(error) = crate::style::apply_input(world, part.root, action)
        {
            world
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
    }
    Ok(())
}

fn on_press(mut event: On<Pointer<Press>>, rows: Query<&EntryRow>, mut commands: Commands) {
    if event.button != PointerButton::Primary {
        return;
    }
    if let Ok(row) = rows.get(event.entity) {
        let row = *row;
        let entity = event.entity;
        let pointer = event.pointer_id;
        event.propagate(false);
        commands.queue(move |world: &mut World| {
            let snapshot = world
                .get::<WidgetryFileDialogState>(row.root)
                .and_then(|state| state.snapshot())
                .filter(|snapshot| Arc::as_ptr(snapshot) as usize == row.snapshot_identity)
                .map(Arc::downgrade);
            if world.get::<EntryRow>(entity) == Some(&row)
                && world.get::<InteractionDisabled>(row.root).is_none()
                && world
                    .get::<WidgetryFileDialogState>(row.root)
                    .is_some_and(|state| {
                        state.session_state() == WidgetryFileDialogSessionState::Open
                    })
                && let Some(snapshot) = snapshot
            {
                world.entity_mut(entity).insert((
                    Pressed,
                    PressIdentity {
                        row,
                        pointer,
                        snapshot,
                    },
                ));
                let area = world
                    .query::<(Entity, &Part)>()
                    .iter(world)
                    .find(|(_, part)| part.root == row.root && part.kind == PartKind::Entries)
                    .map(|(entity, _)| entity);
                if let Some(area) = area {
                    world
                        .resource_mut::<InputFocus>()
                        .set(area, FocusCause::Pressed);
                }
            }
        });
    }
}

fn on_click(
    mut event: On<Pointer<Click>>,
    rows: Query<&EntryRow>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(row) = rows.get(event.entity) else {
        return;
    };
    let entity = event.entity;
    let row = *row;
    let pointer = event.pointer_id;
    let count = event.count;
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        let pressed = world.get::<PressIdentity>(entity).cloned();
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove::<(Pressed, PressIdentity)>();
        }
        if pressed
            .as_ref()
            .is_none_or(|pressed| pressed.pointer != pointer || pressed.row != row)
            || world.get::<EntryRow>(entity) != Some(&row)
            || world.get::<InteractionDisabled>(row.root).is_some()
        {
            return Ok(());
        }
        let Some(state) = world.get::<WidgetryFileDialogState>(row.root) else {
            return Ok(());
        };
        if state.token() != row.token
            || crate::view::snapshot_identity(state) != Some(row.snapshot_identity)
            || pressed.as_ref().is_none_or(|pressed| {
                state
                    .snapshot()
                    .is_none_or(|snapshot| !pressed.snapshot.ptr_eq(&Arc::downgrade(snapshot)))
            })
            || state.session_state() != WidgetryFileDialogSessionState::Open
        {
            return Ok(());
        }
        let Some(entry) = state.snapshot().and_then(|snapshot| snapshot.entry(row.id)) else {
            return Ok(());
        };
        let action = if count == 2 {
            WidgetryFileDialogAction::Activate {
                id: row.id,
                token: row.token,
            }
        } else if !state.mode().accepts(entry.kind()) {
            WidgetryFileDialogAction::Active(Some(row.id))
        } else {
            WidgetryFileDialogAction::Select {
                id: row.id,
                token: row.token,
                operation: if !state.mode().is_multiple() {
                    WidgetryFileDialogSelection::Replace
                } else {
                    match (control, shift) {
                        (true, true) => WidgetryFileDialogSelection::ExtendRange,
                        (false, true) => WidgetryFileDialogSelection::Range,
                        (true, false) => WidgetryFileDialogSelection::Toggle,
                        (false, false) => WidgetryFileDialogSelection::Replace,
                    }
                },
            }
        };
        crate::style::apply_input(world, row.root, action).map(|_| ())
    });
}

fn on_cancel(event: On<Pointer<Cancel>>, rows: Query<(), With<EntryRow>>, mut commands: Commands) {
    if rows.contains(event.entity) {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressIdentity)>();
    }
}

fn on_drag_end(
    event: On<Pointer<DragEnd>>,
    rows: Query<(), With<EntryRow>>,
    mut commands: Commands,
) {
    if rows.contains(event.entity) {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressIdentity)>();
    }
}
