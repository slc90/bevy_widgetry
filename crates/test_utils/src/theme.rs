use bevy_widgetry_theme::WidgetryThemeMode;

use bevy::app::App;
pub fn switch_theme(app: &mut App, mode: WidgetryThemeMode) {
    WidgetryThemeMode::set(&mut app.world_mut().commands(), mode);
    app.world_mut().flush();
}
