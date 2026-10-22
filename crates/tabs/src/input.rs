use crate::tabs::TabOwner;
use crate::{TabsOrientation, tabs::State};
use crate::{WidgetryTab, WidgetryTabs};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::picking::{events::PointerClick, pointer::PointerButton};
use bevy::prelude::*;

pub(crate) fn on_click(
    mut event: On<PointerClick>,
    tabs: Query<(&TabOwner, Has<bevy::ui::InteractionDisabled>), With<WidgetryTab>>,
    mut commands: Commands,
) {
    let Ok((owner, disabled)) = tabs.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary || disabled {
        return;
    }
    let (root, index) = (owner.root, owner.index);
    commands.queue(move |world: &mut World| select_from_user(world, root, index));
}

pub(crate) fn select_from_user(
    world: &mut World,
    root: Entity,
    index: usize,
) -> Result<(), BevyError> {
    if world.get::<WidgetryTabs>(root).is_none()
        || world.get::<bevy::ui::InteractionDisabled>(root).is_some()
    {
        return Ok(());
    }
    let header = WidgetryTabs::header(world, root, index)?;
    if world.get::<bevy::ui::InteractionDisabled>(header).is_some() {
        return Ok(());
    }
    WidgetryTabs::set_selected_in_world(world, root, index).map(|_| ())
}

pub(crate) fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    tabs: Query<&TabOwner, With<WidgetryTab>>,
    roots: Query<&State, (With<WidgetryTabs>, Without<bevy::ui::InteractionDisabled>)>,
    enabled: Query<(), (With<WidgetryTab>, Without<bevy::ui::InteractionDisabled>)>,
    mut commands: Commands,
) {
    let Ok(owner) = tabs.get(event.focused_entity) else {
        return;
    };
    let Ok(state) = roots.get(owner.root) else {
        return;
    };
    if event.input.state != ButtonState::Pressed || event.input.repeat {
        return;
    }
    let backward = match (state.orientation, event.input.key_code) {
        (TabsOrientation::Horizontal, KeyCode::ArrowLeft)
        | (TabsOrientation::Vertical, KeyCode::ArrowUp) => true,
        (TabsOrientation::Horizontal, KeyCode::ArrowRight)
        | (TabsOrientation::Vertical, KeyCode::ArrowDown) => false,
        _ => return,
    };
    event.propagate(false);
    let accepts = |index: &usize| enabled.contains(state.pairs[*index].header);
    let next = if backward {
        (0..owner.index).rev().find(accepts)
    } else {
        (owner.index + 1..state.pairs.len()).find(accepts)
    };
    if let Some(index) = next {
        let (root, origin) = (owner.root, event.focused_entity);
        commands.queue(move |world: &mut World| -> Result {
            if world.resource::<InputFocus>().get() != Some(origin)
                || world.get::<WidgetryTabs>(root).is_none()
                || world.get::<bevy::ui::InteractionDisabled>(root).is_some()
            {
                return Ok(());
            }
            let target = WidgetryTabs::header(world, root, index)?;
            if world.get::<bevy::ui::InteractionDisabled>(target).is_some() {
                return Ok(());
            }
            select_from_user(world, root, index)?;
            if world.resource::<InputFocus>().get() == Some(origin) {
                world
                    .resource_mut::<InputFocus>()
                    .set(target, FocusCause::Navigated);
            }
            Ok(())
        });
    }
}
