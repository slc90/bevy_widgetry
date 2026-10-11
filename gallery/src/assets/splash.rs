use super::constants::*;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SplashLayerKind {
    BodyOpen,
    BodyClosed,
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
    pub(crate) source_reveal_anchor: Option<[f32; 2]>,
}

#[derive(Resource, Deserialize)]
pub(crate) struct SplashArtboard {
    pub(crate) artboard: [f32; 2],
    pub(crate) layers: Vec<SplashLayer>,
    #[serde(skip)]
    pub(crate) palm: Vec2,
}

impl SplashArtboard {
    pub(crate) fn load() -> Result<Self> {
        let mut layout: Self = serde_json::from_slice(SPLASH_LAYOUT.1).map_err(|error| {
            error!(error = %error, "千石冠 Splash layout 读取失败");
            BevyError::error(error)
        })?;
        let anchor = layout.layers.iter().find_map(|layer| {
            (layer.name == SplashLayerKind::Arm)
                .then_some(layer)
                .and_then(|layer| {
                    layer.source_reveal_anchor.map(|anchor| {
                        Vec2::from_array(layer.pivot)
                            + Rot2::degrees(layer.rotation_degrees)
                                * ((Vec2::from_array(anchor)
                                    - Vec2::from_array(layer.source_pivot))
                                    * layer.scale)
                    })
                })
        });
        let Some(palm) = anchor.filter(|point| point.is_finite()) else {
            error!("千石冠 Splash 缺少有效手心锚点");
            return Err(BevyError::error("千石冠 Splash 缺少有效手心锚点"));
        };
        layout.palm = palm;
        Ok(layout)
    }
}
