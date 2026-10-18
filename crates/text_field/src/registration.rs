use crate::{auto_size, composition, count, hint, input, length, style};
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
        app.init_resource::<bevy::text::FontCx>()
            .init_resource::<bevy::text::LayoutCx>();
        app.add_observer(input::focus_text_field_on_press);
        app.add_observer(input::retain_text_field_focus_on_acquire);
        app.add_observer(input::contain_slot_focus);
        app.add_observer(input::redirect_gained_focus);
        app.add_observer(input::navigate_from_editor);
        app.add_systems(
            PreUpdate,
            input::redirect_root_focus.before(bevy::input_focus::InputFocusSystems::Dispatch),
        );
        app.add_systems(
            PostUpdate,
            (
                composition::configure_slots,
                composition::project_text_configuration,
            )
                .chain()
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Materialize)
                .before(bevy::input_focus::InputFocusSystems::FocusChangeEvents),
        );
        app.add_systems(
            PostUpdate,
            input::redirect_root_focus
                .after(bevy_widgetry_core::ui::WidgetryUiSystems::Build)
                .before(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        app.add_systems(
            PostUpdate,
            (
                length::initialize_limits,
                input::block_disabled_text_field_edits,
                input::block_read_only_text_field_edits,
                length::apply_limited_edits,
            )
                .chain()
                .after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                .before(EditableTextSystems),
        );

        app.add_systems(
            PostUpdate,
            (auto_size::resize_editors, count::sync_count)
                .chain()
                .after(EditableTextSystems)
                .before(bevy::ui::UiSystems::PostLayout),
        );
        app.add_systems(
            PostUpdate,
            (
                hint::sync_colors,
                style::update_widgetry_text_field_style_changed,
                style::update_widgetry_text_field_style_focus_changed,
                style::update_widgetry_text_field_style_removed,
            )
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        widgetry_info!("WidgetryTextFieldPlugin 注册完成");
    }
}
