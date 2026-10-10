use bevy::prelude::*;
use bevy::ui::VisualBox;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component)]
pub(crate) struct ThemeWindowBackground;

#[derive(Component)]
pub(crate) struct ImageOpacity(pub(crate) f32);

#[derive(Component, Default)]
pub(crate) struct CoverWindowBackground {
    image_size: Option<UVec2>,
}

#[derive(Clone, Debug)]
pub struct WidgetryWindowImageBackground {
    pub image: Handle<Image>,
    pub mode: WidgetryWindowImageMode,
    pub opacity: f32,
}

#[derive(Clone, Debug)]
pub enum WidgetryWindowBackground {
    Theme,
    Image(WidgetryWindowImageBackground),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryWindowImageMode {
    Stretch,
    Cover,
}

pub(crate) fn window_background(background: WidgetryWindowBackground) -> impl Scene {
    let theme = matches!(background, WidgetryWindowBackground::Theme);
    let image = match background {
        WidgetryWindowBackground::Theme => None,
        WidgetryWindowBackground::Image(config) => Some(config),
    };
    bsn! {
        @{theme.then(|| bsn! {
            template(|_| Ok(ThemeWindowBackground))
            template(|context| Ok(BackgroundColor(context.resource::<WidgetryThemeMode>().colors().window.frame.normal.background)))
        })}
        @{image.map(|config| {
            let cover = config.mode == WidgetryWindowImageMode::Cover;
            bsn! {
            template(move |_| Ok(ImageOpacity(config.opacity)))
            @{cover.then(|| bsn! { template(|_| Ok(CoverWindowBackground::default())) })}
            template(move |_| {
                Ok(ImageNode {
                    image: config.image.clone(),
                    image_mode: NodeImageMode::Stretch,
                    color: Color::srgba(1.0, 1.0, 1.0, config.opacity),
                    visual_box: VisualBox::BorderBox,
                    ..default()
                })
            })
        }})}
    }
}

fn cover_rect(image_size: UVec2, node_size: Vec2) -> Option<Rect> {
    if image_size.min_element() == 0 || !node_size.is_finite() || node_size.min_element() <= 0.0 {
        return None;
    }
    let image = image_size.as_dvec2();
    let node = node_size.as_dvec2();
    let scale = (node / image).max_element();
    let crop = (node / scale).min(image);
    let min = (image - crop) * 0.5;
    let rect = Rect {
        min: min.as_vec2(),
        max: (min + crop).as_vec2(),
    };
    (rect.size().min_element() > 0.0).then_some(rect)
}

pub(crate) fn sync_cover_backgrounds(
    images: Res<Assets<Image>>,
    mut roots: Query<(
        Ref<ComputedNode>,
        &mut ImageNode,
        &mut CoverWindowBackground,
    )>,
) {
    for (node, mut image, mut cover) in &mut roots {
        let first_ready = cover.image_size.is_none();
        let image_size = match cover.image_size {
            Some(size) => size,
            None => {
                let Some(asset) = images.get(&image.image) else {
                    continue;
                };
                let size = asset.size();
                cover.image_size = Some(size);
                size
            }
        };
        if (first_ready || node.is_changed())
            && let Some(rect) = cover_rect(image_size, node.size())
            && image.rect != Some(rect)
        {
            image.rect = Some(rect);
        }
    }
}

// 数值 contract 不满足时测试必须失败，因此只在本测试 scope 允许断言与 unwrap。
#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn cover_rect_crops_from_center_and_preserves_target_aspect() {
        for (image, node, expected) in [
            (
                UVec2::new(1000, 800),
                Vec2::new(1920.0, 1080.0),
                Rect::new(0.0, 118.75, 1000.0, 681.25),
            ),
            (
                UVec2::new(1000, 800),
                Vec2::new(400.0, 800.0),
                Rect::new(300.0, 0.0, 700.0, 800.0),
            ),
            (
                UVec2::new(1000, 800),
                Vec2::new(1000.0, 800.0),
                Rect::new(0.0, 0.0, 1000.0, 800.0),
            ),
        ] {
            assert_eq!(cover_rect(image, node), Some(expected));
        }
        for image in [
            UVec2::ONE,
            UVec2::new(4211, 2872),
            UVec2::new(4015, 2796),
            UVec2::splat(u32::MAX),
        ] {
            for node in [
                Vec2::splat(0.001),
                Vec2::new(1920.0, 1080.0),
                Vec2::new(800.0, 1400.0),
                Vec2::splat(f32::MAX),
            ] {
                let rect = cover_rect(image, node).unwrap();
                let size = rect.size();
                assert!(rect.min.cmpge(Vec2::ZERO).all());
                assert!(rect.max.cmple(image.as_vec2()).all());
                assert!((size.x / size.y - node.x / node.y).abs() < 0.001);
                assert!(
                    (rect.center() - image.as_vec2() * 0.5).length()
                        <= image.max_element() as f32 * 1e-6
                );
                assert!(size.x == image.x as f32 || size.y == image.y as f32);
            }
        }
    }

    #[test]
    fn cover_rect_rejects_zero_negative_and_non_finite_dimensions() {
        for image in [UVec2::ZERO, UVec2::new(0, 800), UVec2::new(1000, 0)] {
            assert!(cover_rect(image, Vec2::ONE).is_none());
        }
        for node in [
            Vec2::ZERO,
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::NEG_ONE,
            Vec2::splat(f32::NAN),
            Vec2::splat(f32::INFINITY),
        ] {
            assert!(cover_rect(UVec2::ONE, node).is_none());
        }
    }
}
