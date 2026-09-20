use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;

/// 由 Gallery 入口在 Bevy AssetPlugin 后显式装配，只注册应用自有 asset。
pub(crate) struct GalleryAssetPlugin;

/// Gallery 自有 icon 的语义标识，不与 Widgetry 库 asset 混用。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GalleryIcon {
    /// 应用 title bar 使用的 logo。
    Logo,
    /// button 内容组合演示使用的星形 icon。
    ButtonStar,
}

impl GalleryIcon {
    /// 返回已由 GalleryAssetPlugin 注册的 embedded asset 路径。
    pub(crate) fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Logo => embedded_path!("assets/icons/logo.svg"),
            Self::ButtonStar => embedded_path!("assets/icons/button_star.svg"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl Plugin for GalleryAssetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/icons/logo.svg");
        embedded_asset!(app, "assets/icons/button_star.svg");
    }
}
