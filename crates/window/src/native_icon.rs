use crate::{WindowError, WindowFailure};
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::FailureState};
use bevy_widgetry_internal_log::widgetry_debug;
use std::sync::Arc;
use winit::platform::windows::WindowExtWindows;
use winit::window::Icon;

#[macro_export]
macro_rules! taskbar_icon {
    ($path:literal $(,)?) => {
        $crate::native_icon_plugin(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
            $path,
        )
    };
}

#[derive(Resource, Clone)]
struct NativeIcon {
    icon: Icon,
    path: String,
}

struct NativeIconPlugin(NativeIcon);

pub fn native_icon_plugin(png: &[u8], path: &str) -> Result<impl Plugin, WindowFailure> {
    let rgba = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|error| {
            WindowFailure::new(
                WindowError::IconDecode {
                    path: path.to_owned(),
                    format: image::ImageFormat::Png,
                    source: Box::new(error),
                },
                ErrorFrame::new("window.icon.decode")
                    .field("path", path)
                    .field("format", "Png"),
            )
        })?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(NativeIconPlugin(NativeIcon {
        icon: native_icon_from_rgba(rgba.into_raw(), width, height, path)?,
        path: path.to_owned(),
    }))
}

fn native_icon_from_rgba(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    path: &str,
) -> Result<Icon, WindowFailure> {
    let bytes = rgba.len();
    Icon::from_rgba(rgba, width, height).map_err(|error| {
        WindowFailure::new(
            WindowError::IconCreate {
                path: path.to_owned(),
                width,
                height,
                bytes,
                source: error,
            },
            ErrorFrame::new("window.icon.create")
                .field("path", path)
                .field("format", "Rgba8")
                .field("width", width)
                .field("height", height)
                .field("bytes", bytes),
        )
    })
}

impl Plugin for NativeIconPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone())
            .add_systems(Update, set_primary_window_icons);
        widgetry_debug!("NativeIconPlugin 注册完成");
    }
}

fn set_primary_window_icons(
    _main_thread: NonSendMarker,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    icon: Res<NativeIcon>,
    mut applied: Local<bool>,
    mut failure: Local<FailureState<Vec<Entity>>>,
) -> Result {
    if *applied {
        return Ok(());
    }
    let primary_entity = primary_window.single().map_err(|error| {
        let mut entities = primary_window.iter().collect::<Vec<_>>();
        entities.sort_unstable();
        failure
            .failure(entities.clone(), || {
                Arc::new(WindowFailure::new(
                    WindowError::PrimaryWindow {
                        entities: entities.clone(),
                        source: error,
                    },
                    ErrorFrame::new("window.icon.apply")
                        .field("path", &icon.path)
                        .field("primary_windows", format_args!("{entities:?}")),
                ))
            })
            .into_bevy_error()
    })?;
    *applied = WINIT_WINDOWS.with_borrow(|windows| {
        let Some(native_window) = windows.get_window(primary_entity) else {
            return false;
        };
        native_window.set_window_icon(Some(icon.icon.clone()));
        native_window.set_taskbar_icon(Some(icon.icon.clone()));
        true
    });
    if *applied {
        failure.recover(|| {
            ErrorFrame::new("window.icon.apply")
                .field("window", primary_entity)
                .field("path", &icon.path)
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {

    use super::*;
    use bevy::ecs::error::Severity;
    use bevy::ecs::{schedule::SingleThreadedExecutor, system::SystemState};
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    use image::{ImageEncoder, codecs::png::PngEncoder};

    fn app_with_icon() -> App {
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&[1, 2, 3, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        let mut app = App::new();
        app.add_plugins(native_icon_plugin(&png, "application.png").unwrap());
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::default());
        });
        app
    }

    #[test]
    fn valid_png_initializes_plugin() {
        let app = app_with_icon();
        assert!(app.world().contains_resource::<NativeIcon>());
    }

    #[test]
    fn invalid_png_is_an_error_before_plugin_installation() {
        let logs = LogCapture::default();
        let error = logs.run(|| {
            native_icon_plugin(b"invalid PNG", "invalid.png")
                .err()
                .unwrap()
        });
        assert!(
            matches!(error.kind(), WindowError::IconDecode { path, format: image::ImageFormat::Png, .. } if path == "invalid.png")
        );
        assert!(
            std::error::Error::source(error.kind())
                .unwrap()
                .downcast_ref::<image::ImageError>()
                .is_some()
        );
        assert_eq!(
            logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }

    #[test]
    fn incompatible_rgba_returns_error_and_context() {
        let logs = LogCapture::default();
        let error = logs
            .run(|| native_icon_from_rgba(vec![1, 2, 3], 1, 1, "invalid-rgba.png").unwrap_err());
        assert!(
            matches!(error.kind(), WindowError::IconCreate { width: 1, height: 1, bytes: 3, path, .. } if path == "invalid-rgba.png")
        );
        assert!(
            std::error::Error::source(error.kind())
                .unwrap()
                .downcast_ref::<winit::window::BadIcon>()
                .is_some()
        );
        assert_eq!(
            logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }

    #[test]
    fn missing_or_ambiguous_primary_window_reaches_host_error_handler() {
        for count in [0, 2] {
            let mut app = app_with_icon();
            for _ in 0..count {
                app.world_mut().spawn(PrimaryWindow);
            }
            let errors = ErrorCapture::default();
            let logs = LogCapture::default();
            logs.run(|| errors.run(|| app.update()));
            let errors = errors.take();
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].severity(), Severity::Error);
            assert!(
                logs.records()
                    .iter()
                    .any(|record| record.level == bevy::log::Level::ERROR)
            );
        }
    }

    #[test]
    fn native_window_not_created_retries_without_error() {
        let mut app = app_with_icon();
        let entity = app.world_mut().spawn(PrimaryWindow).id();
        let errors = ErrorCapture::default();
        errors.run(|| {
            app.update();
            app.update();
        });
        assert!(errors.take().is_empty());
        app.world_mut().despawn(entity);
        errors.run(|| app.update());
        assert_eq!(errors.take().len(), 1);
    }

    #[test]
    fn repeated_primary_failure_logs_once_but_keeps_propagating() {
        let mut app = app_with_icon();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let errors = errors.take_records();
        assert_eq!(errors.len(), 2);
        let first = errors[0].managed().unwrap().diagnostics().next().unwrap();
        let second = errors[1].managed().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(first.origin(), second.origin()));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
    }

    #[test]
    fn different_ambiguous_primary_sets_do_not_reuse_the_previous_diagnostic() {
        let mut app = app_with_icon();
        let first = app.world_mut().spawn(PrimaryWindow).id();
        let second = app.world_mut().spawn(PrimaryWindow).id();
        let errors = ErrorCapture::default();
        let mut origins = Vec::new();
        for previous in [None, Some(first)] {
            if let Some(previous) = previous {
                app.world_mut().despawn(previous);
                app.world_mut().spawn(PrimaryWindow);
            }
            errors.run(|| app.update());
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            origins.push(diagnostic.origin().clone());
            let mut cause = diagnostic.source();
            let mut found = false;
            while let Some(error) = cause {
                if let Some(WindowError::PrimaryWindow { entities, source }) =
                    error.downcast_ref::<WindowError>()
                {
                    assert!(entities.contains(&second));
                    assert_eq!(entities.contains(&first), previous.is_none());
                    assert!(matches!(
                        source,
                        bevy::ecs::query::QuerySingleError::MultipleEntities(_)
                    ));
                    found = true;
                }
                cause = error.source();
            }
            assert!(found);
        }
        assert!(!Arc::ptr_eq(&origins[0], &origins[1]));
    }

    #[test]
    fn already_applied_does_not_query_or_reset_primary_icons() {
        let mut app = app_with_icon();
        let mut state = SystemState::<(
            NonSendMarker,
            Query<Entity, With<PrimaryWindow>>,
            Res<NativeIcon>,
            Local<bool>,
            Local<FailureState<Vec<Entity>>>,
        )>::new(app.world_mut());
        let (marker, primary, icon, mut applied, last_failure) =
            state.get_mut(app.world_mut()).unwrap();
        *applied = true;
        set_primary_window_icons(marker, primary, icon, applied, last_failure).unwrap();
    }

    #[test]
    fn native_wait_preserves_origin_and_real_application_recovers_once() {
        use winit::{
            application::ApplicationHandler,
            event_loop::{ActiveEventLoop, EventLoop},
            platform::windows::EventLoopBuilderExtWindows,
        };
        struct Application {
            app: App,
            entity: Entity,
            logs: LogCapture,
            errors: ErrorCapture,
        }
        impl ApplicationHandler for Application {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                let window = event_loop
                    .create_window(winit::window::Window::default_attributes().with_visible(false))
                    .unwrap();
                let id = window.id();
                WINIT_WINDOWS.with_borrow_mut(|windows| {
                    windows
                        .windows
                        .insert(id, bevy::window::WindowWrapper::new(window));
                    windows.entity_to_winit.insert(self.entity, id);
                    windows.winit_to_entity.insert(id, self.entity);
                });
                self.logs.run(|| {
                    self.errors.run(|| {
                        self.app.update();
                        self.app.world_mut().despawn(self.entity);
                        self.app.update();
                    })
                });
                WINIT_WINDOWS.with_borrow_mut(|windows| {
                    windows.remove_window(self.entity);
                });
                event_loop.exit();
            }
            fn window_event(
                &mut self,
                _: &ActiveEventLoop,
                _: winit::window::WindowId,
                _: winit::event::WindowEvent,
            ) {
            }
        }
        let mut app = app_with_icon();
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        let origin = records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
            .clone();
        let entity = app.world_mut().spawn(PrimaryWindow).id();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        assert!(errors.take_records().is_empty());
        assert!(
            !logs
                .records_for_error(origin.id())
                .iter()
                .any(|record| record.matches_field("phase", "recovered"))
        );
        app.world_mut().despawn(entity);
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert!(Arc::ptr_eq(
            &origin,
            records[0]
                .managed()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap()
                .origin()
        ));
        let entity = app.world_mut().spawn(PrimaryWindow).id();
        let mut application = Application {
            app,
            entity,
            logs,
            errors,
        };
        EventLoop::builder()
            .with_any_thread(true)
            .build()
            .unwrap()
            .run_app(&mut application)
            .unwrap();
        assert!(application.errors.take_records().is_empty());
        assert_eq!(
            application
                .logs
                .records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );
        assert_eq!(
            application
                .logs
                .diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
}
