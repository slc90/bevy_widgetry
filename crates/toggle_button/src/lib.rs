#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::prelude::*;
mod colors;
mod error;
mod headless;
mod style;
pub use colors::*;
pub use error::{ToggleButtonError, ToggleButtonFailure};
pub use headless::{
    WidgetryToggleButtonHeadless, WidgetryToggleButtonHeadlessPlugin,
    WidgetryToggleButtonHeadlessProps,
};
pub use style::{WidgetryToggleButtonPlugin, WidgetryToggleButtonProps};

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryToggleButtonProps)]
#[require(colors::ColorState, WidgetryToggleButtonHeadless)]
pub struct WidgetryToggleButton;

/// Emitted after the new Checked state has been committed.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryToggleButtonChanged {
    pub entity: Entity,
    pub selected: bool,
}
