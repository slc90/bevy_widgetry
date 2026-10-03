//! 提供 ListModel 和泛型 ListView，支持 stable item identity、virtualization、selection、navigation 和 theme style。

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
