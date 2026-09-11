#![cfg(test)]

use bevy::prelude::*;
use bevy_widgetry_window::spawn_window;

/// 用两个窗口检验回调挂载位置，防止构造过程把内容插入另一棵层级树。
#[test]
fn callbacks_receive_distinct_slots_under_their_window() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy_widgetry_window::TitleBarPlugin,
    ));
    let asset_server = app.world().resource::<AssetServer>().clone();
    let target = app.world_mut().spawn_empty().id();
    let mut roots = Vec::new();
    let mut slots = Vec::new();
    for _ in 0..2 {
        let mut title = None;
        let mut content = None;
        let root = spawn_window(
            &mut app.world_mut().commands(),
            &asset_server,
            target,
            |_, entity| title = Some(entity),
            |_, entity| content = Some(entity),
        );
        roots.push(root);
        slots.push((title.unwrap(), content.unwrap()));
    }
    app.world_mut().flush();
    for (root, (title, content)) in roots.into_iter().zip(slots) {
        assert_ne!(title, content);
        assert_eq!(app.world().get::<ChildOf>(content).unwrap().parent(), root);
        let title_bar = app.world().get::<ChildOf>(title).unwrap().parent();
        assert_eq!(
            app.world().get::<ChildOf>(title_bar).unwrap().parent(),
            root
        );
        assert_eq!(app.world().get::<Children>(root).unwrap().len(), 3);
    }
}
