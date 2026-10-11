use crate::WidgetryCursorTheme;
use crate::theme::CursorIntent;
use bevy::{
    asset::LoadState,
    ecs::{entity::EntityHashMap, system::SystemParam},
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage, RequestRedraw, SystemCursorIcon},
};
use bevy_widgetry_asset::BuiltinCursorImage;
use bevy_widgetry_log::{widgetry_info, widgetry_warn};
use std::time::Duration;

const FRAME_PERIOD: Duration = Duration::from_millis(125);
const FRAME_COUNT: u128 = 12;
const HOTSPOT: (u16, u16) = (22, 54);
// 与静态图集及 CursorIntent 的语义顺序一致，坐标落在各款图像的功能锚点。
const STATIC_HOTSPOTS: [(u16, u16); 44] = [
    (7, 19),
    (22, 54),
    (7, 17),
    (7, 19),
    (13, 28),
    (24, 32),
    (22, 34),
    (14, 35),
    (22, 30),
    (15, 35),
    (13, 33),
    (23, 35),
    (15, 28),
    (23, 36),
    (45, 33),
    (14, 24),
    (45, 20),
    (7, 21),
    (13, 44),
    (30, 43),
    (4, 42),
    (4, 34),
    (14, 34),
    (15, 35),
    (14, 34),
    (16, 33),
    (15, 36),
    (21, 37),
    (21, 33),
    (16, 32),
    (16, 32),
    (10, 24),
    (19, 35),
    (12, 24),
    (13, 34),
    (13, 34),
    (21, 36),
    (14, 29),
    (22, 37),
    (10, 37),
    (22, 35),
    (12, 27),
    (12, 24),
    (12, 30),
];

#[derive(Default)]
struct KamuriImages {
    atlas: Option<Handle<Image>>,
    fallback: Option<Handle<Image>>,
    layout: Option<Handle<TextureAtlasLayout>>,
    static_layout: Option<Handle<TextureAtlasLayout>>,
    atlas_issue: Option<String>,
    fallback_issue: Option<String>,
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
    playing: Local<'s, EntityHashMap<Duration>>,
}

impl Presentation<'_, '_> {
    pub(crate) fn prepare(&mut self) {
        if *self.theme == WidgetryCursorTheme::Kamuri
            && self.images.is_some()
            && let Some(server) = &self.server
        {
            self.assets
                .atlas
                .get_or_insert_with(|| server.load(BuiltinCursorImage::KamuriPointerAtlas.path()));
            self.assets
                .fallback
                .get_or_insert_with(|| server.load(BuiltinCursorImage::KamuriStaticAtlas.path()));
        }
    }

    pub(crate) fn retain_windows(&mut self, exists: impl Fn(Entity) -> bool) {
        self.playing.retain(|entity, _| exists(*entity));
    }

    pub(crate) fn stop(&mut self, window: Entity) {
        self.playing.remove(&window);
    }

    pub(crate) fn map(
        &mut self,
        window: Entity,
        intent: CursorIntent,
        focused: bool,
    ) -> CursorIcon {
        if *self.theme != WidgetryCursorTheme::Kamuri || !focused {
            self.stop(window);
            return intent.fallback();
        }
        let Some(static_index) = intent.static_index() else {
            self.stop(window);
            return intent.fallback();
        };
        let pointer = intent == CursorIntent::System(SystemCursorIcon::Pointer);
        if !pointer {
            self.stop(window);
        }
        let (Some(server), Some(images)) = (&self.server, &self.images) else {
            self.stop(window);
            return intent.fallback();
        };
        let assets = &mut *self.assets;
        let (Some(atlas), Some(fallback)) = (&assets.atlas, &assets.fallback) else {
            self.playing.remove(&window);
            return intent.fallback();
        };
        let atlas_ready = pointer
            && usable(
                server,
                images,
                atlas,
                UVec2::new(256, 192),
                &mut assets.atlas_issue,
            );
        let fallback_ready = usable(
            server,
            images,
            fallback,
            UVec2::new(768, 576),
            &mut assets.fallback_issue,
        );
        if atlas_ready && let (Some(layouts), Some(time)) = (&mut self.layouts, &self.time) {
            let layout = assets.layout.get_or_insert_with(|| {
                layouts.add(BuiltinCursorImage::KamuriPointerAtlas.atlas_layout())
            });
            if layouts.contains(layout.id()) {
                let now = time.elapsed();
                let entered = self.playing.entry(window).or_insert(now);
                let index = (now.saturating_sub(*entered).as_nanos() / FRAME_PERIOD.as_nanos()
                    % FRAME_COUNT) as usize;
                // desktop_app 在静止鼠标时不会自行推进 Time。
                // 只在有焦点的活跃动画中请求后续 redraw，实际 CursorIcon 仍按 125ms 切帧。
                self.redraw.write(RequestRedraw);
                return CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                    handle: atlas.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: layout.clone(),
                        index,
                    }),
                    hotspot: HOTSPOT,
                    ..default()
                }));
            }
        }
        self.playing.remove(&window);
        if fallback_ready && let Some(layouts) = &mut self.layouts {
            let layout = assets.static_layout.get_or_insert_with(|| {
                layouts.add(BuiltinCursorImage::KamuriStaticAtlas.atlas_layout())
            });
            if layouts.contains(layout.id()) {
                return CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                    handle: fallback.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: layout.clone(),
                        index: static_index,
                    }),
                    hotspot: STATIC_HOTSPOTS[static_index],
                    ..default()
                }));
            }
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
