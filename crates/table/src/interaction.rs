use crate::layout::TableGeometry;
use crate::projection::required;
use crate::{
    WidgetryTable, WidgetryTableCell, WidgetryTableColumnHeader, WidgetryTableColumnId,
    WidgetryTableModel, WidgetryTableRowHeader, WidgetryTableRowId,
};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusCause, FocusGained, FocusedInput, InputFocus};
use bevy::picking::{
    events::{Click, Pointer, Press},
    pointer::PointerButton,
};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::window::RequestRedraw;
use bevy_widgetry_log::widgetry_error;

#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryTableState {
    selection: WidgetryTableSelection,
    focused_cell: Option<WidgetryTableCell>,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryTableEvent {
    pub entity: Entity,
    pub kind: WidgetryTableEventKind,
}

#[derive(Component, Default)]
struct Navigation {
    reveal: bool,
}

// 官方 AcquireFocus 把 pointer 触发也记为 Navigated。
// 单独记录当帧 press 来源，避免把 click 前的 focus 当成 keyboard navigation 而提前 reveal。
#[derive(Component)]
pub(crate) struct PointerFocus;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum WidgetryTableSelection {
    #[default]
    None,
    Row(WidgetryTableRowId),
    Column(WidgetryTableColumnId),
    Cell {
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetryTableEventKind {
    ColumnSelected(WidgetryTableColumnId),
    RowSelected(WidgetryTableRowId),
    CellSelected {
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    },
    SelectionCleared,
    ColumnResizeStart(WidgetryTableColumnId),
    ColumnResized {
        column: WidgetryTableColumnId,
        width: f32,
    },
    ColumnResizeEnd(WidgetryTableColumnId),
    ColumnResizeCancel(WidgetryTableColumnId),
}

fn clicked(world: &World, root: Entity, target: Entity) -> Option<WidgetryTableSelection> {
    let mut entity = target;
    let mut selection = None;
    loop {
        if world.get::<InteractionDisabled>(entity).is_some() {
            return None;
        }
        if entity == root {
            return selection;
        }
        if world.get::<WidgetryTableState>(entity).is_some() {
            return None;
        }
        if world.get::<crate::resize::ResizeHandle>(entity).is_some() {
            return None;
        }
        if selection.is_none() {
            selection = world
                .get::<WidgetryTableCell>(entity)
                .map(|cell| WidgetryTableSelection::Cell {
                    row: cell.row,
                    column: cell.column,
                })
                .or_else(|| {
                    world
                        .get::<WidgetryTableColumnHeader>(entity)
                        .map(|header| WidgetryTableSelection::Column(header.column))
                })
                .or_else(|| {
                    world
                        .get::<WidgetryTableRowHeader>(entity)
                        .map(|header| WidgetryTableSelection::Row(header.row))
                });
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn valid<T: Send + Sync + 'static>(
    model: &WidgetryTableModel<T>,
    selection: WidgetryTableSelection,
) -> bool {
    match selection {
        WidgetryTableSelection::None => true,
        WidgetryTableSelection::Row(row) => model.row_index(row).is_some(),
        WidgetryTableSelection::Column(column) => model.column_index(column).is_some(),
        WidgetryTableSelection::Cell { row, column } => {
            model.row_index(row).is_some() && model.column_index(column).is_some()
        }
    }
}

pub(crate) fn on_click<T: Send + Sync + 'static>(
    mut event: On<Pointer<Click>>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        click::<T>(world, root, target)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table click 失败"))
    });
}

fn click<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
) -> Result<(), BevyError> {
    if world.get::<InteractionDisabled>(root).is_some()
        || world.get::<WidgetryTable<T>>(root).is_none()
    {
        return Ok(());
    }
    let Some(selection) = clicked(world, root, target) else {
        return Ok(());
    };
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    if !valid(model, selection) {
        return Ok(());
    }
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    let changed = state.selection != selection;
    let old = state.focused_cell;
    let row = old
        .and_then(|cell| model.row_index(cell.row).map(|_| cell.row))
        .or_else(|| model.row_id(0));
    let column = old
        .and_then(|cell| model.column_index(cell.column).map(|_| cell.column))
        .or_else(|| model.column_id(0));
    state.focused_cell = match selection {
        WidgetryTableSelection::Cell { row, column } => Some(WidgetryTableCell { row, column }),
        WidgetryTableSelection::Row(row) => column.map(|column| WidgetryTableCell { row, column }),
        WidgetryTableSelection::Column(column) => row.map(|row| WidgetryTableCell { row, column }),
        WidgetryTableSelection::None => old,
    };
    state.selection = selection;
    world.entity_mut(root).insert(state);
    if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
        focus.set(root, FocusCause::Pressed);
    }
    if changed {
        notify_selection(world, root, selection);
    }
    Ok(())
}

fn notify_selection(world: &mut World, root: Entity, selection: WidgetryTableSelection) {
    let kind = match selection {
        WidgetryTableSelection::Row(row) => WidgetryTableEventKind::RowSelected(row),
        WidgetryTableSelection::Column(column) => WidgetryTableEventKind::ColumnSelected(column),
        WidgetryTableSelection::Cell { row, column } => {
            WidgetryTableEventKind::CellSelected { row, column }
        }
        WidgetryTableSelection::None => WidgetryTableEventKind::SelectionCleared,
    };
    world.trigger(WidgetryTableEvent { entity: root, kind });
}

fn repair<T: Send + Sync + 'static>(model: &WidgetryTableModel<T>, state: &mut WidgetryTableState) {
    if !valid(model, state.selection) {
        state.selection = WidgetryTableSelection::None;
    }
    if state.focused_cell.is_some_and(|cell| {
        !valid(
            model,
            WidgetryTableSelection::Cell {
                row: cell.row,
                column: cell.column,
            },
        )
    }) {
        state.focused_cell = None;
    }
}

pub(crate) fn sync<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    source: Entity,
    body: Entity,
    geometry: &TableGeometry,
    size: Vec2,
) -> Result<(), BevyError> {
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, &mut state);
    let row_index = state
        .focused_cell
        .and_then(|cell| model.row_index(cell.row));
    let current = required(world.get::<WidgetryTableState>(root))?;
    if current.selection != state.selection || current.focused_cell != state.focused_cell {
        world.entity_mut(root).insert(state);
    }
    let reveal = world.get::<Navigation>(root).is_some_and(|nav| nav.reveal);
    if reveal
        && size.x > 0.0
        && size.y > 0.0
        && let Some(cell) = state.focused_cell
        && let Some(row) = row_index
    {
        let column = required(
            geometry
                .columns
                .iter()
                .find(|column| column.id == cell.column),
        )?;
        let start = Vec2::new(column.left, row as f32 * geometry.row_height);
        let end = start + Vec2::new(column.width, geometry.row_height);
        let mut position = required(world.get_mut::<ScrollPosition>(body))?;
        for axis in 0..2 {
            if start[axis] < position.0[axis] {
                position.0[axis] = start[axis];
            } else if end[axis] > position.0[axis] + size[axis] {
                position.0[axis] = (end[axis] - size[axis]).max(start[axis].min(position.0[axis]));
            }
        }
        world.entity_mut(root).remove::<Navigation>();
    }
    if state.focused_cell.is_none() {
        world.entity_mut(root).remove::<Navigation>();
    }
    Ok(())
}

pub(crate) fn clear(world: &mut World, root: Entity) {
    world
        .entity_mut(root)
        .insert(WidgetryTableState::default())
        .remove::<Navigation>();
}

pub(crate) fn on_press<T: Send + Sync + 'static>(
    event: On<Pointer<Press>>,
    views: Query<(), (With<WidgetryTable<T>>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if event.button == PointerButton::Primary && views.contains(event.entity) {
        commands.entity(event.entity).insert(PointerFocus);
    }
}

pub(crate) fn clear_pointer_focus(
    mut commands: Commands,
    roots: Query<Entity, With<PointerFocus>>,
) {
    for root in &roots {
        commands.entity(root).remove::<PointerFocus>();
    }
}

pub(crate) fn on_focus<T: Send + Sync + 'static>(
    event: On<FocusGained>,
    views: Query<(), With<WidgetryTable<T>>>,
    focus: Option<Res<InputFocus>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !views.contains(root)
        || event.original_event_target() != root
        || focus.is_none_or(|focus| focus.get() != Some(root))
    {
        return;
    }
    let cause = event.cause;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        initialize::<T>(world, root, cause)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table focus 初始化失败"))
    });
}

fn initialize<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    cause: FocusCause,
) -> Result<(), BevyError> {
    if world.get::<WidgetryTable<T>>(root).is_none()
        || world.get::<InteractionDisabled>(root).is_some()
        || world
            .get_resource::<InputFocus>()
            .is_none_or(|focus| focus.get() != Some(root))
    {
        return Ok(());
    }
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, &mut state);
    // Pressed 发生在 Click 前。
    // 提前 reveal 会销毁 viewport 中尚待 release 的命中 Cell。
    let pointer_focus = world.get::<PointerFocus>(root).is_some();
    let reveal = !pointer_focus
        && cause != FocusCause::Pressed
        && (cause == FocusCause::Navigated || state.focused_cell.is_none());
    if state.focused_cell.is_none() {
        state.focused_cell = model
            .row_id(0)
            .zip(model.column_id(0))
            .map(|(row, column)| WidgetryTableCell { row, column });
    }
    world
        .entity_mut(root)
        .insert((state, Navigation { reveal }));
    // FocusGained 在 Layout 后才 dispatch。
    // Reactive App 若直接休眠，cursor 视觉与 reveal 会滞后，因此请求下一帧。
    if state.focused_cell.is_some() {
        world.write_message(RequestRedraw);
    }
    Ok(())
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<WidgetryTable<T>>>,
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
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        )
    {
        return;
    }
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        navigate::<T>(world, root, key)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table navigation 失败"))
    });
}

fn navigate<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    key: KeyCode,
) -> Result<(), BevyError> {
    if world.get::<WidgetryTable<T>>(root).is_none()
        || world.get::<InteractionDisabled>(root).is_some()
        || world
            .get_resource::<InputFocus>()
            .is_none_or(|focus| focus.get() != Some(root))
    {
        return Ok(());
    }
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, &mut state);
    if model.row_count() == 0 || model.column_count() == 0 {
        world.entity_mut(root).insert(state);
        return Ok(());
    }
    let mut row = 0;
    let mut column = 0;
    if let Some(cell) = state.focused_cell {
        row = required(model.row_index(cell.row))?;
        column = required(model.column_index(cell.column))?;
        match key {
            KeyCode::ArrowUp => row = row.saturating_sub(1),
            KeyCode::ArrowDown => row = (row + 1).min(model.row_count() - 1),
            KeyCode::ArrowLeft => column = column.saturating_sub(1),
            KeyCode::ArrowRight => column = (column + 1).min(model.column_count() - 1),
            _ => {}
        }
    }
    state.focused_cell = Some(WidgetryTableCell {
        row: required(model.row_id(row))?,
        column: required(model.column_id(column))?,
    });
    world
        .entity_mut(root)
        .insert((state, Navigation { reveal: true }));
    Ok(())
}

pub(crate) fn set_selection<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    selection: WidgetryTableSelection,
) -> Result<bool, BevyError> {
    let view = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table selection requires a matching live View"))?;
    let model = world
        .get::<WidgetryTableModel<T>>(view.source())
        .ok_or_else(|| BevyError::error("Table selection requires a matching live Model source"))?;
    if !valid(model, selection) {
        return Err(BevyError::error(
            "Table selection contains stale source-local identity",
        ));
    }
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    if state.selection == selection {
        return Ok(false);
    }
    state.selection = selection;
    world.entity_mut(root).insert(state);
    notify_selection(world, root, selection);
    Ok(true)
}

pub(crate) fn set_focused_cell<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    cell: Option<WidgetryTableCell>,
) -> Result<bool, BevyError> {
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table cursor requires a matching live View"))?
        .source();
    let model = world
        .get::<WidgetryTableModel<T>>(source)
        .ok_or_else(|| BevyError::error("Table cursor requires a matching live Model source"))?;
    if cell.is_some_and(|cell| {
        !valid(
            model,
            WidgetryTableSelection::Cell {
                row: cell.row,
                column: cell.column,
            },
        )
    }) {
        return Err(BevyError::error(
            "Table cursor contains stale source-local identity",
        ));
    }
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    if state.focused_cell == cell {
        return Ok(false);
    }
    state.focused_cell = cell;
    world.entity_mut(root).insert(state).remove::<Navigation>();
    Ok(true)
}

pub(crate) fn appearance(world: &World, root: Entity, entity: Entity) -> (bool, bool) {
    let Some(state) = world.get::<WidgetryTableState>(root) else {
        return (false, false);
    };
    let cell = world.get::<WidgetryTableCell>(entity);
    let selected = if let Some(cell) = cell {
        match state.selection {
            WidgetryTableSelection::None => false,
            WidgetryTableSelection::Row(row) => cell.row == row,
            WidgetryTableSelection::Column(column) => cell.column == column,
            WidgetryTableSelection::Cell { row, column } => {
                cell.row == row && cell.column == column
            }
        }
    } else if let Some(header) = world.get::<WidgetryTableColumnHeader>(entity) {
        state.selection == WidgetryTableSelection::Column(header.column)
    } else if let Some(header) = world.get::<WidgetryTableRowHeader>(entity) {
        state.selection == WidgetryTableSelection::Row(header.row)
    } else {
        false
    };
    let focused = cell.is_some_and(|cell| state.focused_cell == Some(*cell))
        && world
            .get_resource::<InputFocus>()
            .is_some_and(|focus| focus.get() == Some(root));
    (selected, focused)
}

impl WidgetryTableState {
    pub fn selection(&self) -> WidgetryTableSelection {
        self.selection
    }

    pub fn focused_cell(&self) -> Option<WidgetryTableCell> {
        self.focused_cell
    }
}
