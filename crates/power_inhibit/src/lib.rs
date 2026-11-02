#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod error;
mod power;

pub use error::{PowerError, PowerFailure, PowerFailures, PowerOperation, PowerPhase, RequestKind};
pub use power::{PowerInhibit, WidgetryPowerInhibitPlugin};
