#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod common;
use bevy::prelude::*;
use bevy_widgetry_table::*;

fn register(app: &mut App) {
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(|s: &String| Ok(s.clone())));
}

#[test]
fn copying_immediately_after_projection_or_model_changes_uses_repaired_selection() {
    let (mut app, source, root, _) = common::fixture(3, 2);
    register(&mut app);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let rows = [0, 1, 2].map(|i| model.row_id(i).unwrap());
    let columns = [0, 1].map(|i| model.column_id(i).unwrap());
    for selection in [
        WidgetryTableSelection::Cell {
            row: rows[0],
            column: columns[0],
        },
        WidgetryTableSelection::Row(rows[0]),
        WidgetryTableSelection::Rows {
            anchor: rows[0],
            end: rows[2],
        },
        WidgetryTableSelection::Cells {
            anchor: WidgetryTableCell {
                row: rows[0],
                column: columns[0],
            },
            end: WidgetryTableCell {
                row: rows[2],
                column: columns[1],
            },
        },
    ] {
        WidgetryTable::<u32>::reset_view(app.world_mut(), root).unwrap();
        WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap();
        WidgetryTable::<u32>::update_view(
            app.world_mut(),
            root,
            WidgetryTableViewUpdate::default(),
        )
        .unwrap();
        assert_eq!(
            WidgetryTable::<u32>::selection_tsv(app.world_mut(), root).unwrap(),
            None
        );
        assert!(!WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
    }
    WidgetryTable::<u32>::reset_view(app.world_mut(), root).unwrap();
    WidgetryTable::<u32>::set_selection(
        app.world_mut(),
        root,
        WidgetryTableSelection::Rows {
            anchor: rows[0],
            end: rows[2],
        },
    )
    .unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_row(0);
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), root).unwrap(),
        None
    );
    WidgetryTable::<u32>::set_selection(
        app.world_mut(),
        root,
        WidgetryTableSelection::Column(columns[1]),
    )
    .unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(1);
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), root).unwrap(),
        None
    );
}

#[test]
fn tsv_fields_round_trip_tabs_newlines_quotes_unicode_and_empty_values() {
    let fragments = ["", "plain", " \t ", "\r\n", "\"", "中🙂"];
    let values = fragments
        .iter()
        .flat_map(|a| fragments.iter().map(move |b| format!("{a}{b}")))
        .collect::<Vec<_>>();
    let (mut app, source, root, _) = common::fixture(values.len() as u32, 1);
    register(&mut app);
    let expected = values.clone();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .set_column(
            0,
            WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("Excluded header"),
                values,
                |row: &u32, values: &Vec<String>| {
                    WidgetryTableCellValue::new(values[*row as usize].clone())
                },
            ),
        )
        .unwrap();
    WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::All)
        .unwrap();
    let tsv = WidgetryTable::<u32>::selection_tsv(app.world_mut(), root)
        .unwrap()
        .unwrap();
    let mut fields = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut chars = tsv.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                value.push('"');
            }
            '"' => quoted = !quoted,
            '\t' if !quoted => panic!("unexpected second column"),
            '\n' if !quoted => fields.push(std::mem::take(&mut value)),
            ch => value.push(ch),
        }
    }
    assert!(!quoted);
    fields.push(value);
    assert_eq!(fields, expected);
}

#[test]
fn logical_ranges_use_complete_filtered_view_order_without_headers_or_shells() {
    let (mut app, source, root, body) = common::fixture(100, 20);
    assert!(common::projection(&mut app, root).len() < 2000);
    register(&mut app);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let rows = [99, 50, 0].map(|i| model.row_id(i).unwrap());
    let cols = [0, 1, 19].map(|i| model.column_id(i).unwrap());
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate {
            rows: rows.to_vec(),
            ..default()
        },
    )
    .unwrap();
    common::scroll(&mut app, body, Vec2::new(1200.0, 40.0));
    for (selection, row_indices, column_indices) in [
        (
            WidgetryTableSelection::Cell {
                row: rows[0],
                column: cols[2],
            },
            vec![99],
            vec![19],
        ),
        (
            WidgetryTableSelection::Row(rows[1]),
            vec![50],
            (0..20).collect(),
        ),
        (
            WidgetryTableSelection::Column(cols[1]),
            vec![99, 50, 0],
            vec![1],
        ),
        (
            WidgetryTableSelection::Rows {
                anchor: rows[2],
                end: rows[0],
            },
            vec![99, 50, 0],
            (0..20).collect(),
        ),
        (
            WidgetryTableSelection::Columns {
                anchor: cols[2],
                end: cols[1],
            },
            vec![99, 50, 0],
            (1..20).collect(),
        ),
        (
            WidgetryTableSelection::Cells {
                anchor: WidgetryTableCell {
                    row: rows[2],
                    column: cols[2],
                },
                end: WidgetryTableCell {
                    row: rows[1],
                    column: cols[1],
                },
            },
            vec![50, 0],
            (1..20).collect(),
        ),
        (
            WidgetryTableSelection::All,
            vec![99, 50, 0],
            (0..20).collect(),
        ),
    ] {
        WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap();
        let expected = row_indices
            .into_iter()
            .map(|r| {
                column_indices
                    .iter()
                    .map(|c| format!("{r}/{c}"))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            WidgetryTable::<u32>::selection_tsv(app.world_mut(), root).unwrap(),
            Some(expected)
        );
    }
}

#[test]
fn formatter_overrides_preserve_fields_and_fail_with_stable_cell_identity() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    register(&mut app);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let row = model.row_id(0).unwrap();
    let column = model.column_id(1).unwrap();
    app.register_table_column_copy_formatter(
        source,
        column,
        WidgetryTableCopyFormatter::new(|_: &String| Ok(" \t\r\n\"last\" ".into())),
    );
    WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::Row(row))
        .unwrap();
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), root)
            .unwrap()
            .unwrap(),
        "0/0\t\" \t\r\n\"\"last\"\" \""
    );
    app.register_table_column_copy_formatter(
        source,
        column,
        WidgetryTableCopyFormatter::new(|_: &String| {
            Err(BevyError::error("business formatting failure"))
        }),
    );
    let error = WidgetryTable::<u32>::selection_tsv(app.world_mut(), root).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    let text = error.to_string();
    assert!(
        text.contains(&format!("{row:?}"))
            && text.contains(&format!("{column:?}"))
            && text.contains("business formatting failure"),
        "{text}"
    );
}

#[test]
fn copy_is_atomic_for_missing_mismatched_and_failing_formatters_and_invalid_sources() {
    use bevy::clipboard::Clipboard;
    use bevy_widgetry_test_utils::LogCapture;
    let (mut app, source, root, _) = common::fixture(2, 2);
    let old = app
        .world_mut()
        .resource_mut::<Clipboard>()
        .fetch_text()
        .poll_result();
    app.world_mut()
        .resource_mut::<Clipboard>()
        .set_text("keep previous clipboard")
        .unwrap();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let column = model.column_id(1).unwrap();
    assert!(!WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
    WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::All)
        .unwrap();
    for step in 0..3 {
        if step > 0 {
            register(&mut app);
            if step == 1 {
                app.register_table_column_copy_formatter(
                    source,
                    column,
                    WidgetryTableCopyFormatter::new(|_: &bool| Ok("wrong".into())),
                );
            } else {
                app.register_table_column_copy_formatter(
                    source,
                    column,
                    WidgetryTableCopyFormatter::new(|_: &String| {
                        Err(BevyError::from("rejected format"))
                    }),
                );
            }
        }
        let capture = LogCapture::default();
        let error = capture
            .run(|| WidgetryTable::<u32>::copy_selection(app.world_mut(), root))
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(
            error.to_string().contains("WidgetryTableRowId")
                && error.to_string().contains("WidgetryTableColumnId")
        );
        assert_eq!(capture.records().len(), 1);
        assert_eq!(
            app.world_mut()
                .resource_mut::<Clipboard>()
                .fetch_text()
                .poll_result()
                .unwrap()
                .unwrap(),
            "keep previous clipboard"
        );
    }
    app.world_mut()
        .resource_mut::<WidgetryTableCopyFormatterRegistry>()
        .remove_source(source);
    assert!(WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
    let expected = "0/0\t0/1\n1/0\t1/1";
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        expected
    );
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    assert!(!WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
    app.world_mut()
        .entity_mut(root)
        .remove::<bevy::ui::InteractionDisabled>();
    app.world_mut().despawn(source);
    assert!(WidgetryTable::<u32>::copy_selection(app.world_mut(), root).is_err());
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        expected
    );

    // Keep native clipboard writes in one test so independent cases cannot race.
    native_editor_copy_ownership();
    if let Some(Ok(old)) = old {
        app.world_mut()
            .resource_mut::<Clipboard>()
            .set_text(old)
            .unwrap();
    }
}

fn native_editor_copy_ownership() {
    use bevy::clipboard::Clipboard;
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::text::{EditableText, TextEdit};
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key, queue_key};
    use bevy_widgetry_text_field::*;
    let (mut app, source, root, _) = common::fixture(2, 2);
    use bevy_widgetry_core::WidgetryAppExt;
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(bevy_widgetry_asset::BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        std::time::Duration::from_secs(10),
        "Table editor font",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app.add_plugins(WidgetryTextFieldPlugin);
    app.add_message::<bevy::window::Ime>()
        .add_plugins(bevy::ui_widgets::TextInputPlugin);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    register(&mut app);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::All)
        .unwrap();
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |_: &String| bsn_list! {@WidgetryTextField{@value:{"editor text".to_owned()}}},
        |_, _| Ok("editor text".to_owned()),
    ));
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    app.update();
    app.update();
    assert!(!WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
    let editor = app
        .world_mut()
        .query_filtered::<Entity, With<EditableText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .editor()
            .text(),
        "editor text"
    );
    assert!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .pending_edits
            .is_empty(),
        "pending selection"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .editor()
            .selected_text(),
        Some("editor text")
    );
    let send_copy = |app: &mut App| {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<Key>>()
            .press(Key::Control);
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: Key::Control,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
        );
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::KeyC,
                logical_key: Key::Character("c".into()),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
        );
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<Key>>()
            .release(Key::Control);
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: Key::Control,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window,
            },
        );
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::KeyC,
                logical_key: Key::Character("c".into()),
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window,
            },
        );
        app.update();
    };
    send_copy(&mut app);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        "editor text"
    );
    press_key(&mut app, window, KeyCode::Escape);
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    send_copy(&mut app);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        "0/0\t0/1\n1/0\t1/1"
    );
    WidgetryTable::<u32>::update_view(app.world_mut(), root, WidgetryTableViewUpdate::default())
        .unwrap();
    assert!(!WidgetryTable::<u32>::copy_selection(app.world_mut(), root).unwrap());
}

#[test]
fn different_views_and_sources_keep_order_and_column_formatter_isolation() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    register(&mut app);
    let other = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryTable::<u32>{@source:source}})
        .unwrap()
        .id();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let rows = [model.row_id(1).unwrap(), model.row_id(0).unwrap()];
    let column = model.column_id(0).unwrap();
    app.register_table_column_copy_formatter(
        source,
        column,
        WidgetryTableCopyFormatter::new(|_: &String| Ok("override".into())),
    );
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        other,
        WidgetryTableViewUpdate {
            rows: rows.to_vec(),
            ..default()
        },
    )
    .unwrap();
    for view in [root, other] {
        WidgetryTable::<u32>::set_selection(app.world_mut(), view, WidgetryTableSelection::All)
            .unwrap();
    }
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), root)
            .unwrap()
            .unwrap(),
        "override\t0/1\noverride\t1/1"
    );
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), other)
            .unwrap()
            .unwrap(),
        "override\t1/1\noverride\t0/1"
    );
    let mut model = WidgetryTableModel::default();
    model.push_row(0u32).unwrap();
    let same_id = model
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("H"),
            (),
            |_: &u32, _: &()| WidgetryTableCellValue::new("default".to_owned()),
        ))
        .unwrap();
    assert_eq!(same_id, column);
    let other_source = app.world_mut().spawn(model).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryTable::<u32>{@source:other_source}})
        .unwrap()
        .id();
    WidgetryTable::<u32>::set_selection(app.world_mut(), view, WidgetryTableSelection::All)
        .unwrap();
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), view)
            .unwrap()
            .unwrap(),
        "default"
    );
}
