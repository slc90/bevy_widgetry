#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod behavior;
mod drag;
mod model;
mod projection;
mod registration;
mod selection;
mod source;
mod style;
mod view;
mod virtualization;

pub use model::{WidgetryListItemId, WidgetryListModel};
pub use projection::{WidgetryListProjection, WidgetryListProjectionPolicy};
pub use registration::{WidgetryListViewAppExt, WidgetryListViewPlugin, WidgetryListViewSystems};
pub use selection::{
    WidgetryListSelectAllScope, WidgetryListSelectionChanged, WidgetryListSelectionMode,
    WidgetryListSelectionState,
};
pub use source::{
    WidgetryListData, WidgetryListDataSource, WidgetryListKey, WidgetryListModelSource,
    WidgetryListValue,
};
pub use view::{
    WidgetryListView, WidgetryListViewItem, WidgetryListViewProps, WidgetryListViewRenderer,
    WidgetryListViewState,
};

mod colors;
pub use colors::*;

pub mod internal {
    pub use crate::behavior::{NavigationFocusOwner, set_active};
    pub use crate::drag::{
        TreeDragSource, WidgetryListDropPlacement, WidgetryListMove, register_tree_drag_source,
    };
    pub use crate::projection::refresh_root as refresh_projection;
    pub use crate::selection::set_selection_id_silent;
    pub use crate::style::{apply_owned_list_colors, apply_owned_list_colors_with_key};
    pub use crate::view::source_reference;
}
