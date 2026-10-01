// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! State：BSN 构造中/完整 shell、有效/失效 source 与 props 配置。
//! Stimuli：BSN Scene、source 移除/销毁、内部 shell 破坏、初始 disabled 与 Commands 构造。
//! Guard：source 必填且持续持有 TreeModel，indent 有限非负、item_height 有限正数。
//! Invariant：构造中间态允许延后同步，完整 shell 的无效配置必须 ERROR 后上抛错误。

#![cfg(test)]

use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::log::tracing::Level;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_list_view::WidgetryListView;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use bevy_widgetry_tree::{
    WidgetryTreeModel, WidgetryTreePlugin, WidgetryTreeView, WidgetryTreeVisibleItem,
};

/// 配置验证使用真实 plugin；单 thread schedule 使 ERROR 捕获不依赖 worker thread。
fn app() -> App {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_plugins(WidgetryTreePlugin);
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}

/// 每次失败必须同时满足返回错误与 Widgetry ERROR，不能把诊断降为静默空 UI。
fn assert_configuration_error<T, E: std::fmt::Debug>(action: impl FnOnce() -> Result<T, E>) {
    let capture = LogCapture::default();
    assert!(capture.run(action).is_err());
    assert!(
        capture
            .records()
            .iter()
            .any(|record| record.level == Level::ERROR)
    );
}

/// 真实 schedule 将错误交给宿主，返回 ERROR severity 并保留 Widgetry 日志。
fn assert_runtime_error(app: &mut App) {
    let capture = LogCapture::default();
    let errors = ErrorCapture::default();
    errors.run(|| capture.run(|| app.world_mut().run_schedule(PreUpdate)));
    let errors = errors.take();
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(
        capture
            .records()
            .iter()
            .any(|record| record.level == Level::ERROR)
    );
}

/// Tree 组合入口必须检查必填 source、indent 与传给 ListView 的行高，不允许无效值生成 UI。
#[test]
fn invalid_scene_configuration_is_rejected() {
    let mut app = app();
    assert_configuration_error(|| app.world_mut().spawn_scene(bsn! { @WidgetryTreeView }));
    for (indent, height) in [
        (f32::NAN, 32.0),
        (f32::INFINITY, 32.0),
        (-1.0, 32.0),
        (20.0, 0.0),
        (20.0, -1.0),
        (20.0, f32::NAN),
        (20.0, f32::INFINITY),
    ] {
        let root = app.world_mut().spawn_empty().id();
        let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
        assert_configuration_error(|| {
            app.world_mut().spawn_scene(bsn! { @WidgetryTreeView { @source: source, @indent_width: indent, @item_height: height } })
        });
    }
}

/// source 错误 type、删除或移除 TreeModel 均违反持续 contract，首次及后续 PreUpdate 必须报告。
#[test]
fn source_contract_is_checked_at_creation_and_after_external_mutation() {
    for mutation in 0..3 {
        let mut app = app();
        let root = app.world_mut().spawn_empty().id();
        let source = app.world_mut().spawn_empty().id();
        if mutation != 0 {
            app.world_mut()
                .entity_mut(source)
                .insert(WidgetryTreeModel::new(root));
        }
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
            .unwrap();
        if mutation != 0 {
            app.world_mut().run_schedule(PreUpdate);
            if mutation == 1 {
                app.world_mut().despawn(source);
            } else {
                app.world_mut()
                    .entity_mut(source)
                    .remove::<WidgetryTreeModel>();
            }
        }
        assert_runtime_error(&mut app);
    }
}

/// 以 Commands 构造初始 disabled Scene 不得在 children materialization 前同步，也必须在首次 update 限制输入。
#[test]
fn initially_disabled_commands_scene_builds_before_runtime_synchronization() {
    let mut app = app();
    let root = app.world_mut().spawn_empty().id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @WidgetryTreeView { @source: source, @indent_width: 0.0, @item_height: 48.0 }
            InteractionDisabled
        })
        .id();
    app.update();
    let list = app.world().get::<Children>(view).unwrap()[0];
    assert_eq!(
        app.world()
            .get::<WidgetryListView<WidgetryTreeVisibleItem>>(list)
            .unwrap()
            .item_height(),
        48.0
    );
    assert!(app.world().get::<InteractionDisabled>(list).is_some());
    app.world_mut()
        .entity_mut(view)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert!(app.world().get::<InteractionDisabled>(list).is_none());
}

/// 修复构造时序不得隐藏真正的 shell 损坏；已构造内部 ListView 被删除后仍必须 ERROR 后上抛错误。
#[test]
fn broken_completed_shell_is_not_treated_as_pending_construction() {
    let mut app = app();
    let root = app.world_mut().spawn_empty().id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    app.world_mut().run_schedule(PreUpdate);
    let list = app.world().get::<Children>(view).unwrap()[0];
    app.world_mut().despawn(list);
    app.world_mut().entity_mut(view).insert(InteractionDisabled);
    assert_runtime_error(&mut app);
}
