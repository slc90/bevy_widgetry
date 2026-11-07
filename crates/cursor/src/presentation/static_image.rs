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
    dimensions: HashMap<(AssetId<Image>, UVec2), FailureState<UVec2>>,
    regions: HashMap<CursorImage, FailureState<(UVec2, URect)>>,
    layouts: HashMap<(UVec2, URect, (u16, u16)), Handle<TextureAtlasLayout>>,
}

#[derive(SystemParam)]
pub(super) struct StaticPresentation<'w, 's> {
    server: Option<Res<'w, AssetServer>>,
    images: Option<Res<'w, Assets<Image>>>,
    cache: Local<'s, Cache>,
}

pub(super) struct ImageStatus {
    pub handle: Handle<Image>,
    pub size: UVec2,
    pub ready: bool,
    pub loading: bool,
}

impl StaticPresentation<'_, '_> {
    pub(super) fn preload(&mut self, source: &CursorImageSource) -> Option<Handle<Image>> {
        let server = self.server.as_ref()?;
        self.images.as_ref()?;
        Some(match source {
            CursorImageSource::Handle(handle) => handle.clone(),
            CursorImageSource::Path(path) => self
                .cache
                .paths
                .entry(path.clone())
                .or_insert_with(|| server.load(path.clone()))
                .clone(),
        })
    }

    pub(super) fn status(
        &mut self,
        source: &CursorImageSource,
        expected: Option<UVec2>,
    ) -> Option<ImageStatus> {
        let handle = self.preload(source)?;
        let server = self.server.as_ref()?;
        let images = self.images.as_ref()?;
        let size = images.get(&handle).map_or(UVec2::ZERO, Image::size);
        let mut ready = usable(
            server,
            images,
            &handle,
            size,
            self.cache.images.entry(handle.id()).or_default(),
        );
        if ready && let Some(expected) = expected {
            let dimensions = self
                .cache
                .dimensions
                .entry((handle.id(), expected))
                .or_default();
            if size != expected {
                dimensions.failure(size, || {
                    Arc::new(CursorThemeFailure::new(
                        CursorThemeError::UnexpectedAtlasSize {
                            expected,
                            actual: size,
                        },
                        ErrorFrame::new("cursor.atlas_size")
                            .field("asset", format_args!("{:?}", handle.id()))
                            .field("path", format_args!("{:?}", handle.path()))
                            .field("expected", expected)
                            .field("actual", size),
                    ))
                });
                ready = false;
            } else {
                dimensions.recover(|| {
                    ErrorFrame::new("cursor.atlas_size")
                        .field("asset", format_args!("{:?}", handle.id()))
                });
            }
        }
        let loading =
            !ready && matches!(server.get_load_state(handle.id()), Some(LoadState::Loading));
        Some(ImageStatus {
            handle,
            size,
            ready,
            loading,
        })
    }

    pub(super) fn handled(
        &self,
        handle: &Handle<Image>,
        expected: Option<UVec2>,
        window: Entity,
        outcome: &str,
    ) {
        if let Some(issue) = self.cache.images.get(&handle.id()) {
            handled(issue, window, outcome);
        }
        if let Some(expected) = expected
            && let Some(issue) = self.cache.dimensions.get(&(handle.id(), expected))
        {
            for diagnostic in issue.diagnostics() {
                diagnostic.record_handled(
                    "选择可用的光标回退",
                    &format!("window={window:?}; {outcome}"),
                );
            }
        }
    }

    pub(super) fn map(
        &mut self,
        window: Entity,
        definition: &CursorImage,
        intent: &CursorIntent,
        layouts: Option<&mut Assets<TextureAtlasLayout>>,
        redraw: &mut MessageWriter<RequestRedraw>,
    ) -> CursorIcon {
        let Some(layouts) = layouts else {
            return intent.fallback();
        };
        let Some(status) = self.status(&definition.source, None) else {
            return intent.fallback();
        };
        let ImageStatus {
            handle,
            size,
            ready,
            loading,
        } = status;
        if !ready {
            if loading {
                redraw.write(RequestRedraw);
            }
            self.handled(
                &handle,
                None,
                window,
                &format!("system cursor={:?}", intent.fallback()),
            );
            return intent.fallback();
        }
        let cache = &mut *self.cache;
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
