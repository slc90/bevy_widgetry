// integration test 及其 helper 使用断言和 unwrap 验证 contract，生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

//! Renderer Coverage Model：value type × Cell/Header registration；覆盖异构内容、独立派发、replacement 与缺失错误。
//! stimuli：公开 App 注册、value projection、BSN Content 展开；guard：缺失注册返回 Error，无 fallback。
//! invariant：每个 SceneList 使用本次 value，Header/Cell registry 隔离，Content 不覆盖 shell。
//! Coverage Map：本文件负责 renderer contract；后续 View/virtualization/interaction 文件负责真实 Table lifecycle 和输入。

use bevy::ecs::error::Severity;
use bevy::log::tracing::Level;
use bevy::prelude::*;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{LogCapture, scene_app};

/// 自定义 Progress 语义值，renderer 使用 Reflect registration。
#[derive(Reflect)]
struct Progress(f32);

/// 非 String Header 的组合语义数据。
#[derive(Reflect)]
struct Header {
    label: String,
    badge: u32,
}

/// 在真实 BSN shell 上展开 renderer Content，读取其 direct child 输出。
fn content(app: &mut App, scene: Box<dyn SceneList>) -> (Entity, String) {
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            Node { padding: UiRect::all(px(7)), border: UiRect::all(px(2)) }
            BackgroundColor(Color::srgb(0.1, 0.2, 0.3))
            Children [{scene}]
        })
        .unwrap()
        .id();
    let child = app.world().get::<Children>(root).unwrap()[0];
    (root, app.world().get::<Text>(child).unwrap().0.clone())
}

/// 同一 registry 中各实际类型独立 dispatch，同类型多个 value 不串值，Content 不覆盖 shell style。
#[test]
fn heterogeneous_values_create_owned_content_without_changing_shell() {
    let mut app = scene_app();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        bsn_list![(Text({ value.clone() }))]
    }))
    .unwrap();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
        bsn_list![(Text({ value.to_string() }))]
    }))
    .unwrap();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &bool| {
        bsn_list![(Text({ value.to_string() }))]
    }))
    .unwrap();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &Progress| {
        bsn_list![(Text({ format!("{}%", value.0) }))]
    }))
    .unwrap();
    for (value, expected) in [
        (WidgetryTableCellValue::new("Alice".to_owned()), "Alice"),
        (WidgetryTableCellValue::new(42u32), "42"),
        (WidgetryTableCellValue::new(true), "true"),
        (WidgetryTableCellValue::new(Progress(75.0)), "75%"),
        (WidgetryTableCellValue::new(8u32), "8"),
    ] {
        let scene = app
            .world()
            .resource::<WidgetryTableCellRendererRegistry>()
            .render(&value)
            .unwrap();
        let (root, actual) = content(&mut app, scene);
        assert_eq!(actual, expected);
        assert_eq!(
            app.world().get::<Node>(root).unwrap().padding,
            UiRect::all(px(7))
        );
        assert_eq!(
            app.world().get::<Node>(root).unwrap().border,
            UiRect::all(px(2))
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            Color::srgb(0.1, 0.2, 0.3)
        );
    }
}

/// 同 type 分别注册不同 Header/Cell factory，非 String Header 生成组合 Content，重新注册只替换对应 registry。
#[test]
fn independent_registries_support_custom_headers_and_replacement() {
    let mut app = scene_app();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
        bsn_list![(Text({ format!("cell {value}") }))]
    }))
    .unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &u32| {
        bsn_list![(Text({ format!("header {value}") }))]
    }))
    .unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &Header| {
        bsn_list![
            (Text({ value.label.clone() })),
            (Text({ value.badge.to_string() }))
        ]
    }))
    .unwrap();
    let cell = app
        .world()
        .resource::<WidgetryTableCellRendererRegistry>()
        .render(&WidgetryTableCellValue::new(3u32))
        .unwrap();
    let header = app
        .world()
        .resource::<WidgetryTableHeaderRendererRegistry>()
        .render(&WidgetryTableHeaderValue::new(3u32))
        .unwrap();
    assert_eq!(content(&mut app, cell).1, "cell 3");
    assert_eq!(content(&mut app, header).1, "header 3");
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
        bsn_list![(Text({ format!("replacement {value}") }))]
    }))
    .unwrap();
    let cell = app
        .world()
        .resource::<WidgetryTableCellRendererRegistry>()
        .render(&WidgetryTableCellValue::new(3u32))
        .unwrap();
    let header = app
        .world()
        .resource::<WidgetryTableHeaderRendererRegistry>()
        .render(&WidgetryTableHeaderValue::new(3u32))
        .unwrap();
    assert_eq!(content(&mut app, cell).1, "replacement 3");
    assert_eq!(content(&mut app, header).1, "header 3");
    let header = app
        .world()
        .resource::<WidgetryTableHeaderRendererRegistry>()
        .render(&WidgetryTableHeaderValue::new(Header {
            label: "Age".into(),
            badge: 7,
        }))
        .unwrap();
    let (root, actual) = content(&mut app, header);
    assert_eq!(actual, "Age");
    let children = app.world().get::<Children>(root).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(app.world().get::<Text>(children[1]).unwrap().0, "7");
}

/// 注册 Cell 不隐式注册 Header；两方缺失都返回已记录 ERROR 的 Error，不生成任何 fallback。
#[test]
fn missing_renderers_return_logged_error_and_do_not_cross_registries() {
    let mut app = scene_app();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|_: &u32| {
        bsn_list![(Text("cell"))]
    }))
    .unwrap();
    let capture = LogCapture::default();
    let errors = capture.run(|| {
        let header_error = app
            .world()
            .resource::<WidgetryTableHeaderRendererRegistry>()
            .render(&WidgetryTableHeaderValue::new(3u32))
            .err()
            .unwrap();
        let cell_error = app
            .world()
            .resource::<WidgetryTableCellRendererRegistry>()
            .render(&WidgetryTableCellValue::new(true))
            .err()
            .unwrap();
        [header_error, cell_error]
    });
    for error in errors {
        assert_eq!(error.severity(), Severity::Error);
        assert!(error.to_string().contains("no registered renderer"));
    }
    assert_eq!(capture.records().len(), 2);
    assert!(
        capture
            .records()
            .iter()
            .all(|record| record.level == Level::ERROR)
    );
}

/// plugin 已安装后 registry 丢失，两种注册入口仍记录 ERROR 并返回 Error，不触发宿主 panic。
#[test]
fn removed_registry_returns_logged_error_from_registration() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTablePlugin);
    app.world_mut()
        .remove_resource::<WidgetryTableCellRendererRegistry>();
    app.world_mut()
        .remove_resource::<WidgetryTableHeaderRendererRegistry>();
    let capture = LogCapture::default();
    let errors = capture.run(|| {
        let cell = app
            .register_table_cell_renderer(WidgetryTableCellRenderer::new(|_: &u32| bsn_list![]))
            .err()
            .unwrap();
        let header = app
            .register_table_header_renderer(WidgetryTableHeaderRenderer::new(|_: &u32| bsn_list![]))
            .err()
            .unwrap();
        [cell, header]
    });
    for error in errors {
        assert_eq!(error.severity(), Severity::Error);
        assert!(error.to_string().contains("registry missing"));
    }
    assert_eq!(capture.records().len(), 2);
    assert!(
        capture
            .records()
            .iter()
            .all(|record| record.level == Level::ERROR)
    );
}
