use crate::{
    title_bar::{
        bar::TitleBar, close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton,
    },
    window_root::{WindowContent, WindowRoot},
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_core::{color::WidgetryStyleOwner, foreground::ResolvedForeground};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode, WidgetryWindowColors};

pub fn apply_owned_window_colors(
    world: &mut World,
    root: Entity,
    colors: &WidgetryWindowColors,
) -> Result<(), BevyError> {
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let frame = if disabled {
        colors.frame.disabled
    } else {
        colors.frame.normal
    };
    if world
        .get::<crate::background::ThemeWindowBackground>(root)
        .is_some()
    {
        world
            .get_mut::<BackgroundColor>(root)
            .ok_or_else(|| failure(root))?
            .set_if_neq(BackgroundColor(frame.background));
    }
    world
        .get_mut::<BorderColor>(root)
        .ok_or_else(|| failure(root))?
        .set_if_neq(BorderColor::all(frame.border));
    world
        .get_mut::<ResolvedForeground>(root)
        .ok_or_else(|| failure(root))?
        .set_if_neq(ResolvedForeground(frame.foreground));
    if let Some(opacity) = world
        .get::<crate::background::ImageOpacity>(root)
        .map(|value| value.0)
        && let Some(mut image) = world.get_mut::<ImageNode>(root)
    {
        let tint = colors
            .image_tint
            .with_alpha(colors.image_tint.alpha() * opacity);
        if image.color != tint {
            image.color = tint;
        }
    }
    let mut stack = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    while let Some(entity) = stack.pop() {
        if world.get::<WindowRoot>(entity).is_some() {
            continue;
        }
        let disabled = world.get::<InteractionDisabled>(entity).is_some();
        if world.get::<TitleBar>(entity).is_some() {
            let state = if disabled {
                colors.title_bar.disabled
            } else {
                colors.title_bar.normal
            };
            set_surface(
                world,
                entity,
                state.background,
                Some(state.border),
                state.foreground,
            );
        } else if world.get::<WindowContent>(entity).is_some() {
            let state = if disabled {
                colors.frame.disabled
            } else {
                colors.frame.normal
            };
            if world.get::<ResolvedForeground>(entity)
                != Some(&ResolvedForeground(state.foreground))
            {
                world
                    .entity_mut(entity)
                    .insert(ResolvedForeground(state.foreground));
            }
        } else {
            let button = if world.get::<CloseButton>(entity).is_some() {
                Some(colors.close)
            } else if world.get::<MinimizeButton>(entity).is_some() {
                Some(colors.minimize)
            } else if world.get::<MaximizeButton>(entity).is_some() {
                Some(colors.maximize)
            } else {
                None
            };
            if let Some(button) = button {
                let state = if disabled {
                    button.disabled
                } else if world.get::<Pressed>(entity).is_some() {
                    button.pressed
                } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
                    button.hovered
                } else {
                    button.normal
                };
                set_surface(world, entity, state.background, None, state.foreground);
            }
        }
        if (world.get::<TitleBar>(entity).is_some()
            || world
                .get::<crate::title_bar::bar::WindowControls>(entity)
                .is_some())
            && let Some(children) = world.get::<Children>(entity)
        {
            stack.extend(children.iter());
        }
    }
    Ok(())
}
fn set_surface(
    world: &mut World,
    entity: Entity,
    background: Color,
    border: Option<Color>,
    foreground: Color,
) {
    if world.get::<BackgroundColor>(entity) != Some(&BackgroundColor(background)) {
        world.entity_mut(entity).insert(BackgroundColor(background));
    }
    if let Some(color) = border
        && world.get::<BorderColor>(entity) != Some(&BorderColor::all(color))
    {
        world.entity_mut(entity).insert(BorderColor::all(color));
    }
    if world.get::<ResolvedForeground>(entity) != Some(&ResolvedForeground(foreground)) {
        world
            .entity_mut(entity)
            .insert(ResolvedForeground(foreground));
    }
}
fn failure(entity: Entity) -> BevyError {
    bevy_widgetry_log::widgetry_error!(?entity, "Window 缺失颜色输出");
    BevyError::error("Window 缺失颜色输出")
}
pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().window;
    let roots = world
        .query_filtered::<Entity, (With<WindowRoot>, Without<WidgetryStyleOwner<WindowRoot>>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = crate::WidgetryWindowColorOverrides::get(world, root)?.resolve(&theme);
        apply_owned_window_colors(world, root, &colors)?;
    }
    Ok(())
}
pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(update_colors);
}
