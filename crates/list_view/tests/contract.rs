#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::log::tracing::Level;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, ListBox, ListBoxPlugin, ScrollArea, Scrollbar};
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_core::ui::WidgetryUiPlugin;
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewPlugin,
    WidgetryListViewRenderer,
};
use bevy_widgetry_scroll_area::{
    WidgetryScrollArea, WidgetryScrollAreaContent, WidgetryScrollAreaViewport,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, queue_key, scene_app,
};
use bevy_widgetry_theme::WidgetryThemePlugin;

#[derive(Resource, Default)]
struct AncestorKeyboardCount(usize);

fn app() -> App {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap()
        .register_widgetry_list_view::<u32>()
        .unwrap();
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}

fn view(app: &mut App, source: Entity, height: f32) -> Entity {
    try_view(app, source, height).unwrap()
}

fn try_view(
    app: &mut App,
    source: Entity,
    height: f32,
) -> Result<Entity, bevy::scene::SpawnSceneError> {
    app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @item_height: height,
            @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
        }
    }).map(|entity| entity.id())
}

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

#[test]
fn source_failure_logs_edges_and_keeps_propagating() {
    let mut app = app();
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    view(&mut app, source, 32.0);
    app.update();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    app.world_mut()
        .entity_mut(source)
        .remove::<WidgetryListModel<String>>();
    for _ in 0..2 {
        errors.run(|| logs.run(|| app.update()));
        assert!(!errors.take().is_empty());
    }
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::ERROR)
            .count(),
        1
    );
    app.world_mut()
        .entity_mut(source)
        .insert(WidgetryListModel::<String>::default());
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record
                .fields
                .get("message")
                .is_some_and(|message| message.contains("恢复")))
            .count(),
        1
    );
    app.world_mut()
        .entity_mut(source)
        .remove::<WidgetryListModel<String>>();
    errors.run(|| logs.run(|| app.update()));
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::ERROR)
            .count(),
        2
    );
}

#[test]
fn missing_required_props_are_rejected() {
    let mut app = app();
    assert_configuration_error(|| {
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryListView::<String> })
    });
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    assert_configuration_error(|| {
        app.world_mut().spawn_scene(bsn! {
            @WidgetryListView::<String> { @source: source }
        })
    });
    assert_configuration_error(|| try_view(&mut app, Entity::PLACEHOLDER, 32.0));
}

#[test]
fn invalid_scene_command_reaches_host_once() {
    let mut app = app();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene_with_error_handler(bsn! { @WidgetryListView::<String> })
        .id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(app.world().get_entity(root).is_err());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == Level::ERROR)
            .count(),
        1
    );
}

#[test]
fn invalid_heights_are_rejected() {
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut app = app();
        let source = app
            .world_mut()
            .spawn(WidgetryListModel::<String>::default())
            .id();
        assert_configuration_error(|| try_view(&mut app, source, height));
    }
}

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
        assert_runtime_error(&mut app);
    }
}

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
        assert_runtime_error(&mut app);
    }
}

#[test]
fn renderer_produces_owned_direct_children() {
    let renderer = WidgetryListViewRenderer::new(
        |value: &String| bsn_list! {Text({value.clone()}) bevy_widgetry_core::text::WidgetryText-- Text("suffix") bevy_widgetry_core::text::WidgetryText},
    );
    let mut value = String::from("before");
    let content = renderer.clone().render(&value).unwrap();
    value.clear();
    let mut app = app();
    let row = app
        .world_mut()
        .spawn_scene(bsn! { Node Children [{content}] })
        .unwrap()
        .id();
    let children = app.world().get::<Children>(row).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(app.world().get::<Text>(children[0]).unwrap().0, "before");
    assert_eq!(app.world().get::<Text>(children[1]).unwrap().0, "suffix");
}

#[test]
fn unconfigured_renderer_is_rejected() {
    assert_configuration_error(|| {
        WidgetryListViewRenderer::<String>::default().render(&String::new())
    });
}

#[test]
fn registration_requires_common_plugin() {
    let mut app = App::new();
    assert_configuration_error(|| app.register_widgetry_list_view::<String>());
}

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
            @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
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
    assert!(app.is_plugin_added::<WidgetryThemePlugin>());
    assert!(app.is_plugin_added::<WidgetryUiPlugin>());
}

#[test]
fn shell_leaves_unsupported_keyboard_input_to_ancestors() {
    let mut app = app();
    app.init_resource::<bevy::ui::UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<AncestorKeyboardCount>()
        .add_plugins(ListBoxPlugin);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let mut model = WidgetryListModel::<String>::default();
    for index in 0..20 {
        model.push(index.to_string()).unwrap();
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
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: code,
                logical_key,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
        );
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

#[test]
fn shell_plugin_reuses_existing_infrastructure() {
    let mut app = scene_app();
    app.add_plugins((bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin,))
        .insert_resource(bevy_widgetry_theme::WidgetryThemeMode::Light)
        .add_plugins(WidgetryListViewPlugin);
    assert_eq!(
        *app.world()
            .resource::<bevy_widgetry_theme::WidgetryThemeMode>(),
        bevy_widgetry_theme::WidgetryThemeMode::Light
    );
}
