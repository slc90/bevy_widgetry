use super::static_image::StaticPresentation;
use crate::{CursorAnimation, CursorSemantic, theme::CursorIntent};
use bevy::{
    ecs::{entity::EntityHashMap, system::SystemParam},
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage, RequestRedraw},
};
use std::{collections::HashMap, time::Duration};

struct Playback {
    semantic: CursorSemantic,
    animation: CursorAnimation,
    entered: Duration,
}

#[derive(SystemParam)]
pub(super) struct AnimatedPresentation<'w, 's> {
    time: Option<Res<'w, Time<Real>>>,
    playing: Local<'s, EntityHashMap<Playback>>,
    layouts: Local<'s, HashMap<(UVec2, Vec<URect>, (u16, u16)), Handle<TextureAtlasLayout>>>,
}

impl AnimatedPresentation<'_, '_> {
    pub(super) fn retain_windows(&mut self, exists: impl Fn(Entity) -> bool) {
        self.playing.retain(|entity, _| exists(*entity));
    }

    pub(super) fn stop(&mut self, window: Entity) {
        self.playing.remove(&window);
    }

    pub(super) fn map(
        &mut self,
        window: Entity,
        semantic: CursorSemantic,
        animation: &CursorAnimation,
        intent: &CursorIntent,
        images: &mut StaticPresentation,
        layouts: Option<&mut Assets<TextureAtlasLayout>>,
        redraw: &mut MessageWriter<RequestRedraw>,
    ) -> CursorIcon {
        let status = images.status(&animation.source, Some(animation.size));
        if let (Some(status), Some(available_layouts), Some(time)) =
            (&status, layouts.as_ref(), &self.time)
            && status.ready
        {
            let now = time.elapsed();
            let playback = self.playing.entry(window).or_insert_with(|| Playback {
                semantic,
                animation: animation.clone(),
                entered: now,
            });
            if playback.semantic != semantic || playback.animation != *animation {
                *playback = Playback {
                    semantic,
                    animation: animation.clone(),
                    entered: now,
                };
            }
            if let Some(index) = animation.frame_at(now.saturating_sub(playback.entered)) {
                let definition = animation.atlas_layout();
                let key = (definition.size, definition.textures, animation.hotspot);
                let existing = self
                    .layouts
                    .get(&key)
                    .filter(|handle| available_layouts.contains(handle.id()))
                    .cloned();
                let layout = if let Some(existing) = existing {
                    existing
                } else {
                    // Hotspot is absent from Bevy's native cursor key, so it needs its own layout identity.
                    let Some(layouts) = layouts else {
                        return intent.fallback();
                    };
                    let handle = layouts.add(animation.atlas_layout());
                    self.layouts.insert(key, handle.clone());
                    handle
                };
                redraw.write(RequestRedraw);
                return CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                    handle: status.handle.clone(),
                    texture_atlas: Some(TextureAtlas { layout, index }),
                    hotspot: animation.hotspot,
                    ..default()
                }));
            }
        }
        self.stop(window);
        if status.as_ref().is_some_and(|status| status.loading) {
            redraw.write(RequestRedraw);
        }
        let next = if let Some(fallback) = &animation.fallback {
            images.map(window, fallback, intent, layouts, redraw)
        } else {
            intent.fallback()
        };
        if let Some(status) = status {
            let outcome = match &next {
                CursorIcon::Custom(CustomCursor::Image(image)) => format!(
                    "static atlas={:?}, region={:?}",
                    image.handle.id(),
                    animation.fallback.as_ref().and_then(|image| image.rect)
                ),
                _ => format!("system cursor={next:?}"),
            };
            images.handled(&status.handle, Some(animation.size), window, &outcome);
        }
        next
    }
}
