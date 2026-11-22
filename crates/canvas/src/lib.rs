#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod alpha;
mod basic_path;
mod batch;
mod error;
mod export;
mod geometry;
mod groups;
mod history;
mod input;
mod layers;
mod mesh_cache;
mod model;
mod move_selection;
mod nodes;
mod order;
mod parameters;
mod path;
mod pen;
mod renderer;
mod rotate;
mod scale;
mod selection;
mod view;

pub use export::CanvasExportError;
#[cfg(feature = "png_export")]
pub use export::{
    CanvasPngExport, CanvasPngExportFinished, CanvasPngOptions, CanvasPngStop, CanvasRenderIssue,
};
pub use groups::CanvasGroup;
pub use history::{CanvasEditTarget, CanvasEditToken, CanvasHistory};
pub use layers::{CanvasLayerRemoval, CanvasLayersChanged};
pub use model::{
    CanvasDocument, CanvasEllipse, CanvasFillRule, CanvasGeometry, CanvasItem, CanvasLineCap,
    CanvasLineJoin, CanvasPaint, CanvasRectangle, CanvasShape, CanvasStroke,
};
pub use order::{CanvasContainer, CanvasLayer, CanvasLayerId, CanvasLayerStack, CanvasOrder};
pub use path::{CanvasPath, CanvasPathCommand, CanvasPathSegment};
pub use pen::{CanvasPenChanged, CanvasPenDraft, CanvasPenSegment};
pub use renderer::CanvasPlugin;
pub use view::{
    CanvasActiveLayerChanged, CanvasCenterChanged, CanvasDocumentChanged, CanvasGroupPathChanged,
    CanvasMode, CanvasModeChanged, CanvasSelectionChanged, CanvasView, CanvasViewProps,
    CanvasZoomChanged,
};

pub use error::{
    CanvasError, CanvasErrorKind, CanvasErrorSource, CanvasFailure, CanvasInput, CanvasIssue,
    CanvasPart,
};
