use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::log::tracing::Level;
use bevy::prelude::*;
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewPlugin,
    WidgetryListViewRenderer,
};
use bevy_widgetry_test_utils::{LogCapture, scene_app};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// 构造 contract 测试的 headless App，以单 thread 捕获 runtime 配置错误。
fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .register_widgetry_list_view::<u32>();
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}

/// 创建只有 public identity 的 view；source 的有效性由 typed runtime 检查。
fn view(app: &mut App, source: Entity, height: f32) -> Entity {
    app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @item_height: height,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
    }).expect("合法 ListView Scene 应成功展开").id()
}

/// 确认不可恢复配置错误在 panic 前产生 Widgetry ERROR。
fn assert_configuration_error(action: impl FnOnce()) {
    let capture = LogCapture::default();
    assert!(
        capture
            .run(|| catch_unwind(AssertUnwindSafe(action)))
            .is_err()
    );
    assert!(
        capture
            .records()
            .iter()
            .any(|record| record.level == Level::ERROR)
    );
}

/// 缺少必填 source 或 renderer 的 BSN 配置必须失败，不能生成静默无效的 view。
#[test]
fn missing_required_props_are_rejected() {
    let mut app = app();
    assert_configuration_error(|| {
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryListView::<String> })
            .unwrap();
    });
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    assert_configuration_error(|| {
        app.world_mut()
            .spawn_scene(bsn! {
                @WidgetryListView::<String> { @source: source }
            })
            .unwrap();
    });
    assert_configuration_error(|| {
        view(&mut app, Entity::PLACEHOLDER, 32.0);
    });
}

/// 零、负数和非有限高度均属于配置错误，先记录 ERROR 再终止。
#[test]
fn invalid_heights_are_rejected() {
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut app = app();
        let source = app
            .world_mut()
            .spawn(WidgetryListModel::<String>::default())
            .id();
        assert_configuration_error(|| {
            view(&mut app, source, height);
        });
    }
}

/// source type 不匹配或 entity 已销毁，typed runtime 都必须拒绝。
#[test]
fn source_must_exist_and_match_item_type() {
    for despawn in [false, true] {
        let mut app = app();
        let source = app
            .world_mut()
            .spawn(WidgetryListModel::<u32>::default())
            .id();
        if despawn {
            assert!(app.world_mut().despawn(source));
        }
        view(&mut app, source, 32.0);
        assert_configuration_error(|| {
            app.world_mut().run_schedule(PreUpdate);
        });
    }
}

/// 首次有效 source 在运行期移除匹配 model 或销毁后，同样必须报告 invariant 失效。
#[test]
fn source_invariant_is_checked_after_creation() {
    for despawn in [false, true] {
        let mut app = app();
        let source = app
            .world_mut()
            .spawn(WidgetryListModel::<String>::default())
            .id();
        view(&mut app, source, 32.0);
        app.world_mut().run_schedule(PreUpdate);
        if despawn {
            assert!(app.world_mut().despawn(source));
        } else {
            app.world_mut()
                .entity_mut(source)
                .remove::<WidgetryListModel<String>>();
        }
        assert_configuration_error(|| {
            app.world_mut().run_schedule(PreUpdate);
        });
    }
}

/// type-erased renderer 产出 owned SceneList，value 修改后已构造 scene 仍可作为 direct children 展开。
#[test]
fn renderer_produces_owned_direct_children() {
    let renderer = WidgetryListViewRenderer::new(|index, value: &String| {
        bsn_list![(Text(format!("{index}: {value}"))), (Text("suffix"))]
    });
    let mut value = String::from("before");
    let content = renderer.clone().render(7, &value);
    value.clear();
    let mut app = app();
    let row = app
        .world_mut()
        .spawn_scene(bsn! { Node Children [{content}] })
        .unwrap()
        .id();
    let children = app.world().get::<Children>(row).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(app.world().get::<Text>(children[0]).unwrap().0, "7: before");
    assert_eq!(app.world().get::<Text>(children[1]).unwrap().0, "suffix");
}

/// default renderer 只是未配置占位，直接 render 同样需要诊断配置错误。
#[test]
fn unconfigured_renderer_is_rejected() {
    assert_configuration_error(|| {
        WidgetryListViewRenderer::<String>::default().render(0, &String::new());
    });
}

/// 注册顺序错误不能留下仅部分装配的 typed runtime。
#[test]
fn registration_requires_common_plugin() {
    let mut app = App::new();
    assert_configuration_error(|| {
        app.register_widgetry_list_view::<String>();
    });
}
