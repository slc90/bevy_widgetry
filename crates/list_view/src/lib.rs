//! ListView 的稳定 item identity、数据模型与 generic 构造 contract。

mod model;
mod registration;
mod view;
mod virtualization;

pub use model::{WidgetryListItemId, WidgetryListModel};
pub use registration::{WidgetryListViewAppExt, WidgetryListViewPlugin};
pub use view::{
    WidgetryListView, WidgetryListViewItem, WidgetryListViewProps, WidgetryListViewRenderer,
    WidgetryListViewState,
};
