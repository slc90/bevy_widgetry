use crate::{
    CursorImage, CursorImageSource, CursorThemeError,
    theme::{validate_hotspot, validate_image, validate_source},
};
use bevy::prelude::*;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct CursorAnimationFrame {
    pub rect: URect,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct CursorAnimation {
    pub source: CursorImageSource,
    pub size: UVec2,
    pub frames: Vec<CursorAnimationFrame>,
    pub hotspot: (u16, u16),
    pub fallback: Option<CursorImage>,
}

impl CursorAnimation {
    pub fn frame_at(&self, elapsed: Duration) -> Option<usize> {
        let total = self.frames.iter().try_fold(Duration::ZERO, |sum, frame| {
            if frame.duration.is_zero() {
                return None;
            }
            sum.checked_add(frame.duration)
        })?;
        if total.is_zero() {
            return None;
        }
        let mut phase = elapsed.as_nanos() % total.as_nanos();
        self.frames.iter().position(|frame| {
            if phase < frame.duration.as_nanos() {
                true
            } else {
                phase -= frame.duration.as_nanos();
                false
            }
        })
    }

    pub(crate) fn validate(&self) -> Result<(), CursorThemeError> {
        validate_source(&self.source)?;
        if !self.size.cmpgt(UVec2::ZERO).all() {
            return Err(CursorThemeError::InvalidAtlasSize { size: self.size });
        }
        let first = self
            .frames
            .first()
            .ok_or(CursorThemeError::EmptyAnimation)?;
        let mut total = Duration::ZERO;
        for (index, frame) in self.frames.iter().enumerate() {
            if frame.duration.is_zero() {
                return Err(CursorThemeError::InvalidFrameDuration { frame: index });
            }
            total = total
                .checked_add(frame.duration)
                .ok_or(CursorThemeError::AnimationDurationOverflow)?;
            let rect = frame.rect;
            if !rect.max.cmpgt(rect.min).all() {
                return Err(CursorThemeError::InvalidRegion { rect });
            }
            if rect.max.cmpgt(self.size).any() {
                return Err(CursorThemeError::UnavailableRegion {
                    rect,
                    size: self.size,
                });
            }
            if rect.size() != first.rect.size() {
                return Err(CursorThemeError::InconsistentFrameSize {
                    frame: index,
                    expected: first.rect.size(),
                    actual: rect.size(),
                });
            }
            validate_hotspot(self.hotspot, rect.size())?;
        }
        if let Some(image) = &self.fallback {
            validate_image(image)?;
        }
        Ok(())
    }

    pub fn atlas_layout(&self) -> TextureAtlasLayout {
        TextureAtlasLayout {
            size: self.size,
            textures: self.frames.iter().map(|frame| frame.rect).collect(),
        }
    }
}
