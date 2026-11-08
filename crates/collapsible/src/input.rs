use crate::{CollapsibleError, CollapsibleFailure, error::failure};
use crate::{WidgetryCollapsible, collapsible::Parts};
use bevy::input_focus::{FocusCause, InputFocus, tab_navigation::TabIndex};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry_diagnostics::ErrorFrame;

pub(crate) fn activate(event: On<Activate>, mut commands: Commands) {
    let button = event.entity;
    queue(
        &mut commands,
        move |world: &mut World| -> Result<(), CollapsibleFailure> {
            if !world.entities().contains(button) || !can_focus(world, button) {
                return Ok(());
            }
            let root = world
                .get::<ChildOf>(button)
                .and_then(|parent| world.get::<ChildOf>(parent.parent()))
                .map(ChildOf::parent)
                .ok_or_else(|| {
                    failure(
                        CollapsibleError::InvalidOwner {
                            button,
                            root: None,
                            expected_button: None,
                            parent: world.get::<ChildOf>(button).map(ChildOf::parent),
                        },
                        ErrorFrame::new("collapsible.activate"),
                    )
                })?;
            let parts = crate::state::structure(world, root)?;
            if parts.button != button {
                return Err(failure(
                    CollapsibleError::InvalidOwner {
                        button,
                        root: Some(root),
                        expected_button: Some(parts.button),
                        parent: world.get::<ChildOf>(button).map(ChildOf::parent),
                    },
                    ErrorFrame::new("collapsible.activate"),
                ));
            }
            let expanded = WidgetryCollapsible::expanded(world, root)?;
            WidgetryCollapsible::set_expanded_in_world(world, root, !expanded)?;
            Ok(())
        },
    );
}

fn can_focus(world: &World, button: Entity) -> bool {
    if world.get::<TabIndex>(button).is_none() {
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

fn queue(
    commands: &mut Commands,
    command: impl FnOnce(&mut World) -> Result<(), CollapsibleFailure> + Send + Sync + 'static,
) {
    commands
        .queue(move |world: &mut World| command(world).map_err(|error| error.into_bevy_error()));
}
