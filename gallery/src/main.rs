#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::i18n::HeaderKey;

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
use bevy_widgetry::app_logging::{AppLogging, file_layer, terminal_layer};
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
    );
    if loaded.config.brp_enabled {
        app.add_plugins(BrpRuntimePlugin::with_port(loaded.config.brp_port));
    }
    app.add_plugins((
        GalleryAssetPlugin,
        WidgetryWindowPlugin,
        WidgetryButtonPlugin,
        bevy_widgetry::hold_button::WidgetryHoldButtonPlugin,
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
    mut primary_window: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    config: Res<config::AppConfig>,
    mut next_page: ResMut<NextState<GalleryPage>>,
    splash_artboard: Option<Res<assets::SplashArtboard>>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    splash_materials: Option<ResMut<Assets<splash::SplashRevealMaterial>>>,
) -> Result {
    let (target, mut window) = primary_window.single_mut().map_err(|error| {
        error!(%error, "Gallery 初始化需要唯一主窗口");
        BevyError::error(error)
    })?;
    // winit sets size before positioning the window, so initialization across displays with different DPI can scale its physical size.
    // Restore the configured size in Startup after native creation and positioning so the first frame does not use the scaled size.
    window
        .resolution
        .set_physical_resolution(config.window_width, config.window_height);
    let camera = commands.spawn(Camera2d).id();
    let title = header::scene(&mut commands, target)?;
    let root = commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{@{title}}, bsn_list!{@gallery::scene()})
        template(|_| Ok(i18n::LocalizedWindowTitle(i18n::LocalizedText::new(i18n::TextKey::GalleryHeader(HeaderKey::WidgetGallery)))))
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
    // The default State's OnEnter precedes Startup, so complete deferred shell construction before entering Button.
    next_page.set_if_different(GalleryPage::Button);
    Ok(())
}
