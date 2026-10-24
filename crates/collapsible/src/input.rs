use crate::{WidgetryCollapsible, collapsible::Parts};
use bevy::input_focus::{FocusCause, InputFocus, tab_navigation::TabIndex};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

pub(crate) fn activate(event: On<Activate>, mut commands: Commands) {
    let button = event.entity;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if !world.entities().contains(button) || !can_focus(world, button) {
            return Ok(());
        }
        let root = world
            .get::<ChildOf>(button)
            .and_then(|parent| world.get::<ChildOf>(parent.parent()))
            .map(ChildOf::parent)
            .ok_or_else(|| crate::state::invalid("Collapsible toggle owner is missing"))?;
        let parts = crate::state::structure(world, root)?;
        if parts.button != button {
            return Err(crate::state::invalid("Collapsible toggle owner is invalid"));
        }
        let expanded = WidgetryCollapsible::expanded(world, root)?;
        WidgetryCollapsible::set_expanded_in_world(world, root, !expanded)?;
        Ok(())
    });
}

fn can_focus(world: &World, button: Entity) -> bool {
    if world
        .get::<TabIndex>(button)
        .is_none_or(|index| index.0 < 0)
    {
        return false;
    }
    let mut current = Some(button);
    let mut inherit_visibility = true;
    while let Some(entity) = current {
        if world.get::<InteractionDisabled>(entity).is_some()
            || world
                .get::<Node>(entity)
                .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        if inherit_visibility {
            match world.get::<Visibility>(entity) {
                Some(Visibility::Hidden) => return false,
                Some(Visibility::Visible) => inherit_visibility = false,
                _ => {}
            }
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    true
}

pub(crate) fn collapse_focus(world: &mut World, parts: Parts) {
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get);
    let mut current = focused;
    while let Some(entity) = current {
        if entity == parts.content {
            let can_focus = can_focus(world, parts.button);
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                if can_focus {
                    focus.set(parts.button, FocusCause::Navigated);
                } else {
                    focus.clear();
                }
            }
            return;
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
}
