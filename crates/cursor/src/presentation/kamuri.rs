use crate::{
    CursorAnimation, CursorAnimationFrame, CursorImage, CursorImageSource, CursorStyle,
    theme::CursorIntent,
};
use bevy_widgetry_asset::{BuiltinCursorAnimation, BuiltinCursorImage};

pub(super) fn fallback_source() -> CursorImageSource {
    CursorImageSource::Path(BuiltinCursorImage::KamuriStaticAtlas.path().to_string())
}

pub(super) fn style(intent: &CursorIntent) -> Option<CursorStyle> {
    let index = intent.static_index()?;
    let animation = *BuiltinCursorAnimation::ALL.get(index)?;
    Some(CursorStyle::Animation(CursorAnimation {
        source: CursorImageSource::Path(animation.path().to_string()),
        size: animation.size(),
        frames: animation
            .atlas_layout()
            .textures
            .into_iter()
            .map(|rect| CursorAnimationFrame {
                rect,
                duration: animation.frame_duration(),
            })
            .collect(),
        hotspot: animation.hotspot(),
        fallback: Some(CursorImage {
            source: fallback_source(),
            rect: BuiltinCursorImage::KamuriStaticAtlas
                .atlas_layout()
                .textures
                .get(index)
                .copied(),
            hotspot: animation.hotspot(),
        }),
    }))
}
