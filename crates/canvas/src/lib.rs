#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod alpha;
mod geometry;
mod input;
mod mesh_cache;
mod model;
mod renderer;
mod selection;
mod view;

pub use model::{CanvasDocument, CanvasPaint, CanvasRectangle, CanvasStroke};
pub use renderer::CanvasPlugin;
pub use view::{
    CanvasCenterChanged, CanvasDocumentChanged, CanvasSelectionChanged, CanvasView,
    CanvasViewProps, CanvasZoomChanged,
};
