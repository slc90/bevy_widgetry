mod animated;
mod diagnostics;
mod static_image;

use crate::{CursorSemantic, CursorStyle, WidgetryCursorTheme, theme::CursorIntent};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    window::{CursorIcon, RequestRedraw},
};

#[derive(SystemParam)]
pub(crate) struct Presentation<'w, 's> {
    theme: Res<'w, WidgetryCursorTheme>,
    layouts: Option<ResMut<'w, Assets<TextureAtlasLayout>>>,
    images: static_image::StaticPresentation<'w, 's>,
    redraw: MessageWriter<'w, RequestRedraw>,
    animated: animated::AnimatedPresentation<'w, 's>,
}

impl Presentation<'_, '_> {
    pub(crate) fn retain_windows(&mut self, exists: impl Fn(Entity) -> bool) {
        self.animated.retain_windows(exists);
    }
    pub(crate) fn stop(&mut self, window: Entity) {
        self.animated.stop(window);
    }
    pub(crate) fn map(&mut self, window: Entity, intent: CursorIntent, active: bool) -> CursorIcon {
        let semantic = match &intent {
            CursorIntent::System(icon) => CursorSemantic::System(*icon),
            CursorIntent::Hint(hint) => CursorSemantic::Hint(*hint),
            CursorIntent::Custom(_) => {
                self.stop(window);
                return intent.fallback();
            }
        };
        if !active {
            self.stop(window);
            return intent.fallback();
        }
        let style = self.theme.styles().get(&semantic).cloned();
        if let Some(style) = style {
            return match style {
                CursorStyle::System(icon) => {
                    self.stop(window);
                    CursorIcon::System(icon)
                }
                CursorStyle::Image(image) => {
                    self.stop(window);
                    self.images.map(
                        window,
                        &image,
                        &intent,
                        self.layouts.as_deref_mut(),
                        &mut self.redraw,
                    )
                }
                CursorStyle::Animation(animation) => self.animated.map(
                    window,
                    semantic,
                    &animation,
                    &intent,
                    &mut self.images,
                    self.layouts.as_deref_mut(),
                    &mut self.redraw,
                ),
            };
        }
        self.stop(window);
        intent.fallback()
    }
}
