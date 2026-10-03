//! 提供 Widgetry 内建字体和 SVG icon 的 embedded asset 注册与加载标识。

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
