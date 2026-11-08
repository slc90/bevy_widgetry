const ITEM_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 12.0;

use crate::i18n::{ButtonKey, TextKey, localized};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::toggle_button::WidgetryToggleButtonHeadless;
use bevy_widgetry::{
    button::WidgetryButton,
    text::WidgetryText,
    toggle_button::{WidgetryToggleButton, WidgetryToggleButtonChanged},
};

#[derive(Component)]
struct ToggleExample;

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::ToggleDescription)) WidgetryText--
            Node { column_gap: px(CONTROL_GAP), align_items: AlignItems::Center }
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
    info!(entity=?event.entity, selected, "切换按钮状态已提交");
    Ok(())
}

fn set_selected(
    _: On<Activate>,
    targets: Query<Entity, With<ToggleExample>>,
    mut commands: Commands,
) {
    for entity in &targets {
        commands.queue(move |world: &mut World| -> Result {
            let selected = WidgetryToggleButtonHeadless::selected(world, entity)
                .map_err(|error| error.into_bevy_error())?;
            WidgetryToggleButtonHeadless::set_selected_in_world(world, entity, !selected)
                .map_err(|error| error.into_bevy_error())?;
            Ok(())
        });
    }
}
