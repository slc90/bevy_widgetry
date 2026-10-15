#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod logging;

pub use logging::{AppLogging, file_layer, terminal_layer};
pub use tracing_appender::non_blocking::WorkerGuard;
