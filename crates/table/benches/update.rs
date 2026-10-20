use bevy::camera::NormalizedRenderTarget;
use bevy::picking::hover::Hovered;
use bevy::picking::{
    backend::HitData,
    events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart},
    pointer::{Location, PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui::{ScrollPosition, Selected};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_theme::{WidgetryTableRegionColors, WidgetryThemeMode};

struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    body: Entity,
    handle: Entity,
    expected_hover: Option<(Entity, bool)>,
    expected_first: Option<WidgetryTableRowId>,
    expected_selection: Option<WidgetryTableSelection>,
    expected_header: Option<WidgetryTableColumnId>,
}

#[derive(Resource, Default)]
struct HoverRequest(Option<(Entity, bool)>);

fn apply_hover(world: &mut World) -> Result {
    // Picking projects Hovered before Update, so inject this local style stimulus afterwards.
    if let Some((entity, hovered)) = world.resource_mut::<HoverRequest>().0.take() {
        world
            .get_entity_mut(entity)
            .map_err(BevyError::error)?
            .insert(Hovered(hovered));
    }
    Ok(())
}

fn main() -> Result {
    let mut harness = Harness::new("table-criterion")?;
    for (rows, columns, width, height, rich) in [
        (1_000, 20, 646, 368, false),
        (10_000, 20, 646, 368, false),
        (100_000, 20, 646, 368, false),
        (10_000, 200, 646, 368, false),
        (10_000, 2_000, 646, 368, false),
        (10_000, 20, 1246, 704, false),
        (10_000, 20, 646, 368, true),
    ] {
        let prefix = format!("table/r{rows}_c{columns}/v{width}x{height}/rich{rich}");
        for action in [
            "copy_all",
            "first_scene",
            "idle",
            "editing_idle",
            "editing_scroll",
            "row_projection",
            "header_idle",
            "header_scroll",
            "header_commit",
            "projected_idle",
            "projected_scroll",
            "hover_one",
            "row_selection",
            "range_selection",
            "all_selection",
            "range_scroll",
            "column_selection",
            "scroll_x",
            "scroll_y",
            "scroll_xy",
            "visible_mutation",
            "offscreen_mutation",
            "resize",
            "column_drag",
            "hidden_idle",
            "rebuild",
            "destroy",
        ] {
            if action == "copy_all" && !(rows <= 10_000 && columns == 20 && width == 646 && !rich) {
                continue;
            }
            let name = format!("{prefix}/{action}");
            run(
                &mut harness,
                &name,
                matches!(action, "first_scene" | "destroy"),
                || {
                    let mut fixture = fixture(rows, columns, rich)?;
                    if action != "first_scene" {
                        spawn_view(&mut fixture, width, height)?;
                        if action == "copy_all" {
                            fixture.app.register_table_copy_formatter(
                                WidgetryTableCopyFormatter::new(|v: &String| Ok(v.clone())),
                            );
                            WidgetryTable::<u32>::set_selection(
                                fixture.app.world_mut(),
                                fixture.root,
                                WidgetryTableSelection::All,
                            )?;
                        }
                        if action.starts_with("editing_") {
                            fixture
                                .app
                                .world_mut()
                                .entity_mut(fixture.root)
                                .insert(WidgetryTableEditing::new(|_, _, _| true));
                        }
                        if action == "hidden_idle" {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("root Node"))?
                                .display = Display::None;
                        }
                        if action.starts_with("projected_") {
                            let model = fixture
                                .app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("Model"))?;
                            let rows = (0..model.row_count())
                                .step_by(2)
                                .rev()
                                .filter_map(|index| model.row_id(index))
                                .collect::<Vec<_>>();
                            fixture.expected_first = rows.first().copied();
                            WidgetryTable::<u32>::update_view(
                                fixture.app.world_mut(),
                                fixture.root,
                                WidgetryTableViewUpdate { rows, ..default() },
                            )?;
                        }
                        if action == "range_scroll" {
                            let model = fixture
                                .app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("Model"))?;
                            let anchor = WidgetryTableCell {
                                row: model.row_id(0).ok_or_else(|| missing("row"))?,
                                column: model.column_id(0).ok_or_else(|| missing("column"))?,
                            };
                            let end = WidgetryTableCell {
                                row: model.row_id(rows - 1).ok_or_else(|| missing("row"))?,
                                column: model
                                    .column_id(columns as usize - 1)
                                    .ok_or_else(|| missing("column"))?,
                            };
                            let selection = WidgetryTableSelection::Cells { anchor, end };
                            WidgetryTable::<u32>::set_selection(
                                fixture.app.world_mut(),
                                fixture.root,
                                selection,
                            )?;
                            fixture.expected_selection = Some(selection);
                        }
                        if action.starts_with("header_") {
                            let model = fixture
                                .app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("Model"))?;
                            let columns = (0..model.column_count())
                                .filter_map(|index| model.column_id(index))
                                .collect::<std::collections::HashSet<_>>();
                            fixture.app.world_mut().entity_mut(fixture.root).insert(
                                WidgetryTableHeaderActions {
                                    sortable: columns.clone(),
                                    filterable: columns,
                                },
                            );
                        }
                        settle(&mut fixture.app);
                        validate(&mut fixture, action == "hidden_idle")?;
                        if action == "column_drag" {
                            fixture.handle = resize_handle(&mut fixture.app, fixture.source)?;
                        }
                    }
                    Ok(fixture)
                },
                |fixture, index| {
                    let app = &mut fixture.app;
                    match action {
                        "copy_all" => {
                            let text =
                                WidgetryTable::<u32>::selection_tsv(app.world_mut(), fixture.root)?
                                    .ok_or_else(|| missing("TSV selection"))?;
                            if !text.starts_with("0/0\t")
                                || !text.ends_with(&format!("{}/{}", rows - 1, columns - 1))
                            {
                                return Err(missing("complete TSV contents"));
                            }
                            std::hint::black_box(text);
                        }
                        "row_projection" => {
                            let model = app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("Model"))?;
                            let mut rows = (0..model.row_count())
                                .step_by(2)
                                .filter_map(|index| model.row_id(index))
                                .collect::<Vec<_>>();
                            if index % 2 == 1 {
                                rows.reverse();
                            }
                            fixture.expected_first = rows.first().copied();
                            if !WidgetryTable::<u32>::update_view(
                                app.world_mut(),
                                fixture.root,
                                WidgetryTableViewUpdate { rows, ..default() },
                            )? {
                                return Err(missing("actual projection change"));
                            }
                            app.update();
                        }
                        "header_commit" => {
                            let model = app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("Model"))?;
                            let column = model
                                .column_id(index % 2)
                                .ok_or_else(|| missing("Column"))?;
                            let rows = (0..model.row_count())
                                .filter_map(|index| model.row_id(index))
                                .collect();
                            if !WidgetryTable::<u32>::update_view(
                                app.world_mut(),
                                fixture.root,
                                WidgetryTableViewUpdate {
                                    rows,
                                    sort: Some(WidgetryTableSort {
                                        column,
                                        direction: WidgetryTableSortDirection::Ascending,
                                    }),
                                    filters: [column].into_iter().collect(),
                                },
                            )? {
                                return Err(missing("actual header state change"));
                            }
                            fixture.expected_header = Some(column);
                            app.update();
                        }
                        "hover_one" => {
                            let entity = if let Some((entity, _)) = fixture.expected_hover {
                                entity
                            } else {
                                app.world_mut()
                                    .query_filtered::<Entity, With<WidgetryTableCell>>()
                                    .iter(app.world())
                                    .next()
                                    .ok_or_else(|| missing("Table Cell"))?
                            };
                            let hovered = index.is_multiple_of(2);
                            app.world_mut().resource_mut::<HoverRequest>().0 =
                                Some((entity, hovered));
                            fixture.expected_hover = Some((entity, hovered));
                            app.update();
                        }
                        "row_selection" | "column_selection" | "range_selection"
                        | "all_selection" => {
                            let model = app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("TableModel"))?;
                            let index = index % 2;
                            let selection = if action == "all_selection" {
                                if index == 0 {
                                    WidgetryTableSelection::All
                                } else {
                                    WidgetryTableSelection::None
                                }
                            } else if action == "range_selection" {
                                WidgetryTableSelection::Cells {
                                    anchor: WidgetryTableCell {
                                        row: model
                                            .row_id(0)
                                            .ok_or_else(|| missing("anchor row"))?,
                                        column: model
                                            .column_id(0)
                                            .ok_or_else(|| missing("anchor column"))?,
                                    },
                                    end: WidgetryTableCell {
                                        row: model
                                            .row_id(if index == 0 { rows / 2 } else { rows - 1 })
                                            .ok_or_else(|| missing("end row"))?,
                                        column: model
                                            .column_id(if index == 0 {
                                                1
                                            } else {
                                                columns as usize - 1
                                            })
                                            .ok_or_else(|| missing("end column"))?,
                                    },
                                }
                            } else if action == "row_selection" {
                                WidgetryTableSelection::Row(
                                    model.row_id(index).ok_or_else(|| missing("row ID"))?,
                                )
                            } else {
                                WidgetryTableSelection::Column(
                                    model.column_id(index).ok_or_else(|| missing("column ID"))?,
                                )
                            };
                            if !WidgetryTable::<u32>::set_selection(
                                app.world_mut(),
                                fixture.root,
                                selection,
                            )? {
                                return Err(missing("Table selection change"));
                            }
                            fixture.expected_selection = Some(selection);
                            app.update();
                        }
                        "column_drag" => {
                            let target = fixture.handle;
                            let distance =
                                Vec2::new(if index.is_multiple_of(2) { 20.0 } else { -20.0 }, 0.0);
                            app.world_mut().trigger(pointer(target, |entity, pointer| {
                                PointerDragStart {
                                    entity,
                                    pointer,
                                    button: PointerButton::Primary,
                                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                                }
                            }));
                            app.world_mut().flush();
                            app.world_mut().trigger(pointer(target, |entity, pointer| {
                                PointerDrag {
                                    entity,
                                    pointer,
                                    button: PointerButton::Primary,
                                    distance,
                                    delta: distance,
                                }
                            }));
                            app.world_mut().flush();
                            app.world_mut().trigger(pointer(target, |entity, pointer| {
                                PointerDragEnd {
                                    entity,
                                    pointer,
                                    button: PointerButton::Primary,
                                    distance,
                                }
                            }));
                            app.world_mut().flush();
                            settle(app);
                        }
                        "first_scene" => {
                            spawn_view(fixture, width, height)?;
                            settle(&mut fixture.app);
                        }
                        "scroll_x" | "scroll_y" | "scroll_xy" | "projected_scroll"
                        | "editing_scroll" | "header_scroll" | "range_scroll" => {
                            let offset = if index.is_multiple_of(2) { 120.0 } else { 0.0 };
                            app.world_mut()
                                .get_mut::<ScrollPosition>(fixture.body)
                                .ok_or_else(|| missing("ScrollPosition"))?
                                .0 = Vec2::new(
                                if action != "scroll_y" && action != "projected_scroll" {
                                    offset
                                } else {
                                    0.0
                                },
                                if action != "scroll_x" { offset } else { 0.0 },
                            );
                            app.update();
                        }
                        "visible_mutation" | "offscreen_mutation" => {
                            let row = if action == "visible_mutation" {
                                0
                            } else {
                                rows - 1
                            };
                            {
                                let mut model = app
                                    .world_mut()
                                    .get_mut::<WidgetryTableModel<u32>>(fixture.source)
                                    .ok_or_else(|| missing("TableModel"))?;
                                *model.row_mut(row)?.ok_or_else(|| missing("row"))? = index as u32;
                            }
                            app.update();
                        }
                        "resize" => {
                            app.world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("root Node"))?
                                .width =
                                px(width as f32
                                    + if index.is_multiple_of(2) { 120.0 } else { 0.0 });
                            settle(app);
                        }
                        "rebuild" => {
                            app.world_mut().despawn(fixture.root);
                            spawn_view(fixture, width, height)?;
                            settle(&mut fixture.app);
                        }
                        "destroy" => {
                            app.world_mut().despawn(fixture.root);
                            app.world_mut().despawn(fixture.source);
                            app.update();
                        }
                        _ => app.update(),
                    }
                    Ok(())
                },
                |fixture| {
                    if fixture.root != Entity::PLACEHOLDER && action != "destroy" {
                        validate(fixture, action == "hidden_idle")?;
                    }
                    if action.starts_with("header_") {
                        let world = fixture.app.world_mut();
                        let buttons = world
                            .query::<&WidgetryTableHeaderAction>()
                            .iter(world)
                            .count();
                        if buttons == 0 || buttons > 2 * (width as usize / 120 + 3) {
                            return Err(missing("bounded header buttons"));
                        }
                        if let Some(column) = fixture.expected_header {
                            let state = world
                                .get::<WidgetryTableViewState>(fixture.root)
                                .ok_or_else(|| missing("View state"))?;
                            if state.sort()
                                != Some(WidgetryTableSort {
                                    column,
                                    direction: WidgetryTableSortDirection::Ascending,
                                })
                                || state.filters() != &[column].into_iter().collect()
                            {
                                return Err(missing("confirmed header markers"));
                            }
                        }
                    }
                    Ok(fixture.app.world().entities().count_spawned())
                },
            )?;
        }
    }
    harness.finish()
}

fn fixture(rows: usize, columns: u32, rich: bool) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.register_widgetry_table::<u32>();
    app.init_resource::<HoverRequest>()
        .add_systems(Update, apply_hover);
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(move |value: &String| {
        bsn_list!{Node { flex_direction: FlexDirection::Column } Children [Text({value.clone()}) bevy_widgetry_core::text::WidgetryText-- {rich.then_some(bsn! { Text("details: 123.45 / active") bevy_widgetry_core::text::WidgetryText })}]}
    }))?;
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(
        |value: &String| bsn_list! {Text({ value.clone() }) bevy_widgetry_core::text::WidgetryText},
    ))?;
    let mut model = WidgetryTableModel::default();
    for row in 0..rows {
        model.push_row(row as u32)?;
    }
    for column in 0..columns {
        model.push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new(format!("Column {column}")),
            column,
            |row: &u32, column: &u32| WidgetryTableCellValue::new(format!("{row}/{column}")),
        ))?;
    }
    let source = app.world_mut().spawn(model).id();
    Ok(Fixture {
        app,
        source,
        root: Entity::PLACEHOLDER,
        body: Entity::PLACEHOLDER,
        handle: Entity::PLACEHOLDER,
        expected_hover: None,
        expected_first: None,
        expected_selection: None,
        expected_header: None,
    })
}

fn resize_handle(app: &mut App, source: Entity) -> Result<Entity> {
    let world = app.world_mut();
    let column = world
        .get::<WidgetryTableModel<u32>>(source)
        .and_then(|model| model.column_id(0))
        .ok_or_else(|| missing("first Column"))?;
    let header = world
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(world)
        .find(|(_, header)| header.column == column)
        .map(|(entity, _)| entity)
        .ok_or_else(|| missing("Column Header"))?;
    world
        .get::<Children>(header)
        .into_iter()
        .flat_map(|children| children.iter())
        .find(|&entity| {
            world.get::<Node>(entity).is_some_and(|node| {
                node.position_type == PositionType::Absolute
                    && node.right == px(0)
                    && node.width == px(6)
            })
        })
        .ok_or_else(|| missing("resize strip"))
}

fn pointer<E: bevy::picking::events::PointerEvent>(
    target: Entity,
    event: impl FnOnce(Entity, Pointer) -> E,
) -> E {
    event(
        target,
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1246,
                    height: 704,
                },
                position: Vec2::ZERO,
            },
        ),
    )
}

fn spawn_view(fixture: &mut Fixture, width: u32, height: u32) -> Result {
    let source = fixture.source;
    fixture.root = fixture.app.world_mut().spawn_scene(bsn! { @WidgetryTable::<u32> { @source: source } Node { width: px(width as f32), height: px(height as f32) } })?.id();
    fixture.body = fixture
        .app
        .world()
        .get::<Children>(fixture.root)
        .ok_or_else(|| missing("Table Children"))?
        .iter()
        .find(|&entity| {
            fixture
                .app
                .world()
                .get::<WidgetryTableBody>(entity)
                .is_some()
        })
        .ok_or_else(|| missing("TableBody"))?;
    Ok(())
}

fn validate(fixture: &mut Fixture, hidden: bool) -> Result {
    let world = fixture.app.world_mut();
    let cells = world.query::<&WidgetryTableCell>().iter(world).count();
    if (cells == 0) != hidden {
        return Err(BevyError::error(
            "Table benchmark projection readiness failed",
        ));
    }
    if let Some(expected) = fixture.expected_first {
        let mut query = world.query::<(Entity, &WidgetryTableCell)>();
        let state = world
            .get::<WidgetryTableViewState>(fixture.root)
            .ok_or_else(|| missing("View projection"))?;
        let model = world
            .get::<WidgetryTableModel<u32>>(fixture.source)
            .ok_or_else(|| missing("Model"))?;
        let layout = world
            .get::<WidgetryTableLayout>(fixture.root)
            .ok_or_else(|| missing("Table layout"))?;
        let body = world
            .get::<ComputedNode>(fixture.body)
            .ok_or_else(|| missing("body geometry"))?;
        let size = body.size() * body.inverse_scale_factor();
        let column_width = std::iter::once(&layout.default_column_width)
            .chain(layout.column_widths().values())
            .map(|width| match width {
                WidgetryTableColumnWidth::Fixed(value) => value.max(layout.min_column_width),
                WidgetryTableColumnWidth::Flexible(_) => layout.min_column_width,
            })
            .fold(f32::INFINITY, f32::min);
        // A scrolled viewport can intersect one partial item on either edge.
        let max_rows = ((size.y / layout.row_height).ceil() as usize + 1).min(state.len());
        let max_columns = ((size.x / column_width).ceil() as usize + 1).min(model.column_count());
        if state.row_id(0) != Some(expected)
            || model.row(0) != Some(&0)
            || cells > max_rows * max_columns
        {
            return Err(missing(
                "correct bounded View projection and unchanged Model",
            ));
        }
        for (entity, cell) in query.iter(world) {
            let value = model
                .cell(cell.row, cell.column)
                .ok_or_else(|| missing("Model cell"))?;
            let expected = value
                .downcast_ref::<String>()
                .ok_or_else(|| missing("String value"))?;
            let node = world
                .get::<Children>(entity)
                .and_then(|children| children.first())
                .and_then(|child| world.get::<Children>(*child))
                .and_then(|children| children.first())
                .and_then(|text| world.get::<Text>(*text))
                .ok_or_else(|| missing("rendered cell Text"))?;
            if node.0 != *expected {
                return Err(missing("current projected cell value"));
            }
        }
    }
    if !hidden {
        validate_text(&mut fixture.app)?;
        validate_colors(fixture)?;
    }
    Ok(())
}

fn validate_colors(fixture: &mut Fixture) -> Result {
    let world = fixture.app.world_mut();
    let selection = world
        .get::<WidgetryTableState>(fixture.root)
        .ok_or_else(|| missing("TableState"))?
        .selection();
    if fixture
        .expected_selection
        .is_some_and(|expected| expected != selection)
    {
        return Err(missing("Table selection output"));
    }
    if let Some((entity, hovered)) = fixture.expected_hover
        && world
            .get::<Hovered>(entity)
            .is_none_or(|value| value.0 != hovered)
    {
        return Err(missing("Table hover stimulus"));
    }
    let colors = world.resource::<WidgetryThemeMode>().colors().table;
    let range = WidgetryTable::<u32>::selection_range(world, fixture.root)?;
    let model = world
        .get::<WidgetryTableModel<u32>>(fixture.source)
        .ok_or_else(|| missing("Model"))?;
    let columns = (0..model.column_count())
        .filter_map(|index| model.column_id(index).map(|id| (id, index)))
        .collect::<std::collections::HashMap<_, _>>();
    let view = world
        .get::<WidgetryTableViewState>(fixture.root)
        .ok_or_else(|| missing("View"))?
        .clone();
    for (entity, cell) in world.query::<(Entity, &WidgetryTableCell)>().iter(world) {
        let selected = range
            .zip(
                view.display_index(cell.row)
                    .zip(columns.get(&cell.column).copied()),
            )
            .is_some_and(|(range, (row, column))| range.contains(row, column));
        validate_region(world, entity, colors.cell, selected)?;
    }
    for (entity, header) in world
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(world)
    {
        validate_region(
            world,
            entity,
            colors.column_header,
            matches!(
                selection,
                WidgetryTableSelection::Column(_) | WidgetryTableSelection::Columns { .. }
            ) && range
                .zip(columns.get(&header.column))
                .is_some_and(|(range, column)| range.columns().contains(column)),
        )?;
    }
    for (entity, header) in world
        .query::<(Entity, &WidgetryTableRowHeader)>()
        .iter(world)
    {
        validate_region(
            world,
            entity,
            colors.row_header,
            matches!(
                selection,
                WidgetryTableSelection::Row(_) | WidgetryTableSelection::Rows { .. }
            ) && range.is_some_and(|range| range.rows().contains(&header.index)),
        )?;
    }
    for entity in world
        .query_filtered::<Entity, With<WidgetryTableCorner>>()
        .iter(world)
    {
        validate_region(
            world,
            entity,
            colors.corner,
            selection == WidgetryTableSelection::All,
        )?;
    }
    validate_region(world, fixture.root, colors.table, false)
}

fn validate_region(
    world: &World,
    entity: Entity,
    colors: WidgetryTableRegionColors,
    selected: bool,
) -> Result {
    if world.get::<Selected>(entity).is_some() != selected {
        return Err(missing("Table Selected projection"));
    }
    let state = if world
        .get::<Hovered>(entity)
        .is_some_and(|hovered| hovered.0)
    {
        colors.hovered
    } else if selected {
        colors.selected
    } else {
        colors.normal
    };
    let background = if !selected
        && world.get::<bevy_widgetry_cursor::WidgetryCursorHint>(entity)
            == Some(&bevy_widgetry_cursor::WidgetryCursorHint::ReadOnly)
    {
        state.background.mix(&colors.selected.background, 0.12)
    } else {
        state.background
    };
    if world
        .get::<BackgroundColor>(entity)
        .is_none_or(|color| color.0 != background)
        || world
            .get::<BorderColor>(entity)
            .is_none_or(|color| color.top != state.border)
        || world
            .get::<ResolvedForeground>(entity)
            .is_some_and(|color| color.0 != state.foreground)
    {
        return Err(missing("Table region palette"));
    }
    Ok(())
}
