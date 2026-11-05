#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::i18n::HeaderKey;

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod assets;
mod canvas_experiment;
mod close;
mod color_showcase;
mod config;
mod diagnostics;
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
use bevy_widgetry::app_logging::{AppLogging, log_plugin};
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

fn main() -> std::process::ExitCode {
    let mut logging_ready = false;
    let mut guard = None;
    let result = run(&mut logging_ready, &mut guard);
    let mut exit = match result {
        Ok(exit) => exit,
        Err(error) => {
            diagnostics::startup(error, logging_ready);
            AppExit::error()
        }
    };
    if let Some(guard) = guard
        && let Err(failure) = guard.finish()
    {
        diagnostics::logging_shutdown(&failure);
        exit = AppExit::error();
    }
    match exit {
        AppExit::Success => std::process::ExitCode::SUCCESS,
        AppExit::Error(code) => std::process::ExitCode::from(code.get()),
    }
}

fn run(
    logging_ready: &mut bool,
    guard: &mut Option<bevy_widgetry::app_logging::LoggingGuard>,
) -> Result<AppExit> {
    let executable = std::env::current_exe().map_err(diagnostics::error)?;
    let log_dir = match std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") {
        Some(output) => PathBuf::from(output).join("logs"),
        None => executable.with_file_name("logs"),
    };
    let logging = AppLogging::new(log_dir).map_err(|failure| failure.into_bevy_error())?;
    let mut app = App::new();
    app.set_error_handler(diagnostics::handler);
    *guard = Some(logging.install(&mut app));
    app.add_plugins(log_plugin());
    *logging_ready = true;
    debug!("Gallery 启动配置开始");
    if let Some(mode) = std::env::var_os("GALLERY_DIAGNOSTIC_PROBE") {
        return diagnostics::run_probe(&mut app, &mode);
    }
    let loaded = config::load(executable.with_file_name("config").join("config.ron"));
    app.insert_resource(loaded.config.language);
    app.insert_resource(loaded.config.theme);
    i18n::install(&mut app);
    app.insert_resource(WinitSettings::desktop_app());
    app.insert_resource(WidgetryCursorTheme::System);
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .set(WindowPlugin {
                primary_window: Some(gallery_window(&loaded.config)),
                close_when_requested: false,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: block_on(transparent_render_creation())
                    .map_err(window_startup_failure)?,
                ..default()
            }),
    );
    if loaded.config.brp_enabled {
        app.add_plugins(BrpRuntimePlugin::with_port(loaded.config.brp_port));
    }
    app.add_plugins((
        (
            GalleryAssetPlugin,
            bevy_widgetry::job::JobManagerPlugin,
            bevy_widgetry::power_inhibit::WidgetryPowerInhibitPlugin,
        ),
        WidgetryWindowPlugin,
        WidgetryButtonPlugin,
        bevy_widgetry::hold_button::WidgetryHoldButtonPlugin,
        bevy_widgetry::toggle_button::WidgetryToggleButtonPlugin,
        WidgetryCheckBoxPlugin,
        WidgetryComboBoxPlugin,
        WidgetryRadioGroupPlugin,
        bevy_widgetry::segmented::WidgetrySegmentedPlugin,
        (
            bevy_widgetry::tabs::WidgetryTabsPlugin,
            bevy_widgetry::collapsible::WidgetryCollapsiblePlugin,
        ),
        bevy_widgetry::slider::WidgetrySliderPlugin,
        WidgetryTextFieldPlugin,
        (
            bevy_widgetry::progress_bar::WidgetryProgressBarPlugin,
            bevy_widgetry::splitter::WidgetrySplitterPlugin,
            bevy_widgetry::number_input::WidgetryNumberInputPlugin,
        ),
        WidgetryTooltipPlugin,
        GalleryPlugin,
    ))
    .add_systems(Startup, setup)
    .add_plugins(
        bevy_widgetry::window::taskbar_icon!("src/assets/icons/widget_gallery_taskbar.png")
            .map_err(window_startup_failure)?,
    );
    app.insert_resource(loaded.config)
        .insert_resource(loaded.file);
    header::install(&mut app)?;
    startup_cursor::install(&mut app)?;
    splash::install(&mut app)?;
    waveform_benchmark::install(&mut app)?;
    file_dialog_benchmark::install(&mut app)?;
    page_lifecycle_benchmark::install(&mut app)?;
    close::install(&mut app)?;
    canvas_experiment::install(&mut app);
    debug!("Gallery 配置装配完成，准备进入主循环");
    let exit = app.run();
    debug!(?exit, "Gallery 主循环已返回");
    Ok(exit)
}

fn window_startup_failure(failure: bevy_widgetry::window::WindowFailure) -> BevyError {
    let failure = failure.with_context(bevy_widgetry::diagnostics::ErrorFrame::new(
        "gallery.startup",
    ));
    failure.into_bevy_error()
}

fn gallery_window(config: &config::AppConfig) -> Window {
    prepare_native_window(Window {
        title: "Widget Gallery".into(),
        resolution: WindowResolution::new(1920, 1080)
            .with_scale_factor_override(config.scale_factor),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    mut primary_window: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut next_page: ResMut<NextState<GalleryPage>>,
    splash_enabled: Option<Res<splash::SplashEnabled>>,
    asset_server: Res<AssetServer>,
) -> Result {
    let (target, mut window) = primary_window.single_mut().map_err(|error| {
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("Gallery 初始化需要唯一主窗口"),
        )
    })?;
    // winit sets size before positioning the window, so initialization across displays with different DPI can scale its physical size.
    // Restore the fixed startup size in Startup after native creation and positioning so the first frame does not use the scaled size.
    window.resolution.set_physical_resolution(1920, 1080);
    let camera = commands.spawn(Camera2d).id();
    let title = header::scene(&mut commands, target)?;
    let root = commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{@{title}}, bsn_list!{@gallery::scene()})
        template(|_| Ok(i18n::LocalizedWindowTitle(i18n::LocalizedText::new(i18n::TextKey::GalleryHeader(HeaderKey::WidgetGallery)))))
    }).id();
    commands.insert_resource(startup_cursor::StartupCursor::new(target, camera, root));
    if splash_enabled.is_some() {
        splash::setup(&mut commands, &asset_server, target, camera);
    }
    // The default State's OnEnter precedes Startup, so complete deferred shell construction before entering Button.
    next_page.set_if_different(GalleryPage::Button);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod startup_tests {
    use super::*;
    use bevy_widgetry::diagnostics::bevy::WidgetryEcsError;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

    #[test]
    fn window_startup_failure_reports_original_origin_without_an_ecs_handler() {
        let logs = LogCapture::with_targets(["bevy_widgetry", "widget_gallery"]);
        let (error, origin) = logs.run(|| {
            let failure = bevy_widgetry::window::taskbar_icon!("src/main.rs")
                .err()
                .unwrap();
            let origin = failure.origin().clone();
            (window_startup_failure(failure), origin)
        });
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(&origin, diagnostic.origin()));
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "gallery.startup")
        );
        logs.run(|| diagnostics::startup(error, true));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        let reports = logs.diagnostic_records(origin.id(), DiagnosticPhase::Unhandled);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].fields.contains_key("backtrace"));
        assert!(reports[0].fields.contains_key("origin"));
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    }
}
