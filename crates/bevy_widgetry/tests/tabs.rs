#![cfg(feature = "tabs")]
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry::tabs::{WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::scene_app;

#[test]
fn facade_tabs_feature_constructs_fixed_pages_and_exposes_selection_api() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTabsPlugin);
    let items: Vec<Box<dyn Scene>> = (0..2)
        .map(|_| {
            let header: Box<dyn Scene> = Box::new(bsn! { Text("Header") });
            let content: Box<dyn Scene> = Box::new(bsn! { Text("Content") });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTabs { @children: {Some(items)} } })
        .unwrap()
        .id();
    app.update();
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
    WidgetryTabs::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 1);
}
