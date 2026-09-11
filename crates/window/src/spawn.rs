use bevy::prelude::*;

use crate::{
    title_bar::{TitleBar, WindowResizeArea},
    window_root::{WindowContent, WindowRoot},
};

pub fn spawn_window(
    commands: &mut Commands,
    asset_server: &AssetServer,
    target_window: Entity,
    title_bar_content: impl FnOnce(&mut Commands, Entity),
    window_content: impl FnOnce(&mut Commands, Entity),
) -> Entity {
    let root = commands.spawn(WindowRoot { target_window }).id();

    let mut title_bar_content_entity = None;
    let mut window_content_entity = None;

    commands.entity(root).with_children(|root| {
        title_bar_content_entity = Some(TitleBar::spawn(root, asset_server));

        window_content_entity = Some(root.spawn(WindowContent).id());

        WindowResizeArea::spawn(root);
    });

    title_bar_content(commands, title_bar_content_entity.unwrap());

    window_content(commands, window_content_entity.unwrap());

    root
}
