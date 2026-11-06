use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemePlugin};

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
    picking::hover::Hovered,
    prelude::DetectChangesMut,
    prelude::{Scene, SceneComponent},
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed},
    ui_widgets::ButtonPlugin,
};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::widgetry_debug;
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryButtonProps)]
#[require(crate::colors::ColorState, crate::WidgetryButtonLoading)]
pub struct WidgetryButton;

#[derive(Default)]
pub struct WidgetryButtonProps {
    pub colors: crate::WidgetryButtonColorOverrides,
    pub content: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn bevy::prelude::SceneList>>,
}

#[derive(Debug, PartialEq)]
struct ButtonStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

type ButtonStyleData = (
    &'static crate::colors::ColorState,
    &'static crate::WidgetryButtonLoading,
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
            Changed<crate::WidgetryButtonLoading>,
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
    (overrides, loading, hovered, pressed, disabled, background, border, foreground): <ButtonStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let colors = overrides.0.resolve(&colors.button);
    let style = resolve_button_style(
        &colors,
        hovered.0 && !loading.is_active(),
        pressed && !loading.is_active(),
        disabled && !loading.is_active(),
    );
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
    mode: Res<WidgetryTheme>,
    mut query: ChangedButtonStyleQuery<'_, '_>,
) {
    for item in &mut query {
        apply_button_style(&mode, item);
    }
}

fn update_widgetry_button_style_removed(
    mode: Res<WidgetryTheme>,
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
            apply_button_style(&mode, item);
        }
    }
}

fn refresh_button_theme(_event: On<WidgetryThemeChanged>, mut commands: bevy::prelude::Commands) {
    commands.queue(
        |world: &mut bevy::prelude::World| -> Result<(), bevy::prelude::BevyError> {
            let colors = *world.resource::<WidgetryTheme>();
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
                crate::error::failure(
                    crate::ButtonError::ThemeQuery { source: error },
                    "button.refresh_theme",
                )
                .into_bevy_error()
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
        use crate::loading_style::{ButtonContent, Spinner, SpinnerArc, SpinnerTrack};
        use bevy::prelude::*;
        let WidgetryButtonProps {
            colors,
            content,
            children,
        } = props;
        bsn! {
            template(move |context| colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @crate::WidgetryButtonHeadless
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node {
                min_height: px(32),
                padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor
            BorderColor
            template(|_| Ok(ResolvedForeground::default()))
            Children [
                ButtonContent
                Node { flex_grow: 1.0, min_width: px(0), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                @{content}
                Children [{children}]--
                Spinner Visibility::Hidden Pickable::IGNORE
                Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), bottom: px(0), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [
                    SpinnerTrack Pickable::IGNORE ImageNode
                    Node { width: px(24), height: px(24), flex_shrink: 0.0 }--
                    SpinnerArc Pickable::IGNORE ImageNode
                    Node { position_type: PositionType::Absolute, width: px(24), height: px(24) }
                ]
            ]
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
        crate::headless::install(app);
        crate::link::install(app);
        crate::loading::install(app);
        crate::loading_style::install(app);
        app.add_systems(
            PostUpdate,
            (
                update_widgetry_button_style_changed,
                update_widgetry_button_style_removed,
            )
                // Composite widgets create their Buttons during Build.
                // Run styling afterwards so new Buttons reach foreground propagation in the same frame.
                .in_set(WidgetryUiSystems::Colors),
        );
        widgetry_debug!("WidgetryButtonPlugin 注册完成");
    }
}

pub fn apply_owned_button_colors(
    world: &mut bevy::prelude::World,
    entity: bevy::prelude::Entity,
    colors: bevy_widgetry_theme::WidgetryButtonStateColors,
) -> Result<(), crate::ButtonFailure> {
    use bevy::prelude::*;
    let mut query = world.query::<(
        &mut BackgroundColor,
        &mut BorderColor,
        &mut ResolvedForeground,
    )>();
    match query.get_mut(world, entity) {
        Err(source) => {
            let mut missing = Vec::new();
            if world.get::<BackgroundColor>(entity).is_none() {
                missing.push("BackgroundColor");
            }
            if world.get::<BorderColor>(entity).is_none() {
                missing.push("BorderColor");
            }
            if world.get::<ResolvedForeground>(entity).is_none() {
                missing.push("ResolvedForeground");
            }
            Err(crate::error::failure(
                crate::ButtonError::MissingColorOutputs {
                    root: entity,
                    missing,
                    source,
                },
                "button.apply_owned_colors",
            ))
        }
        Ok(outputs) => {
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
    }
}

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
