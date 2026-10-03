pub(crate) mod bar;
mod close;
mod controls;
mod drag;
mod maximize;
mod minimize;
pub(crate) mod resize;

use bevy::prelude::*;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_core::{ThemePlugin, WidgetryFontPlugin, icon::WidgetryIconPlugin};
use bevy_widgetry_log::widgetry_info;

pub struct WidgetryWindowPlugin;

impl Plugin for WidgetryWindowPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        if !app.is_plugin_added::<WidgetryFontPlugin>() {
            app.add_plugins(WidgetryFontPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.init_resource::<crate::window_root::PendingWindows>()
            .add_observer(crate::window_root::queue_window_initialization)
            .add_observer(crate::window_root::refresh_window_theme)
            .add_observer(crate::window_root::cleanup_owned_window)
            .add_observer(crate::modal::modal_added)
            .add_observer(crate::modal::modal_removed)
            .add_observer(crate::modal::root_removed)
            .add_observer(crate::modal::parent_removed)
            .add_message::<bevy::window::WindowClosed>()
            .add_systems(
                PostUpdate,
                (
                    crate::window_root::initialize_windows
                        .before(bevy::camera::CameraUpdateSystems),
                    maximize::sync_maximize_state,
                    controls::sync_enabled_buttons,
                    resize::sync_resize_handles,
                    crate::window_root::cleanup_closed_windows,
                )
                    .chain()
                    .in_set(WidgetryUiSystems::Build),
            );
        app.add_observer(minimize::on_minimize)
            .add_systems(
                PostUpdate,
                crate::background::sync_cover_backgrounds
                    .in_set(bevy::ui::UiSystems::PostLayout)
                    .after(bevy::ui::UiSystems::Layout),
            )
            .add_observer(maximize::on_maximize_restore)
            .add_observer(close::on_close)
            .add_observer(drag::on_title_bar_press)
            .add_observer(resize::on_window_resize_press)
            .add_observer(resize::on_window_resize_over)
            .add_observer(resize::on_window_resize_out)
            .add_systems(
                Update,
                (
                    controls::update_window_control_style_changed,
                    controls::update_window_control_style_released,
                    close::update_close_button_style_changed,
                    close::update_close_button_style_released,
                    resize::finish_window_resize,
                ),
            );
        widgetry_info!("WidgetryWindowPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, prepare_native_window,
        widgetry_window,
    };
    use bevy::camera::CameraUpdateSystems;
    use bevy::ecs::schedule::NodeId;
    use bevy::ui::InteractionDisabled;
    use bevy::ui_widgets::Activate;
    use bevy::window::{EnabledButtons, WindowCloseRequested};
    use bevy_widgetry_asset::BuiltinIcon;
    use bevy_widgetry_core::icon::WidgetryIcon;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>()
            .add_plugins(WidgetryWindowPlugin);
        app.init_asset::<bevy::scene::ScenePatch>();
        app.init_asset::<Image>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_message::<WindowCloseRequested>();
        app
    }

    #[test]
    fn controls_can_omit_close_and_resize() {
        let controls = WidgetryWindowControlsConfig::default();
        assert!(
            controls.minimize_visible
                && controls.maximize_visible
                && controls.close_visible
                && controls.resizable
        );
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig { close_visible: false, resizable: false, ..default() }, WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&close::CloseButton>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&resize::WindowResizeHandle>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn all_system_buttons_have_rounded_background_corners() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        app.update();
        let mut buttons = app.world_mut().query_filtered::<&Node, Or<(
            With<minimize::MinimizeButton>,
            With<maximize::MaximizeButton>,
            With<close::CloseButton>,
        )>>();
        assert_eq!(buttons.iter(app.world()).count(), 3);
        for node in buttons.iter(app.world()) {
            assert_eq!(node.border_radius, BorderRadius::all(px(4)));
        }
    }

    #[test]
    fn initialization_precedes_camera_updates() {
        let mut app = app();
        app.world_mut()
            .schedule_scope(PostUpdate, |world, schedule| {
                schedule.graph_mut().initialize(world);
                let graph = schedule.graph();
                let initialize = graph
                    .systems
                    .iter()
                    .find_map(|(key, system, _)| {
                        (system.system_type()
                            == IntoSystem::into_system(crate::window_root::initialize_windows)
                                .system_type())
                        .then_some(NodeId::System(key))
                    })
                    .unwrap();
                let camera = graph
                    .system_sets
                    .iter()
                    .find_map(|(key, set, _)| {
                        (set == &CameraUpdateSystems as &dyn SystemSet).then_some(NodeId::Set(key))
                    })
                    .expect("必须显式声明 CameraUpdateSystems 调度约束");
                assert!(graph.dependency().graph().contains_edge(initialize, camera));
            });
    }

    #[test]
    fn visibility_and_native_button_enablement_are_independent() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig { minimize_visible: false, maximize_visible: false, ..default() }, WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&minimize::MinimizeButton>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&maximize::MaximizeButton>()
                .iter(app.world())
                .count(),
            0
        );
        let close = app
            .world_mut()
            .query_filtered::<Entity, With<close::CloseButton>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<Window>(target)
            .unwrap()
            .enabled_buttons = EnabledButtons {
            minimize: false,
            maximize: false,
            close: false,
        };
        app.update();
        assert!(app.world().get::<InteractionDisabled>(close).is_some());
        app.world_mut().trigger(Activate { entity: close });
        assert!(
            app.world()
                .resource::<Messages<WindowCloseRequested>>()
                .is_empty()
        );
        app.world_mut()
            .get_mut::<Window>(target)
            .unwrap()
            .enabled_buttons
            .close = true;
        app.update();
        assert!(app.world().get::<InteractionDisabled>(close).is_none());
        app.world_mut().trigger(Activate { entity: close });
        assert_eq!(
            app.world()
                .resource::<Messages<WindowCloseRequested>>()
                .len(),
            1
        );
    }

    #[test]
    fn non_resizable_window_disables_all_resize_handles() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window {
                resizable: false,
                ..default()
            }))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        app.update();
        let mut handles = app
            .world_mut()
            .query_filtered::<&Pickable, With<resize::WindowResizeHandle>>();
        assert_eq!(handles.iter(app.world()).count(), 8);
        assert!(
            handles
                .iter(app.world())
                .all(|pickable| !pickable.is_hoverable)
        );
        app.world_mut().get_mut::<Window>(target).unwrap().resizable = true;
        app.update();
        assert!(
            handles
                .iter(app.world())
                .all(|pickable| pickable.is_hoverable)
        );
    }

    #[test]
    fn embedded_control_icons_materialize() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        // headless fixture 没有桌面 window，不会进入 winit maximized 分支。
        // 显式加载该分支使用的 restore asset，避免遗漏其 embedded 路径验证。
        app.world_mut().commands().spawn_scene(bsn! {
            @WidgetryIcon {
                @path: {BuiltinIcon::WindowRestore.path()},
                @max_size: { Some(UVec2::new(16, 16)) },
                @color: { Some(Color::WHITE) },
            }
        });
        app.update();
        let icons: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryIcon>>()
            .iter(app.world())
            .collect();
        assert_eq!(icons.len(), 4);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            app.update();
            let all_materialized = icons.iter().all(|&icon| {
                app.world().get::<Children>(icon).is_some_and(|children| {
                    children.iter().any(|child| {
                        app.world().get::<ImageNode>(child).is_some_and(|node| {
                            app.world()
                                .resource::<Assets<Image>>()
                                .contains(&node.image)
                        })
                    })
                })
            });
            if all_materialized {
                let mut images = app
                    .world_mut()
                    .query_filtered::<Option<&Pickable>, With<ImageNode>>();
                assert!(
                    images
                        .iter(app.world())
                        .all(|pickable| pickable.is_some_and(|pickable| !pickable.is_hoverable)),
                    "图标的纯视觉子节点不能阻挡标题栏底层拖动区"
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "内嵌系统图标没有全部加载"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn all_system_buttons_follow_runtime_enabled_buttons() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        });
        app.update();
        let minimize = app
            .world_mut()
            .query_filtered::<Entity, With<minimize::MinimizeButton>>()
            .single(app.world())
            .unwrap();
        let maximize = app
            .world_mut()
            .query_filtered::<Entity, With<maximize::MaximizeButton>>()
            .single(app.world())
            .unwrap();
        let close = app
            .world_mut()
            .query_filtered::<Entity, With<close::CloseButton>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<Window>(target)
            .unwrap()
            .enabled_buttons = EnabledButtons {
            minimize: false,
            maximize: false,
            close: false,
        };
        for entity in [minimize, maximize, close] {
            app.world_mut().trigger(Activate { entity });
        }
        assert!(
            app.world_mut()
                .get_mut::<Window>(target)
                .unwrap()
                .internal
                .take_minimize_request()
                .is_none()
        );
        assert!(
            app.world_mut()
                .get_mut::<Window>(target)
                .unwrap()
                .internal
                .take_maximize_request()
                .is_none()
        );
        assert!(
            app.world()
                .resource::<Messages<WindowCloseRequested>>()
                .is_empty()
        );
        app.update();
        for entity in [minimize, maximize, close] {
            assert!(app.world().get::<InteractionDisabled>(entity).is_some());
        }
        app.world_mut()
            .get_mut::<Window>(target)
            .unwrap()
            .enabled_buttons = EnabledButtons::default();
        app.update();
        for entity in [minimize, maximize, close] {
            assert!(app.world().get::<InteractionDisabled>(entity).is_none());
        }
        app.world_mut().trigger(Activate { entity: minimize });
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(target)
                .unwrap()
                .internal
                .take_minimize_request(),
            Some(true)
        );
    }
}
