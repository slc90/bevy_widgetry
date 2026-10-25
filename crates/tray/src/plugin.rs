use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::winit::EventLoopProxyWrapper;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct WidgetryTrayPlugin {
    config: TrayConfig,
}

#[derive(Resource, Clone)]
struct TrayConfig {
    rgba: image::RgbaImage,
    tooltip: String,
}

#[derive(Default)]
struct TrayOwner(Option<TrayIcon>);

impl WidgetryTrayPlugin {
    pub fn new(ico: &'static [u8], tooltip: impl Into<String>) -> Result<Self> {
        let rgba = image::load_from_memory_with_format(ico, image::ImageFormat::Ico)
            .map_err(|error| tray_error("ICO 图标解码失败", error))?
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
        // The default receiver is unbounded, even when no clicks are consumed.
        TrayIconEvent::set_event_handler(Some(|_| {}));
        app.insert_resource(self.config.clone())
            .insert_non_send(TrayOwner::default())
            .add_systems(Startup, initialize)
            .add_systems(Last, release_on_exit);
        widgetry_info!("WidgetryTrayPlugin 注册完成");
    }
}

fn initialize(
    _main_thread: NonSendMarker,
    config: Res<TrayConfig>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
    mut owner: NonSendMut<TrayOwner>,
) -> Result {
    if proxy.is_none() {
        return Err(tray_error("创建托盘需要 WinitPlugin", "missing event loop"));
    }
    let (width, height) = config.rgba.dimensions();
    let native_icon = Icon::from_rgba(config.rgba.as_raw().clone(), width, height)
        .map_err(|error| tray_error("RGBA 图标转换失败", error))?;
    let icon = TrayIconBuilder::new()
        .with_icon(native_icon)
        .with_tooltip(&config.tooltip)
        .build()
        .map_err(|error| tray_error("原生托盘图标创建失败", error))?;
    // tray-icon keeps a helper window even if Shell registration fails.
    // A successful tooltip update verifies registration and preserves the OS error on failure.
    icon.set_tooltip(Some(&config.tooltip))
        .map_err(|error| tray_error("原生托盘图标注册确认失败", error))?;
    owner.0 = Some(icon);
    Ok(())
}

fn release_on_exit(mut owner: NonSendMut<TrayOwner>, mut exit: MessageReader<AppExit>) {
    if exit.read().next().is_some() {
        owner.0 = None;
    }
}

fn tray_error(context: &str, error: impl std::fmt::Display) -> BevyError {
    widgetry_error!(context, error = %error, "托盘初始化失败");
    BevyError::error(format!("{context}: {error}"))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_ico_without_panicking() {
        let error = WidgetryTrayPlugin::new(b"not an icon", "test")
            .err()
            .expect("invalid ICO must fail");
        assert!(error.to_string().contains("ICO"));
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
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
