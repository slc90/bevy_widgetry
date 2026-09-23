use bevy::prelude::*;
use bevy::ui::{BorderRadius, UiRect};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;

/// 标识共享 indicator，供 style system 查找固定视觉实体。
#[derive(Component)]
pub(crate) struct CheckBoxIndicator;

/// 标识唯一 mark，并缓存上次可见 SVG 语义，避免重复加载。
#[derive(Component, Default)]
pub(crate) struct CheckBoxMark {
    /// 上一次实际请求的 SVG。
    pub(crate) icon: Option<BuiltinIcon>,
    /// 上一次实际写入的 icon color。
    pub(crate) color: Option<Color>,
}

/// 只抽取结构，indicator 不需要额外 SceneComponent 身份。
pub(crate) fn checkbox_indicator_scene() -> impl Scene {
    bsn! {
        template(|_| Ok(CheckBoxIndicator))
        Node {
            width: px(18), height: px(18),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor
        BorderColor
        Children [(
            @WidgetryIcon {
                @path: {BuiltinIcon::CheckboxCheck.path()},
                @max_size: {Some(UVec2::new(12, 12))},
            }
            template(|_| Ok(CheckBoxMark::default()))
            Visibility::Hidden
            Node { width: px(12), height: px(12) }
        )]
    }
}
