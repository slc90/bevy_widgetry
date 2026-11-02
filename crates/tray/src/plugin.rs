use crate::bridge::{self, Bridge, Registration, WidgetryTrayClick};
use crate::{TrayError, TrayFailure};
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::winit::EventLoopProxyWrapper;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_internal_log::widgetry_info;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct WidgetryTrayPlugin {
    config: TrayConfig,
}

#[derive(Resource, Clone)]
struct TrayConfig {
    rgba: image::RgbaImage,
    tooltip: String,
}

#[derive(Default)]
struct TrayOwner(Option<TrayInstance>);

struct TrayInstance {
    _registration: Registration,
    _icon: TrayIcon,
    _bridge: std::sync::Arc<Bridge>,
}

impl WidgetryTrayPlugin {
    pub fn new(ico: &'static [u8], tooltip: impl Into<String>) -> Result<Self, TrayFailure> {
        let rgba = image::load_from_memory_with_format(ico, image::ImageFormat::Ico)
            .map_err(|source| {
                TrayFailure::new(
                    TrayError::Decode {
                        source: Box::new(source),
                    },
                    ErrorFrame::new("decode ICO").field("bytes", ico.len()),
                )
            })?
            .into_rgba8();
        Ok(Self {
            config: TrayConfig {
                rgba,
                tooltip: tooltip.into(),
            },
        })
    }
}

impl Plugin for WidgetryTrayPlugin {
    fn build(&self, app: &mut App) {
        bridge::install();
        app.insert_resource(self.config.clone())
            .insert_non_send(TrayOwner::default())
            .add_message::<WidgetryTrayClick>()
            .add_systems(Startup, initialize)
            .add_systems(PreUpdate, bridge::forward_clicks)
            .add_systems(Last, release_on_exit);
        widgetry_info!("WidgetryTrayPlugin 注册完成");
    }
}

fn initialize(
    _main_thread: NonSendMarker,
    config: Res<TrayConfig>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
    mut owner: NonSendMut<TrayOwner>,
    mut commands: Commands,
) -> Result {
    initialize_instance(&config, proxy.as_deref(), &mut owner, &mut commands)
        .map_err(TrayFailure::into_bevy_error)
}

fn initialize_instance(
    config: &TrayConfig,
    proxy: Option<&EventLoopProxyWrapper>,
    owner: &mut TrayOwner,
    commands: &mut Commands,
) -> Result<(), TrayFailure> {
    let (width, height) = config.rgba.dimensions();
    let frame = || {
        ErrorFrame::new("initialize tray")
            .field("width", width)
            .field("height", height)
            .field("tooltip", &config.tooltip)
    };
    let Some(proxy) = proxy else {
        return Err(TrayFailure::new(TrayError::MissingEventLoop, frame()));
    };
    let native_icon =
        Icon::from_rgba(config.rgba.as_raw().clone(), width, height).map_err(|source| {
            TrayFailure::new(
                TrayError::Rgba {
                    width,
                    height,
                    source,
                },
                frame(),
            )
        })?;
    let icon = TrayIconBuilder::new()
        .with_icon(native_icon)
        .with_tooltip(&config.tooltip)
        .build()
        .map_err(|source| TrayFailure::new(TrayError::Create { source }, frame()))?;
    // tray-icon keeps a helper window even if Shell registration fails.
    // A successful tooltip update verifies registration and preserves the OS error on failure.
    icon.set_tooltip(Some(&config.tooltip)).map_err(|source| {
        TrayFailure::new(
            TrayError::Register {
                id: icon.id().clone(),
                source,
            },
            frame().field("tray", format_args!("{:?}", icon.id())),
        )
    })?;
    let proxy = (*proxy).clone();
    let (bridge, inbox) = Bridge::new(icon.id().clone(), move || {
        proxy
            .send_event(bevy::winit::WinitUserEvent::WakeUp)
            .map_err(|error| Box::new(error) as _)
    });
    let registration = Registration::new(icon.id().clone(), &bridge);
    owner.0 = Some(TrayInstance {
        _registration: registration,
        _icon: icon,
        _bridge: bridge,
    });
    commands.insert_resource(inbox);
    Ok(())
}

fn release_on_exit(
    mut owner: NonSendMut<TrayOwner>,
    mut exit: MessageReader<AppExit>,
    inbox: Option<Res<bridge::Inbox>>,
) -> Result {
    if exit.read().next().is_some() {
        owner.0 = None;
        if let Some(inbox) = inbox {
            return bridge::forward_errors(&inbox);
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn real_startup_reports_source_id_and_original_origin_to_host() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
        let mut app = App::new();
        app.insert_resource(TrayConfig {
            rgba: image::RgbaImage::new(16, 16),
            tooltip: "test".into(),
        })
        .insert_non_send(TrayOwner::default())
        .add_systems(Startup, initialize);
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(
            diagnostic.origin().backtrace().status(),
            std::backtrace::BacktraceStatus::Captured
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        assert!(
            diagnostic.contexts()[0]
                .fields()
                .contains(&("width".into(), "16".into()))
        );
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
    }

    #[test]
    fn rejects_invalid_ico_without_panicking() {
        let error = WidgetryTrayPlugin::new(b"not an icon", "test")
            .err()
            .expect("invalid ICO must fail");
        assert!(error.to_string().contains("ICO"));
        assert!(matches!(error.kind(), TrayError::Decode { .. }));
        use std::error::Error;
        assert!(error.kind().source().unwrap().is::<image::ImageError>());
    }

    #[test]
    fn rejects_png_even_when_image_decoding_would_succeed() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let bytes = Box::leak(png.into_inner().into_boxed_slice());
        assert!(WidgetryTrayPlugin::new(bytes, "test").is_err());
    }

    #[test]
    fn accepts_embedded_ico_and_keeps_configuration_without_native_creation() {
        let mut ico = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(32, 32, image::Rgba([0, 120, 255, 255]))
            .write_to(&mut ico, image::ImageFormat::Ico)
            .unwrap();
        let bytes = Box::leak(ico.into_inner().into_boxed_slice());
        let mut app = App::new();
        app.add_plugins(WidgetryTrayPlugin::new(bytes, "Embedded tooltip").unwrap());
        assert_eq!(
            app.world().resource::<TrayConfig>().tooltip,
            "Embedded tooltip"
        );
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
    }

    #[test]
    fn missing_winit_returns_error_and_leaves_no_native_instance() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.insert_resource(TrayConfig {
            rgba: image::RgbaImage::new(16, 16),
            tooltip: "test".into(),
        })
        .insert_non_send(TrayOwner::default());
        let error = app
            .world_mut()
            .run_system_once::<_, Result, _>(initialize)
            .unwrap()
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("WinitPlugin"));
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
    }
}
