const INDICATOR_SIZE: f32 = 18.0;
const INDICATOR_BORDER_WIDTH: f32 = 1.0;
const INDICATOR_CORNER_RADIUS: f32 = 4.0;
const CHECK_ICON_SIZE: f32 = 12.0;

use bevy::prelude::*;
use bevy::ui::{BorderRadius, UiRect};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;

#[derive(Component)]
pub(crate) struct CheckBoxIndicator;

#[derive(Component, Default)]
pub(crate) struct CheckBoxMark {
    pub(crate) icon: Option<BuiltinIcon>,
    pub(crate) color: Option<Color>,
}

pub(crate) fn own_mark(
    event: On<Add<CheckBoxMark>>,
    parents: Query<&ChildOf>,
    roots: Query<
        (),
        Or<(
            With<crate::WidgetryCheckBox>,
            With<crate::WidgetryTriStateCheckbox>,
        )>,
    >,
    mut commands: Commands,
) {
    if let Some(root) = parents
        .iter_ancestors(event.entity)
        .find(|root| roots.contains(*root))
    {
        commands
            .entity(event.entity)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryIcon>::new(root));
    }
}

pub(crate) fn checkbox_indicator_scene() -> impl Scene {
    bsn! {
        template(|_| Ok(CheckBoxIndicator))
        Node {
            width: px(INDICATOR_SIZE), height: px(INDICATOR_SIZE),
            border: UiRect::all(px(INDICATOR_BORDER_WIDTH)),
            border_radius: BorderRadius::all(px(INDICATOR_CORNER_RADIUS)),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor
        BorderColor
        Children [
            @WidgetryIcon {
                @path: {BuiltinIcon::CheckboxCheck.path()},
                @max_size: {Some(UVec2::splat(CHECK_ICON_SIZE as u32))},
            }
            template(|_| Ok(CheckBoxMark::default()))
            Visibility::Hidden
            Node { width: px(CHECK_ICON_SIZE), height: px(CHECK_ICON_SIZE) }
        ]
    }
}
