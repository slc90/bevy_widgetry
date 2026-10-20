use super::*;
#[derive(Component)]
struct BooleanEditor;

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
        Node {width: percent(100), min_width: px(0), align_items: AlignItems::Center, column_gap: px(4)}
        Children [
            @WidgetryTextField { @value: value, @multiline: multiline, @visible_lines: {Some(if multiline {3.0} else {1.0})} }
            Name(name) Node { flex_grow: 1.0, flex_basis: px(0), min_width: px(0) }--
            @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Table(TableKey::SubmitEdit)) bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)} }
            Name({format!("{name}Submit")}) on(submit_editor)
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
    app.init_resource::<PendingEdits>();
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
                Node {width: percent(100), min_width: px(0), align_items: AlignItems::Center, column_gap: px(4)}
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
                    Ok(BooleanEditor)
                })--
                @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Table(TableKey::SubmitEdit)) bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)} }
                Name("TableSubmitBoolEdit")
                on(submit_editor)
                ]
            }
        }, |world, root| {
            let combo = descendant::<WidgetryComboBox<bool>>(world, root).ok_or_else(|| "ComboBox missing".to_owned())?;
            let selected = WidgetryComboBox::<bool>::selected_id(world, combo).map_err(|e| e.to_string())?.ok_or_else(|| "Choose a value".to_owned())?;
            let model = world.get::<WidgetryListModel<bool>>(combo).ok_or_else(|| "Model missing".to_owned())?;
            model.index_of(selected).and_then(|i| model.get(i)).copied().ok_or_else(|| "Selection missing".to_owned())
        }));
    app.add_observer(|event: On<WidgetryTableEditRequested>, mut pending: ResMut<PendingEdits>| {
        pending.0.insert(event.entity, event.clone());
        info!(view = ?event.entity, cell = ?event.cell, request = ?event.request_id, "Table 编辑等待应用回执");
    });
    Ok(())
}

fn submit_editor(
    event: On<Activate>,
    parents: Query<&ChildOf>,
    views: Query<(), With<WidgetryTable<DemoRow>>>,
    mut commands: Commands,
) {
    let mut entity = event.entity;
    while !views.contains(entity) {
        let Ok(parent) = parents.get(entity) else {
            return;
        };
        entity = parent.parent();
    }
    commands.trigger(bevy_widgetry::table::WidgetryTableSubmitEdit { entity });
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

pub(super) fn resolve(world: &mut World, root: Entity, accept: bool) -> Result {
    let Some(event) = world.resource_mut::<PendingEdits>().0.remove(&root) else {
        return Ok(());
    };
    if world
        .get::<WidgetryTableEditState>(root)
        .is_none_or(|s| s.pending() != Some(event.request_id))
    {
        return Ok(());
    }
    let language = *world.resource::<Language>();
    let rejected = !accept || event.value.downcast_ref::<u32>().is_some_and(|v| *v > 1000);
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
