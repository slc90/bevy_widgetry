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

pub const WIDGETRY_PINK_DREAM_THEME: WidgetryTheme = WidgetryTheme {
    text: crate::text::PINK_DREAM,
    icon: crate::icon::PINK_DREAM,
    button: crate::button::PINK_DREAM,
    check_box: crate::check_box::PINK_DREAM,
    radio_group: crate::radio_group::PINK_DREAM,
    text_field: crate::text_field::PINK_DREAM,
    combo_box: crate::combo_box::PINK_DREAM,
    scroll_area: crate::scroll_area::PINK_DREAM,
    list_view: crate::list_view::PINK_DREAM,
    tree: crate::tree::PINK_DREAM,
    table: crate::table::PINK_DREAM,
    tooltip: crate::tooltip::PINK_DREAM,
    window: crate::window::PINK_DREAM,
    message_box: crate::message_box::PINK_DREAM,
    file_dialog: crate::file_dialog::PINK_DREAM,
    waveform: crate::waveform::PINK_DREAM,
};

pub const WIDGETRY_KAMURI_VIOLET_THEME: WidgetryTheme = WidgetryTheme {
    text: crate::text::KAMURI_VIOLET,
    icon: crate::icon::KAMURI_VIOLET,
    button: crate::button::KAMURI_VIOLET,
    check_box: crate::check_box::KAMURI_VIOLET,
    radio_group: crate::radio_group::KAMURI_VIOLET,
    text_field: crate::text_field::KAMURI_VIOLET,
    combo_box: crate::combo_box::KAMURI_VIOLET,
    scroll_area: crate::scroll_area::KAMURI_VIOLET,
    list_view: crate::list_view::KAMURI_VIOLET,
    tree: crate::tree::KAMURI_VIOLET,
    table: crate::table::KAMURI_VIOLET,
    tooltip: crate::tooltip::KAMURI_VIOLET,
    window: crate::window::KAMURI_VIOLET,
    message_box: crate::message_box::KAMURI_VIOLET,
    file_dialog: crate::file_dialog::KAMURI_VIOLET,
    waveform: crate::waveform::KAMURI_VIOLET,
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
    PinkDream,
    KamuriViolet,
}

impl WidgetryThemeMode {
    pub const fn colors(self) -> &'static WidgetryTheme {
        match self {
            Self::Light => &WIDGETRY_LIGHT_THEME,
            Self::Dark => &WIDGETRY_DARK_THEME,
            Self::PinkDream => &WIDGETRY_PINK_DREAM_THEME,
            Self::KamuriViolet => &WIDGETRY_KAMURI_VIOLET_THEME,
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
    //! Coverage Model：mode 为四个固定主题，stimuli 为 World 与 Commands setter。
    //! guard 为同值 no-op 与缺失 resource，Commands 仅在 queue apply 后提交。
    //! invariant 为默认 Dark、完整固定 palette 与先提交再通知。
    //! coupling 覆盖 mode 与文字、选区及非文字标记的实际前景 / 背景。

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
        assert_eq!(
            WidgetryThemeMode::PinkDream.colors(),
            &WIDGETRY_PINK_DREAM_THEME
        );
        assert_eq!(
            WidgetryThemeMode::KamuriViolet.colors(),
            &WIDGETRY_KAMURI_VIOLET_THEME
        );
    }

    #[test]
    fn new_modes_resolve_approved_window_backgrounds() {
        assert_eq!(
            WidgetryThemeMode::PinkDream
                .colors()
                .window
                .frame
                .normal
                .background,
            Color::srgb_u8(246, 220, 233)
        );
        assert_eq!(
            WidgetryThemeMode::KamuriViolet
                .colors()
                .window
                .frame
                .normal
                .background,
            Color::srgb_u8(227, 217, 241)
        );
    }

    #[test]
    fn normal_text_contrast_matches_reference_budget() {
        for theme in [
            &WIDGETRY_LIGHT_THEME,
            &WIDGETRY_DARK_THEME,
            &WIDGETRY_PINK_DREAM_THEME,
            &WIDGETRY_KAMURI_VIOLET_THEME,
        ] {
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
            assert!(
                contrast(
                    theme.file_dialog.status.normal.foreground,
                    theme.file_dialog.body.normal.background
                ) >= 4.5
            );
        }
    }

    #[test]
    fn new_theme_marks_focus_and_danger_icons_have_non_text_contrast() {
        for theme in [&WIDGETRY_PINK_DREAM_THEME, &WIDGETRY_KAMURI_VIOLET_THEME] {
            for state in [theme.check_box.checked, theme.check_box.indeterminate] {
                for state in [state.normal, state.hovered, state.pressed] {
                    assert!(contrast(state.mark, state.background) >= 3.0);
                }
            }
            for state in [theme.window.close.hovered, theme.window.close.pressed] {
                assert!(contrast(state.foreground, state.background) >= 3.0);
            }
            assert!(
                contrast(
                    theme.text_field.editable.focused.border,
                    theme.text_field.editable.focused.background
                ) >= 3.0
            );
            for foreground in theme.waveform.normal.palette {
                assert!(contrast(*foreground, theme.waveform.normal.background) >= 3.0);
            }
        }
    }

    #[test]
    fn all_mode_transitions_match_world_and_deferred_commands() -> Result<(), BevyError> {
        let modes = [
            WidgetryThemeMode::Dark,
            WidgetryThemeMode::Light,
            WidgetryThemeMode::PinkDream,
            WidgetryThemeMode::KamuriViolet,
        ];
        for initial in modes {
            for target in modes {
                let mut immediate = app();
                let mut deferred = app();
                WidgetryThemeMode::set_in_world(immediate.world_mut(), initial)?;
                WidgetryThemeMode::set_in_world(deferred.world_mut(), initial)?;
                immediate
                    .world_mut()
                    .resource_mut::<Notifications>()
                    .0
                    .clear();
                deferred
                    .world_mut()
                    .resource_mut::<Notifications>()
                    .0
                    .clear();
                assert_eq!(
                    WidgetryThemeMode::set_in_world(immediate.world_mut(), target)?,
                    initial != target
                );
                assert!(!WidgetryThemeMode::set_in_world(
                    immediate.world_mut(),
                    target
                )?);
                let mut queue = bevy::ecs::world::CommandQueue::default();
                let mut commands = Commands::new(&mut queue, deferred.world());
                WidgetryThemeMode::set(&mut commands, target);
                WidgetryThemeMode::set(&mut commands, target);
                assert_eq!(*deferred.world().resource::<WidgetryThemeMode>(), initial);
                assert!(deferred.world().resource::<Notifications>().0.is_empty());
                queue.apply(deferred.world_mut());
                let expected = if initial == target {
                    vec![]
                } else {
                    vec![target]
                };
                assert_eq!(*immediate.world().resource::<WidgetryThemeMode>(), target);
                assert_eq!(*deferred.world().resource::<WidgetryThemeMode>(), target);
                assert_eq!(immediate.world().resource::<Notifications>().0, expected);
                assert_eq!(deferred.world().resource::<Notifications>().0, expected);
            }
        }
        Ok(())
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
        for initial in [
            WidgetryThemeMode::Dark,
            WidgetryThemeMode::Light,
            WidgetryThemeMode::PinkDream,
            WidgetryThemeMode::KamuriViolet,
        ] {
            let mut app = App::new();
            app.insert_resource(initial)
                .add_plugins(WidgetryThemePlugin);
            assert_eq!(*app.world().resource::<WidgetryThemeMode>(), initial);
        }
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
