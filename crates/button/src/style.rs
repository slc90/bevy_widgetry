use bevy::{
    app::{App, Plugin, PostUpdate, Propagate},
    color::Color,
    ecs::{
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::{Scene, SceneComponent, bsn, template},
    ui::{
        BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, Pressed, UiRect,
        UiSystems, px,
    },
    ui_widgets::{Button, ButtonPlugin},
};
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_core::{
    ColorTheme, ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin,
};
use bevy_widgetry_log::widgetry_info;

#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryButton;

#[derive(Debug, PartialEq)]
struct ButtonStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

type ButtonStyleData = (
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Propagate<ForegroundColor>,
);

pub struct WidgetryButtonPlugin;

type ChangedButtonStyleQuery<'w, 's> = Query<
    'w,
    's,
    ButtonStyleData,
    (
        With<WidgetryButton>,
        Or<(
            Added<WidgetryButton>,
            Changed<Hovered>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

fn resolve_button_style(
    colors: &ColorTheme,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> ButtonStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if pressed {
        (
            colors.control_background_pressed,
            colors.control_border_pressed,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };
    ButtonStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

fn apply_button_style(
    colors: &ColorTheme,
    (hovered, pressed, disabled, mut background, mut border, mut foreground): <ButtonStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let style = resolve_button_style(colors, hovered.0, pressed, disabled);
    background.0 = style.background;
    *border = BorderColor::all(style.border);
    foreground.0 = ForegroundColor(style.foreground);
}

fn update_widgetry_button_style_changed(
    mode: Res<ThemeMode>,
    mut query: ChangedButtonStyleQuery<'_, '_>,
) {
    for item in &mut query {
        apply_button_style(mode.colors(), item);
    }
}

fn update_widgetry_button_style_removed(
    mode: Res<ThemeMode>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<ButtonStyleData, With<WidgetryButton>>,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok(item) = query.get_mut(entity) {
            apply_button_style(mode.colors(), item);
        }
    }
}

fn refresh_button_theme(
    event: On<ThemeChanged>,
    mut query: Query<ButtonStyleData, With<WidgetryButton>>,
) {
    for item in &mut query {
        apply_button_style(event.mode.colors(), item);
    }
}

impl WidgetryButton {
    fn scene() -> impl Scene {
        bsn! {
            Button
            Hovered(false)
            TabIndex(-1)
            Node {
                min_height: px(32),
                padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor
            BorderColor
            template(|_| Ok(Propagate(ForegroundColor::default())))
        }
    }
}

impl Plugin for WidgetryButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<ButtonPlugin>() {
            app.add_plugins(ButtonPlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(refresh_button_theme);
        app.add_systems(
            PostUpdate,
            (
                update_widgetry_button_style_changed,
                update_widgetry_button_style_removed,
            )
                // 组合 Widget 到 Build 才创建 Button；style 必须在其后执行，避免新 Button 错过当帧 foreground propagation。
                .after(WidgetryUiSystems::Build)
                .before(UiSystems::Prepare),
        );
        widgetry_info!("WidgetryButtonPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    const TEST_THEME: ColorTheme = ColorTheme {
        window_background: Color::BLACK,
        window_border: Color::WHITE,
        title_bar_border: Color::WHITE,
        foreground: Color::srgb_u8(1, 0, 0),
        foreground_disabled: Color::srgb_u8(2, 0, 0),
        control_background: Color::srgb_u8(3, 0, 0),
        control_background_hovered: Color::srgb_u8(4, 0, 0),
        control_background_pressed: Color::srgb_u8(5, 0, 0),
        control_background_active: Color::srgb_u8(6, 0, 0),
        control_background_disabled: Color::srgb_u8(7, 0, 0),
        control_border: Color::srgb_u8(8, 0, 0),
        control_border_hovered: Color::srgb_u8(9, 0, 0),
        control_border_pressed: Color::srgb_u8(10, 0, 0),
        control_border_active: Color::srgb_u8(11, 0, 0),
        control_border_disabled: Color::srgb_u8(12, 0, 0),
        popup_background: Color::srgb_u8(13, 0, 0),
        popup_border: Color::srgb_u8(14, 0, 0),
        item_background_hovered: Color::srgb_u8(15, 0, 0),
        item_background_selected: Color::srgb_u8(16, 0, 0),
        text_selection: Color::srgb_u8(52, 92, 140),
        text_selection_unfocused: Color::srgb_u8(65, 70, 78),
    };

    #[test]
    fn resolves_complete_style_with_state_priority() {
        let c = &TEST_THEME;
        for (hovered, pressed, disabled, background, border, foreground) in [
            (
                false,
                false,
                false,
                c.control_background,
                c.control_border,
                c.foreground,
            ),
            (
                true,
                false,
                false,
                c.control_background_hovered,
                c.control_border_hovered,
                c.foreground,
            ),
            (
                true,
                true,
                false,
                c.control_background_pressed,
                c.control_border_pressed,
                c.foreground,
            ),
            (
                true,
                true,
                true,
                c.control_background_disabled,
                c.control_border_disabled,
                c.foreground_disabled,
            ),
        ] {
            assert_eq!(
                resolve_button_style(c, hovered, pressed, disabled),
                ButtonStyle {
                    background,
                    border,
                    foreground
                }
            );
        }
    }
}
