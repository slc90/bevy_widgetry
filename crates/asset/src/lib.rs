//! Widgetry 库内部的静态 asset 标识与 embedded 注册，不由顶层 facade 导出。

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy_widgetry_log::widgetry_info;

/// 由使用内建 asset 的库 plugin 自动添加；必须在 Bevy AssetPlugin 之后注册。
pub struct WidgetryAssetPlugin;

/// 跨 crate 使用的内建 icon 语义标识，隐藏 asset 文件组织方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinIcon {
    /// Checkbox 已选中的 check mark。
    CheckboxCheck,
    /// Checkbox 部分选中的 mark。
    CheckboxIndeterminate,
    /// ComboBox 关闭时的向下箭头。
    ChevronDown,
    /// ComboBox 展开时的向上箭头。
    ChevronUp,
    /// 关闭 window 的系统按钮 icon。
    WindowClose,
    /// maximize window 的系统按钮 icon。
    WindowMaximize,
    /// minimize window 的系统按钮 icon。
    WindowMinimize,
    /// 从 maximized state 还原 window 的系统按钮 icon。
    WindowRestore,
}

/// 跨 crate 使用的内建字体标识，不暴露文件组织方式给字体使用方。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinFont {
    /// App 默认字体：Smiley Sans / 得意黑 v2.0.1。
    Default,
}

impl BuiltinFont {
    /// 返回 embedded 字体标识；加载前须确保 WidgetryAssetPlugin 已注册。
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Default => embedded_path!("assets/fonts/SmileySans-Oblique.ttf"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl BuiltinIcon {
    /// 返回 embedded asset 标识；加载前须由库 plugin 确保 WidgetryAssetPlugin 已注册。
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::CheckboxCheck => embedded_path!("assets/icons/checkbox_check.svg"),
            Self::CheckboxIndeterminate => {
                embedded_path!("assets/icons/checkbox_indeterminate.svg")
            }
            Self::ChevronDown => embedded_path!("assets/icons/chevron_down.svg"),
            Self::ChevronUp => embedded_path!("assets/icons/chevron_up.svg"),
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
        widgetry_info!("WidgetryAssetPlugin 注册完成");
    }
}
