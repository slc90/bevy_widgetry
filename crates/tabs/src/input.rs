use crate::tabs::TabOwner;
use crate::{WidgetryTab, WidgetryTabs};
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
