pub(crate) mod bar;
mod close;
mod controls;
mod drag;
mod maximize;
mod minimize;
pub(crate) mod resize;

use bevy::prelude::*;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_core::{ThemePlugin, WidgetryFontPlugin, icon::WidgetryIconPlugin};
use bevy_widgetry_log::widgetry_info;

/// 注册 window Scene 的校验、lifecycle、theme 与 native 交互；使用内建字体时须在 Bevy asset 与文本 plugin 后添加（通常为 DefaultPlugins）。
/// 外部绑定保留调用方资源，owned Scene 则随 root 销毁回收 native window 和 camera。
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
                    .before(bevy::ui::UiSystems::Prepare),
            );
        app.add_observer(minimize::on_minimize)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WidgetryWindowControlsConfig, prepare_native_window, widgetry_window};
    use bevy::camera::CameraUpdateSystems;
    use bevy::ecs::schedule::NodeId;
    use bevy::ui::InteractionDisabled;
    use bevy::ui_widgets::Activate;
    use bevy::window::{EnabledButtons, WindowCloseRequested};
    use bevy_widgetry_asset::BuiltinIcon;
    use bevy_widgetry_core::icon::WidgetryIcon;
    use bevy_widgetry_test_utils::press;

    /// 提供 window 私有交互测试所需的最小 resource，不创建真实桌面 window。
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

    /// 默认保留所有能力，显式禁用 close button 与 resize 后不应生成对应 hit entity。
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
            widgetry_window(target, camera, WidgetryWindowControlsConfig { close_visible: false, resizable: false, ..default() }, bsn_list![], bsn_list![])
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

    /// 直接检查初始化到 camera update system set 的依赖边，避免运行顺序偶然正确时漏掉 schedule regression。
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

    /// 隐藏的 button 不生成 entity，close button 保留；native enabled_buttons 改变后必须同步 disabled state。
    #[test]
    fn visibility_and_native_button_enablement_are_independent() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig { minimize_visible: false, maximize_visible: false, ..default() }, bsn_list![], bsn_list![])
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

    /// 禁止 native resize 时八个 hit area 都应穿透 picking，运行时开启后恢复。
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
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
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

    /// 默认三个 button 及仅 maximized 时使用的 restore icon 都必须经真实 AssetServer 生成 image，防止 embedded 路径失配。
    #[test]
    fn embedded_control_icons_materialize() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
        });
        // 无桌面 window 时不会进入 winit maximized 分支，显式请求该分支使用的 restore asset。
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

    /// 外部修改 native button 配置后，全部 button 同步 disabled state；直接 Activate 也不能绕过 native 配置。
    #[test]
    fn all_system_buttons_follow_runtime_enabled_buttons() {
        let mut app = app();
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().commands().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
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
        // 在下一帧 state 同步之前也必须拒绝已禁用的操作。
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

    /// 即使显式发送 press event 也不能绕过 resizable，恢复后将正确 window 及方向传给 native 请求。
    #[test]
    fn resize_press_respects_native_resizable_and_window_binding() {
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
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&crate::window_root::WindowRoot>()
                .single(app.world())
                .unwrap()
                .target_window,
            target
        );
        let handle = app
            .world_mut()
            .query_filtered::<Entity, With<resize::WindowResizeHandle>>()
            .iter(app.world())
            .next()
            .unwrap();
        press(&mut app, handle);
        assert!(
            app.world_mut()
                .get_mut::<Window>(target)
                .unwrap()
                .internal
                .take_resize_request()
                .is_none()
        );
        assert!(app.world().get::<resize::Resizing>(handle).is_none());
        app.world_mut().get_mut::<Window>(target).unwrap().resizable = true;
        press(&mut app, handle);
        assert!(
            app.world_mut()
                .get_mut::<Window>(target)
                .unwrap()
                .internal
                .take_resize_request()
                .is_some()
        );
        assert!(app.world().get::<resize::Resizing>(handle).is_some());
    }
}
