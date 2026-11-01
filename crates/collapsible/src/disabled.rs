use crate::{
    CollapsibleFailure, WidgetryCollapsible,
    error::{core, missing},
    state::structure,
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Component)]
pub(crate) struct HeaderDisabled(pub(crate) bool);

pub(crate) fn project(
    world: &mut World,
    root: Entity,
    header: Entity,
) -> Result<(), CollapsibleFailure> {
    let disabled = world
        .get::<HeaderDisabled>(root)
        .ok_or_else(|| missing(root, root, "HeaderDisabled"))?
        .0;
    bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<HeaderDisabled>(
        world, header, disabled,
    )
    .map_err(|error| {
        core(
            error,
            ErrorFrame::new("collapsible.project_header_disabled")
                .field("root", root)
                .field("header", header)
                .field("disabled", disabled),
        )
    })?;
    Ok(())
}

impl WidgetryCollapsible {
    pub fn header_disabled(world: &World, root: Entity) -> Result<bool, CollapsibleFailure> {
        structure(world, root)?;
        Ok(world
            .get::<HeaderDisabled>(root)
            .ok_or_else(|| missing(root, root, "HeaderDisabled"))?
            .0)
    }

    pub fn set_header_disabled(commands: &mut Commands, root: Entity, disabled: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_header_disabled_in_world(world, root, disabled)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    pub fn set_header_disabled_in_world(
        world: &mut World,
        root: Entity,
        disabled: bool,
    ) -> Result<bool, CollapsibleFailure> {
        let frame = || {
            ErrorFrame::new("collapsible.set_header_disabled")
                .field("root", root)
                .field("disabled", disabled)
        };
        let parts = structure(world, root).map_err(|error| error.with_context(frame()))?;
        let mut state = world
            .get_mut::<HeaderDisabled>(root)
            .ok_or_else(|| missing(root, root, "HeaderDisabled"))?;
        let changed = state.0 != disabled;
        if changed {
            state.0 = disabled;
        }
        project(world, root, parts.header).map_err(|error| error.with_context(frame()))?;
        Ok(changed)
    }
}
