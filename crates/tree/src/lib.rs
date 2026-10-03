//! 提供基于 ECS hierarchy 和 Entity identity 的 Tree，支持展开、selection、lazy loading、renderer 与 virtualized TreeView。

mod behavior;
mod model;
mod registration;
mod renderer;
mod view;

pub use behavior::{WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind};
pub use model::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeState, WidgetryTreeVisibleItem};
pub use registration::WidgetryTreePlugin;
pub use renderer::{WidgetryTreeAppExt, WidgetryTreeRenderer};
pub use view::{WidgetryTreeIcons, WidgetryTreeView, WidgetryTreeViewProps};
