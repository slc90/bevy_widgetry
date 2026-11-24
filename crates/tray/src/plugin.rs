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

/// The outcome of native tray initialization during Startup.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryTrayStatus {
    /// The plugin is installed and native initialization is pending.
    #[default]
    Initializing,
    /// Native initialization completed, including Windows Shell registration.
    Ready,
    /// Native initialization failed and its diagnostic went to the host error handler.
    Failed,
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
            .insert_resource(WidgetryTrayStatus::Initializing)
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
    mut status: ResMut<WidgetryTrayStatus>,
    mut commands: Commands,
) -> Result {
    match initialize_instance(
        &config,
        menu.as_deref(),
        proxy.as_deref(),
        &mut owner,
        &mut commands,
    ) {
        Ok(()) => {
            // Publish readiness after the deferred Inbox insertion is applied.
            commands.insert_resource(WidgetryTrayStatus::Ready);
            Ok(())
        }
        Err(failure) => {
            *status = WidgetryTrayStatus::Failed;
            Err(failure.into_bevy_error())
        }
    }
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
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    use std::error::Error;

    fn test_plugin(tooltip: &str) -> WidgetryTrayPlugin {
        let mut ico = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(16, 16, image::Rgba([0, 120, 255, 255]))
            .write_to(&mut ico, image::ImageFormat::Ico)
            .unwrap();
        let bytes = Box::leak(ico.into_inner().into_boxed_slice());
        WidgetryTrayPlugin::new(bytes, tooltip).unwrap()
    }

    #[test]
    fn startup_without_winit_publishes_failed_and_preserves_the_host_diagnostic() {
        let mut app = App::new();
        app.add_plugins(test_plugin("Failed tray"));
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| app.update()));
        assert_eq!(
            app.world().get_resource::<WidgetryTrayStatus>(),
            Some(&WidgetryTrayStatus::Failed)
        );
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(matches!(
            diagnostic.source().unwrap().downcast_ref::<TrayError>(),
            Some(TrayError::MissingEventLoop)
        ));
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
        let frame = &diagnostic.contexts()[0];
        assert_eq!(frame.operation(), "initialize tray");
        assert!(frame.fields().contains(&("width".into(), "16".into())));
        assert!(
            frame
                .fields()
                .contains(&("tooltip".into(), "Failed tray".into()))
        );
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
        assert!(!app.world().contains_resource::<bridge::Inbox>());
    }

    #[test]
    fn rejects_invalid_ico_before_installing_a_tray_status() {
        let mut app = App::new();
        let error = WidgetryTrayPlugin::new(b"not an icon", "test")
            .map(|plugin| {
                app.add_plugins(plugin);
            })
            .expect_err("invalid ICO must fail");
        assert!(error.to_string().contains("ICO"));
        assert!(matches!(error.kind(), TrayError::Decode { .. }));
        assert!(error.kind().source().unwrap().is::<image::ImageError>());
        assert!(!app.world().contains_resource::<WidgetryTrayStatus>());
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
    fn installing_the_plugin_publishes_initializing_before_native_creation() {
        let mut app = App::new();
        app.add_plugins(test_plugin("Embedded tooltip"));
        assert_eq!(
            app.world().resource::<TrayConfig>().tooltip,
            "Embedded tooltip"
        );
        assert_eq!(
            app.world().get_resource::<WidgetryTrayStatus>(),
            Some(&WidgetryTrayStatus::Initializing)
        );
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
    }

    #[test]
    fn native_startup_publishes_ready_after_the_inbox_and_menu_errors_leave_it_ready() {
        use crate::TrayMenuEntry;
        use bevy::a11y::AccessibilityPlugin;
        use bevy::asset::AssetPlugin;
        use bevy::image::ImagePlugin;
        use bevy::input::InputPlugin;
        use bevy::window::ExitCondition;
        use bevy::winit::WinitPlugin;

        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            },
            AccessibilityPlugin,
            AssetPlugin::default(),
            WinitPlugin {
                run_on_any_thread: true,
            },
            ImagePlugin::default(),
            test_plugin("Ready tray"),
        ));
        app.set_error_handler(ErrorCapture::handler());
        app.insert_resource(
            WidgetryTrayMenu::new(vec![TrayMenuEntry::item("open", "Open")]).unwrap(),
        );
        app.add_systems(
            Startup,
            (|status: Res<WidgetryTrayStatus>, inbox: Option<Res<bridge::Inbox>>| {
                assert_eq!(*status, WidgetryTrayStatus::Initializing);
                assert!(inbox.is_none());
            })
            .after_ignore_deferred(initialize),
        );
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        let startup_errors = errors.take();
        assert!(startup_errors.is_empty(), "{startup_errors:?}");
        assert_eq!(
            *app.world().resource::<WidgetryTrayStatus>(),
            WidgetryTrayStatus::Ready
        );
        assert!(app.world().contains_resource::<bridge::Inbox>());
        assert!(app.world().non_send::<TrayOwner>().0.is_some());

        app.world_mut()
            .resource_mut::<WidgetryTrayMenu>()
            .set_text("open", "invalid\0label")
            .unwrap();
        errors.run(|| app.update());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(matches!(
            diagnostic.source().unwrap().downcast_ref::<TrayError>(),
            Some(TrayError::MenuProjection { .. })
        ));
        assert_eq!(
            *app.world().resource::<WidgetryTrayStatus>(),
            WidgetryTrayStatus::Ready
        );

        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert!(app.world().non_send::<TrayOwner>().0.is_none());
    }
}
