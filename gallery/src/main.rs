//! 提供可运行的 Widget Gallery，用于浏览 Widgetry 的功能、体验输入交互和检查组合场景的界面表现。
//! 当前展示常驻窗口、Sidebar、theme、语言与 scale_factor 选择器和页面承载区域。
//!
//! Sidebar 提供 Button 到 Cursor 共 12 个页面的导航入口，点击后切换 GalleryPage State。
//! theme 选择器提供 Dark 与 Light 配色切换，同时更新窗口内容与 Sidebar 边框。
//! 语言选择器提供 English 与简体中文实时切换，默认 English。
//! 切换语言原地更新展示文本，保留当前页面、控件 state 和用户输入。
//! scale_factor 选择器提供 1.0、1.25、1.5、2.0 四档主窗口缩放，运行时调整内容显示大小。
//! 主窗口首帧呈现后启用 Kamuri Cursor，Cursor 页面提供独立 System/Kamuri 切换与 44 种语义目标。
//! Pointer 动画需要真实 Windows Mouse 验收。
//! 提供 BRP runtime 接入，便于外部工具检查运行时 state、执行交互和获取截图。
//! 默认在主窗口播放一次千石冠 Splash，首屏就绪后从手心展开圆形揭幕，期间隔离主窗口输入。
//! 设置 GALLERY_SPLASH_DISABLE 或既有 Gallery benchmark 输出变量时跳过 Splash。
//! GALLERY_SPLASH_PREVIEW 支持 play、static、ripple、0–800ms 静帧与专用 baseline/benchmark 开发模式。
//!
//! 启动后从 Initializing 转入 Button State，主窗口、Camera、Sidebar 和 theme 选择器保持常驻。
//! 每次只挂载当前页面及演示内容，颜色区域按需展开，切走后销毁，重进重新折叠。
//! 页面 UI 的实例 state、ComboBox/ListView/Tree/Table 演示数据与 Waveform producer 在重进时重新创建。
//! Header 的 theme 与 scale_factor Model 保持常驻，主窗口 scale_factor 默认使用并选中 1.25。
//! 启动时从 executable 同级的 config/config.ron 恢复窗口宽高、scale_factor、theme 与语言。
//! 配置不存在时创建默认文件，读取、解析或校验失败时整体回退默认值并保留原文件。
//! 三个 Header 选择器实际切换成功后保存完整配置，窗口 resize 不自动保存，保存失败保留已应用的 UI 修改。
//! Waveform 共享 assets 按正常 Handle lifecycle 使用。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod assets;
mod color_showcase;
mod config;
mod file_dialog_benchmark;
mod gallery;
mod header;
mod i18n;
mod page_lifecycle_benchmark;
mod pages;
mod splash;
mod startup_cursor;
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
use bevy_widgetry::cursor::WidgetryCursorTheme;
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
    let executable = std::env::current_exe().map_err(|error| {
        error!(%error, "无法确定 Gallery executable 路径");
        BevyError::error(error)
    })?;
    let loaded = config::load(executable.with_file_name("config").join("config.ron"));
    let log_dir = match std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") {
        Some(output) => PathBuf::from(output).join("logs"),
        None => executable.with_file_name("logs"),
    };
    let logging = AppLogging::new(log_dir)?;
    let mut app = App::new();
    app.insert_resource(loaded.config.language);
    app.insert_resource(loaded.config.theme);
    i18n::install(&mut app);
    let _log_guard = logging.install(&mut app);
    app.insert_resource(WinitSettings::desktop_app());
    app.insert_resource(WidgetryCursorTheme::System);
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                fmt_layer: terminal_layer,
                custom_layer: file_layer,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(gallery_window(&loaded.config)),
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
    loaded.report();
    app.insert_resource(loaded.config)
        .insert_resource(loaded.file);
    header::install(&mut app)?;
    startup_cursor::install(&mut app)?;
    splash::install(&mut app)?;
    waveform_benchmark::install(&mut app)?;
    file_dialog_benchmark::install(&mut app)?;
    page_lifecycle_benchmark::install(&mut app)?;
    app.run();
    Ok(())
}

fn gallery_window(config: &config::AppConfig) -> Window {
    prepare_native_window(Window {
        title: "Widget Gallery".into(),
        resolution: WindowResolution::new(config.window_width, config.window_height)
            .with_scale_factor_override(config.scale_factor),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut next_page: ResMut<NextState<GalleryPage>>,
    splash_artboard: Option<Res<assets::SplashArtboard>>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    splash_materials: Option<ResMut<Assets<splash::SplashRevealMaterial>>>,
) -> Result {
    let target = primary_window.single()?;
    let camera = commands.spawn(Camera2d).id();
    let title = header::scene(&mut commands, target)?;
    let root = commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{@{title}}, bsn_list!{@gallery::scene()})
        template(|_| Ok(i18n::LocalizedWindowTitle(i18n::LocalizedText::new(i18n::TextKey::WidgetGallery))))
    }).id();
    commands.insert_resource(startup_cursor::StartupCursor::new(target, camera, root));
    if let (Some(artboard), Some(mut materials)) = (splash_artboard, splash_materials) {
        splash::setup(
            &mut commands,
            &asset_server,
            &mut images,
            &mut materials,
            &artboard,
            target,
            camera,
        );
    }
    // 默认 State 的 OnEnter 早于 Startup，先完成 deferred 外壳构造再进入 Button。
    next_page.set_if_different(GalleryPage::Button);
    Ok(())
}
