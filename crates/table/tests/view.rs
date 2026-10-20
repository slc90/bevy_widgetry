#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::error::Severity;
use bevy::ecs::message::Messages;
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::log::tracing::Level;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Pointer, PointerScroll};
use bevy::picking::pointer::{Location, PointerId};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiGlobalTransform};
use bevy::window::RequestRedraw;
use bevy_widgetry_theme::WidgetryThemeMode;

use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, scene_app, spawn_ui_camera,
};

fn fixture() -> (App, Entity, Entity) {
    let (mut app, source, root) = unmeasured_fixture();
    // The first Layout measures the viewport.
    // Only the next update has valid dimensions for cells, so an empty first-frame projection is not construction failure.
    app.update();
    (app, source, root)
}

#[test]
fn shared_range_endpoints_use_each_views_order_and_reorder_recolors_the_current_membership() {
    let (mut app, source, a) = fixture();
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap();
        model.push_row("Carol".into()).unwrap();
        model.push_row("Dan".into()).unwrap();
    }
    let b = app.world_mut().spawn_scene(bsn! {
        @WidgetryTable::<String> { @source: source }
        Node { position_type: PositionType::Absolute, left: px(300), width: px(300), height: px(150) }
    }).unwrap().id();
    app.update();
    app.update();
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    let ids = [0, 1, 2, 3].map(|index| model.row_id(index).unwrap());
    let column = model.column_id(0).unwrap();
    WidgetryTable::<String>::update_view(
        app.world_mut(),
        b,
        WidgetryTableViewUpdate {
            rows: vec![ids[3], ids[0], ids[2], ids[1]],
            ..default()
        },
    )
    .unwrap();
    let selection = WidgetryTableSelection::Cells {
        anchor: WidgetryTableCell {
            row: ids[0],
            column,
        },
        end: WidgetryTableCell {
            row: ids[1],
            column,
        },
    };
    WidgetryTable::<String>::set_selection(app.world_mut(), a, selection).unwrap();
    WidgetryTable::<String>::set_selection(app.world_mut(), b, selection).unwrap();
    app.update();
    assert_eq!(
        WidgetryTable::<String>::selection_range(app.world(), a)
            .unwrap()
            .unwrap()
            .rows(),
        0..2
    );
    assert_eq!(
        WidgetryTable::<String>::selection_range(app.world(), b)
            .unwrap()
            .unwrap()
            .rows(),
        1..4
    );
    let count = |app: &mut App, root| {
        cells(app, root)
            .iter()
            .filter(|(entity, _)| app.world().get::<bevy::ui::Selected>(*entity).is_some())
            .count()
    };
    assert_eq!(count(&mut app, a), 2);
    assert_eq!(count(&mut app, b), 3);
    WidgetryTable::<String>::update_view(
        app.world_mut(),
        b,
        WidgetryTableViewUpdate {
            rows: ids.to_vec(),
            ..default()
        },
    )
    .unwrap();
    app.update();
    assert_eq!(count(&mut app, b), 2);
    WidgetryTable::<String>::set_selection(app.world_mut(), a, WidgetryTableSelection::None)
        .unwrap();
    app.update();
    assert_eq!(count(&mut app, a), 0);
    assert_eq!(count(&mut app, b), 2);
}

fn unmeasured_fixture() -> (App, Entity, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
    unmeasured_fixture_in(app)
}

fn unmeasured_fixture_in(mut app: App) -> (App, Entity, Entity) {
    app.register_widgetry_table::<String>();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list! {Text({ value.clone() }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| {
        bsn_list! {Text({ value.clone() }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    let mut model = WidgetryTableModel::default();
    model.push_row("Alice".to_owned()).unwrap();
    model.push_row("Bob".to_owned()).unwrap();
    model
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("Name".to_owned()),
            (),
            |row: &String, _: &()| WidgetryTableCellValue::new(row.clone()),
        ))
        .unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryTable::<String> { @source: source } Node { width: px(300), height: px(150) } }).unwrap().id();
    (app, source, root)
}

#[test]
fn first_update_projects_source_and_owns_only_view_subtree() {
    let (mut app, source, root) = fixture();
    app.update();
    let cells: Vec<_> = app
        .world_mut()
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(app.world())
        .map(|(entity, cell)| (entity, *cell))
        .collect();
    assert_eq!(cells.len(), 2);
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    for (entity, cell) in &cells {
        let index = model.row_index(cell.row).unwrap();
        assert_eq!(cell.column, model.column_id(0).unwrap());
        let content = app.world().get::<Children>(*entity).unwrap()[0];
        assert_eq!(
            &app.world().get::<Text>(content).unwrap().0,
            model.row(index).unwrap()
        );
        let canvas = app.world().get::<ChildOf>(*entity).unwrap().parent();
        let viewport = app.world().get::<ChildOf>(canvas).unwrap().parent();
        assert!(app.world().get::<WidgetryTableBody>(viewport).is_some());
        assert_eq!(app.world().get::<ChildOf>(viewport).unwrap().parent(), root);
    }
    assert_eq!(app.world().get::<Children>(root).unwrap().len(), 6);
    app.world_mut().despawn(root);
    assert!(
        app.world()
            .get::<WidgetryTableModel<String>>(source)
            .is_some()
    );
    assert!(
        cells
            .iter()
            .all(|(entity, _)| !app.world().entities().contains(*entity))
    );
}

fn cells(app: &mut App, root: Entity) -> Vec<(Entity, WidgetryTableCell)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .filter(|(entity, _)| {
            let canvas = world.get::<ChildOf>(*entity).unwrap().parent();
            let body = world.get::<ChildOf>(canvas).unwrap().parent();
            world.get::<ChildOf>(body).unwrap().parent() == root
        })
        .map(|(entity, cell)| (entity, *cell))
        .collect()
}

fn text(app: &App, shell: Entity) -> (Entity, String) {
    let mut child = app.world().get::<Children>(shell).unwrap()[0];
    while app.world().get::<Text>(child).is_none() {
        child = app.world().get::<Children>(child).unwrap()[0];
    }
    (child, app.world().get::<Text>(child).unwrap().0.clone())
}

#[test]
fn revisions_type_changes_replacement_and_sources_are_independent() {
    let (mut app, source, root) = fixture();
    let mut other = WidgetryTableModel::default();
    other.push_row("Other".to_owned()).unwrap();
    other
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("Other header".to_owned()),
            (),
            |row: &String, _| WidgetryTableCellValue::new(row.clone()),
        ))
        .unwrap();
    let other_source = app.world_mut().spawn(other).id();
    let other_root = app.world_mut().spawn_scene(bsn! { @WidgetryTable::<String> { @source: other_source } Node { width: px(300), height: px(150) } }).unwrap().id();
    app.update();
    app.update();
    let first = cells(&mut app, root)
        .into_iter()
        .find(|(_, cell)| {
            app.world()
                .get::<WidgetryTableModel<String>>(source)
                .unwrap()
                .row_index(cell.row)
                == Some(0)
        })
        .unwrap()
        .0;
    let before = text(&app, first).0;
    let other_cell = cells(&mut app, other_root)[0].0;
    let other_before = text(&app, other_cell).0;
    *app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .row_mut(0)
        .unwrap()
        .unwrap() = "Changed".to_owned();
    app.update();
    assert_eq!(text(&app, first).1, "Changed");
    assert!(!app.world().entities().contains(before));
    assert_eq!(text(&app, other_cell), (other_before, "Other".to_owned()));
    app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .move_row(0, 1);
    app.update();
    assert_eq!(text(&app, first).1, "Changed");
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list! {Text({ format!("new {value}") }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    app.update();
    assert_eq!(text(&app, first).1, "new Changed");
    assert_eq!(text(&app, other_cell).1, "new Other");
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &bool| {
        bsn_list! {Text({ format!("bool {value}") }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .set_column(
            0,
            WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("Changed header".to_owned()),
                (),
                |_: &String, _| WidgetryTableCellValue::new(true),
            ),
        )
        .unwrap();
    app.update();
    assert!(
        cells(&mut app, root)
            .iter()
            .all(|(entity, _)| text(&app, *entity).1 == "bool true")
    );
    assert_eq!(text(&app, other_cell).1, "new Other");
    let headers: Vec<_> = app
        .world_mut()
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(app.world())
        .map(|(entity, _)| text(&app, entity).1)
        .collect();
    assert!(headers.contains(&"Changed header".to_owned()));
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryTableCell>()
            .iter(app.world())
            .count(),
        3
    );
}

#[test]
fn style_and_disabled_preserve_content_and_picking() {
    let (mut app, _, root) = fixture();
    app.update();
    let cell = cells(&mut app, root)[0].0;
    let content = text(&app, cell).0;
    let original = Pickable {
        should_block_lower: false,
        is_hoverable: true,
    };
    app.world_mut().entity_mut(content).insert(original);
    let picking =
        [root, cell, content].map(|entity| (entity, app.world().get::<Pickable>(entity).copied()));
    {
        let mut style = app.world_mut().get_mut::<WidgetryTableStyle>(root).unwrap();
        style.cell.padding = UiRect::all(px(3));
        style.cell.border = UiRect::all(px(2));
    }
    let mut colors = bevy_widgetry_table::WidgetryTableColorOverrides::default();
    colors.cell.normal.background = Some(Color::srgb(0.1, 0.2, 0.3));
    colors.cell.disabled.background = Some(Color::srgb(0.3, 0.2, 0.1));
    colors.corner.normal.background = Some(Color::srgb(0.2, 0.4, 0.6));
    bevy_widgetry_table::WidgetryTableColorOverrides::set_in_world(app.world_mut(), root, colors)
        .unwrap();
    app.update();
    assert_eq!(text(&app, cell).0, content);
    assert_eq!(
        app.world().get::<Node>(cell).unwrap().padding,
        UiRect::all(px(3))
    );
    assert_eq!(
        app.world().get::<Node>(cell).unwrap().border,
        UiRect::all(px(2))
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(cell).unwrap().0,
        Color::srgb(0.1, 0.2, 0.3)
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(cell).unwrap().0,
        Color::srgb(0.3, 0.2, 0.1)
    );
    for (entity, expected) in picking {
        assert_eq!(app.world().get::<Pickable>(entity).copied(), expected);
        assert!(app.world().get::<InteractionDisabled>(entity).is_some());
    }
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light)
        .expect("theme switch succeeds");
    app.update();
    assert_eq!(text(&app, cell).0, content);
    assert_eq!(
        app.world().get::<Pickable>(content).unwrap().is_hoverable,
        original.is_hoverable
    );
    assert_eq!(
        app.world()
            .get::<Pickable>(content)
            .unwrap()
            .should_block_lower,
        original.should_block_lower
    );
    assert!(app.world().get::<Pickable>(cell).is_none());
    assert_eq!(
        app.world().get::<BorderColor>(cell).unwrap().top,
        WidgetryThemeMode::Light.colors().table.cell.normal.border
    );
}

#[test]
fn shared_views_prepare_new_and_reused_cell_content_with_cached_palettes() {
    use bevy::camera::visibility::VisibilitySystems;
    use bevy::ui::{ComputedStackIndex, UiSystems};
    use bevy_widgetry_test_utils::{benchmark::ui_app, switch_theme};
    let (mut app, source, first) = unmeasured_fixture_in(ui_app().unwrap());
    app.configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    );
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap();
        for row in 0..40 {
            model.push_row(format!("Row {row}")).unwrap();
        }
        for column in 1..8 {
            model
                .push_column(WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new(format!("Column {column}")),
                    column,
                    |row: &String, column: &u32| {
                        WidgetryTableCellValue::new(format!("{row}/{column}"))
                    },
                ))
                .unwrap();
        }
    }
    let second = app.world_mut().spawn_scene(bsn! { @WidgetryTable::<String> { @source: source } InteractionDisabled Node { width: px(300), height: px(150) } }).unwrap().id();
    app.update();
    app.update();
    let first_cell = cells(&mut app, first)[0].0;
    let label = text(&app, first_cell).0;
    let font = app.world().get::<TextFont>(label).unwrap().font.clone();
    let first_color = Color::srgb_u8(17, 34, 51);
    let second_color = Color::srgb_u8(85, 102, 119);
    let mut colors = WidgetryTableColorOverrides::default();
    colors.cell.normal.foreground = Some(first_color);
    WidgetryTableColorOverrides::set_in_world(app.world_mut(), first, colors).unwrap();
    let mut colors = WidgetryTableColorOverrides::default();
    colors.cell.disabled.foreground = Some(second_color);
    WidgetryTableColorOverrides::set_in_world(app.world_mut(), second, colors).unwrap();
    app.update();
    let check = |app: &mut App, first_color: Color, second_color: Color| {
        for (view, foreground) in [(first, first_color), (second, second_color)] {
            let visible = cells(app, view);
            assert!(!visible.is_empty());
            for (cell, _) in visible {
                let label = text(app, cell).0;
                assert_eq!(app.world().get::<TextColor>(label).unwrap().0, foreground);
                assert_eq!(app.world().get::<TextFont>(label).unwrap().font, font);
                assert!(
                    !app.world()
                        .get::<bevy::text::TextLayoutInfo>(label)
                        .unwrap()
                        .glyphs
                        .is_empty()
                );
                assert!(app.world().get::<InheritedVisibility>(label).unwrap().get());
                assert!(
                    app.world()
                        .get::<ComputedNode>(label)
                        .unwrap()
                        .size()
                        .min_element()
                        > 0.0
                );
                assert!(
                    app.world().get::<ComputedStackIndex>(label).unwrap().0
                        > app.world().get::<ComputedStackIndex>(cell).unwrap().0
                );
            }
        }
    };
    check(&mut app, first_color, second_color);
    let old = [first, second].map(|view| {
        cells(&mut app, view)
            .into_iter()
            .filter(|(_, cell)| {
                app.world()
                    .get::<WidgetryTableModel<String>>(source)
                    .unwrap()
                    .row_index(cell.row)
                    == Some(0)
            })
            .map(|(cell, _)| text(&app, cell).0)
            .collect::<Vec<_>>()
    });
    *app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .row_mut(0)
        .unwrap()
        .unwrap() = "updated with cached colors".into();
    app.update();
    assert!(
        old.into_iter()
            .flatten()
            .all(|entity| app.world().get_entity(entity).is_err())
    );
    check(&mut app, first_color, second_color);
    for (view, offset) in [
        (first, Vec2::new(120.0, 64.0)),
        (second, Vec2::new(240.0, 96.0)),
    ] {
        let body = app
            .world()
            .get::<Children>(view)
            .unwrap()
            .iter()
            .find(|entity| app.world().get::<WidgetryTableBody>(*entity).is_some())
            .unwrap();
        app.world_mut().get_mut::<ScrollPosition>(body).unwrap().0 = offset;
    }
    app.update();
    check(&mut app, first_color, second_color);
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| bsn_list! { Text({format!("new {value}")}) bevy_widgetry_core::text::WidgetryText })).unwrap();
    app.update();
    check(&mut app, first_color, second_color);
    WidgetryTableColorOverrides::clear_in_world(app.world_mut(), first).unwrap();
    WidgetryTableColorOverrides::clear_in_world(app.world_mut(), second).unwrap();
    switch_theme(&mut app, WidgetryThemeMode::Light);
    app.update();
    check(
        &mut app,
        WidgetryThemeMode::Light
            .colors()
            .table
            .cell
            .normal
            .foreground,
        WidgetryThemeMode::Light
            .colors()
            .table
            .cell
            .disabled
            .foreground,
    );
}

#[test]
fn invalid_source_cleans_projection_and_recovers_through_host_handler() {
    let (mut app, source, root) = fixture();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app.update();
    let model = app
        .world_mut()
        .entity_mut(source)
        .take::<WidgetryTableModel<String>>()
        .unwrap();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        })
    });
    assert_eq!(errors.take().len(), 2);
    assert!(cells(&mut app, root).is_empty());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::ERROR)
            .count(),
        1
    );
    app.world_mut().entity_mut(source).insert(model);
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_eq!(cells(&mut app, root).len(), 2);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::INFO)
            .count(),
        1
    );
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .row_height = f32::NAN;
    errors.run(|| app.update());
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), Severity::Error);
    assert!(cells(&mut app, root).is_empty());
}

#[test]
fn real_layout_scrolls_each_header_with_only_its_body_axis() {
    let (mut app, source, root) = fixture();
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap();
        for index in 0..15 {
            model.push_row(format!("Row {index}")).unwrap();
        }
        for index in 0..4 {
            model
                .push_column(WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new(format!("Column {index}")),
                    (),
                    |row: &String, _| WidgetryTableCellValue::new(row.clone()),
                ))
                .unwrap();
        }
    }
    for _ in 0..3 {
        app.update();
    }
    let part = |app: &App, check: fn(&World, Entity) -> bool| {
        app.world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|&entity| check(app.world(), entity))
            .unwrap()
    };
    let body = part(&app, |world, entity| {
        world.get::<WidgetryTableBody>(entity).is_some()
    });
    let columns = part(&app, |world, entity| {
        world.get::<WidgetryTableColumnHeaders>(entity).is_some()
    });
    let rows = part(&app, |world, entity| {
        world.get::<WidgetryTableRowHeaders>(entity).is_some()
    });
    let corner = part(&app, |world, entity| {
        world.get::<WidgetryTableCorner>(entity).is_some()
    });
    let fixed = *app.world().get::<UiGlobalTransform>(corner).unwrap();
    app.world_mut().trigger(PointerScroll {
        entity: body,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 600,
                    height: 400,
                },
                position: Vec2::ZERO,
            },
        ),
        unit: MouseScrollUnit::Pixel,
        x: -180.0,
        y: -95.0,
        phase: TouchPhase::Moved,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    });
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::new(180.0, 95.0)
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(columns).unwrap().0,
        Vec2::new(180.0, 0.0)
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(rows).unwrap().0,
        Vec2::new(0.0, 95.0)
    );
    assert_eq!(
        *app.world().get::<UiGlobalTransform>(corner).unwrap(),
        fixed
    );
    let cell = cells(&mut app, root)[0];
    let header = app
        .world_mut()
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(app.world())
        .find(|(_, header)| header.column == cell.1.column)
        .unwrap()
        .0;
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryTableRowHeader)>()
        .iter(app.world())
        .find(|(_, row)| row.row == cell.1.row)
        .unwrap()
        .0;
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(cell.0)
            .unwrap()
            .translation
            .x,
        app.world()
            .get::<UiGlobalTransform>(header)
            .unwrap()
            .translation
            .x
    );
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(cell.0)
            .unwrap()
            .translation
            .y,
        app.world()
            .get::<UiGlobalTransform>(row)
            .unwrap()
            .translation
            .y
    );
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(360);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().get::<ScrollPosition>(columns).unwrap().0.x,
        app.world().get::<ScrollPosition>(body).unwrap().0.x
    );
}

#[test]
fn invalid_scene_configuration_is_logged_and_leaves_no_root() {
    let mut app = scene_app();
    app.register_widgetry_table::<String>();
    let count = app.world_mut().query::<Entity>().iter(app.world()).count();
    let logs = LogCapture::default();
    let missing = logs
        .run(|| {
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! { @WidgetryTable::<String> },
            )
        })
        .unwrap_err();
    let error = BevyError::error(missing);
    assert_eq!(error.severity(), Severity::Error);
    assert!(error.to_string().contains("requires source"));
    let source = app
        .world_mut()
        .spawn(WidgetryTableModel::<String>::default())
        .id();
    let invalid = logs.run(|| bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryTable::<String> { @source: source, @layout: {{ let mut layout = WidgetryTableLayout::default(); layout.row_height = 0.0; layout }} }
    })).unwrap_err();
    assert!(invalid.to_string().contains("finite and positive"));
    assert!(
        app.world()
            .get::<WidgetryTableModel<String>>(source)
            .is_some()
    );
    assert_eq!(
        app.world_mut().query::<Entity>().iter(app.world()).count(),
        count + 1
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::ERROR)
            .count(),
        2
    );
}

#[test]
fn layout_change_requests_redraw_until_flexible_width_converges() {
    let (mut app, _, root) = unmeasured_fixture();
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .default_column_width = WidgetryTableColumnWidth::Flexible(1.0);
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    let cell = cells(&mut app, root)[0].0;
    assert_eq!(app.world().get::<Node>(cell).unwrap().width, px(254));
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(360);
    app.update();
    assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert_eq!(app.world().get::<Node>(cell).unwrap().width, px(314));
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}

#[test]
fn controlled_row_projection_uses_display_order_and_source_values_without_reordering_model() {
    let (mut app, source, root) = fixture();
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    let rows = [model.row_id(0).unwrap(), model.row_id(1).unwrap()];
    let column = model.column_id(0).unwrap();
    WidgetryTable::<String>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate {
            rows: vec![rows[1], rows[0]],
            sort: Some(WidgetryTableSort {
                column,
                direction: WidgetryTableSortDirection::Descending,
            }),
            filters: [column].into(),
        },
    )
    .unwrap();
    app.update();
    let mut rendered = cells(&mut app, root)
        .into_iter()
        .map(|(shell, cell)| {
            let y = app
                .world()
                .get::<UiGlobalTransform>(shell)
                .unwrap()
                .translation
                .y;
            (y, cell.row, text(&app, shell).1)
        })
        .collect::<Vec<_>>();
    rendered.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(
        rendered
            .iter()
            .map(|item| (item.1, item.2.as_str()))
            .collect::<Vec<_>>(),
        vec![(rows[1], "Bob"), (rows[0], "Alice")]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableModel<String>>(source)
            .unwrap()
            .row_id(0),
        Some(rows[0])
    );
}

#[test]
fn view_updates_validate_rows_and_all_header_markers_atomically_and_queue_checks_at_execution() {
    let (mut app, source, root) = fixture();
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    let a = model.row_id(0).unwrap();
    let b = model.row_id(1).unwrap();
    let column = model.column_id(0).unwrap();
    let valid = WidgetryTableViewUpdate {
        rows: vec![b],
        sort: Some(WidgetryTableSort {
            column,
            direction: WidgetryTableSortDirection::Ascending,
        }),
        filters: [column].into(),
    };
    assert!(WidgetryTable::<String>::update_view(app.world_mut(), root, valid.clone()).unwrap());
    assert!(!WidgetryTable::<String>::update_view(app.world_mut(), root, valid.clone()).unwrap());
    let (stale_row, stale_column) = {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap();
        let row = model.push_row("removed".to_owned()).unwrap();
        model.remove_row(2);
        let col = model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("removed".to_owned()),
                (),
                |row: &String, _| WidgetryTableCellValue::new(row.clone()),
            ))
            .unwrap();
        model.remove_column(1);
        (row, col)
    };
    let capture = LogCapture::default();
    for invalid in [
        WidgetryTableViewUpdate {
            rows: vec![a, a],
            ..valid.clone()
        },
        WidgetryTableViewUpdate {
            rows: vec![stale_row],
            ..valid.clone()
        },
        WidgetryTableViewUpdate {
            sort: Some(WidgetryTableSort {
                column: stale_column,
                direction: WidgetryTableSortDirection::Descending,
            }),
            ..valid.clone()
        },
        WidgetryTableViewUpdate {
            filters: [column, stale_column].into(),
            ..valid.clone()
        },
    ] {
        let error = capture
            .run(|| WidgetryTable::<String>::update_view(app.world_mut(), root, invalid))
            .unwrap_err();
        assert_eq!(error.severity(), Severity::Error);
        let state = app.world().get::<WidgetryTableViewState>(root).unwrap();
        assert_eq!(state.ordered_rows(), &[b]);
        assert_eq!(state.sort(), valid.sort);
        assert_eq!(state.filters(), &valid.filters);
    }
    assert!(
        capture
            .records()
            .iter()
            .any(|record| record.level == Level::ERROR)
    );
    app.set_error_handler(ErrorCapture::handler());
    app.world_mut().commands().queue(move |world: &mut World| {
        world
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap()
            .remove_row(0);
    });
    WidgetryTable::<String>::queue_update_view(
        &mut app.world_mut().commands(),
        root,
        WidgetryTableViewUpdate {
            rows: vec![a],
            ..default()
        },
    );
    let errors = ErrorCapture::default();
    capture.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .ordered_rows(),
        &[b]
    );
    WidgetryTable::<String>::update_view(app.world_mut(), root, WidgetryTableViewUpdate::default())
        .unwrap();
    app.update();
    assert!(cells(&mut app, root).is_empty());
    assert!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .is_controlled()
    );
    WidgetryTable::<String>::queue_reset_view(&mut app.world_mut().commands(), root);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .ordered_rows(),
        &[b]
    );
    assert!(
        !app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .is_controlled()
    );
}

#[test]
fn shared_controlled_projection_preserves_identity_and_waits_for_application_recomputation() {
    let (mut app, source, a) = fixture();
    let b = app.world_mut().spawn_scene(bsn! { @WidgetryTable::<String> { @source: source } Node { width: px(300), height: px(150) } }).unwrap().id();
    app.update();
    app.update();
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    let ids = [model.row_id(0).unwrap(), model.row_id(1).unwrap()];
    let column = model.column_id(0).unwrap();
    WidgetryTable::<String>::update_view(
        app.world_mut(),
        a,
        WidgetryTableViewUpdate {
            rows: vec![ids[1], ids[0]],
            sort: Some(WidgetryTableSort {
                column,
                direction: WidgetryTableSortDirection::Descending,
            }),
            filters: [column].into(),
        },
    )
    .unwrap();
    app.update();
    let old_shell = cells(&mut app, a)
        .into_iter()
        .find(|(_, cell)| cell.row == ids[1])
        .unwrap()
        .0;
    let inserted = {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<String>>(source)
            .unwrap();
        let id = model.insert_row(0, "New".to_owned()).unwrap();
        *model.row_mut(2).unwrap().unwrap() = "Updated Bob".to_owned();
        model.move_row(0, 2);
        id
    };
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(a)
            .unwrap()
            .ordered_rows(),
        &[ids[1], ids[0]]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(b)
            .unwrap()
            .ordered_rows(),
        &[ids[0], ids[1], inserted]
    );
    assert_eq!(text(&app, old_shell).1, "Updated Bob");
    assert!(
        cells(&mut app, a)
            .iter()
            .any(|(shell, _)| *shell == old_shell)
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .remove_row(0);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(a)
            .unwrap()
            .ordered_rows(),
        &[ids[1]]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(a)
            .unwrap()
            .display_index(ids[1]),
        Some(0)
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .remove_column(0);
    app.update();
    let state = app.world().get::<WidgetryTableViewState>(a).unwrap();
    assert_eq!(state.sort(), None);
    assert!(state.filters().is_empty());
}

#[test]
fn synchronous_view_apis_observe_successive_structural_edits_within_one_world_tick() {
    let (mut app, source, root) = fixture();
    let c = app
        .world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .push_row("C".to_owned())
        .unwrap();
    WidgetryTable::<String>::reset_view(app.world_mut(), root).unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<String>>(source)
        .unwrap()
        .move_row(0, 2);
    let model = app
        .world()
        .get::<WidgetryTableModel<String>>(source)
        .unwrap();
    let expected = (0..model.row_count())
        .map(|i| model.row_id(i).unwrap())
        .collect::<Vec<_>>();
    let column = model.column_id(0).unwrap();
    WidgetryTable::<String>::set_focused_cell(
        app.world_mut(),
        root,
        Some(WidgetryTableCell { row: c, column }),
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .ordered_rows(),
        expected
    );
}
