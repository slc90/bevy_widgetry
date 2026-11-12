#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod diagnostic_format;
mod error;
mod event_format;
mod logging;
mod writer;

pub use diagnostic_format::{safe_backtrace, safe_diagnostic, safe_location};
pub use error::{LoggingError, LoggingFailure, LoggingSource, LoggingStage};
pub use logging::{AppLogging, file_layer, log_plugin, terminal_layer};
pub use writer::LoggingGuard;
