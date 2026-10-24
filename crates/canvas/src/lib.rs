#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod model;

pub use model::{CanvasDocument, CanvasPaint, CanvasRectangle, CanvasStroke};
