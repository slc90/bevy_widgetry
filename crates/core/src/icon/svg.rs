use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};

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
impl SvgAsset {
    fn render_with_scale(&self, scale: f32) -> Option<Pixmap> {
        let size = self.tree.size();

        let width = (size.width() * scale).ceil() as u32;
        let height = (size.height() * scale).ceil() as u32;

        let mut pixmap = Pixmap::new(width, height)?;

        resvg::render(
            &self.tree,
            Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        Some(pixmap)
    }

    fn render_to_pixmap(&self, max_width: u32, max_height: u32) -> Option<Pixmap> {
        let size = self.tree.size();

        let scale = (max_width as f32 / size.width()).min(max_height as f32 / size.height());

        self.render_with_scale(scale)
    }

    fn render_intrinsic_to_pixmap(&self) -> Option<Pixmap> {
        self.render_with_scale(1.0)
    }

    pub(super) fn render_to_image(&self, max_width: u32, max_height: u32) -> Option<Image> {
        let pixmap = self.render_to_pixmap(max_width, max_height)?;
        Some(image_from_pixmap(pixmap))
    }

    pub(super) fn render_intrinsic_to_image(&self) -> Option<Image> {
        let pixmap = self.render_intrinsic_to_pixmap()?;
        Some(image_from_pixmap(pixmap))
    }
}

fn image_from_pixmap(pixmap: Pixmap) -> Image {
    let width = pixmap.width();
    let height = pixmap.height();

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixmap.take(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}
