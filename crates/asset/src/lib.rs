//! Widgetry 库内部的静态资源标识与嵌入注册，不由顶层 facade 导出。

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy_widgetry_log::widgetry_info;

/// 由使用内建资源的库插件自动添加；必须在 Bevy AssetPlugin 之后注册。
pub struct WidgetryAssetPlugin;

/// 跨 crate 使用的内建图标语义标识，隐藏资源文件布局。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinIcon {
    /// 关闭窗口的系统按钮图标。
    WindowClose,
    /// 最大化窗口的系统按钮图标。
    WindowMaximize,
    /// 最小化窗口的系统按钮图标。
    WindowMinimize,
    /// 从最大化状态还原窗口的系统按钮图标。
    WindowRestore,
}

/// 跨 crate 使用的内建字体标识，不暴露文件布局给字体使用方。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinFont {
    /// App 默认字体：Smiley Sans / 得意黑 v2.0.1。
    Default,
}

impl BuiltinFont {
    /// 返回嵌入字体标识；加载前须确保 WidgetryAssetPlugin 已注册。
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Default => embedded_path!("assets/fonts/SmileySans-Oblique.ttf"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl BuiltinIcon {
    /// 返回嵌入资源标识；加载前须由库插件确保 WidgetryAssetPlugin 已注册。
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
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
        embedded_asset!(app, "assets/fonts/SmileySans-Oblique.ttf");
        embedded_asset!(app, "assets/icons/window_close.svg");
        embedded_asset!(app, "assets/icons/window_maximize.svg");
        embedded_asset!(app, "assets/icons/window_minimize.svg");
        embedded_asset!(app, "assets/icons/window_restore.svg");
        widgetry_info!("WidgetryAssetPlugin 注册完成");
    }
}
