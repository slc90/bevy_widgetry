use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{
    FocusCause, FocusedInput, InputFocus, InputFocusSystems, dispatch_focused_input,
};
use bevy::log::tracing::Level;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, ListBox, ListBoxPlugin, ScrollArea, Scrollbar};
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::{ForegroundColorPlugin, ThemePlugin};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewPlugin,
    WidgetryListViewRenderer,
};
use bevy_widgetry_scroll_area::{
    WidgetryScrollArea, WidgetryScrollAreaContent, WidgetryScrollAreaViewport,
};
use bevy_widgetry_test_utils::{LogCapture, scene_app};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// 记录 ListView shell 未消费且抵达 ancestor 的 keyboard event。
#[derive(Resource, Default)]
struct AncestorKeyboardCount(usize);

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

/// BSN shell 在同 root 复用 ScrollArea，公开唯一 Viewport/Content 与原生 ScrollPosition，保留调用方 Node patch。
#[test]
fn shell_shares_scroll_root_and_exposes_public_content() {
    let mut app = app();
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
        Node { width: px(240), height: px(128), padding: UiRect::all(px(3)) }
    }).unwrap().id();
    assert!(app.world().get::<WidgetryScrollArea>(root).is_some());
    assert!(app.world().get::<ListBox>(root).is_none());
    assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, 0);
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    assert_eq!(app.world().get::<ChildOf>(viewport).unwrap().parent(), root);
    assert!(app.world().get::<ScrollArea>(viewport).is_some());
    assert!(app.world().get::<ScrollPosition>(viewport).is_some());
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<ChildOf>(content).unwrap().parent(),
        viewport
    );
    assert_eq!(app.world().get::<Children>(content).unwrap().len(), 2);
    for child in app.world().get::<Children>(content).unwrap().iter() {
        assert!(app.world().get::<TabIndex>(child).is_none());
        assert_eq!(app.world().get::<Node>(child).unwrap().height, px(0));
    }
    app.update();
    let root_node = app.world().get::<Node>(root).unwrap();
    assert_eq!(root_node.width, px(240));
    assert_eq!(root_node.height, px(128));
    assert_eq!(root_node.padding, UiRect::all(px(3)));
    let bars = app
        .world_mut()
        .query::<(Entity, &Scrollbar)>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].1.target, viewport);
    assert_eq!(bars[0].1.orientation, ControlOrientation::Vertical);
    assert_eq!(
        app.world().get::<Node>(bars[0].0).unwrap().display,
        Display::None
    );
    assert!(app.is_plugin_added::<ThemePlugin>());
    assert!(app.is_plugin_added::<ForegroundColorPlugin>());
}

/// 已装配官方 ListBoxPlugin 时，ListView root 不消费其未支持的横向导航和业务按键。
#[test]
fn shell_leaves_unsupported_keyboard_input_to_ancestors() {
    let mut app = app();
    app.init_resource::<bevy::ui::UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<AncestorKeyboardCount>()
        .add_message::<KeyboardInput>()
        .add_systems(
            PreUpdate,
            dispatch_focused_input::<KeyboardInput>.in_set(InputFocusSystems::Dispatch),
        )
        .add_plugins(ListBoxPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let mut model = WidgetryListModel::<String>::default();
    for index in 0..20 {
        model.push(index.to_string());
    }
    let source = app.world_mut().spawn(model).id();
    let root = view(&mut app, source, 32.0);
    let parent = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(parent).add_child(root).observe(
        |_: On<FocusedInput<KeyboardInput>>, mut count: ResMut<AncestorKeyboardCount>| {
            count.0 += 1;
        },
    );
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert((
        ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::new(100.0, 600.0),
            ..default()
        },
        ScrollPosition(Vec2::new(0.0, 150.0)),
    ));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    let keys = [
        (KeyCode::ArrowLeft, Key::ArrowLeft),
        (KeyCode::ArrowRight, Key::ArrowRight),
        (KeyCode::Escape, Key::Escape),
    ];
    for (code, logical_key) in keys.iter().cloned() {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
            150.0
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    }
    assert_eq!(
        app.world().resource::<AncestorKeyboardCount>().0,
        keys.len()
    );
    assert!(app.world().get::<ListBox>(root).is_none());
}

/// 预装共享 infrastructure 后添加 ListView 不会重复注册，也不改写现有 ThemeMode。
#[test]
fn shell_plugin_reuses_existing_infrastructure() {
    let mut app = scene_app();
    app.add_plugins((
        ThemePlugin,
        ForegroundColorPlugin,
        bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin,
    ))
    .insert_resource(bevy_widgetry_core::ThemeMode::Light)
    .add_plugins(WidgetryListViewPlugin);
    assert_eq!(
        *app.world().resource::<bevy_widgetry_core::ThemeMode>(),
        bevy_widgetry_core::ThemeMode::Light
    );
}
