use crate::*;
use bevy::prelude::*;
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTheme {
    pub text: WidgetryTextColors,
    pub icon: WidgetryIconColors,
    pub button: WidgetryButtonColors,
    pub check_box: WidgetryCheckBoxColors,
    pub radio_group: WidgetryRadioGroupColors,
    pub text_field: WidgetryTextFieldColors,
    pub combo_box: WidgetryComboBoxColors,
    pub scroll_area: WidgetryScrollAreaColors,
    pub list_view: WidgetryListViewColors,
    pub tree: WidgetryTreeColors,
    pub table: WidgetryTableColors,
    pub tooltip: WidgetryTooltipColors,
    pub window: WidgetryWindowColors,
    pub message_box: WidgetryMessageBoxColors,
    pub file_dialog: WidgetryFileDialogColors,
    pub waveform: WidgetryWaveformColors,
}

pub const WIDGETRY_LIGHT_THEME: WidgetryTheme = WidgetryTheme {
    text: crate::text::LIGHT,
    icon: crate::icon::LIGHT,
    button: crate::button::LIGHT,
    check_box: crate::check_box::LIGHT,
    radio_group: crate::radio_group::LIGHT,
    text_field: crate::text_field::LIGHT,
    combo_box: crate::combo_box::LIGHT,
    scroll_area: crate::scroll_area::LIGHT,
    list_view: crate::list_view::LIGHT,
    tree: crate::tree::LIGHT,
    table: crate::table::LIGHT,
    tooltip: crate::tooltip::LIGHT,
    window: crate::window::LIGHT,
    message_box: crate::message_box::LIGHT,
    file_dialog: crate::file_dialog::LIGHT,
    waveform: crate::waveform::LIGHT,
};

pub const WIDGETRY_DARK_THEME: WidgetryTheme = WidgetryTheme {
    text: crate::text::DARK,
    icon: crate::icon::DARK,
    button: crate::button::DARK,
    check_box: crate::check_box::DARK,
    radio_group: crate::radio_group::DARK,
    text_field: crate::text_field::DARK,
    combo_box: crate::combo_box::DARK,
    scroll_area: crate::scroll_area::DARK,
    list_view: crate::list_view::DARK,
    tree: crate::tree::DARK,
    table: crate::table::DARK,
    tooltip: crate::tooltip::DARK,
    window: crate::window::DARK,
    message_box: crate::message_box::DARK,
    file_dialog: crate::file_dialog::DARK,
    waveform: crate::waveform::DARK,
};

#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryThemeChanged {
    pub mode: WidgetryThemeMode,
}

pub struct WidgetryThemePlugin;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryThemeMode {
    Light,
    #[default]
    Dark,
}

impl WidgetryThemeMode {
    pub const fn colors(self) -> &'static WidgetryTheme {
        match self {
            Self::Light => &WIDGETRY_LIGHT_THEME,
            Self::Dark => &WIDGETRY_DARK_THEME,
        }
    }

    pub fn set_in_world(world: &mut World, mode: Self) -> Result<bool, BevyError> {
        let Some(mut current) = world.get_resource_mut::<Self>() else {
            widgetry_error!(
                ?mode,
                "主题切换缺少 WidgetryThemeMode，请先注册 WidgetryThemePlugin"
            );
            return Err(BevyError::error(
                "WidgetryThemeMode requires WidgetryThemePlugin",
            ));
        };
        if *current == mode {
            return Ok(false);
        }
        *current = mode;
        world.trigger(WidgetryThemeChanged { mode });
        world.flush();
        Ok(true)
    }

    pub fn set(commands: &mut Commands, mode: Self) {
        commands.queue(move |world: &mut World| Self::set_in_world(world, mode).map(|_| ()));
    }
}

impl Plugin for WidgetryThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WidgetryThemeMode>();
        widgetry_info!("WidgetryThemePlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Notifications(Vec<WidgetryThemeMode>);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(WidgetryThemePlugin)
            .init_resource::<Notifications>()
            .add_observer(
                |event: On<WidgetryThemeChanged>,
                 mode: Res<WidgetryThemeMode>,
                 mut seen: ResMut<Notifications>| {
                    assert_eq!(*mode, event.mode);
                    seen.0.push(event.mode);
                },
            );
        app
    }

    #[test]
    fn modes_resolve_complete_fixed_palettes() {
        assert_eq!(WidgetryThemeMode::default(), WidgetryThemeMode::Dark);
        assert_eq!(WidgetryThemeMode::Light.colors(), &WIDGETRY_LIGHT_THEME);
        assert_eq!(WidgetryThemeMode::Dark.colors(), &WIDGETRY_DARK_THEME);
    }

    #[test]
    fn normal_text_contrast_matches_reference_budget() {
        for theme in [&WIDGETRY_LIGHT_THEME, &WIDGETRY_DARK_THEME] {
            let text = theme.text.normal.foreground;
            for background in [
                theme.window.frame.normal.background,
                theme.button.normal.background,
                theme.button.hovered.background,
                theme.button.pressed.background,
                theme.combo_box.popup.normal.background,
                theme.list_view.item.selected.background,
                theme.text_field.editable.normal.selection_background,
                theme
                    .text_field
                    .editable
                    .normal
                    .unfocused_selection_background,
                theme.table.column_header.normal.background,
                theme.message_box.body.normal.background,
            ] {
                assert!(contrast(text, background) >= 4.5);
            }
            assert!(
                contrast(
                    theme.tooltip.popup.normal.foreground,
                    theme.tooltip.popup.normal.background
                ) >= 4.5
            );
        }
    }

    fn contrast(foreground: Color, background: Color) -> f32 {
        let luminance = |color: Color| {
            let color = color.to_linear();
            0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue
        };
        let foreground = luminance(foreground);
        let background = luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    #[test]
    fn plugin_preserves_initial_mode() {
        let mut app = App::new();
        app.insert_resource(WidgetryThemeMode::Light)
            .add_plugins(WidgetryThemePlugin);
        assert_eq!(
            *app.world().resource::<WidgetryThemeMode>(),
            WidgetryThemeMode::Light
        );
    }

    #[test]
    fn switching_commits_before_notification_and_same_value_is_noop() -> Result<(), BevyError> {
        let mut app = app();
        assert!(!WidgetryThemeMode::set_in_world(
            app.world_mut(),
            WidgetryThemeMode::Dark
        )?);
        assert!(WidgetryThemeMode::set_in_world(
            app.world_mut(),
            WidgetryThemeMode::Light
        )?);
        assert!(!WidgetryThemeMode::set_in_world(
            app.world_mut(),
            WidgetryThemeMode::Light
        )?);
        assert!(WidgetryThemeMode::set_in_world(
            app.world_mut(),
            WidgetryThemeMode::Dark
        )?);
        assert_eq!(
            app.world().resource::<Notifications>().0,
            [WidgetryThemeMode::Light, WidgetryThemeMode::Dark]
        );
        Ok(())
    }

    #[test]
    fn commands_validate_and_commit_at_execution_time() {
        let mut app = app();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetryThemeMode::set(&mut commands, WidgetryThemeMode::Light);
        WidgetryThemeMode::set(&mut commands, WidgetryThemeMode::Light);
        WidgetryThemeMode::set(&mut commands, WidgetryThemeMode::Dark);
        assert_eq!(
            *app.world().resource::<WidgetryThemeMode>(),
            WidgetryThemeMode::Dark
        );
        assert!(app.world().resource::<Notifications>().0.is_empty());
        queue.apply(app.world_mut());
        assert_eq!(
            app.world().resource::<Notifications>().0,
            [WidgetryThemeMode::Light, WidgetryThemeMode::Dark]
        );
    }

    #[test]
    fn missing_mode_is_error_without_notification() {
        let mut app = app();
        app.world_mut().remove_resource::<WidgetryThemeMode>();
        let error = WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light)
            .expect_err("missing mode must fail");
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("WidgetryThemeMode"));
        assert!(app.world().resource::<Notifications>().0.is_empty());
        assert!(!app.world().contains_resource::<WidgetryThemeMode>());
    }
}
