#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod channel;
mod lifecycle;
mod manager;
mod receiver;
mod shutdown;
mod types;

pub use manager::JobManager;
pub use receiver::{JobContext, JobReceiver};
pub use shutdown::{ShutdownHandle, ShutdownMessage, ShutdownPhase, ShutdownStatus};
pub use types::*;
