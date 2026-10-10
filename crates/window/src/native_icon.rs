use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
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
struct NativeIcon(Icon);

struct NativeIconPlugin(NativeIcon);

pub fn native_icon_plugin(png: &[u8], path: &str) -> Result<impl Plugin> {
    let rgba = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|error| {
            widgetry_error!(path, error = %error, "native window PNG 图标解码失败");
            BevyError::error(error)
        })?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(NativeIconPlugin(NativeIcon(native_icon_from_rgba(
        rgba.into_raw(),
        width,
        height,
        path,
    )?)))
}

fn native_icon_from_rgba(rgba: Vec<u8>, width: u32, height: u32, path: &str) -> Result<Icon> {
    Icon::from_rgba(rgba, width, height).map_err(|error| {
        widgetry_error!(path, width, height, error = %error, "native window 图标创建失败");
        BevyError::error(error)
    })
}

impl Plugin for NativeIconPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone())
            .add_systems(Update, set_primary_window_icons);
        widgetry_info!("NativeIconPlugin 注册完成");
    }
}

fn set_primary_window_icons(
    _main_thread: NonSendMarker,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    icon: Res<NativeIcon>,
    mut applied: Local<bool>,
    mut last_failure: Local<Option<String>>,
) -> Result {
    if *applied {
        return Ok(());
    }
    let primary_entity = primary_window.single().map_err(|error| {
        let failure = error.to_string();
        if last_failure.as_ref() != Some(&failure) {
            widgetry_error!(error = %error, "设置 native window 图标时无法确定主窗口");
            *last_failure = Some(failure);
        }
        BevyError::error(error)
    })?;
    *applied = WINIT_WINDOWS.with_borrow(|windows| {
        let Some(native_window) = windows.get_window(primary_entity) else {
            return false;
        };
        native_window.set_window_icon(Some(icon.0.clone()));
        native_window.set_taskbar_icon(Some(icon.0.clone()));
        true
    });
    if *applied && last_failure.take().is_some() {
        widgetry_info!(entity = ?primary_entity, "native window 图标设置异常已恢复");
    }
    Ok(())
}

// 测试通过断言与 fixture 的 unwrap 报告 contract 失败，仅在本测试 scope 允许这些表达式。
#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    //! 初始化由 PNG bytes 驱动，失败必须返回 Severity::Error 与定位日志。
    //! 主窗口唯一性是 guard，native window 未创建属于等待，已应用后不再查询或重复设置。

    use super::*;
    use bevy::ecs::error::Severity;
    use bevy::ecs::{schedule::SingleThreadedExecutor, system::SystemState};
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
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
        assert_eq!(error.severity(), Severity::Error);
        assert!(error.to_string().contains("PNG"));
        assert!(logs.records().iter().any(|record| {
            record.level == bevy::log::Level::ERROR
                && record
                    .fields
                    .get("path")
                    .is_some_and(|path| path.contains("invalid.png"))
        }));
    }

    #[test]
    fn incompatible_rgba_returns_error_and_context() {
        let logs = LogCapture::default();
        let error = logs
            .run(|| native_icon_from_rgba(vec![1, 2, 3], 1, 1, "invalid-rgba.png").unwrap_err());
        assert_eq!(error.severity(), Severity::Error);
        assert!(
            logs.records()
                .iter()
                .any(|record| record.level == bevy::log::Level::ERROR
                    && record.fields.contains_key("width")
                    && record.fields.contains_key("height"))
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
        assert_eq!(errors.take().len(), 2);
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
    }

    #[test]
    fn already_applied_does_not_query_or_reset_primary_icons() {
        let mut app = app_with_icon();
        let mut state = SystemState::<(
            NonSendMarker,
            Query<Entity, With<PrimaryWindow>>,
            Res<NativeIcon>,
            Local<bool>,
            Local<Option<String>>,
        )>::new(app.world_mut());
        let (marker, primary, icon, mut applied, last_failure) =
            state.get_mut(app.world_mut()).unwrap();
        *applied = true;
        set_primary_window_icons(marker, primary, icon, applied, last_failure).unwrap();
    }
}
