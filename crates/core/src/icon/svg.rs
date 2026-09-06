use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    reflect::TypePath,
};
use resvg::usvg::{Options, Tree};

#[derive(Asset, TypePath)]
pub(crate) struct SvgAsset {
    tree: Tree,
}

#[derive(Default, TypePath)]
pub(crate) struct SvgAssetLoader;

impl AssetLoader for SvgAssetLoader {
    type Asset = SvgAsset;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let options = Options::default();
        let tree = Tree::from_data(&bytes, &options)?;

        Ok(SvgAsset { tree })
    }

    fn extensions(&self) -> &[&str] {
        &["svg"]
    }
}
