//! 提供 Widgetry 内建字体与 SVG icon，供应用在 Widget 内容和窗口 controls 中使用。
//! asset 随程序嵌入，调用方可通过公开标识取得用于加载的 AssetPath。
//!
//! WidgetryAssetPlugin 注册内建 asset，使 AssetServer 能通过 embedded source 加载它们。
//! BuiltinFont 提供默认中文字体的加载路径。
//! BuiltinIcon 提供 CheckBox 指示、上下 Chevron、Tree 展开收起和 Window controls 等 icon 的加载路径。
//! 字体与 icon 均可通过 path 方法取得 AssetPath，并交给对应字体或 Icon API 使用。
//!
//! 取得路径与完成 asset 加载是两个步骤，实际加载仍通过 AssetServer 进行。
//! 使用内建路径前需要注册 WidgetryAssetPlugin，并由宿主提供相应的 asset 加载能力。
//! 内建字体的注册与应用默认字体的选择分别进行，调用方可按需选择字体。

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy_widgetry_log::widgetry_info;

pub struct WidgetryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinIcon {
    CheckboxCheck,
    CheckboxIndeterminate,
    ChevronDown,
    ChevronUp,
    TreeExpand,
    TreeCollapse,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinFont {
    Default,
}

impl BuiltinFont {
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Default => embedded_path!("assets/fonts/SmileySans-Oblique.ttf"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl BuiltinIcon {
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::CheckboxCheck => embedded_path!("assets/icons/checkbox_check.svg"),
            Self::CheckboxIndeterminate => {
                embedded_path!("assets/icons/checkbox_indeterminate.svg")
            }
            Self::ChevronDown => embedded_path!("assets/icons/chevron_down.svg"),
            Self::ChevronUp => embedded_path!("assets/icons/chevron_up.svg"),
            Self::TreeExpand => embedded_path!("assets/icons/tree_expand.svg"),
            Self::TreeCollapse => embedded_path!("assets/icons/tree_collapse.svg"),
            Self::WindowClose => embedded_path!("assets/icons/window_close.svg"),
            Self::WindowMaximize => embedded_path!("assets/icons/window_maximize.svg"),
            Self::WindowMinimize => embedded_path!("assets/icons/window_minimize.svg"),
            Self::WindowRestore => embedded_path!("assets/icons/window_restore.svg"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl Plugin for WidgetryAssetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/icons/checkbox_check.svg");
        embedded_asset!(app, "assets/icons/checkbox_indeterminate.svg");
        embedded_asset!(app, "assets/fonts/SmileySans-Oblique.ttf");
        embedded_asset!(app, "assets/icons/window_close.svg");
        embedded_asset!(app, "assets/icons/window_maximize.svg");
        embedded_asset!(app, "assets/icons/window_minimize.svg");
        embedded_asset!(app, "assets/icons/window_restore.svg");
        embedded_asset!(app, "assets/icons/chevron_down.svg");
        embedded_asset!(app, "assets/icons/chevron_up.svg");
        embedded_asset!(app, "assets/icons/tree_expand.svg");
        embedded_asset!(app, "assets/icons/tree_collapse.svg");
        widgetry_info!("WidgetryAssetPlugin 注册完成");
    }
}
