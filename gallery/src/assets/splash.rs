use super::constants::*;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SplashLayerKind {
    BodyOpen,
    BodyClosed,
    SleevePatch,
    Arm,
    RibbonUpper,
    RibbonLower,
}

#[derive(Clone, Deserialize)]
pub(crate) struct SplashLayer {
    pub(crate) name: SplashLayerKind,
    pub(crate) source_size: [f32; 2],
    pub(crate) source_pivot: [f32; 2],
    pub(crate) pivot: [f32; 2],
    pub(crate) scale: f32,
    pub(crate) z: i32,
    pub(crate) rotation_degrees: f32,
    pub(crate) source_rect: Option<[f32; 4]>,
    pub(crate) draw_size: Option<[f32; 2]>,
}

#[derive(Resource, Deserialize)]
pub(crate) struct SplashArtboard {
    pub(crate) artboard: [f32; 2],
    pub(crate) layers: Vec<SplashLayer>,
}

impl SplashArtboard {
    pub(crate) fn load() -> Result<Self> {
        serde_json::from_slice(SPLASH_LAYOUT.1).map_err(|error| {
            error!(error = %error, "千石冠 Splash layout 读取失败");
            BevyError::error(error)
        })
    }
}
