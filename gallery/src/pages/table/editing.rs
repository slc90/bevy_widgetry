use super::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::list_view::WidgetryListItemId;
use bevy_widgetry::table::WidgetryTableSubmitEdit;

#[derive(Component)]
struct BooleanEditor {
    value: bool,
}

fn submit_boolean_change(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    mut editors: Query<(&mut BooleanEditor, &WidgetryListModel<bool>)>,
    parents: Query<&ChildOf>,
    tables: Query<&WidgetryTableEditState>,
    mut commands: Commands,
) {
    let Ok((mut editor, model)) = editors.get_mut(event.source) else {
        return;
    };
    let Some(value) = event
        .value
        .and_then(|id| model.index_of(id))
        .and_then(|i| model.get(i))
        .copied()
    else {
        return;
    };
    if editor.value == value {
        return;
    }
    editor.value = value;
    if let Some(root) = parents
        .iter_ancestors(event.source)
        .find(|entity| tables.contains(*entity))
    {
        commands.trigger(WidgetryTableSubmitEdit { entity: root });
    }
}

fn descendant<C: Component>(world: &World, root: Entity) -> Option<Entity> {
    let mut entities = vec![root];
    while let Some(entity) = entities.pop() {
        if world.get::<C>(entity).is_some() {
            return Some(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            entities.extend(children.iter());
        }
    }
    None
}

fn text(world: &World, root: Entity) -> Result<String, String> {
    let field = descendant::<WidgetryTextField>(world, root)
        .ok_or_else(|| "TextField missing".to_owned())?;
    WidgetryTextField::text(world, field).map_err(|e| e.to_string())
}

fn text_scene(value: String, multiline: bool, name: &'static str) -> impl Scene {
    bsn! {
        Node {width: percent(100), min_width: px(0), align_items: AlignItems::Center}
        Children [
            @WidgetryTextField { @value: value, @multiline: multiline, @visible_lines: {Some(if multiline {3.0} else {1.0})} }
            Name(name) Node { flex_grow: 1.0, flex_basis: px(0), min_width: px(0) }
        ]
    }
}

fn string_editor(multiline: bool) -> WidgetryTableCellEditor<String> {
    WidgetryTableCellEditor::new(
        if multiline {
            WidgetryTableEditorMode::MultiLine
        } else {
            WidgetryTableEditorMode::SingleLine
        },
        move |value: &String| {
            bsn_list! {
                @text_scene(value.clone(),multiline,"TableNotesEditor")
            }
        },
        text,
    )
}

pub(super) fn install(app: &mut App) -> Result {
    app.add_observer(submit_boolean_change);
    app.register_table_cell_editor(string_editor(false));
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |value: &u32| {
            bsn_list! {
                @text_scene(value.to_string(),false,"TableNumberEditor")
            }
        },
        |world, root| {
            text(world, root)?.parse::<u32>().map_err(|_| {
                tr(
                    *world.resource::<Language>(),
                    TextKey::Table(TableKey::InvalidEditValue),
                )
                .to_owned()
            })
        },
    ));
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |value: &ProgressValue| {
            bsn_list! {
                @text_scene(value.0.to_string(),false,"TableProgressEditor")
            }
        },
        |world, root| {
            let value = text(world, root)?
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite() && (0.0..=1.0).contains(v));
            value.map(ProgressValue).ok_or_else(|| {
                tr(
                    *world.resource::<Language>(),
                    TextKey::Table(TableKey::InvalidEditValue),
                )
                .to_owned()
            })
        },
    ));
    app.register_widgetry_combo_box::<bool>()?;
    app.register_table_cell_editor(WidgetryTableCellEditor::new(WidgetryTableEditorMode::SingleLine,
        |value: &bool| {
            let selected = usize::from(*value);
            bsn_list! {
                Node {width: percent(100), min_width: px(0), align_items: AlignItems::Center}
                Children [
                template(move |context| {
                    let source = context.entity.id();
                    let mut model = WidgetryListModel::default();
                    model.push(false)?; model.push(true)?;
                    context.entity.insert(model);
                    context.entity.apply_scene(bsn! {
                        @WidgetryComboBox::<bool> {
                            @source: source, @item_height: 30.0, @max_visible_items: 2,
                            @renderer: {WidgetryListViewRenderer::new(|v: &bool| bsn_list! {Text({v.to_string()}) bevy_widgetry::text::WidgetryText})},
                        }
                        Name("TableBoolEditor") Node { width: percent(100), min_width: px(0) }
                    }).map_err(BevyError::error)?;
                    context.entity.world_scope(|world| WidgetryComboBox::<bool>::select_index(&mut world.commands(), source, selected));
                    Ok(BooleanEditor { value: selected != 0 })
                })
                ]
            }
        }, |world, root| {
            let combo = descendant::<WidgetryComboBox<bool>>(world, root).ok_or_else(|| "ComboBox missing".to_owned())?;
            let selected = WidgetryComboBox::<bool>::selected_id(world, combo).map_err(|e| e.to_string())?.ok_or_else(|| "Choose a value".to_owned())?;
            let model = world.get::<WidgetryListModel<bool>>(combo).ok_or_else(|| "Model missing".to_owned())?;
            model.index_of(selected).and_then(|i| model.get(i)).copied().ok_or_else(|| "Selection missing".to_owned())
        }));
    app.add_observer(
        |event: On<WidgetryTableEditRequested>,
         views: Query<(), With<WidgetryTable<DemoRow>>>,
         mut commands: Commands| {
            if views.contains(event.entity) {
                let request = event.clone();
                commands.queue(move |world: &mut World| resolve(world, &request));
            }
        },
    );
    Ok(())
}

pub(super) fn register_columns(world: &mut World, sources: &[Entity; 8]) {
    for source in sources {
        let column = world
            .get::<WidgetryTableModel<DemoRow>>(*source)
            .and_then(|m| m.column_id(6));
        if let Some(column) = column {
            world
                .resource_mut::<WidgetryTableEditorRegistry>()
                .register_column(*source, column, string_editor(true));
        }
    }
}

fn resolve(world: &mut World, event: &WidgetryTableEditRequested) -> Result {
    let root = event.entity;
    if world
        .get::<WidgetryTableEditState>(root)
        .is_none_or(|s| s.pending() != Some(event.request_id))
    {
        return Ok(());
    }
    let language = *world.resource::<Language>();
    let rejected = event.value.downcast_ref::<u32>().is_some_and(|v| *v > 1000);
    if rejected {
        WidgetryTable::<DemoRow>::resolve_edit(
            world,
            root,
            event.request_id,
            WidgetryTableEditResult::Rejected(
                tr(language, TextKey::Table(TableKey::EditRejected)).to_owned(),
            ),
        )?;
        info!(?root, "应用拒绝 Table 编辑");
        return Ok(());
    }
    let source = world
        .get::<WidgetryTable<DemoRow>>(root)
        .ok_or_else(|| BevyError::error("Table View missing"))?
        .source();
    let mut model = world
        .get_mut::<WidgetryTableModel<DemoRow>>(source)
        .ok_or_else(|| BevyError::error("Table Model missing"))?;
    let index = model
        .row_index(event.cell.row)
        .ok_or_else(|| BevyError::error("Edit Row removed"))?;
    let row = model
        .row_mut(index)?
        .ok_or_else(|| BevyError::error("Edit Row missing"))?;
    if let Some(value) = event.value.downcast_ref::<u32>() {
        row.number = *value;
    } else if let Some(value) = event.value.downcast_ref::<bool>() {
        row.enabled = *value;
    } else if let Some(value) = event.value.downcast_ref::<ProgressValue>() {
        row.progress = value.0;
    } else if let Some(value) = event.value.downcast_ref::<String>() {
        row.notes = value.clone();
    }
    WidgetryTable::<DemoRow>::resolve_edit(
        world,
        root,
        event.request_id,
        WidgetryTableEditResult::Accepted,
    )?;
    info!(?root, "应用接受 Table 编辑并写回共享 Model");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    fn fixture(initial: bool) -> (App, Entity, Entity, [WidgetryTableCell; 3]) {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
        app.insert_resource(Language::English);
        app.register_widgetry_table::<DemoRow>();
        app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|v: &bool| {
            bsn_list! { Text({v.to_string()}) bevy_widgetry::text::WidgetryText }
        }))
        .unwrap();
        app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|v: &u32| {
            bsn_list! { Text({v.to_string()}) bevy_widgetry::text::WidgetryText }
        }))
        .unwrap();
        app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|v: &String| {
            bsn_list! { Text({v.clone()}) bevy_widgetry::text::WidgetryText }
        }))
        .unwrap();
        app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|v: &String| {
            bsn_list! { Text({v.clone()}) bevy_widgetry::text::WidgetryText }
        }))
        .unwrap();
        install(&mut app).unwrap();
        let mut model = WidgetryTableModel::default();
        let row = model
            .push_row(DemoRow {
                notes: "old".to_owned(),
                index: 0,
                label: TableLabel::Item(0),
                number: 3,
                enabled: initial,
                progress: 0.5,
                edits: 0,
            })
            .unwrap();
        let number = model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("number".to_owned()),
                (),
                |row: &DemoRow, _: &()| WidgetryTableCellValue::new(row.number),
            ))
            .unwrap();
        let enabled = model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("bool".to_owned()),
                (),
                |row: &DemoRow, _: &()| WidgetryTableCellValue::new(row.enabled),
            ))
            .unwrap();
        let notes = model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("notes".to_owned()),
                (),
                |row: &DemoRow, _: &()| WidgetryTableCellValue::new(row.notes.clone()),
            ))
            .unwrap();
        let source = app.world_mut().spawn(model).id();
        app.register_table_column_editor(source, notes, string_editor(true));
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryTable::<DemoRow> { @source: source }
                Node { width: px(580), height: px(144) }
            })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryTableEditing::new(|_, _, _| true));
        app.update();
        app.update();
        (
            app,
            source,
            root,
            [number, enabled, notes].map(|column| WidgetryTableCell { row, column }),
        )
    }

    fn set_text_and_submit(app: &mut App, root: Entity, value: &str) {
        let editor = app
            .world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .editor();
        let field = descendant::<WidgetryTextField>(app.world(), editor).unwrap();
        WidgetryTextField::set_text_in_world(app.world_mut(), field, value).unwrap();
        assert!(WidgetryTable::<DemoRow>::submit_edit(app.world_mut(), root).unwrap());
        app.world_mut().flush();
        app.update();
    }

    #[test]
    fn text_edits_write_back_automatically_and_validation_allows_retry_or_cancel() {
        let (mut app, source, root, [number, _, notes]) = fixture(false);
        assert!(WidgetryTable::<DemoRow>::begin_edit(app.world_mut(), root, number).unwrap());
        app.update();
        set_text_and_submit(&mut app, root, "1332");
        let state = app.world().get::<WidgetryTableEditState>(root).unwrap();
        assert!(state.pending().is_none());
        assert!(state.error().is_some());
        assert_eq!(text(app.world(), state.editor()).unwrap(), "1332");
        assert_eq!(
            app.world()
                .get::<WidgetryTableModel<DemoRow>>(source)
                .unwrap()
                .row(0)
                .unwrap()
                .number,
            3
        );
        set_text_and_submit(&mut app, root, "712");
        assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
        assert_eq!(
            app.world()
                .get::<WidgetryTableModel<DemoRow>>(source)
                .unwrap()
                .row(0)
                .unwrap()
                .number,
            712
        );

        assert!(WidgetryTable::<DemoRow>::begin_edit(app.world_mut(), root, notes).unwrap());
        app.update();
        set_text_and_submit(&mut app, root, "first\nsecond");
        assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
        assert_eq!(
            app.world()
                .get::<WidgetryTableModel<DemoRow>>(source)
                .unwrap()
                .row(0)
                .unwrap()
                .notes,
            "first\nsecond"
        );

        assert!(WidgetryTable::<DemoRow>::begin_edit(app.world_mut(), root, number).unwrap());
        app.update();
        set_text_and_submit(&mut app, root, "1001");
        assert!(WidgetryTable::<DemoRow>::cancel_edit(app.world_mut(), root).unwrap());
        assert_eq!(
            app.world()
                .get::<WidgetryTableModel<DemoRow>>(source)
                .unwrap()
                .row(0)
                .unwrap()
                .number,
            712
        );
    }

    #[test]
    fn boolean_changes_write_back_automatically_but_initial_selection_does_not_submit() {
        for initial in [false, true] {
            let (mut app, source, root, [_, cell, _]) = fixture(initial);
            for value in [!initial, initial] {
                assert!(WidgetryTable::<DemoRow>::begin_edit(app.world_mut(), root, cell).unwrap());
                app.update();
                app.update();
                let state = app.world().get::<WidgetryTableEditState>(root).unwrap();
                assert!(state.pending().is_none());
                let combo =
                    descendant::<WidgetryComboBox<bool>>(app.world(), state.editor()).unwrap();
                WidgetryComboBox::<bool>::select_index(
                    &mut app.world_mut().commands(),
                    combo,
                    usize::from(value),
                );
                app.world_mut().flush();
                app.update();
                assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
                assert_eq!(
                    app.world()
                        .get::<WidgetryTableModel<DemoRow>>(source)
                        .unwrap()
                        .row(0)
                        .unwrap()
                        .enabled,
                    value
                );
                app.update();
            }
        }
    }
}
