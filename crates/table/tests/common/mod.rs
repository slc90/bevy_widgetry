use bevy::camera::NormalizedRenderTarget;
use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
use bevy::picking::{
    backend::HitData,
    events::{Pointer, Scroll},
    pointer::{Location, PointerId},
};
use bevy::prelude::*;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use std::collections::HashMap;

pub fn fixture(rows: u32, columns: u32) -> (App, Entity, Entity, Entity) {
    let (mut app, source, root, body) = uninitialized_fixture(rows, columns);
    app.update();
    app.update();
    (app, source, root, body)
}

pub fn uninitialized_fixture(rows: u32, columns: u32) -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
    app.register_widgetry_table::<u32>();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list![(Text({ value.clone() }))]
    }))
    .unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| {
        bsn_list![(Text({ value.clone() }))]
    }))
    .unwrap();
    let mut model = WidgetryTableModel::default();
    for row in 0..rows {
        model.push_row(row).unwrap();
    }
    for column in 0..columns {
        model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new(format!("C{column}")),
                column,
                |row: &u32, column: &u32| WidgetryTableCellValue::new(format!("{row}/{column}")),
            ))
            .unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryTable::<u32> { @source: source } Node { width: px(286), height: px(144) } }).unwrap().id();
    let body = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&entity| app.world().get::<WidgetryTableBody>(entity).is_some())
        .unwrap();
    (app, source, root, body)
}

pub fn projection(
    app: &mut App,
    root: Entity,
) -> HashMap<(WidgetryTableRowId, WidgetryTableColumnId), (Entity, String)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .map(|(entity, cell)| {
            let canvas = world.get::<ChildOf>(entity).unwrap().parent();
            let body = world.get::<ChildOf>(canvas).unwrap().parent();
            assert_eq!(world.get::<ChildOf>(body).unwrap().parent(), root);
            let content = world.get::<Children>(entity).unwrap()[0];
            (
                (cell.row, cell.column),
                (entity, world.get::<Text>(content).unwrap().0.clone()),
            )
        })
        .collect()
}

pub fn scroll(app: &mut App, body: Entity, delta: Vec2) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 600,
                height: 400,
            },
            position: Vec2::ZERO,
        },
        Scroll {
            unit: MouseScrollUnit::Pixel,
            x: -delta.x,
            y: -delta.y,
            phase: TouchPhase::Moved,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        body,
    ));
    app.update();
}
