//! 提供可运行的 Widget Gallery，用于浏览 Widgetry 的功能、体验输入交互和检查组合场景的界面表现。
//! 当前展示常驻窗口、Sidebar、theme 与 scale_factor 选择器和页面承载区域。
//!
//! Sidebar 提供 Button 到 Waveform 共 11 个页面的导航入口，点击后切换 GalleryPage State。
//! theme 选择器提供 Dark 与 Light 配色切换，同时更新窗口内容与 Sidebar 边框。
//! scale_factor 选择器提供 1.0、1.25、1.5、2.0 四档主窗口缩放，运行时调整内容显示大小。
//! 提供 BRP runtime 接入，便于外部工具检查运行时 state、执行交互和获取截图。
//!
//! 启动后从 Initializing 转入 Button State，主窗口、Camera、Sidebar 和 theme 选择器保持常驻。
//! 每次只挂载当前页面及演示内容，颜色区域按需展开，切走后销毁，重进重新折叠。
//! 页面 UI 的实例 state、ComboBox/ListView/Tree/Table 演示数据与 Waveform producer 在重进时重新创建。
//! Header 的 theme 与 scale_factor Model 保持常驻，scale_factor 默认选择 1.0。
//! Waveform 共享 assets 按正常 Handle lifecycle 使用。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod assets;
mod color_showcase;
mod file_dialog_benchmark;
mod gallery;
mod header;
mod page_lifecycle_benchmark;
mod pages;
mod waveform_benchmark;
mod waveform_data;

use crate::assets::GalleryAssetPlugin;
use crate::gallery::{GalleryPage, GalleryPlugin};
use bevy::log::LogPlugin;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowPosition, WindowResolution};
use bevy::winit::WinitSettings;
use bevy::{prelude::*, render::RenderPlugin, tasks::block_on};
use bevy_brp_runtime::BrpRuntimePlugin;
use bevy_widgetry::button::WidgetryButtonPlugin;
use bevy_widgetry::check_box::WidgetryCheckBoxPlugin;
use bevy_widgetry::combo_box::WidgetryComboBoxPlugin;
use bevy_widgetry::radio_group::WidgetryRadioGroupPlugin;
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::text_field::WidgetryTextFieldPlugin;
use bevy_widgetry::tooltip::WidgetryTooltipPlugin;
use bevy_widgetry::window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
    prepare_native_window, transparent_render_creation, widgetry_window,
};
use bevy_widgetry_app_logging::{AppLogging, file_layer, terminal_layer};
use std::path::PathBuf;

fn main() -> Result {
    let log_dir = match std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") {
        Some(output) => PathBuf::from(output).join("logs"),
        None => std::env::current_exe()
            .map_err(|error| {
                error!(error = %error, "无法确定 Gallery executable 路径");
                BevyError::error(error)
            })?
            .with_file_name("logs"),
    };
    let logging = AppLogging::new(log_dir)?;
    let mut app = App::new();
    let _log_guard = logging.install(&mut app);
    app.insert_resource(WinitSettings::desktop_app());
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                fmt_layer: terminal_layer,
                custom_layer: file_layer,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(gallery_window()),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: block_on(transparent_render_creation())?,
                ..default()
            }),
    )
    .add_plugins((
        BrpRuntimePlugin::default(),
        GalleryAssetPlugin,
        WidgetryWindowPlugin,
        WidgetryButtonPlugin,
        WidgetryCheckBoxPlugin,
        WidgetryComboBoxPlugin,
        WidgetryRadioGroupPlugin,
        WidgetryTextFieldPlugin,
        WidgetryTooltipPlugin,
        GalleryPlugin,
    ))
    .add_systems(Startup, setup)
    .add_plugins(bevy_widgetry::window::taskbar_icon!(
        "src/assets/icons/widget_gallery_taskbar.png"
    )?);
    header::install(&mut app)?;
    waveform_benchmark::install(&mut app)?;
    file_dialog_benchmark::install(&mut app)?;
    page_lifecycle_benchmark::install(&mut app)?;
    app.run();
    Ok(())
}

fn gallery_window() -> Window {
    prepare_native_window(Window {
        title: "Widget Gallery".into(),
        resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut next_page: ResMut<NextState<GalleryPage>>,
) -> Result {
    let target = primary_window.single()?;
    let camera = commands.spawn(Camera2d).id();
    let title = header::scene(&mut commands, target)?;
    commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{@{title}}, bsn_list!{@gallery::scene()})
    });
    // 默认 State 的 OnEnter 早于 Startup，先完成 deferred 外壳构造再进入 Button。
    next_page.set_if_different(GalleryPage::Button);
    Ok(())
}
