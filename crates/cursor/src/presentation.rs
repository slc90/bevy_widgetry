mod diagnostics;
#[cfg(feature = "kamuri_cursor")]
mod kamuri;
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
    #[cfg(feature = "kamuri_cursor")]
    kamuri: kamuri::KamuriPresentation<'w, 's>,
}

impl Presentation<'_, '_> {
    pub(crate) fn prepare(&mut self) {
        #[cfg(feature = "kamuri_cursor")]
        self.kamuri.prepare();
    }
    pub(crate) fn retain_windows(&mut self, _exists: impl Fn(Entity) -> bool) {
        #[cfg(feature = "kamuri_cursor")]
        self.kamuri.retain_windows(_exists);
    }
    pub(crate) fn stop(&mut self, _window: Entity) {
        #[cfg(feature = "kamuri_cursor")]
        self.kamuri.stop(_window);
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
        if let Some(style) = self.theme.styles().get(&semantic).cloned() {
            self.stop(window);
            return match style {
                CursorStyle::System(icon) => CursorIcon::System(icon),
                CursorStyle::Image(image) => self.images.map(
                    window,
                    &image,
                    &intent,
                    self.layouts.as_deref_mut(),
                    &mut self.redraw,
                ),
            };
        }
        #[cfg(feature = "kamuri_cursor")]
        {
            self.kamuri.map(
                window,
                intent,
                active,
                self.layouts.as_deref_mut(),
                &mut self.redraw,
            )
        }
        #[cfg(not(feature = "kamuri_cursor"))]
        {
            intent.fallback()
        }
    }
}
