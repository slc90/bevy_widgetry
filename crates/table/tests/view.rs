// integration test 使用断言和 unwrap 验证公开 contract，生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
//! View Coverage Model：source 空/非空/失效、区域 layout/style、Content revision 与 lifecycle。
//! stimuli：BSN spawn、update、model mutation、renderer replacement、实际 scroll、style/disabled 和 despawn。
//! invariant：四区独立同步对应 scroll 轴，直接 Cell pair 对应当前 source，shell 与 Content ownership 分离。
//! Coverage Map：renderers.rs 负责类型注册；本文件负责 View/layout/style/source failure；virtualization.rs 负责两轴可见范围与回收；interaction.rs 负责 selection/cursor/focus、用户通知、输入guard/resize及其与virtualization/Model生命周期的组合。

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::error::Severity;
use bevy::ecs::message::Messages;
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::log::tracing::Level;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Pointer, Scroll};
use bevy::picking::pointer::{Location, PointerId};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiGlobalTransform};
use bevy::window::RequestRedraw;
use bevy_widgetry_core::ThemeMode;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, scene_app, spawn_ui_camera,
};

/// 通过公共 renderer 和 BSN 构造有界 View，不接触内部 runtime。
fn fixture() -> (App, Entity, Entity) {
    let (mut app, source, root) = unmeasured_fixture();
    // 首次 Layout 只测量 viewport，下一次 update 才构造可见 Cell。
    app.update();
    (app, source, root)
}

/// 保留首次 Layout 尚未测量的场景，供 redraw contract 验证。
fn unmeasured_fixture() -> (App, Entity, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
    app.register_widgetry_table::<String>();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list![(Text({ value.clone() }))]
    }))
    .unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| {
        bsn_list![(Text({ value.clone() }))]
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

/// source 接入后首次测量后 update 生成真实 Cell/Header Content，Cell 直接属于 Body canvas，销毁只清理自有 subtree。
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
    assert_eq!(app.world().get::<Children>(root).unwrap().len(), 4);
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

/// 仅依据公开 identity 查找该 Table 的 physical Cell，避免跨 source-local ID 串用。
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

/// 读取 shell 的业务 Text direct child，Content entity identity 用于验证正确的 replacement。
fn text(app: &App, shell: Entity) -> (Entity, String) {
    let child = app.world().get::<Children>(shell).unwrap()[0];
    (child, app.world().get::<Text>(child).unwrap().0.clone())
}

/// 成功 mutable access、Header/schema replacement、move 与 renderer replacement 更新本 View，另一 source 不受影响。
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
        bsn_list![(Text({ format!("new {value}") }))]
    }))
    .unwrap();
    app.update();
    assert_eq!(text(&app, first).1, "new Changed");
    assert_eq!(text(&app, other_cell).1, "new Other");
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &bool| {
        bsn_list![(Text({ format!("bool {value}") }))]
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

/// shell style 与 theme/disabled 更新保留 Content entity，disabled 覆盖全部 subtree picking 并正确恢复。
#[test]
fn style_and_disabled_preserve_content_and_restore_picking() {
    let (mut app, _, root) = fixture();
    app.update();
    let cell = cells(&mut app, root)[0].0;
    let content = text(&app, cell).0;
    let original = Pickable {
        should_block_lower: false,
        is_hoverable: true,
    };
    app.world_mut().entity_mut(content).insert(original);
    {
        let mut style = app.world_mut().get_mut::<WidgetryTableStyle>(root).unwrap();
        style.cell.background = Some(Color::srgb(0.1, 0.2, 0.3));
        style.cell.padding = UiRect::all(px(3));
        style.cell.border = UiRect::all(px(2));
        style.cell.disabled_background = Some(Color::srgb(0.3, 0.2, 0.1));
        style.corner.background = Some(Color::srgb(0.2, 0.4, 0.6));
    }
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
    for entity in [root, cell, content] {
        assert!(!app.world().get::<Pickable>(entity).unwrap().is_hoverable);
    }
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    *app.world_mut().resource_mut::<ThemeMode>() = ThemeMode::Light;
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
        ThemeMode::Light.colors().control_border
    );
}

/// source Component 失效清理旧 projection，持续错误只记录一次 ERROR，恢复重建并记录 INFO。
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

/// 真实 UI layout 与官方 pointer scroll 验证四区两轴对齐；viewport 扩缩后保持当前 ID 和区域坐标关系。
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
            x: -180.0,
            y: -95.0,
            phase: TouchPhase::Moved,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        body,
    ));
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

/// 公共 Scene 必填 source 与非法 layout 在构造入口记录 ERROR，失败清理本次 root，返回 Severity::Error。
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

/// 新建/resize 当帧 Layout 得到新 viewport 时必须请求下一帧，flexible width 收敛后停止请求。
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
