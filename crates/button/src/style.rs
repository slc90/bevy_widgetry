use bevy_widgetry_theme::{
    WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode, WidgetryThemePlugin,
};

use bevy::{
    app::{App, Plugin, PostUpdate},
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
    prelude::DetectChangesMut,
    prelude::{Scene, SceneComponent, bsn, template},
    ui::{
        BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, Pressed, UiRect, px,
    },
    ui_widgets::{Button, ButtonPlugin},
};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_log::widgetry_info;
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryButtonProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryButton;

#[derive(Default, Clone, Debug)]
pub struct WidgetryButtonProps {
    pub colors: crate::WidgetryButtonColorOverrides,
}

#[derive(Debug, PartialEq)]
struct ButtonStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

type ButtonStyleData = (
    &'static crate::colors::ColorState,
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut ResolvedForeground,
);

pub struct WidgetryButtonPlugin;

type ChangedButtonStyleQuery<'w, 's> = Query<
    'w,
    's,
    ButtonStyleData,
    (
        With<WidgetryButton>,
        bevy::prelude::Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryButton>>,
        Or<(
            Added<WidgetryButton>,
            Changed<Hovered>,
            Changed<crate::colors::ColorState>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

fn resolve_button_style(
    colors: &bevy_widgetry_theme::WidgetryButtonColors,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> ButtonStyle {
    let state = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered {
        colors.hovered
    } else {
        colors.normal
    };
    ButtonStyle {
        background: state.background,
        border: state.border,
        foreground: state.foreground,
    }
}

fn apply_button_style(
    colors: &WidgetryTheme,
    (overrides, hovered, pressed, disabled, background, border, foreground): <ButtonStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let colors = overrides.0.resolve(&colors.button);
    let style = resolve_button_style(&colors, hovered.0, pressed, disabled);
    apply_outputs(style, (background, border, foreground));
}

fn apply_outputs(
    style: ButtonStyle,
    (mut background, mut border, mut foreground): (
        bevy::prelude::Mut<BackgroundColor>,
        bevy::prelude::Mut<BorderColor>,
        bevy::prelude::Mut<ResolvedForeground>,
    ),
) {
    if background.0 != style.background {
        background.0 = style.background;
    }
    border.set_if_neq(BorderColor::all(style.border));
    foreground.set_if_neq(ResolvedForeground(style.foreground));
}

fn update_widgetry_button_style_changed(
    mode: Res<WidgetryThemeMode>,
    mut query: ChangedButtonStyleQuery<'_, '_>,
) {
    for item in &mut query {
        apply_button_style(mode.colors(), item);
    }
}

fn update_widgetry_button_style_removed(
    mode: Res<WidgetryThemeMode>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<
        ButtonStyleData,
        (
            With<WidgetryButton>,
            bevy::prelude::Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryButton>>,
        ),
    >,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok(item) = query.get_mut(entity) {
            apply_button_style(mode.colors(), item);
        }
    }
}

fn refresh_button_theme(_event: On<WidgetryThemeChanged>, mut commands: bevy::prelude::Commands) {
    commands.queue(
        |world: &mut bevy::prelude::World| -> Result<(), bevy::prelude::BevyError> {
            let colors = *world.resource::<WidgetryThemeMode>().colors();
            let mut state = bevy::ecs::system::SystemState::<
                Query<
                    ButtonStyleData,
                    (
                        With<WidgetryButton>,
                        bevy::prelude::Without<
                            bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryButton>,
                        >,
                    ),
                >,
            >::new(world);
            let mut query = state.get_mut(world).map_err(|error| {
                bevy_widgetry_log::widgetry_error!(%error, "Button Theme 颜色查询失败");
                bevy::prelude::BevyError::error(error.to_string())
            })?;
            for item in &mut query {
                apply_button_style(&colors, item);
            }
            Ok(())
        },
    );
}

impl WidgetryButton {
    fn scene(props: WidgetryButtonProps) -> impl Scene {
        bsn! {
            template(move |_| props.colors.clone().initial())
            Button
            bevy_widgetry_core::pointer::WidgetryPointerPressed
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
            template(|_| Ok(ResolvedForeground::default()))
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
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        app.add_observer(refresh_button_theme);
        app.add_systems(
            PostUpdate,
            (
                update_widgetry_button_style_changed,
                update_widgetry_button_style_removed,
            )
                // 组合 Widget 到 Build 才创建 Button。
                // style 必须在其后执行，避免新 Button 错过当帧 foreground propagation。
                .in_set(WidgetryUiSystems::Colors),
        );
        widgetry_info!("WidgetryButtonPlugin 注册完成");
    }
}

pub fn apply_owned_button_colors(
    world: &mut bevy::prelude::World,
    entity: bevy::prelude::Entity,
    colors: bevy_widgetry_theme::WidgetryButtonStateColors,
) -> Result<(), bevy::prelude::BevyError> {
    use bevy::prelude::*;
    let invalid = || {
        bevy_widgetry_log::widgetry_error!(?entity, "Button 缺失托管颜色输出");
        BevyError::error("Button 缺失托管颜色输出")
    };
    let mut query = world.query::<(
        &mut BackgroundColor,
        &mut BorderColor,
        &mut ResolvedForeground,
    )>();
    let outputs = query.get_mut(world, entity).map_err(|_| invalid())?;
    apply_outputs(
        ButtonStyle {
            background: colors.background,
            border: colors.border,
            foreground: colors.foreground,
        },
        outputs,
    );
    Ok(())
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    const TEST_THEME: WidgetryTheme = WidgetryTheme {
        button: bevy_widgetry_theme::WidgetryButtonColors {
            normal: bevy_widgetry_theme::WidgetryButtonStateColors {
                background: Color::srgb_u8(3, 0, 0),
                border: Color::srgb_u8(8, 0, 0),
                foreground: Color::srgb_u8(1, 0, 0),
            },
            hovered: bevy_widgetry_theme::WidgetryButtonStateColors {
                background: Color::srgb_u8(4, 0, 0),
                border: Color::srgb_u8(9, 0, 0),
                foreground: Color::srgb_u8(13, 0, 0),
            },
            pressed: bevy_widgetry_theme::WidgetryButtonStateColors {
                background: Color::srgb_u8(5, 0, 0),
                border: Color::srgb_u8(10, 0, 0),
                foreground: Color::srgb_u8(14, 0, 0),
            },
            disabled: bevy_widgetry_theme::WidgetryButtonStateColors {
                background: Color::srgb_u8(7, 0, 0),
                border: Color::srgb_u8(12, 0, 0),
                foreground: Color::srgb_u8(2, 0, 0),
            },
        },
        ..bevy_widgetry_theme::WIDGETRY_DARK_THEME
    };

    #[test]
    fn resolves_complete_style_with_state_priority() {
        let c = &TEST_THEME;
        for (hovered, pressed, disabled, background, border, foreground) in [
            (
                false,
                false,
                false,
                c.button.normal.background,
                c.button.normal.border,
                c.button.normal.foreground,
            ),
            (
                true,
                false,
                false,
                c.button.hovered.background,
                c.button.hovered.border,
                c.button.hovered.foreground,
            ),
            (
                true,
                true,
                false,
                c.button.pressed.background,
                c.button.pressed.border,
                c.button.pressed.foreground,
            ),
            (
                true,
                true,
                true,
                c.button.disabled.background,
                c.button.disabled.border,
                c.button.disabled.foreground,
            ),
        ] {
            assert_eq!(
                resolve_button_style(&c.button, hovered, pressed, disabled),
                ButtonStyle {
                    background,
                    border,
                    foreground
                }
            );
        }
    }
}
