// integration test 通过公开输入与 identity 验证业务 projection，允许测试断言和 unwrap。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
//! Coverage Model：二维 viewport、可见/不可见数据和空 Axis。
//! stimuli：真实 Scroll、viewport resize、Model mutation、despawn。
//! invariant：只有相交 Cell，pair 唯一且 Content 对应当前数据；重叠 pair 保留实体。
//! coupling：scroll × 两个 Axis；selection/focus 的真实输入由 interaction.rs 负责。

use bevy::camera::NormalizedRenderTarget;
use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
use bevy::picking::{
    backend::HitData,
    events::{Pointer, Scroll},
    pointer::{Location, PointerId},
};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use std::collections::HashMap;

/// 两个 Axis 都超出真实 headless viewport，Cell Text 明确包含当前 pair 对应的数据。
fn fixture() -> (App, Entity, Entity, Entity) {
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
    for row in 0..100 {
        model.push_row(row).unwrap();
    }
    for column in 0..30 {
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
    app.update();
    app.update();
    let body = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&entity| app.world().get::<WidgetryTableBody>(entity).is_some())
        .unwrap();
    (app, source, root, body)
}

/// 读取公开 Cell marker 与 renderer Text，同时验证 Canvas 两级层次。
fn projection(
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

/// 真实 Scroll 走官方 ScrollArea observer，不替换私有 viewport cache。
fn scroll(app: &mut App, body: Entity, delta: Vec2) {
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

/// exact boundary 只包含相交的2×4 Cell；两轴部分可见和快速往返逐帧验证 pair、Content 与回收。
#[test]
fn two_axes_keep_only_intersecting_pairs_and_current_content() {
    let (mut app, source, root, body) = fixture();
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

/// 可见 revision 立即更新，offscreen 数据进入 viewport 时取最新值；viewport 扩缩、move 和空 Axis 都清理旧 pair。
#[test]
fn mutations_resize_empty_axes_and_despawn_keep_projection_current() {
    let (mut app, source, root, body) = fixture();
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

/// viewport 为零清理可见 Content，恢复后重建；无效 scroll 修复，超出末端按完整 canvas clamp。
#[test]
fn zero_viewport_and_invalid_scroll_do_not_leave_stale_cells() {
    let (mut app, source, root, body) = fixture();
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

/// fractional logical 尺寸与非整数 DPI 的真实 scroll 保留仍有 physical 像素相交的首行/首列。
#[test]
fn fractional_geometry_matches_physical_scroll_quantization() {
    for scale in [1.0, 1.25, 2.0] {
        let (mut app, source, root, body) = fixture();
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
