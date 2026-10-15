use crate::{input, style};
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::text::EditableTextSystems;
use bevy_widgetry_internal_log::widgetry_info;
use bevy_widgetry_theme::WidgetryThemePlugin;

pub struct WidgetryTextFieldPlugin;

impl Plugin for WidgetryTextFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }

        app.add_observer(style::refresh_text_field_theme);
        app.add_observer(input::focus_text_field_on_press);
        app.add_observer(input::retain_text_field_focus_on_acquire);
        app.add_systems(
            PostUpdate,
            (
                input::block_disabled_text_field_edits,
                input::block_read_only_text_field_edits,
            )
                .after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                .before(EditableTextSystems),
        );

        app.add_systems(
            PostUpdate,
            (
                style::update_widgetry_text_field_style_changed,
                style::update_widgetry_text_field_style_focus_changed,
                style::update_widgetry_text_field_style_removed,
            )
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        widgetry_info!("WidgetryTextFieldPlugin 注册完成");
    }
}
