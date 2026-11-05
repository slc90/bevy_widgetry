use crate::bridge::{
    self, Bridge, MenuRegistration, Registration, WidgetryTrayClick, WidgetryTrayMenuClick,
};
use crate::menu::NativeMenu;
use crate::native::MenuSession;
use crate::{TrayError, TrayFailure, WidgetryTrayMenu};
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::winit::EventLoopProxyWrapper;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_internal_log::widgetry_debug;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct WidgetryTrayPlugin {
    config: TrayConfig,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetryTraySystems {
    Sync,
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
    menu_registration: Option<MenuRegistration>,
    session: Option<MenuSession>,
    icon: TrayIcon,
    menu: Option<NativeMenu>,
    bridge: std::sync::Arc<Bridge>,
    applied: Option<WidgetryTrayMenu>,
    failed_request: Option<Option<WidgetryTrayMenu>>,
    last_sync_error: Option<std::sync::Arc<TrayFailure>>,
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
            .add_message::<WidgetryTrayMenuClick>()
            .add_systems(Startup, initialize)
            .add_systems(PreUpdate, bridge::forward_clicks)
            .add_systems(
                Last,
                (
                    synchronize_menu.in_set(WidgetryTraySystems::Sync),
                    release_on_exit.after(WidgetryTraySystems::Sync),
                ),
            );
        widgetry_debug!("WidgetryTrayPlugin 注册完成");
    }
}

fn initialize(
    _main_thread: NonSendMarker,
    config: Res<TrayConfig>,
    menu: Option<Res<WidgetryTrayMenu>>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
    mut owner: NonSendMut<TrayOwner>,
    mut commands: Commands,
) -> Result {
    initialize_instance(
        &config,
        menu.as_deref(),
        proxy.as_deref(),
        &mut owner,
        &mut commands,
    )
    .map_err(TrayFailure::into_bevy_error)
}

fn initialize_instance(
    config: &TrayConfig,
    menu: Option<&WidgetryTrayMenu>,
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
    let applied = menu.filter(|menu| !menu.entries().is_empty()).cloned();
    let menu = applied.as_ref().map(NativeMenu::new).transpose()?;
    let mut builder = TrayIconBuilder::new()
        .with_icon(native_icon)
        .with_tooltip(&config.tooltip)
        .with_menu_on_left_click(false);
    if let Some(menu) = &menu {
        builder = builder.with_menu(Box::new(menu.root.clone()));
    }
    let icon = builder
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
    let session = menu
        .as_ref()
        .map(|_| MenuSession::new(&icon, &bridge))
        .transpose()?;
    let menu_registration = menu
        .as_ref()
        .map(|menu| MenuRegistration::new(&menu.routes, &bridge));
    owner.0 = Some(TrayInstance {
        _registration: registration,
        menu_registration,
        session,
        icon,
        menu,
        bridge,
        applied,
        failed_request: None,
        last_sync_error: None,
    });
    commands.insert_resource(inbox);
    Ok(())
}

fn synchronize_menu(
    _main_thread: NonSendMarker,
    menu: Option<Res<WidgetryTrayMenu>>,
    mut owner: NonSendMut<TrayOwner>,
    exit: Res<Messages<AppExit>>,
) -> Result {
    if !exit.is_empty() {
        return Ok(());
    }
    let Some(instance) = &mut owner.0 else {
        return Ok(());
    };
    let changed = menu.as_ref().is_some_and(|menu| menu.is_changed());
    let desired = menu.as_deref().filter(|menu| !menu.entries().is_empty());
    let dirty = instance
        .bridge
        .menu_dirty
        .swap(false, std::sync::atomic::Ordering::AcqRel);
    if !changed
        && !dirty
        && instance
            .failed_request
            .as_ref()
            .is_some_and(|failed| failed.as_ref() == desired)
    {
        return Ok(());
    }
    match instance.synchronize(desired, dirty) {
        Ok(()) => {
            instance.failed_request = None;
            if let Some(failure) = instance.last_sync_error.take() {
                failure.record_handled("恢复托盘菜单同步", "已按宿主当前期望状态完成原生菜单同步");
            }
            Ok(())
        }
        Err(failure) => {
            instance.failed_request = Some(desired.cloned());
            let failure = std::sync::Arc::new(failure);
            instance.last_sync_error = Some(failure.clone());
            Err(
                bevy_widgetry_diagnostics::bevy::WidgetryEcsError::from_shared(failure)
                    .into_bevy_error(),
            )
        }
    }
}

impl TrayInstance {
    fn synchronize(
        &mut self,
        desired: Option<&WidgetryTrayMenu>,
        dirty: bool,
    ) -> Result<(), TrayFailure> {
        if desired == self.applied.as_ref() && self.failed_request.is_none() && !dirty {
            return Ok(());
        }
        let next = desired.map(NativeMenu::new).transpose()?;
        if next.is_some() && self.session.is_none() {
            self.session = Some(MenuSession::new(&self.icon, &self.bridge)?);
        }
        // Revoke retired IDs before replacing the projection, including queued activations.
        self.menu_registration = None;
        self.icon
            .set_menu(next.as_ref().map(|menu| Box::new(menu.root.clone()) as _));
        self.menu_registration = next
            .as_ref()
            .map(|menu| MenuRegistration::new(&menu.routes, &self.bridge));
        self.menu = next;
        self.applied = desired.cloned();
        Ok(())
    }
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
mod menu_tests;

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
