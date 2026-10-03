use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_widgetry_log::widgetry_error;
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

#[derive(Debug)]
pub(super) struct RasterizationError {
    pub width: u32,
    pub height: u32,
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

impl AssetLoader for SvgAssetLoader {
    type Asset = SvgAsset;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.map_err(|error| {
            widgetry_error!(path = ?load_context.path(), %error, "SVG asset 读取失败");
            BevyError::error(error)
        })?;

        let options = Options::default();
        let tree = Tree::from_data(&bytes, &options).map_err(|error| {
            widgetry_error!(path = ?load_context.path(), %error, "SVG asset 解析失败");
            BevyError::error(error)
        })?;

        Ok(SvgAsset { tree })
    }

    fn extensions(&self) -> &[&str] {
        &["svg"]
    }
}

impl SvgAsset {
    #[cfg(test)]
    pub(super) fn from_tree(tree: Tree) -> Self {
        Self { tree }
    }

    fn render_with_scale(&self, scale: f32) -> Result<Pixmap, RasterizationError> {
        let size = self.tree.size();

        let width = (size.width() * scale).ceil() as u32;
        let height = (size.height() * scale).ceil() as u32;

        let mut pixmap = Pixmap::new(width, height).ok_or(RasterizationError { width, height })?;

        resvg::render(
            &self.tree,
            Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        Ok(pixmap)
    }

    fn render_to_pixmap(
        &self,
        max_width: u32,
        max_height: u32,
    ) -> Result<Pixmap, RasterizationError> {
        let size = self.tree.size();

        let scale = (max_width as f32 / size.width()).min(max_height as f32 / size.height());

        self.render_with_scale(scale)
    }

    fn render_intrinsic_to_pixmap(&self) -> Result<Pixmap, RasterizationError> {
        self.render_with_scale(1.0)
    }

    pub(super) fn render_to_image(
        &self,
        max_width: u32,
        max_height: u32,
    ) -> Result<Image, RasterizationError> {
        let pixmap = self.render_to_pixmap(max_width, max_height)?;
        Ok(image_from_pixmap(pixmap))
    }

    pub(super) fn render_intrinsic_to_image(&self) -> Result<Image, RasterizationError> {
        let pixmap = self.render_intrinsic_to_pixmap()?;
        Ok(image_from_pixmap(pixmap))
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn rectangle(width: f32, height: f32) -> SvgAsset {
        SvgAsset::from_tree(Tree::from_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"><rect width="100%" height="100%" fill="white"/></svg>"#
        ), &Options::default()).unwrap())
    }

    fn assert_image(image: &Image, width: u32, height: u32) {
        assert_eq!(
            image.texture_descriptor.size,
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1
            }
        );
        assert_eq!(image.texture_descriptor.dimension, TextureDimension::D2);
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        assert_eq!(
            image.data.as_ref().unwrap().len(),
            (width * height * 4) as usize
        );
        assert!(
            image
                .data
                .as_ref()
                .unwrap()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] > 0)
        );
    }

    #[test]
    fn intrinsic_dimensions_preserve_rectangle_and_round_up() {
        assert_image(
            &rectangle(13.0, 7.0).render_intrinsic_to_image().unwrap(),
            13,
            7,
        );
        assert_image(
            &rectangle(10.25, 3.5).render_intrinsic_to_image().unwrap(),
            11,
            4,
        );
    }

    #[test]
    fn bounded_dimensions_use_limiting_edge_and_round_up() {
        for (width, height, max_width, max_height, expected) in [
            (30.0, 10.0, 15, 40, (15, 5)),
            (10.0, 30.0, 40, 15, (5, 15)),
            (13.0, 7.0, 10, 10, (10, 6)),
            (7.0, 13.0, 10, 10, (6, 10)),
            (3.0, 2.0, 12, 12, (12, 8)),
        ] {
            assert_image(
                &rectangle(width, height)
                    .render_to_image(max_width, max_height)
                    .unwrap(),
                expected.0,
                expected.1,
            );
        }
    }

    #[test]
    fn image_conversion_preserves_pixel_bytes() {
        let mut pixmap = Pixmap::new(2, 1).unwrap();
        pixmap
            .data_mut()
            .copy_from_slice(&[10, 20, 30, 255, 0, 0, 0, 0]);
        let image = image_from_pixmap(pixmap);
        assert_image(&image, 2, 1);
        assert_eq!(image.data.unwrap(), [10, 20, 30, 255, 0, 0, 0, 0]);
    }
}
