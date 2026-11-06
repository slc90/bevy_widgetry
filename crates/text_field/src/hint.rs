use bevy::prelude::*;
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_theme::WidgetryTheme;

#[derive(Component, Default, Clone)]
pub(crate) struct HintDisplay;

pub(crate) fn sync_colors(
    mode: Res<WidgetryTheme>,
    mut hints: Query<(&ChildOf, &mut TextColor), With<HintDisplay>>,
    owned: Query<(), With<WidgetryStyleOwner<crate::WidgetryTextField>>>,
) {
    for (parent, mut color) in &mut hints {
        if !owned.contains(parent.parent()) {
            color.set_if_neq(TextColor(mode.text_field.hint_warning));
        }
    }
}

pub(crate) fn apply_color(
    world: &mut World,
    root: Entity,
    color: Color,
) -> Result<(), crate::TextFieldFailure> {
    let hint = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|&entity| world.get::<HintDisplay>(entity).is_some())
    });
    if let Some(hint) = hint {
        world
            .get_mut::<TextColor>(hint)
            .ok_or_else(|| {
                crate::error::missing(root, hint, "TextColor", "text_field.apply_hint_color")
            })?
            .set_if_neq(TextColor(color));
    }
    Ok(())
}
