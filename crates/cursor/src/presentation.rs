use crate::WidgetryCursorTheme;
use crate::theme::CursorIntent;
use bevy::{
    asset::LoadState,
    ecs::{entity::EntityHashMap, system::SystemParam},
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage, RequestRedraw},
};
use bevy_widgetry_asset::{BuiltinCursorAnimation, BuiltinCursorImage};
use bevy_widgetry_internal_log::{widgetry_info, widgetry_warn};
use std::{collections::HashMap, time::Duration};

#[derive(Default)]
struct CursorImage {
    image: Option<Handle<Image>>,
    layout: Option<Handle<TextureAtlasLayout>>,
    issue: Option<String>,
}

struct Playback {
    animation: BuiltinCursorAnimation,
    entered: Duration,
}

#[derive(Default)]
struct KamuriImages {
    animations: HashMap<BuiltinCursorAnimation, CursorImage>,
    fallback: CursorImage,
}

#[derive(SystemParam)]
pub(crate) struct Presentation<'w, 's> {
    theme: Res<'w, WidgetryCursorTheme>,
    server: Option<Res<'w, AssetServer>>,
    images: Option<Res<'w, Assets<Image>>>,
    layouts: Option<ResMut<'w, Assets<TextureAtlasLayout>>>,
    time: Option<Res<'w, Time<Real>>>,
    redraw: MessageWriter<'w, RequestRedraw>,
    assets: Local<'s, KamuriImages>,
    playing: Local<'s, EntityHashMap<Playback>>,
}

impl Presentation<'_, '_> {
    pub(crate) fn prepare(&mut self) {
        if *self.theme == WidgetryCursorTheme::Kamuri
            && self.images.is_some()
            && let Some(server) = &self.server
        {
            self.assets
                .fallback
                .image
                .get_or_insert_with(|| server.load(BuiltinCursorImage::KamuriStaticAtlas.path()));
        }
    }

    pub(crate) fn retain_windows(&mut self, exists: impl Fn(Entity) -> bool) {
        self.playing.retain(|entity, _| exists(*entity));
    }

    pub(crate) fn stop(&mut self, window: Entity) {
        self.playing.remove(&window);
    }

    pub(crate) fn map(&mut self, window: Entity, intent: CursorIntent, active: bool) -> CursorIcon {
        if *self.theme != WidgetryCursorTheme::Kamuri || !active {
            self.stop(window);
            return intent.fallback();
        }
        let Some(index) = intent.static_index() else {
            self.stop(window);
            return intent.fallback();
        };
        let Some(animation) = BuiltinCursorAnimation::ALL.get(index).copied() else {
            self.stop(window);
            return intent.fallback();
        };
        let (Some(server), Some(images), Some(layouts)) =
            (&self.server, &self.images, &mut self.layouts)
        else {
            self.stop(window);
            return intent.fallback();
        };
        let asset = self.assets.animations.entry(animation).or_default();
        let image = asset
            .image
            .get_or_insert_with(|| server.load(animation.path()));
        if usable(server, images, image, animation.size(), &mut asset.issue)
            && let Some(time) = &self.time
        {
            let layout = asset
                .layout
                .get_or_insert_with(|| layouts.add(animation.atlas_layout()));
            if !layouts.contains(layout.id()) {
                *layout = layouts.add(animation.atlas_layout());
            }
            let now = time.elapsed();
            let playback = self.playing.entry(window).or_insert(Playback {
                animation,
                entered: now,
            });
            if playback.animation != animation {
                *playback = Playback {
                    animation,
                    entered: now,
                };
            }
            let index = animation.frame_at(now.saturating_sub(playback.entered));
            // Reactive desktop apps need a wake while the real mouse remains stationary.
            // Missing or failed images never keep this wake alive.
            self.redraw.write(RequestRedraw);
            return CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle: image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout.clone(),
                    index,
                }),
                hotspot: animation.hotspot(),
                ..default()
            }));
        }
        self.playing.remove(&window);
        let loading = matches!(server.get_load_state(image.id()), Some(LoadState::Loading));
        let fallback = &mut self.assets.fallback;
        // A cold load must keep the reactive app updating until pixels or a failure arrive.
        // Loaded-but-unusable and failed assets must not turn this into an idle redraw loop.
        if loading
            || fallback.image.as_ref().is_some_and(|image| {
                matches!(server.get_load_state(image.id()), Some(LoadState::Loading))
            })
        {
            self.redraw.write(RequestRedraw);
        }
        if let Some(image) = &fallback.image
            && usable(
                server,
                images,
                image,
                UVec2::new(640, 480),
                &mut fallback.issue,
            )
        {
            let layout = fallback.layout.get_or_insert_with(|| {
                layouts.add(BuiltinCursorImage::KamuriStaticAtlas.atlas_layout())
            });
            if !layouts.contains(layout.id()) {
                *layout = layouts.add(BuiltinCursorImage::KamuriStaticAtlas.atlas_layout());
            }
            return CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle: image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout.clone(),
                    index,
                }),
                hotspot: animation.hotspot(),
                ..default()
            }));
        }
        intent.fallback()
    }
}

fn usable(
    server: &AssetServer,
    images: &Assets<Image>,
    handle: &Handle<Image>,
    size: UVec2,
    previous: &mut Option<String>,
) -> bool {
    let issue = if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
        Some(error.to_string())
    } else if let Some(image) = images.get(handle) {
        if image.size() != size
            || !matches!(
                image.texture_descriptor.format,
                bevy::render::render_resource::TextureFormat::Rgba8Unorm
                    | bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
            )
            || image
                .data
                .as_ref()
                .is_none_or(|data| data.len() != (size.x * size.y * 4) as usize)
        {
            Some("Kamuri cursor image must have the expected RGBA8 size and CPU pixels".to_owned())
        } else {
            if previous.take().is_some() {
                widgetry_info!(asset = ?handle.id(), "Kamuri Cursor 素材恢复可用");
            }
            return true;
        }
    } else {
        return false;
    };
    if *previous != issue {
        widgetry_warn!(asset = ?handle.id(), error = ?issue, "Kamuri Cursor 素材不可用，使用 fallback");
        *previous = issue;
    }
    false
}
