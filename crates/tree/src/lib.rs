#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod behavior;
mod model;
mod movement;
mod registration;
mod renderer;
mod source;
mod view;

pub use behavior::{WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind};
pub use model::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeState, WidgetryTreeVisibleItem};
pub use movement::{WidgetryTreeDropPlacement, WidgetryTreeMove};
pub use registration::WidgetryTreePlugin;
pub use renderer::{WidgetryTreeAppExt, WidgetryTreeRenderer};
pub use view::{WidgetryTreeIcons, WidgetryTreeView, WidgetryTreeViewProps};

mod colors;
pub use colors::*;
