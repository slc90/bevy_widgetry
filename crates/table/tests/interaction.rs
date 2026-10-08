//! State：None/Row/Column/Cell selection、logical cursor、真实 input focus、enabled 与 resize gesture。
//! Stimuli：Content/Header click、keyboard dispatch、scroll、API、Model 删除、disabled、drag/cancel/despawn。
//! Guards：primary pointer、真实 root focus、enabled。
//! 同 selection 不重复通知。
//! Transitions：click 提交 selection/cursor，方向键移动 cursor。
//! resize 从 Start 经 width 更新到 End 或 Cancel。
//! Invariants：state 引用有效 stable ID。
//! 程序与 UI 先提交再通知，repair 静默。
//! gesture 最多一次 terminal。
//! Couplings：selection/focus × virtualization。
//! resize × DPI/disabled/Column lifecycle。
//! callback mutation 参与当前 projection。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod common;

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::message::MessageCursor;
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput, NativeKey},
};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::{AcquireFocus, InputFocus};
use bevy::picking::hover::HoverMap;
use bevy::picking::{
    backend::HitData,
    events::{Drag, DragEnd, DragStart, Pointer},
    pointer::{Location, PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::window::PrimaryWindow;
use bevy::window::RequestRedraw;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, press_key, primary_cancel, primary_click,
    primary_press, queue_key,
};
use common::{fixture, projection, scroll};

#[derive(Resource, Default)]
struct Events(Vec<WidgetryTableEventKind>);

#[derive(Resource, Default)]
struct HoverTarget(Option<Entity>);

fn inject_hover(target: Res<HoverTarget>, mut map: ResMut<HoverMap>) {
    map.clear();
    if let Some(entity) = target.0 {
        let mut hits = EntityHashMap::default();
        hits.insert(entity, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        map.insert(PointerId::Mouse, hits);
    }
}

fn interaction_fixture() -> (App, Entity, Entity, Entity) {
    let (mut app, source, root, body) = fixture(20, 10);
    app.init_resource::<Events>().add_observer(
        |event: On<WidgetryTableEvent>,
         states: Query<&WidgetryTableState>,
         layouts: Query<&WidgetryTableLayout>,
         mut events: ResMut<Events>| {
            let expected = match event.kind {
                WidgetryTableEventKind::RowSelected(row) => Some(WidgetryTableSelection::Row(row)),
                WidgetryTableEventKind::ColumnSelected(column) => {
                    Some(WidgetryTableSelection::Column(column))
                }
                WidgetryTableEventKind::CellSelected { row, column } => {
                    Some(WidgetryTableSelection::Cell { row, column })
                }
                WidgetryTableEventKind::SelectionCleared => Some(WidgetryTableSelection::None),
                _ => None,
            };
            if let Some(expected) = expected {
                assert_eq!(states.get(event.entity).unwrap().selection(), expected);
            }
            if let WidgetryTableEventKind::ColumnResized { column, width } = event.kind {
                assert_eq!(
                    layouts
                        .get(event.entity)
                        .unwrap()
                        .column_widths()
                        .get(&column),
                    Some(&WidgetryTableColumnWidth::Fixed(width))
                );
            }
            events.0.push(event.kind);
        },
    );
    add_keyboard_dispatch(&mut app);
    (app, source, root, body)
}

#[test]
fn selection_clear_notifications_and_program_cursor_preserve_independent_state() {
    let (mut app, source, root, body) = interaction_fixture();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let row = model.row_id(0).unwrap();
    let column = model.column_id(0).unwrap();
    app.add_observer(
        |event: On<WidgetryTableEvent>, states: Query<&WidgetryTableState>| {
            let expected = match event.kind {
                WidgetryTableEventKind::RowSelected(row) => Some(WidgetryTableSelection::Row(row)),
                WidgetryTableEventKind::ColumnSelected(column) => {
                    Some(WidgetryTableSelection::Column(column))
                }
                WidgetryTableEventKind::CellSelected { row, column } => {
                    Some(WidgetryTableSelection::Cell { row, column })
                }
                WidgetryTableEventKind::SelectionCleared => Some(WidgetryTableSelection::None),
                _ => None,
            };
            if let Some(expected) = expected {
                assert_eq!(states.get(event.entity).unwrap().selection(), expected);
            }
        },
    );
    let cursor = WidgetryTableCell { row, column };
    let focus = app.world().resource::<InputFocus>().get();
    let offset = app.world().get::<ScrollPosition>(body).unwrap().0;
    assert!(WidgetryTable::<u32>::set_focused_cell(app.world_mut(), root, Some(cursor)).unwrap());
    assert!(!WidgetryTable::<u32>::set_focused_cell(app.world_mut(), root, Some(cursor)).unwrap());
    for selection in [
        WidgetryTableSelection::Row(row),
        WidgetryTableSelection::Column(column),
        WidgetryTableSelection::Cell { row, column },
        WidgetryTableSelection::None,
    ] {
        assert!(WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap());
        assert!(!WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap());
        assert_eq!(
            app.world()
                .get::<WidgetryTableState>(root)
                .unwrap()
                .focused_cell(),
            Some(cursor)
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), focus);
        assert_eq!(app.world().get::<ScrollPosition>(body).unwrap().0, offset);
    }
    assert_eq!(app.world().resource::<Events>().0.len(), 4);
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::SelectionCleared)
    );
    assert!(WidgetryTable::<u32>::set_focused_cell(app.world_mut(), root, None).unwrap());
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_row(0);
    let capture = LogCapture::default();
    let error = capture
        .run(|| WidgetryTable::<u32>::set_focused_cell(app.world_mut(), root, Some(cursor)))
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(
        capture
            .records()
            .iter()
            .any(|record| record.level == bevy::log::tracing::Level::ERROR)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        None
    );
}

#[test]
fn program_width_commits_before_notification_and_rejects_invalid_targets() {
    let (mut app, source, root, _) = interaction_fixture();
    let column = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    app.add_observer(
        |event: On<WidgetryTableEvent>, layouts: Query<&WidgetryTableLayout>| {
            if let WidgetryTableEventKind::ColumnResized { column, width } = event.kind {
                assert_eq!(
                    layouts
                        .get(event.entity)
                        .unwrap()
                        .column_widths()
                        .get(&column),
                    Some(&WidgetryTableColumnWidth::Fixed(width))
                );
            }
        },
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    assert!(WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 170.0).unwrap());
    assert!(!WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 170.0).unwrap());
    assert!(WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 1.0).unwrap());
    assert!(!WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 2.0).unwrap());
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTableEventKind::ColumnResized {
                column,
                width: 170.0
            },
            WidgetryTableEventKind::ColumnResized {
                column,
                width: 24.0
            }
        ]
    );
    let logs = LogCapture::default();
    for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let error = logs
            .run(|| WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, width))
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    }
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(0);
    assert!(
        logs.run(|| WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 180.0))
            .is_err()
    );
    assert!(
        logs.run(|| WidgetryTable::<u32>::set_column_width(
            app.world_mut(),
            Entity::PLACEHOLDER,
            column,
            180.0
        ))
        .is_err()
    );
    app.world_mut()
        .entity_mut(source)
        .remove::<WidgetryTableModel<u32>>();
    assert!(
        logs.run(|| WidgetryTable::<u32>::set_column_width(app.world_mut(), root, column, 180.0))
            .is_err()
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        7
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()[&column],
        WidgetryTableColumnWidth::Fixed(24.0)
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
}

#[test]
fn resize_header_identity_loss_cancels_without_width_mutation() {
    let (mut app, source, root, _) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    app.world_mut().trigger(pointer(
        target,
        DragStart {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
    ));
    app.world_mut().flush();
    let header = app.world().get::<ChildOf>(target).unwrap().parent();
    app.world_mut()
        .entity_mut(header)
        .remove::<WidgetryTableColumnHeader>();
    app.update();
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTableEventKind::ColumnResizeStart(column),
            WidgetryTableEventKind::ColumnResizeCancel(column)
        ]
    );
    assert!(
        !app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()
            .contains_key(&column)
    );
}

#[test]
fn final_width_observer_changes_are_checked_before_terminal_notification() {
    for action in 0..4 {
        let (mut app, source, root, _) = interaction_fixture();
        let (target, column) = handle(&mut app, source, 0);
        app.add_observer(
            move |event: On<WidgetryTableEvent>, mut commands: Commands| {
                if matches!(event.kind, WidgetryTableEventKind::ColumnResized { .. }) {
                    commands.queue(move |world: &mut World| match action {
                        0 => {
                            world
                                .get_mut::<WidgetryTableModel<u32>>(source)
                                .unwrap()
                                .remove_column(0);
                        }
                        1 => {
                            world.entity_mut(target).despawn();
                        }
                        2 => {
                            world.entity_mut(root).insert(InteractionDisabled);
                        }
                        _ => {
                            world.entity_mut(root).despawn();
                        }
                    });
                }
            },
        );
        app.world_mut().trigger(pointer(
            target,
            DragStart {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
        ));
        app.world_mut().flush();
        app.world_mut().trigger(pointer(
            target,
            DragEnd {
                button: PointerButton::Primary,
                distance: Vec2::new(30.0, 0.0),
            },
        ));
        app.world_mut().flush();
        let mut expected = vec![
            WidgetryTableEventKind::ColumnResizeStart(column),
            WidgetryTableEventKind::ColumnResized {
                column,
                width: 150.0,
            },
        ];
        if action != 3 {
            expected.push(WidgetryTableEventKind::ColumnResizeCancel(column));
        }
        assert_eq!(app.world().resource::<Events>().0, expected);
        if action != 3 {
            assert_eq!(
                app.world()
                    .get::<WidgetryTableLayout>(root)
                    .unwrap()
                    .column_widths()[&column],
                WidgetryTableColumnWidth::Fixed(150.0)
            );
        }
    }
}

fn cell(app: &mut App, source: Entity, row: usize, column: usize) -> Entity {
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let pair = (model.row_id(row).unwrap(), model.column_id(column).unwrap());
    app.world_mut()
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(app.world())
        .find(|(_, c)| (c.row, c.column) == pair)
        .unwrap()
        .0
}

#[test]
fn content_and_headers_select_logical_ids_without_duplicate_notifications() {
    let (mut app, _, root, _) = interaction_fixture();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    let (entity, cell) = app
        .world_mut()
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(app.world())
        .map(|(e, c)| (e, *c))
        .next()
        .unwrap();
    let content = app.world().get::<Children>(entity).unwrap()[0];
    app.world_mut().trigger(primary_click(content));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Cell {
            row: cell.row,
            column: cell.column
        }
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![WidgetryTableEventKind::CellSelected {
            row: cell.row,
            column: cell.column
        }]
    );
    app.world_mut().trigger(primary_click(content));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Events>().0.len(), 1);
    let column = app
        .world_mut()
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(app.world())
        .find(|(_, h)| h.column == cell.column)
        .unwrap()
        .0;
    app.world_mut().trigger(primary_click(column));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Column(cell.column)
    );
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryTableRowHeader)>()
        .iter(app.world())
        .find(|(_, h)| h.row == cell.row)
        .unwrap()
        .0;
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Row(cell.row)
    );
    assert_eq!(
        app.world().resource::<Events>().0[1..],
        [
            WidgetryTableEventKind::ColumnSelected(cell.column),
            WidgetryTableEventKind::RowSelected(cell.row)
        ]
    );
}

#[test]
fn keyboard_focus_guards_and_boundaries_preserve_single_selection() {
    let (mut app, source, root, _) = interaction_fixture();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        None
    );
    let entity = cell(&mut app, source, 0, 0);
    let first = *app.world().get::<WidgetryTableCell>(entity).unwrap();
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    for (key, row, column) in [
        (KeyCode::ArrowRight, 0, 1),
        (KeyCode::ArrowDown, 1, 1),
        (KeyCode::ArrowLeft, 1, 0),
        (KeyCode::ArrowUp, 0, 0),
        (KeyCode::ArrowUp, 0, 0),
        (KeyCode::ArrowLeft, 0, 0),
    ] {
        press_key(&mut app, window, key);
        let cursor = app
            .world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell()
            .unwrap();
        let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
        assert_eq!(
            (
                model.row_index(cursor.row),
                model.column_index(cursor.column)
            ),
            (Some(row), Some(column))
        );
        assert_eq!(
            app.world()
                .get::<WidgetryTableState>(root)
                .unwrap()
                .selection(),
            WidgetryTableSelection::Cell {
                row: first.row,
                column: first.column
            }
        );
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(first)
    );
    let disabled_target = cell(&mut app, source, 1, 1);
    app.world_mut().trigger(primary_click(disabled_target));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(first)
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().resource_mut::<InputFocus>().clear();
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(first)
    );
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    for _ in 0..25 {
        press_key(&mut app, window, KeyCode::ArrowDown);
    }
    for _ in 0..15 {
        press_key(&mut app, window, KeyCode::ArrowRight);
    }
    let cursor = app
        .world()
        .get::<WidgetryTableState>(root)
        .unwrap()
        .focused_cell()
        .unwrap();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    assert_eq!(
        (
            model.row_index(cursor.row),
            model.column_index(cursor.column)
        ),
        (Some(19), Some(9))
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_columns();
    for key in [
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ] {
        press_key(&mut app, window, key);
    }
    let state = app.world().get::<WidgetryTableState>(root).unwrap();
    assert_eq!(state.focused_cell(), None);
    assert_eq!(state.selection(), WidgetryTableSelection::None);
    assert_eq!(app.world().resource::<Events>().0.len(), 1);
}

#[test]
fn first_pointer_focus_preserves_scrolled_click_target() {
    let (mut app, source, root, body) = interaction_fixture();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    scroll(&mut app, body, Vec2::new(480.0, 280.0));
    let target = cell(&mut app, source, 10, 4);
    let pair = *app.world().get::<WidgetryTableCell>(target).unwrap();
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::new(480.0, 280.0)
    );
    assert!(app.world().entities().contains(target));
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Cell {
            row: pair.row,
            column: pair.column
        }
    );
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![WidgetryTableEventKind::CellSelected {
            row: pair.row,
            column: pair.column
        }]
    );
    // 已 focused 的再次 press 不产生 FocusGained。
    // 来源标记仍须在本帧结束时清理。
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    app.update();
    scroll(&mut app, body, Vec2::new(480.0, 280.0));
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    app.world_mut().trigger(AcquireFocus {
        focused_entity: root,
        window,
    });
    app.world_mut().flush();
    app.update();
    app.update();
    assert!(projection(&mut app, root).contains_key(&(pair.row, pair.column)));
}

#[test]
fn scroll_and_model_changes_keep_logical_identity_independent_of_cells() {
    let (mut app, source, root, body) = interaction_fixture();
    let first = cell(&mut app, source, 0, 0);
    let pair = *app.world().get::<WidgetryTableCell>(first).unwrap();
    app.world_mut().trigger(primary_click(first));
    app.world_mut().flush();
    scroll(&mut app, body, Vec2::new(480.0, 280.0));
    app.update();
    assert!(!app.world().entities().contains(first));
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(pair)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Cell {
            row: pair.row,
            column: pair.column
        }
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::new(480.0, 280.0)
    );
    let current = cell(&mut app, source, 10, 4);
    app.world_mut().trigger(primary_click(current));
    app.world_mut().flush();
    let selected = *app.world().get::<WidgetryTableCell>(current).unwrap();
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::CellSelected {
            row: selected.row,
            column: selected.column
        })
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .move_row(10, 11);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(selected)
    );
    assert!(
        projection(&mut app, root)
            .get(&(selected.row, selected.column))
            .unwrap()
            .1
            .ends_with("10/4")
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_row(11);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        None
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
}

#[test]
fn programmatic_selection_notifies_and_invalid_ids_preserve_state() {
    let (mut app, source, root, _) = interaction_fixture();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let row = model.row_id(0).unwrap();
    let column = model.column_id(0).unwrap();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    assert!(
        WidgetryTable::<u32>::set_selection(
            app.world_mut(),
            root,
            WidgetryTableSelection::Row(row)
        )
        .unwrap()
    );
    assert!(
        !WidgetryTable::<u32>::set_selection(
            app.world_mut(),
            root,
            WidgetryTableSelection::Row(row)
        )
        .unwrap()
    );
    assert!(
        WidgetryTable::<u32>::set_selection(
            app.world_mut(),
            root,
            WidgetryTableSelection::Column(column)
        )
        .unwrap()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        None
    );
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTableEventKind::RowSelected(row),
            WidgetryTableEventKind::ColumnSelected(column)
        ]
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_row(0);
    let logs = LogCapture::default();
    let error = logs
        .run(|| {
            WidgetryTable::<u32>::set_selection(
                app.world_mut(),
                root,
                WidgetryTableSelection::Row(row),
            )
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(error.to_string().contains("stale"));
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Column(column)
    );
    let error = logs
        .run(|| {
            WidgetryTable::<u32>::set_selection(
                app.world_mut(),
                Entity::PLACEHOLDER,
                WidgetryTableSelection::None,
            )
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    app.world_mut()
        .entity_mut(source)
        .remove::<WidgetryTableModel<u32>>();
    let error = logs
        .run(|| {
            WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::None)
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Column(column)
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        3
    );
}

#[test]
fn selection_focus_and_disabled_styles_preserve_content() {
    let (mut app, source, root, _) = interaction_fixture();
    let entity = cell(&mut app, source, 0, 0);
    let content = app.world().get::<Children>(entity).unwrap()[0];
    app.world_mut().trigger(primary_click(content));
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<bevy::ui::Selected>(entity).is_some());
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        bevy_widgetry_core::ThemeMode::Dark
            .colors()
            .item_background_selected
    );
    assert_eq!(
        app.world().get::<BorderColor>(entity).unwrap().top,
        bevy_widgetry_core::ThemeMode::Dark
            .colors()
            .control_border_active
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        app.world().get::<BorderColor>(entity).unwrap().top,
        bevy_widgetry_core::ThemeMode::Dark.colors().control_border
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert!(!app.world().get::<Pickable>(entity).unwrap().is_hoverable);
    assert!(
        !app.world()
            .get::<bevy::picking::hover::Hovered>(entity)
            .unwrap()
            .0
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        bevy_widgetry_core::ThemeMode::Dark
            .colors()
            .control_background_disabled
    );
    assert_eq!(app.world().get::<Children>(entity).unwrap()[0], content);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    let next = cell(&mut app, source, 1, 1);
    app.world_mut().trigger(primary_click(next));
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<bevy::ui::Selected>(next).is_some());
    assert!(app.world().get::<bevy::ui::Selected>(entity).is_none());
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
}

fn handle(app: &mut App, source: Entity, column: usize) -> (Entity, WidgetryTableColumnId) {
    let id = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(column)
        .unwrap();
    let header = app
        .world_mut()
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(app.world())
        .find(|(_, h)| h.column == id)
        .unwrap()
        .0;
    let handle = app
        .world()
        .get::<Children>(header)
        .unwrap()
        .iter()
        .find(|&entity| {
            app.world().get::<Node>(entity).is_some_and(|node| {
                node.position_type == PositionType::Absolute
                    && node.right == px(0)
                    && node.width == px(6)
            })
        })
        .unwrap();
    (handle, id)
}

fn pointer<E: Clone + Reflect + std::fmt::Debug>(target: Entity, event: E) -> Pointer<E> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 600,
                height: 400,
            },
            position: Vec2::ZERO,
        },
        event,
        target,
    )
}

#[test]
fn resize_drag_events_clamp_and_end_once_on_interruptions() {
    let (mut app, source, root, _) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    assert!(app.world().resource::<Events>().0.is_empty());
    let start = || DragStart {
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    };
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    app.world_mut().trigger(pointer(
        target,
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::new(50.0, 0.0),
            delta: Vec2::new(50.0, 0.0),
        },
    ));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()[&column],
        WidgetryTableColumnWidth::Fixed(170.0)
    );
    app.world_mut().trigger(pointer(
        target,
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::new(-500.0, 0.0),
            delta: Vec2::new(-550.0, 0.0),
        },
    ));
    app.world_mut().flush();
    app.world_mut().trigger(pointer(
        target,
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::new(-500.0, 0.0),
        },
    ));
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTableEventKind::ColumnResizeStart(column),
            WidgetryTableEventKind::ColumnResized {
                column,
                width: 170.0
            },
            WidgetryTableEventKind::ColumnResized {
                column,
                width: 24.0
            },
            WidgetryTableEventKind::ColumnResizeEnd(column)
        ]
    );
    app.update();
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    app.world_mut()
        .trigger(pointer(target, primary_cancel(target).event));
    app.world_mut().flush();
    app.world_mut()
        .trigger(pointer(target, primary_cancel(target).event));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Events>().0.len(), 6);
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    app.update();
    app.world_mut().trigger(pointer(
        target,
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::new(80.0, 0.0),
        },
    ));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Events>().0.len(), 8);
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::ColumnResizeCancel(column))
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()[&column],
        WidgetryTableColumnWidth::Fixed(24.0)
    );
}

#[test]
fn acquired_focus_and_non_actions_obey_input_guards() {
    let (mut app, source, root, body) = interaction_fixture();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.world_mut().trigger(AcquireFocus {
        focused_entity: root,
        window,
    });
    app.world_mut().flush();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    let first = app
        .world()
        .get::<WidgetryTableState>(root)
        .unwrap()
        .focused_cell()
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    for (state, key, repeat) in [
        (ButtonState::Released, KeyCode::ArrowRight, false),
        (ButtonState::Pressed, KeyCode::Enter, false),
        (ButtonState::Pressed, KeyCode::ShiftLeft, false),
    ] {
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(first)
    );
    let corner = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&entity| app.world().get::<WidgetryTableCorner>(entity).is_some())
        .unwrap();
    app.world_mut().trigger(primary_click(corner));
    app.world_mut().flush();
    let next = cell(&mut app, source, 1, 1);
    let mut secondary = primary_click(next);
    secondary.event.button = PointerButton::Secondary;
    app.world_mut().trigger(secondary);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    app.world_mut().trigger(primary_click(next));
    app.world_mut().flush();
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::ArrowRight,
            logical_key: Key::ArrowRight,
            state: ButtonState::Pressed,
            text: None,
            repeat: true,
            window,
        },
    );
    app.update();
    let cursor = app
        .world()
        .get::<WidgetryTableState>(root)
        .unwrap()
        .focused_cell()
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_index(cursor.column),
        Some(2)
    );
    assert!(app.world().get::<ScrollPosition>(body).unwrap().0.x > 0.0);
    assert!(projection(&mut app, root).contains_key(&(cursor.row, cursor.column)));
    assert_eq!(app.world().resource::<Events>().0.len(), 1);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(1);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(cursor)
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_rows();
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        None
    );
}

#[test]
fn resize_handles_scale_flexible_width_and_model_lifecycle() {
    let (mut app, source, root, _) = interaction_fixture();
    *app.world_mut().resource_mut::<bevy::ui::UiScale>() = bevy::ui::UiScale(2.0);
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        for _ in 0..9 {
            model.remove_column(1);
        }
    }
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .default_column_width = WidgetryTableColumnWidth::Flexible(1.0);
    app.update();
    app.update();
    let (target, column) = handle(&mut app, source, 0);
    let shell = app.world().get::<ChildOf>(target).unwrap().parent();
    let initial = app.world().get::<ComputedNode>(shell).unwrap().size().x / 2.0;
    let start = || DragStart {
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    };
    let mut secondary = pointer(target, start());
    secondary.event.button = PointerButton::Secondary;
    app.world_mut().trigger(secondary);
    app.world_mut().flush();
    assert!(app.world().resource::<Events>().0.is_empty());
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    let mut other = pointer(
        target,
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::splat(40.0),
            delta: Vec2::splat(40.0),
        },
    );
    other.pointer_id = PointerId::Touch(1);
    app.world_mut().trigger(other);
    app.world_mut().flush();
    app.world_mut().trigger(pointer(
        target,
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::splat(f32::NAN),
            delta: Vec2::ZERO,
        },
    ));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Events>().0.len(), 1);
    app.world_mut().trigger(pointer(
        target,
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::new(40.0, 0.0),
        },
    ));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()[&column],
        WidgetryTableColumnWidth::Fixed(initial + 20.0)
    );
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::ColumnResizeEnd(column))
    );
    app.update();
    app.world_mut().trigger(pointer(target, start()));
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(0);
    app.update();
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::ColumnResizeCancel(column))
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 5);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("new".to_owned()),
            (),
            |row: &u32, _| WidgetryTableCellValue::new(row.to_string()),
        ))
        .unwrap();
    app.update();
    let (new_handle, new_column) = handle(&mut app, source, 0);
    assert_ne!(new_column, column);
    app.world_mut().trigger(pointer(new_handle, start()));
    app.world_mut().flush();
    let before = app.world().resource::<Events>().0.len();
    app.world_mut().despawn(root);
    app.update();
    assert_eq!(app.world().resource::<Events>().0.len(), before);
    assert!(app.world().get::<WidgetryTableModel<u32>>(source).is_some());
}

#[test]
fn header_replacement_ends_resize_without_waiting_another_update() {
    let (mut app, source, root, _) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    app.world_mut().trigger(pointer(
        target,
        DragStart {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
    ));
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .set_header(0, WidgetryTableHeaderValue::new("replacement".to_owned()))
        .unwrap();
    app.update();
    assert!(!app.world().entities().contains(target));
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTableEventKind::ColumnResizeStart(column),
            WidgetryTableEventKind::ColumnResizeCancel(column)
        ]
    );
    let (new_handle, _) = handle(&mut app, source, 0);
    app.world_mut().trigger(pointer(
        new_handle,
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::new(60.0, 0.0),
            delta: Vec2::new(60.0, 0.0),
        },
    ));
    app.world_mut().flush();
    assert!(
        !app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()
            .contains_key(&column)
    );
}

#[test]
fn scrolling_resize_header_out_applies_cancel_commands_before_projection() {
    let (mut app, source, root, body) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    app.add_observer(
        move |event: On<WidgetryTableEvent>, mut commands: Commands| {
            if matches!(event.kind, WidgetryTableEventKind::ColumnResizeCancel(_)) {
                commands.queue(move |world: &mut World| {
                    world
                        .get_mut::<WidgetryTableModel<u32>>(source)
                        .unwrap()
                        .clear_rows();
                });
            }
        },
    );
    app.world_mut().trigger(pointer(
        target,
        DragStart {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
    ));
    app.world_mut().flush();
    scroll(&mut app, body, Vec2::new(600.0, 0.0));
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&WidgetryTableEventKind::ColumnResizeCancel(column))
    );
    assert!(!app.world().entities().contains(target));
    assert!(projection(&mut app, root).is_empty());
}

#[test]
fn header_replacement_cancel_observer_width_applies_before_layout() {
    let (mut app, source, root, _) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    app.add_observer(
        move |event: On<WidgetryTableEvent>, mut commands: Commands| {
            if matches!(event.kind, WidgetryTableEventKind::ColumnResizeCancel(_)) {
                commands.queue(move |world: &mut World| {
                    WidgetryTable::<u32>::set_column_width(world, root, column, 200.0).unwrap();
                });
            }
        },
    );
    app.world_mut().trigger(pointer(
        target,
        DragStart {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
    ));
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .set_header(0, WidgetryTableHeaderValue::new("replacement".to_owned()))
        .unwrap();
    app.update();
    let (new_handle, _) = handle(&mut app, source, 0);
    let header = app.world().get::<ChildOf>(new_handle).unwrap().parent();
    assert_eq!(
        app.world().get::<ComputedNode>(header).unwrap().size().x,
        200.0
    );
    for ((_, cell_column), (entity, _)) in projection(&mut app, root) {
        if cell_column == column {
            assert_eq!(
                app.world().get::<ComputedNode>(entity).unwrap().size().x,
                200.0
            );
        }
    }
}

#[test]
fn resize_end_observer_model_mutation_uses_current_axes() {
    for (clear_rows, replace_header) in [(true, false), (false, false), (true, true), (false, true)]
    {
        let (mut app, source, root, _) = interaction_fixture();
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let selected = cell(&mut app, source, 0, 0);
        app.world_mut().trigger(primary_click(selected));
        app.world_mut().flush();
        app.update();
        app.world_mut().resource_mut::<Events>().0.clear();
        app.add_observer(
            move |event: On<WidgetryTableEvent>,
                  mut models: Query<&mut WidgetryTableModel<u32>>| {
                if matches!(event.kind, WidgetryTableEventKind::ColumnResizeCancel(_)) {
                    let mut model = models.get_mut(source).unwrap();
                    if clear_rows {
                        model.clear_rows();
                    } else {
                        model.remove_column(0);
                    }
                }
            },
        );
        let (target, column) = handle(&mut app, source, 0);
        app.world_mut().trigger(pointer(
            target,
            DragStart {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
        ));
        app.world_mut().flush();
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        if replace_header {
            model
                .set_header(0, WidgetryTableHeaderValue::new("replacement".to_owned()))
                .unwrap();
        } else {
            model.remove_column(0);
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        let current = projection(&mut app, root);
        let state = app.world().get::<WidgetryTableState>(root).unwrap();
        assert_eq!(state.selection(), WidgetryTableSelection::None);
        assert_eq!(state.focused_cell(), None);
        if clear_rows {
            assert!(current.is_empty());
        } else {
            assert!(!current.is_empty());
            let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
            for ((row, column), _) in current {
                assert!(model.row_index(row).is_some());
                assert!(model.column_index(column).is_some());
            }
        }
        assert_eq!(
            app.world().resource::<Events>().0,
            vec![
                WidgetryTableEventKind::ColumnResizeStart(column),
                WidgetryTableEventKind::ColumnResizeCancel(column),
            ]
        );
    }
}

#[test]
fn resize_end_observer_commands_despawn_cancel_projection() {
    for (with_cursor, replace_header) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let (mut app, source, root, _) = interaction_fixture();
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        if with_cursor {
            let target = cell(&mut app, source, 0, 0);
            app.world_mut().trigger(primary_click(target));
            app.world_mut().flush();
        }
        app.add_observer(
            move |event: On<WidgetryTableEvent>, mut commands: Commands| {
                if matches!(event.kind, WidgetryTableEventKind::ColumnResizeCancel(_)) {
                    commands.entity(root).despawn();
                }
            },
        );
        let (target, column) = handle(&mut app, source, 0);
        app.world_mut().trigger(pointer(
            target,
            DragStart {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
        ));
        app.world_mut().flush();
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        if replace_header {
            model
                .set_header(0, WidgetryTableHeaderValue::new("replacement".to_owned()))
                .unwrap();
        } else {
            model.remove_column(0);
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert!(!app.world().entities().contains(root));
        assert!(app.world().get::<WidgetryTableModel<u32>>(source).is_some());
        assert_eq!(
            app.world().resource::<Events>().0.last(),
            Some(&WidgetryTableEventKind::ColumnResizeCancel(column))
        );
    }
}

#[test]
fn hover_style_priority_and_theme_follow_public_state() {
    let (mut app, source, root, _) = interaction_fixture();
    let entity = cell(&mut app, source, 0, 0);
    let content = app.world().get::<Children>(entity).unwrap()[0];
    app.init_resource::<HoverTarget>().add_systems(
        PreUpdate,
        inject_hover
            .after(bevy::picking::hover::generate_hovermap)
            .before(bevy::picking::hover::update_is_hovered),
    );
    app.world_mut().trigger(primary_click(content));
    app.world_mut().flush();
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(content);
    app.update();
    assert!(
        app.world()
            .get::<bevy::picking::hover::Hovered>(entity)
            .unwrap()
            .0
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        bevy_widgetry_core::ThemeMode::Dark
            .colors()
            .item_background_hovered
    );
    app.world_mut()
        .get_mut::<WidgetryTableStyle>(root)
        .unwrap()
        .cell
        .hovered_background = Some(Color::srgb(0.2, 0.4, 0.6));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        Color::srgb(0.2, 0.4, 0.6)
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(!app.world().get::<Pickable>(content).unwrap().is_hoverable);
    app.update();
    assert!(
        !app.world()
            .get::<bevy::picking::hover::Hovered>(entity)
            .unwrap()
            .0
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().resource_mut::<HoverTarget>().0 = None;
    *app.world_mut()
        .resource_mut::<bevy_widgetry_core::ThemeMode>() = bevy_widgetry_core::ThemeMode::Light;
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        bevy_widgetry_core::ThemeMode::Light
            .colors()
            .item_background_selected
    );
    assert_eq!(app.world().get::<Children>(entity).unwrap()[0], content);
}

#[test]
fn tab_navigation_enters_table_through_caller_group() {
    let (mut app, source, root, _) = common::uninitialized_fixture(20, 10);
    add_keyboard_dispatch(&mut app);
    app.init_resource::<Events>().add_observer(
        |event: On<WidgetryTableEvent>, mut events: ResMut<Events>| events.0.push(event.kind),
    );
    app.init_resource::<ButtonInput<KeyCode>>();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let group = app
        .world_mut()
        .spawn_scene(bsn! { TabGroup::default() Node::default() })
        .unwrap()
        .id();
    app.world_mut().entity_mut(group).add_child(root);
    app.update();
    let mut redraw = MessageCursor::<RequestRedraw>::default();
    redraw.clear(app.world().resource::<Messages<RequestRedraw>>());
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    assert!(
        redraw
            .read(app.world().resource::<Messages<RequestRedraw>>())
            .count()
            > 0
    );
    app.update();
    let state = app.world().get::<WidgetryTableState>(root).unwrap();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    assert_eq!(
        state.focused_cell(),
        Some(WidgetryTableCell {
            row: model.row_id(0).unwrap(),
            column: model.column_id(0).unwrap(),
        })
    );
    assert_eq!(state.selection(), WidgetryTableSelection::None);
    assert!(app.world().resource::<Events>().0.is_empty());
}

#[test]
fn resize_window_logical_distance_ignores_native_dpi() {
    for scale in [1.0, 1.5, 2.0] {
        let (mut app, source, root, _) = interaction_fixture();
        *app.world_mut().resource_mut::<bevy::ui::UiScale>() = bevy::ui::UiScale(scale);
        for mut camera in app
            .world_mut()
            .query::<&mut Camera>()
            .iter_mut(app.world_mut())
        {
            camera.computed.target_info.as_mut().unwrap().scale_factor = 2.0;
        }
        app.update();
        app.update();
        let (target, column) = handle(&mut app, source, 0);
        assert_eq!(
            app.world()
                .get::<ComputedNode>(target)
                .unwrap()
                .inverse_scale_factor(),
            1.0 / (2.0 * scale)
        );
        app.world_mut().trigger(pointer(
            target,
            DragStart {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
        ));
        app.world_mut().flush();
        app.world_mut().trigger(pointer(
            target,
            DragEnd {
                button: PointerButton::Primary,
                distance: Vec2::new(100.0, 0.0),
            },
        ));
        app.world_mut().flush();
        let WidgetryTableColumnWidth::Fixed(width) = app
            .world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()[&column]
        else {
            panic!("resize 必须建立 Fixed width");
        };
        assert!((width - (120.0 + 100.0 / scale)).abs() < 0.0001);
    }
}

#[test]
fn real_pointer_resize_terminates_once_after_cancel_or_invalid_source() {
    use bevy::picking::pointer::{PointerAction, PointerLocation};
    use bevy_widgetry_test_utils::{pointer_ids, queue_pointer};
    for id in pointer_ids() {
        for failure in ["cancel", "location", "pointer", "window", "release"] {
            let (mut app, source, root, _) = interaction_fixture();
            let window = app
                .world_mut()
                .spawn((
                    Window {
                        resolution: (600, 400).into(),
                        ..default()
                    },
                    PrimaryWindow,
                ))
                .id();
            let camera = app
                .world_mut()
                .query_filtered::<Entity, With<Camera>>()
                .single(app.world())
                .unwrap();
            app.world_mut()
                .entity_mut(camera)
                .insert(bevy::camera::RenderTarget::Window(
                    bevy::window::WindowRef::Entity(window),
                ));
            app.world_mut()
                .entity_mut(root)
                .insert(UiTargetCamera(camera));
            app.world_mut()
                .resource_mut::<bevy::picking::input::PointerInputSettings>()
                .is_mouse_enabled = false;
            if id != PointerId::Mouse {
                app.world_mut().spawn(id);
            }
            app.update();
            app.update();
            let (handle, column) = handle(&mut app, source, 0);
            let mut location = Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: app
                    .world()
                    .get::<UiGlobalTransform>(handle)
                    .unwrap()
                    .translation,
            };
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: location.position,
                },
            );
            app.update();
            assert!(app.world().resource::<HoverMap>()[&id].contains_key(&handle));
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Press(PointerButton::Primary),
            );
            app.update();
            location.position.x += 20.0;
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: Vec2::new(20.0, 0.0),
                },
            );
            app.update();
            assert!(
                app.world()
                    .resource::<Events>()
                    .0
                    .contains(&WidgetryTableEventKind::ColumnResizeStart(column))
            );

            let pointer = app
                .world_mut()
                .query::<(Entity, &PointerId)>()
                .iter(app.world())
                .find(|(_, pointer)| **pointer == id)
                .unwrap()
                .0;
            if failure == "cancel" {
                location.position = Vec2::new(550.0, 350.0);
                queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Move {
                        delta: Vec2::splat(100.0),
                    },
                );
                app.update();
            }
            let width = app
                .world()
                .get::<WidgetryTableLayout>(root)
                .unwrap()
                .column_widths()[&column];
            match failure {
                "cancel" => queue_pointer(&mut app, id, location.clone(), PointerAction::Cancel),
                "location" => {
                    app.world_mut()
                        .get_mut::<PointerLocation>(pointer)
                        .unwrap()
                        .location = None
                }
                "pointer" => {
                    app.world_mut().despawn(pointer);
                }
                "window" => {
                    app.world_mut().despawn(window);
                }
                _ => queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Release(PointerButton::Primary),
                ),
            }
            app.update();
            app.update();
            let events = &app.world().resource::<Events>().0;
            let cancels = events
                .iter()
                .filter(|kind| **kind == WidgetryTableEventKind::ColumnResizeCancel(column))
                .count();
            let ends = events
                .iter()
                .filter(|kind| **kind == WidgetryTableEventKind::ColumnResizeEnd(column))
                .count();
            assert_eq!(
                (cancels, ends),
                if failure == "release" { (0, 1) } else { (1, 0) },
                "{id:?}/{failure}"
            );
            {
                assert_eq!(
                    app.world()
                        .get::<WidgetryTableLayout>(root)
                        .unwrap()
                        .column_widths()[&column],
                    width
                );
            }
        }
    }
}

#[test]
fn stale_target_terminal_does_not_finish_current_resize() {
    let (mut app, source, root, _) = interaction_fixture();
    let (target, column) = handle(&mut app, source, 0);
    let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut().trigger(pointer(
        target,
        DragStart {
            button: PointerButton::Primary,
            hit,
        },
    ));
    app.world_mut().flush();
    app.world_mut().trigger(primary_cancel(target));
    app.world_mut().flush();
    assert!(
        !app.world()
            .resource::<Events>()
            .0
            .contains(&WidgetryTableEventKind::ColumnResizeCancel(column))
    );
    app.world_mut().trigger(pointer(
        target,
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        },
    ));
    app.update();
    assert_eq!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .filter(|kind| **kind == WidgetryTableEventKind::ColumnResizeEnd(column))
            .count(),
        1
    );
    assert!(app.world().get::<WidgetryTableLayout>(root).is_some());
}
