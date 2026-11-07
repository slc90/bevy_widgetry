use super::diagnostics::{FailureKey, handled, usable};
use crate::{
    CursorImage, CursorImageSource, CursorThemeError, CursorThemeFailure, theme::CursorIntent,
};
use bevy::{
    asset::LoadState,
    ecs::system::SystemParam,
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage, RequestRedraw},
};
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::FailureState};
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
struct Cache {
    paths: HashMap<String, Handle<Image>>,
    images: HashMap<AssetId<Image>, FailureState<FailureKey>>,
    regions: HashMap<CursorImage, FailureState<(UVec2, URect)>>,
    layouts: HashMap<(UVec2, URect, (u16, u16)), Handle<TextureAtlasLayout>>,
}

#[derive(SystemParam)]
pub(super) struct StaticPresentation<'w, 's> {
    server: Option<Res<'w, AssetServer>>,
    images: Option<Res<'w, Assets<Image>>>,
    cache: Local<'s, Cache>,
}

impl StaticPresentation<'_, '_> {
    pub(super) fn map(
        &mut self,
        window: Entity,
        definition: &CursorImage,
        intent: &CursorIntent,
        layouts: Option<&mut Assets<TextureAtlasLayout>>,
        redraw: &mut MessageWriter<RequestRedraw>,
    ) -> CursorIcon {
        let (Some(server), Some(images), Some(layouts)) = (&self.server, &self.images, layouts)
        else {
            return intent.fallback();
        };
        let cache = &mut *self.cache;
        let handle = match &definition.source {
            CursorImageSource::Handle(handle) => handle.clone(),
            CursorImageSource::Path(path) => cache
                .paths
                .entry(path.clone())
                .or_insert_with(|| server.load(path.clone()))
                .clone(),
        };
        let issue = cache.images.entry(handle.id()).or_default();
        let size = images.get(&handle).map_or(UVec2::ZERO, Image::size);
        if !usable(server, images, &handle, size, issue) {
            if matches!(server.get_load_state(handle.id()), Some(LoadState::Loading)) {
                redraw.write(RequestRedraw);
            }
            handled(
                issue,
                window,
                &format!("system cursor={:?}", intent.fallback()),
            );
            return intent.fallback();
        }
        let rect = definition.rect.unwrap_or(URect {
            min: UVec2::ZERO,
            max: size,
        });
        let invalid = if !rect.max.cmpgt(rect.min).all() || rect.max.cmpgt(size).any() {
            Some(CursorThemeError::UnavailableRegion { rect, size })
        } else if u32::from(definition.hotspot.0) >= rect.width()
            || u32::from(definition.hotspot.1) >= rect.height()
        {
            Some(CursorThemeError::InvalidHotspot {
                hotspot: definition.hotspot,
                size: rect.size(),
            })
        } else {
            None
        };
        let region = cache.regions.entry(definition.clone()).or_default();
        if let Some(error) = invalid {
            region.failure((size, rect), || {
                Arc::new(CursorThemeFailure::new(
                    error,
                    ErrorFrame::new("cursor.image_region")
                        .field("asset", format_args!("{:?}", handle.id()))
                        .field("definition", format_args!("{definition:?}"))
                        .field("size", size),
                ))
            });
            for diagnostic in region.diagnostics() {
                diagnostic.record_handled(
                    "选择可用的光标回退",
                    &format!("window={window:?}; system cursor={:?}", intent.fallback()),
                );
            }
            return intent.fallback();
        }
        region.recover(|| {
            ErrorFrame::new("cursor.image_region").field("asset", format_args!("{:?}", handle.id()))
        });
        // Bevy's native cursor cache omits hotspot from its key.
        // Distinct layout identities keep shared pixels with different hotspots independent.
        let layout = cache
            .layouts
            .entry((size, rect, definition.hotspot))
            .or_insert_with(|| {
                layouts.add(TextureAtlasLayout {
                    size,
                    textures: vec![rect],
                })
            });
        if !layouts.contains(layout.id()) {
            *layout = layouts.add(TextureAtlasLayout {
                size,
                textures: vec![rect],
            });
        }
        CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            texture_atlas: Some(TextureAtlas {
                layout: layout.clone(),
                index: 0,
            }),
            hotspot: definition.hotspot,
            ..default()
        }))
    }
}
