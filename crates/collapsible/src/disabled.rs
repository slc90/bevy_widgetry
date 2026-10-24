use crate::{
    WidgetryCollapsible,
    state::{invalid, structure},
};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct HeaderDisabled(pub(crate) bool);

pub(crate) fn project(world: &mut World, root: Entity, header: Entity) -> Result<(), BevyError> {
    let disabled = world
        .get::<HeaderDisabled>(root)
        .ok_or_else(|| invalid("Collapsible Header request is missing"))?
        .0;
    bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<HeaderDisabled>(
        world, header, disabled,
    );
    Ok(())
}

impl WidgetryCollapsible {
    pub fn header_disabled(world: &World, root: Entity) -> Result<bool, BevyError> {
        structure(world, root)?;
        Ok(world
            .get::<HeaderDisabled>(root)
            .ok_or_else(|| invalid("Collapsible Header request is missing"))?
            .0)
    }

    pub fn set_header_disabled(commands: &mut Commands, root: Entity, disabled: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_header_disabled_in_world(world, root, disabled).map(|_| ())
        });
    }

    pub fn set_header_disabled_in_world(
        world: &mut World,
        root: Entity,
        disabled: bool,
    ) -> Result<bool, BevyError> {
        let parts = structure(world, root)?;
        let mut state = world
            .get_mut::<HeaderDisabled>(root)
            .ok_or_else(|| invalid("Collapsible Header request is missing"))?;
        let changed = state.0 != disabled;
        if changed {
            state.0 = disabled;
        }
        project(world, root, parts.header)?;
        Ok(changed)
    }
}
