#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod background;
mod input;
mod modal;
mod native_icon;
mod render;
mod scene;
mod title_bar;
mod window_root;

pub use background::{
    WidgetryWindowBackground, WidgetryWindowImageBackground, WidgetryWindowImageMode,
};
pub use modal::WidgetryModalWindow;
#[doc(hidden)]
pub use native_icon::native_icon_plugin;
pub use render::transparent_render_creation;
pub use scene::{
    WidgetryWindowControlsConfig, owned_widgetry_window, prepare_native_window, widgetry_window,
};
pub use title_bar::WidgetryWindowPlugin;

#[derive(bevy::prelude::Component, bevy::prelude::FromTemplate, Default, Clone, Copy, Debug)]
pub struct WidgetryWindowInitialFocus(pub i32);

pub fn widgetry_window_target(world: &World, root: Entity) -> Option<Entity> {
    world
        .get::<window_root::WindowRoot>(root)
        .map(|root| root.target_window)
}

pub fn is_widgetry_window(world: &mut World, native: Entity) -> bool {
    if world.get::<Window>(native).is_none_or(|window| {
        !window.transparent
            || window.decorations
            || window.composite_alpha_mode != bevy::window::CompositeAlphaMode::PreMultiplied
    }) {
        return false;
    }
    let roots: Vec<_> = world
        .query::<(
            Entity,
            &window_root::WindowRoot,
            Option<&bevy::prelude::UiTargetCamera>,
        )>()
        .iter(world)
        .map(|(entity, root, camera)| (entity, root.target_window, camera.map(|camera| camera.0)))
        .collect();
    let matches: Vec<_> = roots
        .iter()
        .filter(|(_, target, _)| *target == native)
        .collect();
    let [bound] = matches.as_slice() else {
        return false;
    };
    let Some(camera) = bound.2 else {
        return false;
    };
    if world.get::<bevy::prelude::Camera>(camera).is_none()
        || roots
            .iter()
            .any(|root| root.0 != bound.0 && root.2 == Some(camera))
    {
        return false;
    }
    world
        .get::<window_root::WindowInitialized>(bound.0)
        .is_none()
        || matches!(world.get::<bevy::camera::RenderTarget>(camera),Some(bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(target))) if *target==native)
}

use bevy::prelude::{Entity, Window, World};

mod colors;
mod style;
pub use colors::*;
pub mod internal {
    pub use crate::style::apply_owned_window_colors;
    pub use crate::window_root::WindowRoot;
}
