mod constants;

use bevy::asset::io::embedded::{EmbeddedAssetRegistry, watched_path};
use bevy::asset::{AssetPath, embedded_path};
use bevy::prelude::*;
use constants::{
    BASIC_REPLAY_WAVEFORM, BUTTON_STAR_ICON, EMBEDDED_SOURCE, LOGO_ICON, NATIVE_WINDOW_ICON,
    WINDOW_BACKGROUND_1_IMAGE, WINDOW_BACKGROUND_2_IMAGE,
};
use winit::window::Icon;

pub(crate) fn native_window_icon() -> Result<Icon> {
    let rgba = image::load_from_memory(NATIVE_WINDOW_ICON.1)
        .map_err(|error| {
            error!(path = NATIVE_WINDOW_ICON.0, error = %error, "Gallery native window 图标解码失败");
            BevyError::error(error)
        })?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).map_err(|error| {
        error!(path = NATIVE_WINDOW_ICON.0, width, height, error = %error, "Gallery native window 图标创建失败");
        BevyError::error(error)
    })
}

pub(crate) struct GalleryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GalleryIcon {
    Logo,
    ButtonStar,
}

#[derive(Clone, Copy)]
pub(crate) enum GalleryImage {
    WindowBackground1,
    WindowBackground2,
}

impl GalleryImage {
    pub(crate) fn path(self) -> AssetPath<'static> {
        AssetPath::from_path_buf(match self {
            Self::WindowBackground1 => embedded_path!(WINDOW_BACKGROUND_1_IMAGE.0),
            Self::WindowBackground2 => embedded_path!(WINDOW_BACKGROUND_2_IMAGE.0),
        })
        .with_source(EMBEDDED_SOURCE)
    }
}

impl GalleryIcon {
    pub(crate) fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Logo => embedded_path!(LOGO_ICON.0),
            Self::ButtonStar => embedded_path!(BUTTON_STAR_ICON.0),
        };
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }
}

impl Plugin for GalleryAssetPlugin {
    fn build(&self, app: &mut App) {
        {
            let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
            for (path, bytes) in [
                LOGO_ICON,
                BUTTON_STAR_ICON,
                BASIC_REPLAY_WAVEFORM,
                WINDOW_BACKGROUND_1_IMAGE,
                WINDOW_BACKGROUND_2_IMAGE,
            ] {
                embedded.insert_asset(watched_path(file!(), path), &embedded_path!(path), bytes);
            }
        }
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
            Self::BasicReplay => embedded_path!(BASIC_REPLAY_WAVEFORM.0),
        })
        .with_source(EMBEDDED_SOURCE)
    }
}
