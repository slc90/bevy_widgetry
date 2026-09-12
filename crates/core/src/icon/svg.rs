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

/// 保留已解析的 SVG 树，供不同尺寸的图标重复栅格化。
#[derive(Asset, TypePath)]
pub(crate) struct SvgAsset {
    /// 解析完成的矢量树，栅格化时保持不变。
    tree: Tree,
}

/// 通过 Bevy 资产管线传播读取与 SVG 解析错误。
#[derive(Default, TypePath)]
pub(crate) struct SvgAssetLoader;

/// SVG 已解析但无法创建目标像素缓冲区；与正常资产等待严格区分。
#[derive(Debug)]
pub(super) struct RasterizationError {
    /// 实际请求的像素宽度。
    pub width: u32,
    /// 实际请求的像素高度。
    pub height: u32,
}

/// 转移 RGBA 像素所有权，保持栅格化输出的尺寸和颜色格式。
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
    /// 测试直接提供解析树以重现像素分配边界，不绕过生产加载器的错误语义。
    #[cfg(test)]
    pub(super) fn from_tree(tree: Tree) -> Self {
        Self { tree }
    }

    /// 按比例生成像素缓冲区，失败时携带目标尺寸交给图标层诊断。
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

    /// 选择能同时满足宽高上限的比例，保持 SVG 宽高比。
    fn render_to_pixmap(
        &self,
        max_width: u32,
        max_height: u32,
    ) -> Result<Pixmap, RasterizationError> {
        let size = self.tree.size();

        let scale = (max_width as f32 / size.width()).min(max_height as f32 / size.height());

        self.render_with_scale(scale)
    }

    /// 以原始 SVG 尺寸生成像素，避免隐式拉伸。
    fn render_intrinsic_to_pixmap(&self) -> Result<Pixmap, RasterizationError> {
        self.render_with_scale(1.0)
    }

    /// 在指定像素范围内等比栅格化，并转换为 Bevy 图像。
    pub(super) fn render_to_image(
        &self,
        max_width: u32,
        max_height: u32,
    ) -> Result<Image, RasterizationError> {
        let pixmap = self.render_to_pixmap(max_width, max_height)?;
        Ok(image_from_pixmap(pixmap))
    }

    /// 按 SVG 固有尺寸生成可供 ImageNode 使用的图像。
    pub(super) fn render_intrinsic_to_image(&self) -> Result<Image, RasterizationError> {
        let pixmap = self.render_intrinsic_to_pixmap()?;
        Ok(image_from_pixmap(pixmap))
    }
}
