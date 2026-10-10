use crate::{
    WidgetryMessageBox,
    scene::{MessageBoxAction, MessageBoxBody},
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{color::WidgetryStyleOwner, foreground::ResolvedForeground};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};
use bevy_widgetry_window::internal::WindowRoot;

pub(crate) fn establish_owners(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryMessageBox>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        if world.get::<WidgetryStyleOwner<WindowRoot>>(root).is_none() {
            world
                .entity_mut(root)
                .insert(WidgetryStyleOwner::<WindowRoot>::new(root));
        }
        let mut stack = world
            .get::<Children>(root)
            .map(|c| c.to_vec())
            .unwrap_or_default();
        while let Some(entity) = stack.pop() {
            if world.get::<MessageBoxAction>(entity).is_some()
                && world
                    .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                    .is_none()
            {
                world
                    .entity_mut(entity)
                    .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
            }
            if let Some(c) = world.get::<Children>(entity) {
                stack.extend(c.iter());
            }
        }
    }
}
pub fn apply_owned_message_box_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryMessageBoxColors,
) -> Result<(), BevyError> {
    bevy_widgetry_window::internal::apply_owned_window_colors(world, root, &colors.window)?;
    let mut stack = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    while let Some(entity) = stack.pop() {
        let disabled = world.get::<InteractionDisabled>(entity).is_some();
        if world.get::<MessageBoxBody>(entity).is_some() {
            let state = if disabled {
                colors.body.disabled
            } else {
                colors.body.normal
            };
            world
                .get_mut::<BackgroundColor>(entity)
                .ok_or_else(|| failure(entity))?
                .set_if_neq(BackgroundColor(state.background));
            world
                .get_mut::<ResolvedForeground>(entity)
                .ok_or_else(|| failure(entity))?
                .set_if_neq(ResolvedForeground(state.foreground));
        } else if world.get::<MessageBoxAction>(entity).is_some() {
            let state = if disabled {
                colors.action_button.disabled
            } else if world.get::<Pressed>(entity).is_some() {
                colors.action_button.pressed
            } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
                colors.action_button.hovered
            } else {
                colors.action_button.normal
            };
            bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
        }
        if let Some(c) = world.get::<Children>(entity) {
            stack.extend(c.iter());
        }
    }
    Ok(())
}
fn failure(entity: Entity) -> BevyError {
    bevy_widgetry_log::widgetry_error!(?entity, "MessageBox 缺失颜色输出");
    BevyError::error("MessageBox 缺失颜色输出")
}
pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().message_box;
    let roots = world
        .query_filtered::<Entity, (
            With<WidgetryMessageBox>,
            Without<WidgetryStyleOwner<WidgetryMessageBox>>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = crate::WidgetryMessageBoxColorOverrides::get(world, root)?.resolve(&theme);
        apply_owned_message_box_colors(world, root, &colors)?;
    }
    Ok(())
}
pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(update_colors);
}

pub(crate) fn own_window(event: On<Add<WidgetryMessageBox>>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .insert(WidgetryStyleOwner::<WindowRoot>::new(event.entity));
}
pub(crate) fn own_action(
    event: On<Add<MessageBoxAction>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryMessageBox>>,
    mut commands: Commands,
) {
    if let Some(root) = parents
        .iter_ancestors(event.entity)
        .find(|root| roots.contains(*root))
    {
        commands
            .entity(event.entity)
            .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
    }
}
