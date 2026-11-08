const ATLAS_ROWS: u32 = 6;

use super::{
    GalleryCursorAnimation, constants::EMBEDDED_SOURCE, cursor_animation::KAMURI_ANIMATIONS,
};
use bevy::{
    asset::{
        AssetPath, embedded_path,
        io::embedded::{EmbeddedAssetRegistry, watched_path},
    },
    prelude::*,
};
use bevy_widgetry::cursor::{
    CursorAnimation, CursorAnimationFrame, CursorImage, CursorImageSource,
};

#[derive(Clone, Copy)]
pub(crate) enum GalleryCursorImage {
    KamuriStaticAtlas,
}

impl GalleryCursorImage {
    pub(crate) fn path(self) -> AssetPath<'static> {
        AssetPath::from_path_buf(embedded_path!("cursors/kamuri/static_atlas_80.png"))
            .with_source(EMBEDDED_SOURCE)
    }
    pub(crate) fn atlas_layout(self) -> TextureAtlasLayout {
        let mut layout = TextureAtlasLayout::from_grid(
            UVec2::splat(super::cursor_animation::FRAME_SIZE),
            super::cursor_animation::ATLAS_COLUMNS,
            ATLAS_ROWS,
            None,
            None,
        );
        layout.textures.truncate(GalleryCursorAnimation::ALL.len());
        layout
    }
}

pub(super) fn register(embedded: &EmbeddedAssetRegistry) {
    for (path, bytes) in KAMURI_ANIMATIONS.into_iter().chain([(
        "cursors/kamuri/static_atlas_80.png",
        include_bytes!("cursors/kamuri/static_atlas_80.png") as &[u8],
    )]) {
        embedded.insert_asset(watched_path(file!(), path), &embedded_path!(path), bytes);
    }
}

impl GalleryCursorAnimation {
    pub(crate) fn definition(self) -> CursorAnimation {
        let fallback = GalleryCursorImage::KamuriStaticAtlas;
        let index = Self::ALL.iter().position(|animation| *animation == self);
        CursorAnimation {
            source: CursorImageSource::Path(self.path().to_string()),
            size: self.size(),
            frames: self
                .atlas_layout()
                .textures
                .into_iter()
                .map(|rect| CursorAnimationFrame {
                    rect,
                    duration: self.frame_duration(),
                })
                .collect(),
            hotspot: self.hotspot(),
            fallback: Some(CursorImage {
                source: CursorImageSource::Path(fallback.path().to_string()),
                rect: index.and_then(|index| fallback.atlas_layout().textures.get(index).copied()),
                hotspot: self.hotspot(),
            }),
        }
    }
}

#[cfg(test)]
mod tests;
