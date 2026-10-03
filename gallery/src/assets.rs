use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;

pub(crate) struct GalleryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GalleryIcon {
    Logo,
    ButtonStar,
}

impl GalleryIcon {
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
        embedded_asset!(app, "assets/waveform/basic_replay.wfrm");
        app.init_asset::<crate::waveform_data::ReplayAsset>()
            .init_asset_loader::<crate::waveform_data::ReplayLoader>();
    }
}

pub(crate) enum GalleryWaveform {
    BasicReplay,
}

impl GalleryWaveform {
    pub(crate) fn path(self) -> AssetPath<'static> {
        AssetPath::from_path_buf(match self {
            Self::BasicReplay => embedded_path!("assets/waveform/basic_replay.wfrm"),
        })
        .with_source("embedded")
    }
}
