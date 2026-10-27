#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod alpha;
mod export;
mod geometry;
mod input;
mod mesh_cache;
mod model;
mod order;
mod path;
mod renderer;
mod rotate;
mod scale;
mod selection;
mod view;

pub use export::{CanvasExportError, CanvasPngExport, CanvasPngExportFinished, CanvasPngOptions};
pub use model::{
    CanvasDocument, CanvasEllipse, CanvasFillRule, CanvasGeometry, CanvasLineCap, CanvasLineJoin,
    CanvasPaint, CanvasRectangle, CanvasShape, CanvasStroke,
};
pub use order::CanvasOrder;
pub use path::{CanvasPath, CanvasPathCommand};
pub use renderer::CanvasPlugin;
pub use view::{
    CanvasCenterChanged, CanvasDocumentChanged, CanvasMode, CanvasModeChanged,
    CanvasSelectionChanged, CanvasView, CanvasViewProps, CanvasZoomChanged,
};
