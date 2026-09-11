mod bar;
mod close;
mod controls;
mod drag;
mod maximize;
mod minimize;
mod resize;

pub(crate) use bar::TitleBar;
use bevy::{asset::io::embedded::EmbeddedAssetRegistry, prelude::*};
use bevy_widgetry_core::icon::IconPlugin;
pub(crate) use resize::WindowResizeArea;
use std::path::{Path, PathBuf};

/// 注册内嵌图标及窗口控制观察者；需在 Bevy 资产插件之后添加，运行时需窗口和输入资源。
pub struct TitleBarPlugin;

impl Plugin for TitleBarPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<IconPlugin>() {
            app.add_plugins(IconPlugin);
        }

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/minimize.svg"
                )),
                Path::new("bevy_widgetry_window/icons/minimize.svg"),
                include_bytes!("../assets/icons/minimize.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/maximize.svg"
                )),
                Path::new("bevy_widgetry_window/icons/maximize.svg"),
                include_bytes!("../assets/icons/maximize.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/restore.svg"
                )),
                Path::new("bevy_widgetry_window/icons/restore.svg"),
                include_bytes!("../assets/icons/restore.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/close.svg"
                )),
                Path::new("bevy_widgetry_window/icons/close.svg"),
                include_bytes!("../assets/icons/close.svg"),
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
                    maximize::sync_maximize_state,
                    controls::update_window_control_style_changed,
                    controls::update_window_control_style_released,
                    close::update_close_button_style_changed,
                    close::update_close_button_style_released,
                    resize::finish_window_resize,
                ),
            );
    }
}
