use crate::{
    title_bar::{TitleBar, WindowResizeArea},
    window_root::{WindowContent, WindowRoot},
};
use bevy::prelude::*;

/// 为 target_window 创建自定义标题栏、内容区和缩放边缘，返回 UI 根实体。
/// 必须先注册 TitleBarPlugin 及其资产基础插件。两个回调依次接收标题栏内容区与窗口内容区实体。
/// 调用方负责创建真实窗口、UI 相机，并关闭系统装饰以避免重复标题栏。
pub fn spawn_window(
    commands: &mut Commands,
    asset_server: &AssetServer,
    target_window: Entity,
    title_bar_content: impl FnOnce(&mut Commands, Entity),
    window_content: impl FnOnce(&mut Commands, Entity),
) -> Entity {
    let root = commands.spawn(WindowRoot { target_window }).id();

    let title_bar_content_entity = TitleBar::spawn(commands, root, asset_server);
    let window_content_entity = commands.spawn((WindowContent, ChildOf(root))).id();
    commands.entity(root).with_children(|children| {
        WindowResizeArea::spawn(children);
    });

    title_bar_content(commands, title_bar_content_entity);
    window_content(commands, window_content_entity);

    root
}
