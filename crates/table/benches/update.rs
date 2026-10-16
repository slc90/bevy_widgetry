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
    expected_selection: Option<WidgetryTableSelection>,
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
            "first_scene",
            "idle",
            "hover_one",
            "row_selection",
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
            let name = format!("{prefix}/{action}");
            run(
                &mut harness,
                &name,
                matches!(action, "first_scene" | "destroy"),
                || {
                    let mut fixture = fixture(rows, columns, rich)?;
                    if action != "first_scene" {
                        spawn_view(&mut fixture, width, height)?;
                        if action == "hidden_idle" {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("root Node"))?
                                .display = Display::None;
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
                        "row_selection" | "column_selection" => {
                            let model = app
                                .world()
                                .get::<WidgetryTableModel<u32>>(fixture.source)
                                .ok_or_else(|| missing("TableModel"))?;
                            let index = index % 2;
                            let selection = if action == "row_selection" {
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
                        "scroll_x" | "scroll_y" | "scroll_xy" => {
                            let offset = if index.is_multiple_of(2) { 120.0 } else { 0.0 };
                            app.world_mut()
                                .get_mut::<ScrollPosition>(fixture.body)
                                .ok_or_else(|| missing("ScrollPosition"))?
                                .0 = Vec2::new(
                                if action != "scroll_y" { offset } else { 0.0 },
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
        expected_selection: None,
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
    for (entity, cell) in world.query::<(Entity, &WidgetryTableCell)>().iter(world) {
        let selected = match selection {
            WidgetryTableSelection::None => false,
            WidgetryTableSelection::Row(row) => cell.row == row,
            WidgetryTableSelection::Column(column) => cell.column == column,
            WidgetryTableSelection::Cell { row, column } => {
                cell.row == row && cell.column == column
            }
        };
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
            selection == WidgetryTableSelection::Column(header.column),
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
            selection == WidgetryTableSelection::Row(header.row),
        )?;
    }
    for entity in world
        .query_filtered::<Entity, With<WidgetryTableCorner>>()
        .iter(world)
    {
        validate_region(world, entity, colors.corner, false)?;
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
    if world
        .get::<BackgroundColor>(entity)
        .is_none_or(|color| color.0 != state.background)
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
