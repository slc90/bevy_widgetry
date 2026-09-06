mod svg;

use bevy::prelude::*;

pub struct IconPlugin;

impl Plugin for IconPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<svg::SvgAsset>()
            .init_asset_loader::<svg::SvgAssetLoader>();
    }
}
