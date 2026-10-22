use crate::tabs::TabOwner;
use crate::{WidgetryTab, WidgetryTabs};
use bevy::picking::{events::PointerClick, pointer::PointerButton};
use bevy::prelude::*;

pub(crate) fn on_click(
    mut event: On<PointerClick>,
    tabs: Query<&TabOwner, With<WidgetryTab>>,
    mut commands: Commands,
) {
    let Ok(owner) = tabs.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary {
        return;
    }
    let (root, index) = (owner.root, owner.index);
    commands.queue(move |world: &mut World| {
        WidgetryTabs::set_selected_in_world(world, root, index).map(|_| ())
    });
}
