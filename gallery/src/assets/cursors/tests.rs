#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::assets::GalleryAssetPlugin;
use bevy::tasks::block_on;
use std::collections::HashSet;
fn cursor_image(server: &AssetServer, path: bevy::asset::AssetPath<'static>) -> image::RgbaImage {
    let source = server.get_source(path.source()).unwrap();
    let bytes = block_on(async {
        let mut reader = source.reader().read(path.path()).await.unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!(image.color(), image::ColorType::Rgba8);
    image.to_rgba8()
}

#[test]
fn all_cursor_loops_have_80px_frames_fixed_anchors_and_matching_static_fallbacks() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), GalleryAssetPlugin));
    let server = app.world().resource::<AssetServer>();
    let fallback = cursor_image(server, GalleryCursorImage::KamuriStaticAtlas.path());
    let static_layout = GalleryCursorImage::KamuriStaticAtlas.atlas_layout();
    let mut identities = HashSet::new();
    let mut paths = HashSet::new();
    for (index, animation) in GalleryCursorAnimation::ALL.into_iter().enumerate() {
        assert!(paths.insert(animation.path()));
        let atlas = cursor_image(server, animation.path());
        assert_eq!(atlas.dimensions(), (animation.size().x, animation.size().y));
        let layout = animation.atlas_layout();
        assert_eq!(layout.textures.len(), animation.frame_count());
        let (hx, hy) = animation.hotspot();
        assert!(hx > 0 && hy > 0 && hx < 79 && hy < 79);
        let mut frames = HashSet::new();
        let mut anchor = None;
        for (frame_index, rect) in layout.textures.iter().enumerate() {
            assert_eq!(rect.size(), UVec2::splat(80));
            assert!(rect.max.x <= atlas.width() && rect.max.y <= atlas.height());
            let frame =
                image::imageops::crop_imm(&atlas, rect.min.x, rect.min.y, 80, 80).to_image();
            assert!(frame.pixels().any(|p| p[3] == 255));
            assert!(frame.pixels().all(|p| p[3] != 0 || p.0 == [0; 4]));
            assert!((0..80).all(|p| frame.get_pixel(p, 0)[3] == 0
                && frame.get_pixel(p, 79)[3] == 0
                && frame.get_pixel(0, p)[3] == 0
                && frame.get_pixel(79, p)[3] == 0));
            let patch =
                image::imageops::crop_imm(&frame, u32::from(hx) - 1, u32::from(hy) - 1, 3, 3)
                    .to_image();
            if let Some(previous) = &anchor {
                assert_eq!(previous, &patch, "{animation:?} anchor drift");
            } else {
                anchor = Some(patch);
            }
            if frame_index == 0 {
                let fallback_rect = static_layout.textures[index];
                assert_eq!(fallback_rect.size(), UVec2::splat(80));
                assert_eq!(
                    frame,
                    image::imageops::crop_imm(
                        &fallback,
                        fallback_rect.min.x,
                        fallback_rect.min.y,
                        80,
                        80
                    )
                    .to_image()
                );
                assert!(
                    identities.insert(frame.clone().into_raw()),
                    "{animation:?} shares another semantic's artwork"
                );
            }
            frames.insert(frame.into_raw());
        }
        assert!(frames.len() > 3, "{animation:?} must actually animate");
    }
}

#[test]
fn cursor_timing_wraps_at_each_independent_duration_without_overflow() {
    use std::time::Duration;
    let mut periods = HashSet::new();
    for animation in GalleryCursorAnimation::ALL {
        let period = animation.frame_duration();
        assert!(!period.is_zero());
        assert!(animation.frame_count() > 1);
        periods.insert(period);
        assert_eq!(animation.definition().frame_at(Duration::ZERO), Some(0));
        assert_eq!(
            animation
                .definition()
                .frame_at(period - Duration::from_nanos(1)),
            Some(0)
        );
        assert_eq!(animation.definition().frame_at(period), Some(1));
        assert_eq!(
            animation
                .definition()
                .frame_at(period * animation.frame_count() as u32),
            Some(0)
        );
        assert!(animation.definition().frame_at(Duration::MAX).unwrap() < animation.frame_count());
    }
    assert!(periods.len() > 1);
}
