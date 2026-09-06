use bevy::{color::Color, ecs::entity::Entity};

use bevy_widgetry::button::{
    LongPressButton, LongPressEvent, LongPressPlugin, StyledButton, StyledButtonPlugin,
};
use bevy_widgetry::combo_box::{ComboBox, ComboBoxPlugin, SetComboBoxSelected};
use bevy_widgetry::style::{
    ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
};

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
        selected: 1,
    };
    let _ = ForegroundColor(Color::WHITE);
}

#[test]
fn style_theme_api_and_plugins_work_together() {
    use bevy::app::App;
    use bevy_widgetry::combo_box::StyledComboBoxPlugin;
    let _: &ColorTheme = &DARK_THEME;
    assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    let mut app = App::new();
    app.insert_resource(ThemeMode::Light).add_plugins((
        ThemePlugin,
        StyledButtonPlugin,
        StyledComboBoxPlugin,
    ));
    app.world_mut().trigger(ThemeChanged {
        mode: ThemeMode::Light,
    });
    app.update();
    assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
}
