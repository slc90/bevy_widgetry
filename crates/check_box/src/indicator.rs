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
    event: On<Add, CheckBoxMark>,
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
