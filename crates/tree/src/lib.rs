//! 提供用于浏览 ECS hierarchy 中层级数据的 Tree Model 与 TreeView。
//! 业务 node 使用 Entity 作为 identity，可用于目录、对象层级和按需展开的数据浏览场景。
//!
//! WidgetryTreeModel 从指定 root 组织带有 WidgetryTreeNode 的 hierarchy，并提供当前可见 node 与 depth。
//! 支持展开、收起、切换展开和设置或清空 selection，并通过 WidgetryTreeEvent 通知变化。
//! TreeView 按 depth 展示缩进、展开图标与 node 内容，并生成 viewport 中的可见行。
//! 行内容通过按 Component 类型注册的 renderer 构造，可组合业务文字、Icon 和其他 Widget。
//! 用户可通过展开图标操作层级，通过行输入选择 node，并通过 keyboard navigation 浏览可见行。
//! lazy node 可使用 Unknown、Loading 和 Loaded 表达 children 的加载状态。
//! 首次展开 Unknown node 时发出 ChildrenRequested，调用方加载并添加 children 后可通过 set_loaded 完成请求。
//! 支持配置行高、缩进宽度与展开收起 icon，界面配色跟随 theme 和交互 state。
//!
//! 展开 state 与 selection 属于 Model，多个 TreeView 共享同一 source 时展示同一业务 state。
//! 各 TreeView 的 disabled 输入边界独立，可分别控制用户操作。
//! 每个可见业务 node 应匹配一个已注册的 Component renderer。
//! 展开图标用于调整展开 state，行 selection 通过 node Entity 表达。
//! hierarchy 与业务 Component 更新会刷新可见内容，View 中行的回收保持业务 node lifecycle 独立。
//! lazy loading 的实际数据获取由调用方响应 ChildrenRequested 完成。
//! 程序化展开与 selection 更新先提交 state 再通知，合法同值操作保持静默。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

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

mod colors;
pub use colors::*;
