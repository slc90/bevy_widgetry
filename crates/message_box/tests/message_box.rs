use bevy::app::Propagate;
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ButtonPlugin},
    window::WindowClosed,
};
use bevy_widgetry_button::{StyledButton, StyledButtonPlugin};
use bevy_widgetry_core::{ForegroundColor, ThemeMode};
use bevy_widgetry_message_box::{
    MessageBox, MessageBoxButtons, MessageBoxPlugin, MessageBoxResult, MessageBoxResultEvent,
    message_box,
};
use bevy_widgetry_test_utils::switch_theme;
use bevy_widgetry_test_utils::{press, primary_click, scene_app};
use bevy_widgetry_window::{WindowControlsConfig, WindowPlugin, owned_window};

/// 从公共 API 观察结果，不依赖内部 action 或所有权 marker。
#[derive(Resource, Default)]
struct Results(Vec<(Entity, MessageBoxResult)>);

/// 真实 ButtonPlugin 会截断指针冒泡，点击仍须通过 Activate 桥接得到正确结果。
#[test]
fn real_button_clicks_return_all_results_and_release_last_blocker() {
    for (label, expected) in [
        ("OK", MessageBoxResult::Ok),
        ("Yes", MessageBoxResult::Yes),
        ("No", MessageBoxResult::No),
        ("Cancel", MessageBoxResult::Cancel),
    ] {
        let mut app = scene_app();
        app.add_plugins((ButtonPlugin, MessageBoxPlugin))
            .init_resource::<Results>();
        app.add_observer(
            |event: On<MessageBoxResultEvent>,
             roots: Query<(), With<MessageBox>>,
             mut results: ResMut<Results>| {
                assert!(roots.contains(event.entity));
                results.0.push((event.entity, event.result));
            },
        );
        let parent_root = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
        }).id();
        app.update();
        let parent = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let baseline_children = app.world().get::<Children>(parent_root).unwrap().len();
        let buttons = if label == "OK" {
            MessageBoxButtons::Ok
        } else {
            MessageBoxButtons::YesNoCancel
        };
        let roots: Vec<_> = (0..2)
            .map(|_| {
                app.world_mut()
                    .commands()
                    .spawn_scene(bsn! {
                        message_box(parent, "Choose", buttons, bsn_list![(Text("Body"))])
                    })
                    .id()
            })
            .collect();
        app.update();
        assert_eq!(
            app.world().get::<Children>(parent_root).unwrap().len(),
            baseline_children + 1
        );
        for (index, root) in roots.iter().copied().enumerate() {
            let button = app
                .world_mut()
                .query::<(Entity, &StyledButton)>()
                .iter(app.world())
                .map(|(entity, _)| entity)
                .find(|&entity| {
                    let mut current = entity;
                    while let Some(parent) = app.world().get::<ChildOf>(current) {
                        current = parent.parent();
                    }
                    current == root
                        && app.world().get::<Children>(entity).is_some_and(|children| {
                            children.iter().any(|child| {
                                app.world()
                                    .get::<Text>(child)
                                    .is_some_and(|text| text.0 == label)
                            })
                        })
                })
                .unwrap();
            press(&mut app, button);
            app.world_mut().trigger(primary_click(button));
            app.world_mut().flush();
            assert!(app.world().get_entity(root).is_err());
            assert_eq!(app.world().resource::<Results>().0[index], (root, expected));
            assert_eq!(
                app.world().get::<Children>(parent_root).unwrap().len(),
                baseline_children + usize::from(index == 0)
            );
        }
        assert_eq!(
            app.world_mut().query::<&Window>().iter(app.world()).count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Camera>().iter(app.world()).count(),
            1
        );
    }
}

/// 原生窗口关闭仅销毁 UI 和相机，不触发 Cancel 或任何结果 observer。
#[test]
fn native_close_has_no_result() {
    let mut app = scene_app();
    app.add_plugins(MessageBoxPlugin).init_resource::<Results>();
    app.add_observer(
        |event: On<MessageBoxResultEvent>, mut results: ResMut<Results>| {
            results.0.push((event.entity, event.result))
        },
    );
    app.world_mut().commands().spawn_scene(bsn! { owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![]) });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(
            bsn! { message_box(parent, "Close", MessageBoxButtons::YesNoCancel, bsn_list![]) },
        )
        .id();
    app.update();
    let native = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .iter(app.world())
        .find(|&entity| entity != parent)
        .unwrap();
    app.world_mut().entity_mut(native).despawn();
    app.world_mut()
        .write_message(WindowClosed { window: native });
    app.update();
    assert!(app.world().get_entity(root).is_err());
    assert!(app.world().resource::<Results>().0.is_empty());
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        1
    );
}

/// 依赖首次注册或提前注册均可使用，不重复添加内部插件。
#[test]
fn plugin_ensures_dependencies_once() {
    for pre_registered in [false, true] {
        let mut app = scene_app();
        if pre_registered {
            app.add_plugins((WindowPlugin, StyledButtonPlugin));
        }
        app.add_plugins(MessageBoxPlugin);
        assert_eq!(app.get_added_plugins::<WindowPlugin>().len(), 1);
        assert_eq!(app.get_added_plugins::<StyledButtonPlugin>().len(), 1);
    }
}

/// 禁用结果按钮不能通过直接 Activate 发布决议。
#[test]
fn disabled_action_does_not_resolve() {
    let mut app = scene_app();
    app.add_plugins(MessageBoxPlugin);
    app.world_mut().commands().spawn_scene(bsn! { owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![]) });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! { message_box(parent, "Disabled", MessageBoxButtons::Ok, bsn_list![]) })
        .id();
    app.update();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<StyledButton>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(button)
        .insert(bevy::ui::InteractionDisabled);
    app.world_mut().trigger(Activate { entity: button });
    app.world_mut().flush();
    assert!(app.world().get_entity(root).is_ok());
}

/// 对话框标题与任意正文的继承前景色初始化及切换均跟随共享主题。
#[test]
fn message_box_text_tracks_theme() {
    let mut app = scene_app();
    app.add_plugins(MessageBoxPlugin);
    app.world_mut().commands().spawn_scene(bsn! { owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![]) });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(
            bsn! { message_box(parent, "Theme", MessageBoxButtons::Ok, bsn_list![(Text("Body"))]) },
        )
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(root)
            .unwrap()
            .0
            .0,
        app.world().resource::<ThemeMode>().colors().foreground
    );
    switch_theme(&mut app, ThemeMode::Light);
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(root)
            .unwrap()
            .0
            .0,
        ThemeMode::Light.colors().foreground
    );
}
