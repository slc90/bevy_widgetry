use bevy::{color::Color, ecs::entity::Entity};
use bevy_widgetry::button::{
    LongPressButton, LongPressEvent, LongPressPlugin, StyledButton, StyledButtonPlugin,
};
use bevy_widgetry::combo_box::{ComboBox, ComboBoxPlugin, SetComboBoxSelected};
use bevy_widgetry::style::ForegroundColor;

#[test]
fn facade_public_types_are_usable() {
    let _ = LongPressButton::default();
    let _ = LongPressEvent {
        entity: Entity::PLACEHOLDER,
    };
    let _ = LongPressPlugin;
    let _ = StyledButton;
    let _ = StyledButtonPlugin;
    let _ = ComboBox;
    let _ = ComboBoxPlugin;
    let _ = SetComboBoxSelected {
        entity: Entity::PLACEHOLDER,
        selected: None,
    };
    let _ = ForegroundColor(Color::WHITE);
}
