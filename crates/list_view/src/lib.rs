#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod behavior;
mod model;
mod registration;
mod style;
mod view;
mod virtualization;

pub use model::{WidgetryListItemId, WidgetryListModel};
pub use registration::{WidgetryListViewAppExt, WidgetryListViewPlugin, WidgetryListViewSystems};
pub use view::{
    WidgetryListView, WidgetryListViewItem, WidgetryListViewProps, WidgetryListViewRenderer,
    WidgetryListViewState,
};

mod colors;
pub use colors::*;

pub mod internal {
    pub use crate::style::apply_owned_list_colors;
}
