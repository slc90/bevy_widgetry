#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::prelude::*;
use bevy::ui::Checked;
mod colors;
mod style;
pub use colors::*;
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
    pub fn selected(world: &World, entity: Entity) -> Result<bool, BevyError> {
        if world.get::<Self>(entity).is_none() {
            bevy_widgetry_internal_log::widgetry_error!(
                ?entity,
                "Toggle Button 目标不存在或类型不匹配"
            );
            return Err(BevyError::error("Invalid Toggle Button target"));
        }
        Ok(world.get::<Checked>(entity).is_some())
    }
    pub fn set_selected_in_world(
        world: &mut World,
        entity: Entity,
        selected: bool,
    ) -> Result<bool, BevyError> {
        if Self::selected(world, entity)? == selected {
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
            Self::set_selected_in_world(world, entity, selected).map(|_| ())
        });
    }
}
