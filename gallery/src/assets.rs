use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;

/// 由 Gallery 入口在 Bevy AssetPlugin 后显式装配，只注册应用自有资源。
pub(crate) struct GalleryAssetPlugin;

/// Gallery 自有图标的语义标识，不与 Widgetry 库资源混用。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GalleryIcon {
    /// 应用标题栏使用的标志。
    Logo,
}

impl GalleryIcon {
    /// 返回已由 GalleryAssetPlugin 注册的嵌入资源路径。
    pub(crate) fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Logo => embedded_path!("assets/icons/logo.svg"),
        };
        AssetPath::from_path_buf(path).with_source("embedded")
    }
}

impl Plugin for GalleryAssetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/icons/logo.svg");
    }
}
