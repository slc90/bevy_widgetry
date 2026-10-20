use crate::layout::TableGeometry;
use crate::projection::required;
use crate::{
    WidgetryTable, WidgetryTableCell, WidgetryTableColumnHeader, WidgetryTableColumnId,
    WidgetryTableModel, WidgetryTableRowHeader, WidgetryTableRowId, WidgetryTableSelection,
    WidgetryTableViewState,
};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusCause, FocusGained, FocusedInput, InputFocus};
use bevy::picking::{
    events::{PointerClick, PointerPress},
    pointer::PointerButton,
};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::window::RequestRedraw;
use bevy_widgetry_internal_log::widgetry_error;

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

// Upstream AcquireFocus also reports pointer-triggered focus as Navigated.
// Track the current frame's press separately to avoid treating focus before a click as keyboard navigation and revealing too early.
#[derive(Component)]
pub(crate) struct PointerFocus;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetryTableEventKind {
    SortRequested {
        column: WidgetryTableColumnId,
        direction: Option<crate::WidgetryTableSortDirection>,
    },
    FilterRequested(WidgetryTableColumnId),
    ColumnSelected(WidgetryTableColumnId),
    RowSelected(WidgetryTableRowId),
    CellSelected {
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    },
    SelectionCleared,
    RangeSelected(WidgetryTableSelection),
    ColumnResizeStart(WidgetryTableColumnId),
    ColumnResized {
        column: WidgetryTableColumnId,
        width: f32,
    },
    ColumnResizeEnd(WidgetryTableColumnId),
    ColumnResizeCancel(WidgetryTableColumnId),
}

pub(crate) fn clicked(
    world: &World,
    root: Entity,
    target: Entity,
) -> Option<WidgetryTableSelection> {
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
        if world.get::<crate::resize::ResizeHandle>(entity).is_some()
            || world
                .get::<crate::WidgetryTableHeaderAction>(entity)
                .is_some()
        {
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
                })
                .or_else(|| {
                    world
                        .get::<crate::WidgetryTableCorner>(entity)
                        .map(|_| WidgetryTableSelection::All)
                });
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

pub(crate) fn valid<T: Send + Sync + 'static>(
    model: &WidgetryTableModel<T>,
    projection: &WidgetryTableViewState,
    selection: WidgetryTableSelection,
) -> bool {
    match selection {
        WidgetryTableSelection::None => true,
        WidgetryTableSelection::Row(row) => {
            model.row_index(row).is_some() && projection.display_index(row).is_some()
        }
        WidgetryTableSelection::Column(column) => model.column_index(column).is_some(),
        WidgetryTableSelection::Cell { row, column } => {
            model.row_index(row).is_some()
                && projection.display_index(row).is_some()
                && model.column_index(column).is_some()
        }
        WidgetryTableSelection::Rows { anchor, end } => {
            valid(model, projection, WidgetryTableSelection::Row(anchor))
                && valid(model, projection, WidgetryTableSelection::Row(end))
        }
        WidgetryTableSelection::Columns { anchor, end } => {
            valid(model, projection, WidgetryTableSelection::Column(anchor))
                && valid(model, projection, WidgetryTableSelection::Column(end))
        }
        WidgetryTableSelection::Cells { anchor, end } => {
            valid(
                model,
                projection,
                WidgetryTableSelection::Cell {
                    row: anchor.row,
                    column: anchor.column,
                },
            ) && valid(
                model,
                projection,
                WidgetryTableSelection::Cell {
                    row: end.row,
                    column: end.column,
                },
            )
        }
        WidgetryTableSelection::All => true,
    }
}

pub(crate) fn on_click<T: Send + Sync + 'static>(
    mut event: On<PointerClick>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let double_click = event.count == 2;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world
            .get::<crate::WidgetryTableEditState>(root)
            .is_some_and(|s| crate::editing::inside(world, target, s.editor()))
        {
            return Ok(());
        }
        click::<T>(world, root, target)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table click 失败"))?;
        if double_click
            && let Some(WidgetryTableSelection::Cell { row, column }) = clicked(world, root, target)
        {
            WidgetryTable::<T>::begin_edit(world, root, WidgetryTableCell { row, column })?;
        }
        Ok(())
    });
}

fn click<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
) -> Result<(), BevyError> {
    if world.get::<InteractionDisabled>(root).is_some()
        || world
            .get::<crate::selection_drag::SuppressClick>(root)
            .is_some()
        || world.get::<WidgetryTable<T>>(root).is_none()
    {
        return Ok(());
    }
    let Some(selection) = clicked(world, root, target) else {
        return Ok(());
    };
    crate::view_state::refresh::<T>(world, root)?;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    if !valid(model, projection, selection) {
        return Ok(());
    }
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, projection, &mut state);
    let selection = if world
        .get_resource::<ButtonInput<KeyCode>>()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
    {
        state.selection.extend(selection)
    } else {
        selection
    }
    .normalize(projection.is_empty() || model.column_count() == 0);
    let changed = required(world.get::<WidgetryTableState>(root))?.selection != selection;
    let old = state.focused_cell;
    let row = old
        .and_then(|cell| projection.display_index(cell.row).map(|_| cell.row))
        .or_else(|| projection.row_id(0));
    let column = old
        .and_then(|cell| model.column_index(cell.column).map(|_| cell.column))
        .or_else(|| model.column_id(0));
    state.focused_cell = match selection {
        WidgetryTableSelection::Cell { row, column } => Some(WidgetryTableCell { row, column }),
        WidgetryTableSelection::Row(row) => column.map(|column| WidgetryTableCell { row, column }),
        WidgetryTableSelection::Column(column) => row.map(|row| WidgetryTableCell { row, column }),
        WidgetryTableSelection::None => old,
        WidgetryTableSelection::Cells { end, .. } => Some(end),
        WidgetryTableSelection::Rows { end, .. } => {
            column.map(|column| WidgetryTableCell { row: end, column })
        }
        WidgetryTableSelection::Columns { end, .. } => {
            row.map(|row| WidgetryTableCell { row, column: end })
        }
        WidgetryTableSelection::All => old.or_else(|| {
            row.zip(column)
                .map(|(row, column)| WidgetryTableCell { row, column })
        }),
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
        _ => WidgetryTableEventKind::RangeSelected(selection),
    };
    world.trigger(WidgetryTableEvent { entity: root, kind });
}

fn repair<T: Send + Sync + 'static>(
    model: &WidgetryTableModel<T>,
    projection: &WidgetryTableViewState,
    state: &mut WidgetryTableState,
) {
    state.selection = state
        .selection
        .normalize(projection.is_empty() || model.column_count() == 0);
    if !valid(model, projection, state.selection) {
        state.selection = WidgetryTableSelection::None;
    }
    if state.focused_cell.is_some_and(|cell| {
        !valid(
            model,
            projection,
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
    crate::view_state::refresh::<T>(world, root)?;
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, projection, &mut state);
    let row_index = state
        .focused_cell
        .and_then(|cell| projection.display_index(cell.row));
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
    event: On<PointerPress>,
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
    crate::view_state::refresh::<T>(world, root)?;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, projection, &mut state);
    // Pressed occurs before PointerClick.
    // Revealing early would destroy the hit cell while it is still waiting for release.
    let pointer_focus = world.get::<PointerFocus>(root).is_some();
    let reveal = !pointer_focus
        && cause != FocusCause::Pressed
        && (cause == FocusCause::Navigated || state.focused_cell.is_none());
    if state.focused_cell.is_none() {
        state.focused_cell = projection
            .row_id(0)
            .zip(model.column_id(0))
            .map(|(row, column)| WidgetryTableCell { row, column });
    }
    world
        .entity_mut(root)
        .insert((state, Navigation { reveal }));
    // FocusGained is dispatched after Layout.
    // Request another frame so a reactive App cannot sleep with stale cursor visuals and reveal state.
    if state.focused_cell.is_some() {
        world.write_message(RequestRedraw);
    }
    Ok(())
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<WidgetryTable<T>>>,
    focus: Option<Res<InputFocus>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let root = event.focused_entity;
    let key = event.input.key_code;
    if !views.contains(root)
        || focus.is_none_or(|focus| focus.get() != Some(root))
        || event.input.state != ButtonState::Pressed
        || !(matches!(
            key,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        ) || key == KeyCode::KeyA
            && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]))
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
    if key == KeyCode::KeyA {
        set_selection::<T>(world, root, WidgetryTableSelection::All)?;
        return Ok(());
    }
    crate::view_state::refresh::<T>(world, root)?;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    let mut state = *required(world.get::<WidgetryTableState>(root))?;
    repair(model, projection, &mut state);
    if projection.is_empty() || model.column_count() == 0 {
        world.entity_mut(root).insert(state);
        return Ok(());
    }
    let mut row = 0;
    let mut column = 0;
    if let Some(cell) = state.focused_cell {
        row = required(projection.display_index(cell.row))?;
        column = required(model.column_index(cell.column))?;
        match key {
            KeyCode::ArrowUp => row = row.saturating_sub(1),
            KeyCode::ArrowDown => row = (row + 1).min(projection.len() - 1),
            KeyCode::ArrowLeft => column = column.saturating_sub(1),
            KeyCode::ArrowRight => column = (column + 1).min(model.column_count() - 1),
            _ => {}
        }
    }
    state.focused_cell = Some(WidgetryTableCell {
        row: required(projection.row_id(row))?,
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
    crate::view_state::refresh::<T>(world, root)?;
    let view = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table selection requires a matching live View"))?;
    let model = world
        .get::<WidgetryTableModel<T>>(view.source())
        .ok_or_else(|| BevyError::error("Table selection requires a matching live Model source"))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    let selection = selection.normalize(projection.is_empty() || model.column_count() == 0);
    if !valid(model, projection, selection) {
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
    crate::view_state::refresh::<T>(world, root)?;
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table cursor requires a matching live View"))?
        .source();
    let model = world
        .get::<WidgetryTableModel<T>>(source)
        .ok_or_else(|| BevyError::error("Table cursor requires a matching live Model source"))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    if cell.is_some_and(|cell| {
        !valid(
            model,
            projection,
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

impl WidgetryTableState {
    pub fn selection(&self) -> WidgetryTableSelection {
        self.selection
    }

    pub fn focused_cell(&self) -> Option<WidgetryTableCell> {
        self.focused_cell
    }
}
