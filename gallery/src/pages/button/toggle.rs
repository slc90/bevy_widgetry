use crate::i18n::{ButtonKey, TextKey, localized};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{
    button::WidgetryButton,
    text::WidgetryText,
    toggle_button::{WidgetryToggleButton, WidgetryToggleButtonChanged},
};

#[derive(Component)]
struct ToggleExample;

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::ToggleDescription)) WidgetryText--
            Node { column_gap: px(12), align_items: AlignItems::Center }
            Children [
                @WidgetryToggleButton
                Name("ToggleButtonDemo") template(|_| Ok(ToggleExample))
                on(changed)
                Children [@localized(TextKey::Button(ButtonKey::ToggleButton)) WidgetryText]--
                @WidgetryToggleButton { @selected: true }
                Name("ToggleButtonDisabledDemo") InteractionDisabled
                Children [@localized(TextKey::Common(crate::i18n::CommonKey::Disabled)) WidgetryText]--
                @WidgetryButton
                Name("ToggleButtonProgrammatic") on(set_selected)
                Children [@localized(TextKey::Button(ButtonKey::ToggleProgrammatic)) WidgetryText]
            ]
        ]
    }
}

fn changed(
    event: On<WidgetryToggleButtonChanged>,
    checked: Query<Has<bevy::ui::Checked>>,
) -> Result {
    let selected = checked.get(event.entity)?;
    info!(entity=?event.entity, selected, "Toggle Button selection committed");
    Ok(())
}

fn set_selected(
    _: On<Activate>,
    targets: Query<Entity, With<ToggleExample>>,
    mut commands: Commands,
) {
    for entity in &targets {
        commands.queue(move |world: &mut World| -> Result {
            let selected = WidgetryToggleButton::selected(world, entity)?;
            WidgetryToggleButton::set_selected_in_world(world, entity, !selected)?;
            Ok(())
        });
    }
}
