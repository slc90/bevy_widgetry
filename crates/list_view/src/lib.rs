//! 提供保存列表数据的 ListModel 和浏览这些数据的 generic ListView。
//! 适用于需要单项 selection、keyboard navigation 和自定义行内容的垂直列表。
//!
//! Model 支持插入、删除、移动和修改条目，并为条目提供 stable item ID 与内容 revision。
//! renderer 根据当前 index 和条目数据生成 BSN 内容，可组合文字、Icon 或其他 Widget。
//! ListView 根据 viewport 生成可见行，滚动时回收离开可见范围的行并展示新进入的条目。
//! pointer 操作可选择条目，方向键与 Home、End 可调整 active，Space、Enter 可确认 selection。
//! PageUp、PageDown 用于翻页浏览，navigation 可使 active 条目进入可见范围。
//! 提供设置或清空 selection、设置 active 的程序化入口。
//! selection 变化通过 ValueChange 通知，payload 使用 stable item ID 并可表达未选中。
//! 提供条目 disabled 配置，以及随 theme、hover、selection 和 focus 更新的外观。
//!
//! 使用前通过 register_widgetry_list_view 注册数据类型，并提供匹配的 Model source 与 renderer。
//! 行高通过 item_height 统一配置，必须是有限正数。
//! selection 表达已选择条目，active 表达当前 navigation 位置，两者可分别存在。
//! 多个 ListView 可共享同一 Model，各自保持独立的 selection 和 active。
//! stable item ID 在条目移动后保持，条目删除后会修复失效的视图 state。
//! 程序化 selection 先提交 state 再通知，同值不通知，Model 自动 repair 保持静默。
//! disabled 限制用户操作，程序化 selection 仍可设置对应条目。

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
