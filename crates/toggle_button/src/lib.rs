#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::prelude::*;
use bevy::ui::Checked;
mod colors;
mod error;
mod style;
pub use colors::*;
pub use error::{ToggleButtonError, ToggleButtonFailure};
pub use style::{WidgetryToggleButtonPlugin, WidgetryToggleButtonProps};

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryToggleButtonProps)]
#[require(colors::ColorState)]
pub struct WidgetryToggleButton;

/// Emitted after the new Checked state has been committed.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryToggleButtonChanged {
    pub entity: Entity,
    pub selected: bool,
}
impl WidgetryToggleButton {
    pub fn selected(world: &World, entity: Entity) -> Result<bool, ToggleButtonFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(error::failure(
                ToggleButtonError::InvalidTarget { root: entity },
                "toggle_button.selected",
            ));
        }
        Ok(world.get::<Checked>(entity).is_some())
    }
    pub fn set_selected_in_world(
        world: &mut World,
        entity: Entity,
        selected: bool,
    ) -> Result<bool, ToggleButtonFailure> {
        if Self::selected(world, entity).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("toggle_button.set_selected")
                    .field("root", entity),
            )
        })? == selected
        {
            return Ok(false);
        }
        if selected {
            world.entity_mut(entity).insert(Checked);
        } else {
            world.entity_mut(entity).remove::<Checked>();
        }
        world.trigger(WidgetryToggleButtonChanged { entity, selected });
        Ok(true)
    }
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_in_world(world, entity, selected)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}
