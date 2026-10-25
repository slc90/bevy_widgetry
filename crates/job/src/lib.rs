#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod channel;
mod manager;
mod receiver;
mod types;

pub use manager::JobManager;
pub use receiver::{JobContext, JobReceiver};
pub use types::*;
