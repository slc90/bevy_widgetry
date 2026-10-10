//! Coverage Model：二维 viewport、可见/不可见数据和空 Axis。
//! stimuli：真实 Scroll、viewport resize、Model mutation、despawn。
//! invariant：只有相交 Cell/Header，pair 唯一且 Content 对应当前数据。
//! 重叠 identity 保留 entity。
//! 增量 contract：静止及重叠区域不重复调用 schema。
//! revision/type renderer replacement 更新对应内容。
//! coupling：scroll × 两个 Axis。
//! selection/focus 的真实输入由 interaction.rs 负责。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod common;

use bevy::prelude::*;
use bevy::ui::{ScrollPosition, UiSystems};
use bevy_widgetry_table::*;
use common::{fixture, projection, scroll, uninitialized_fixture};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn schema_projection_only_runs_for_new_or_invalidated_cells() {
    let (mut app, source, root, body) = fixture(2000, 40);
    let calls = Arc::new(AtomicUsize::new(0));
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        for index in 0..40 {
            let calls = calls.clone();
            model
                .set_column(
                    index,
                    WidgetryTableColumn::new(
                        WidgetryTableHeaderValue::new(format!("C{index}")),
                        index,
                        move |row: &u32, column: &usize| {
                            calls.fetch_add(1, Ordering::Relaxed);
                            WidgetryTableCellValue::new(format!("{row}/{column}"))
                        },
                    ),
                )
                .unwrap();
        }
    }
    app.update();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 8);
    app.update();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 0);
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
        bsn_list! {Text({ value.to_string() }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    app.update();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 0);
    scroll(&mut app, body, Vec2::new(0.0, 1.0));
    assert_eq!(calls.swap(0, Ordering::Relaxed), 2);
    scroll(&mut app, body, Vec2::new(0.0, 1.0));
    assert_eq!(calls.swap(0, Ordering::Relaxed), 0);
    *app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_mut(0)
        .unwrap()
        .unwrap() = 777;
    *app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_mut(1800)
        .unwrap()
        .unwrap() = 999;
    app.update();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 2);
    assert!(
        projection(&mut app, root)
            .values()
            .any(|(_, text)| text == "777/0")
    );
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list! {Text({ format!("new {value}") }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
    app.update();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 10);
    assert!(
        projection(&mut app, root)
            .values()
            .all(|(_, text)| text.starts_with("new "))
    );
    scroll(&mut app, body, Vec2::new(0.0, 50398.0));
    assert_eq!(calls.swap(0, Ordering::Relaxed), 8);
    assert!(
        projection(&mut app, root)
            .values()
            .any(|(_, text)| text == "new 999/0")
    );
}

#[test]
fn column_headers_are_bounded_and_independent_of_body_rows() {
    let (mut app, source, root, body) = fixture(2000, 40);
    let initial = column_headers(&mut app);
    assert_eq!(initial.len(), 2);
    scroll(&mut app, body, Vec2::new(1.0, 0.0));
    let partial = column_headers(&mut app);
    assert_eq!(partial.len(), 3);
    for (id, entity) in &initial {
        assert_eq!(partial[id], *entity);
    }
    scroll(&mut app, body, Vec2::new(599.0, 1400.0));
    let distant = column_headers(&mut app);
    assert_eq!(distant.len(), 2);
    for (entity, _) in initial.values() {
        assert!(!app.world().entities().contains(*entity));
    }
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_rows();
    app.update();
    assert!(projection(&mut app, root).is_empty());
    assert_eq!(column_headers(&mut app), distant);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(46);
    app.update();
    app.update();
    assert!(column_headers(&mut app).is_empty());
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(286);
    app.update();
    app.update();
    assert_eq!(column_headers(&mut app).len(), 2);
}

fn column_headers(app: &mut App) -> HashMap<WidgetryTableColumnId, (Entity, Entity)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(world)
        .map(|(entity, header)| {
            (
                header.column,
                (entity, world.get::<Children>(entity).unwrap()[0]),
            )
        })
        .collect()
}

#[test]
fn initially_hidden_table_defers_all_axis_content() {
    let (mut app, _, root, _) = uninitialized_fixture(2000, 40);
    app.world_mut().get_mut::<Node>(root).unwrap().display = Display::None;
    app.update();
    app.update();
    assert!(projection(&mut app, root).is_empty());
    assert!(column_headers(&mut app).is_empty());
    assert!(row_headers(&mut app).is_empty());
    app.world_mut().get_mut::<Node>(root).unwrap().display = Display::Grid;
    app.update();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 8);
    assert_eq!(column_headers(&mut app).len(), 2);
    assert_eq!(row_headers(&mut app).len(), 4);
}

#[test]
fn settled_table_does_not_invalidate_layout_on_unrelated_updates() {
    let (mut app, _, root, _) = fixture(2000, 40);
    app.add_systems(PostUpdate, assert_quiet_nodes.before(UiSystems::Layout));
    app.update();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 8);
}

fn assert_quiet_nodes(nodes: Query<Entity, Changed<Node>>, mut initialized: Local<bool>) {
    if *initialized {
        assert_eq!(nodes.iter().count(), 0, "settled Table dirtied Node");
    }
    *initialized = true;
}

#[test]
fn settled_table_does_not_republish_state_or_shell_identity() {
    let (mut app, _, _, _) = fixture(2000, 40);
    app.add_systems(
        PostUpdate,
        assert_quiet_identity.after(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
    );
    app.update();
    app.update();
}

fn assert_quiet_identity(
    changed: Query<
        Entity,
        Or<(
            Changed<WidgetryTableState>,
            Changed<WidgetryTableCell>,
            Changed<WidgetryTableColumnHeader>,
            Changed<WidgetryTableRowHeader>,
        )>,
    >,
    mut initialized: Local<bool>,
) {
    if *initialized {
        assert_eq!(
            changed.iter().count(),
            0,
            "settled Table dirtied identity/state"
        );
    }
    *initialized = true;
}

#[test]
fn row_headers_are_bounded_by_viewport_and_reuse_overlapping_rows() {
    let (mut app, source, _, body) = fixture(2000, 40);
    let initial = row_headers(&mut app);
    assert_eq!(initial.len(), 4);
    scroll(&mut app, body, Vec2::new(0.0, 1.0));
    let partial = row_headers(&mut app);
    assert_eq!(partial.len(), 5);
    for (row, (entity, _)) in &initial {
        assert_eq!(partial[row].0, *entity);
    }
    scroll(&mut app, body, Vec2::new(600.0, 1399.0));
    let distant = row_headers(&mut app);
    assert_eq!(distant.len(), 4);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    for (row, (_, label)) in &distant {
        let index = model.row_index(*row).unwrap();
        assert!((50..54).contains(&index));
        assert_eq!(label, &(index + 1).to_string());
    }
    for (_, (entity, _)) in initial {
        assert!(!app.world().entities().contains(entity));
    }
    app.world_mut().get_mut::<Node>(body).unwrap().display = Display::None;
    app.update();
    app.update();
    assert!(row_headers(&mut app).is_empty());
    app.world_mut().get_mut::<Node>(body).unwrap().display = Display::Flex;
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .move_row(50, 0);
    app.update();
    app.update();
    for (row, (_, label)) in row_headers(&mut app) {
        let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
        assert_eq!(label, (model.row_index(row).unwrap() + 1).to_string());
    }
}

fn row_headers(app: &mut App) -> std::collections::HashMap<WidgetryTableRowId, (Entity, String)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableRowHeader)>()
        .iter(world)
        .map(|(entity, header)| {
            let child = world.get::<Children>(entity).unwrap()[0];
            (
                header.row,
                (entity, world.get::<Text>(child).unwrap().0.clone()),
            )
        })
        .collect()
}

#[test]
fn two_axes_keep_only_intersecting_pairs_and_current_content() {
    let (mut app, source, root, body) = fixture(100, 30);
    let initial = projection(&mut app, root);
    assert_eq!(initial.len(), 8);
    scroll(&mut app, body, Vec2::new(1.0, 1.0));
    let partial = projection(&mut app, root);
    assert_eq!(partial.len(), 15);
    for (pair, (entity, _)) in &initial {
        assert_eq!(partial[pair].0, *entity);
    }
    for target in [
        Vec2::new(600.0, 560.0),
        Vec2::new(121.0, 29.0),
        Vec2::new(1400.0, 1500.0),
        Vec2::ZERO,
    ] {
        let current = app.world().get::<ScrollPosition>(body).unwrap().0;
        let old = projection(&mut app, root);
        scroll(&mut app, body, target - current);
        let actual = projection(&mut app, root);
        let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
        let first_row = (target.y / 28.0).floor() as usize;
        let last_row = ((target.y + 112.0) / 28.0).ceil() as usize;
        let first_column = (target.x / 120.0).floor() as usize;
        let last_column = ((target.x + 240.0) / 120.0).ceil() as usize;
        assert_eq!(
            actual.len(),
            (last_row - first_row) * (last_column - first_column)
        );
        for ((row, column), (_, content)) in &actual {
            let ri = model.row_index(*row).unwrap();
            let ci = model.column_index(*column).unwrap();
            assert!((first_row..last_row).contains(&ri));
            assert!((first_column..last_column).contains(&ci));
            assert_eq!(content, &format!("{ri}/{ci}"));
        }
        for (pair, (entity, _)) in old {
            if !actual.contains_key(&pair) {
                assert!(!app.world().entities().contains(entity));
            }
        }
    }
}

#[test]
fn mutations_resize_empty_axes_and_despawn_keep_projection_current() {
    let (mut app, source, root, body) = fixture(100, 30);
    let initial = projection(&mut app, root);
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        *model.row_mut(0).unwrap().unwrap() = 777;
        *model.row_mut(50).unwrap().unwrap() = 999;
    }
    app.update();
    let changed = projection(&mut app, root);
    for (pair, (entity, _)) in &initial {
        assert_eq!(changed[pair].0, *entity);
    }
    let row = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_id(0)
        .unwrap();
    assert!(
        changed
            .iter()
            .filter(|(pair, _)| pair.0 == row)
            .all(|(_, (_, text))| text.starts_with("777/"))
    );
    scroll(&mut app, body, Vec2::new(240.0, 1400.0));
    assert!(
        projection(&mut app, root)
            .values()
            .any(|(_, text)| text == "999/2")
    );
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(406);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(200);
    app.update();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 18);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(166);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(88);
    app.update();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 2);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .move_column(2, 0);
    app.update();
    assert!(
        projection(&mut app, root)
            .values()
            .any(|(_, text)| text == "999/1")
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_rows();
    app.update();
    assert!(projection(&mut app, root).is_empty());
    assert_eq!(app.world().get::<ScrollPosition>(body).unwrap().0.y, 0.0);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .push_row(123)
        .unwrap();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 1);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_columns();
    app.update();
    assert!(projection(&mut app, root).is_empty());
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::ZERO
    );
    app.world_mut().despawn(root);
    assert!(app.world().get::<WidgetryTableModel<u32>>(source).is_some());
    assert!(!app.world().entities().contains(body));
}

#[test]
fn zero_viewport_and_invalid_scroll_do_not_leave_stale_cells() {
    let (mut app, source, root, body) = fixture(100, 30);
    app.world_mut().get_mut::<ScrollPosition>(body).unwrap().0 = Vec2::splat(f32::NAN);
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::ZERO
    );
    app.world_mut().get_mut::<ScrollPosition>(body).unwrap().0 = Vec2::splat(f32::MAX);
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0,
        Vec2::new(3360.0, 2688.0)
    );
    assert_eq!(projection(&mut app, root).len(), 8);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(46);
    app.update();
    app.update();
    assert!(projection(&mut app, root).is_empty());
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(286);
    app.update();
    app.update();
    assert_eq!(projection(&mut app, root).len(), 8);
    let old = projection(&mut app, root);
    app.world_mut().despawn(root);
    assert!(
        old.values()
            .all(|(entity, _)| !app.world().entities().contains(*entity))
    );
    assert!(app.world().get::<WidgetryTableModel<u32>>(source).is_some());
}

#[test]
fn fractional_geometry_matches_physical_scroll_quantization() {
    for scale in [1.0, 1.25, 2.0] {
        let (mut app, source, root, body) = fixture(100, 30);
        *app.world_mut().resource_mut::<bevy::ui::UiScale>() = bevy::ui::UiScale(scale);
        {
            let mut layout = app
                .world_mut()
                .get_mut::<WidgetryTableLayout>(root)
                .unwrap();
            layout.row_height = 27.5;
            layout.default_column_width = WidgetryTableColumnWidth::Fixed(120.5);
        }
        app.update();
        app.update();
        let inverse = app
            .world()
            .get::<ComputedNode>(body)
            .unwrap()
            .inverse_scale_factor();
        assert!((inverse - 1.0 / scale).abs() < 0.001);
        let first_row = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .row_id(0)
            .unwrap();
        let first_column = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        scroll(&mut app, body, Vec2::new(120.5, 27.5));
        let actual = projection(&mut app, root);
        let physical_offset = app
            .world()
            .get::<ComputedNode>(body)
            .unwrap()
            .scroll_position
            * inverse;
        assert_eq!(
            physical_offset,
            (Vec2::new(120.5, 27.5) * scale).floor() / scale
        );
        if physical_offset.y < 27.5 {
            assert!(actual.keys().any(|pair| pair.0 == first_row));
        }
        if physical_offset.x < 120.5 {
            assert!(actual.keys().any(|pair| pair.1 == first_column));
        }
        let size = app.world().get::<ComputedNode>(body).unwrap().size() * inverse;
        let rows = (physical_offset.y / 27.5).floor() as usize
            ..((physical_offset.y + size.y) / 27.5).ceil() as usize;
        let columns = (physical_offset.x / 120.5).floor() as usize
            ..((physical_offset.x + size.x) / 120.5).ceil() as usize;
        assert_eq!(actual.len(), rows.len() * columns.len());
        let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
        for (pair, (entity, _)) in &actual {
            assert!(rows.contains(&model.row_index(pair.0).unwrap()));
            assert!(columns.contains(&model.column_index(pair.1).unwrap()));
            let computed = app.world().get::<ComputedNode>(*entity).unwrap();
            assert!((computed.size().y * inverse - 27.5).abs() < 0.001);
            assert!((computed.size().x * inverse - 120.5).abs() < 0.001);
        }
    }
}
