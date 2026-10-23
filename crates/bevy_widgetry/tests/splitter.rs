#![cfg(feature = "splitter")]
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry::{
    scene::spawn_scene,
    splitter::{WidgetrySplitter, WidgetrySplitterPlugin},
};

#[test]
fn facade_exposes_splitter_bsn_and_ratio() {
    let mut app = bevy_widgetry_test_utils::picking_app();
    app.add_plugins(WidgetrySplitterPlugin);
    bevy_widgetry_test_utils::spawn_ui_camera(&mut app, UVec2::splat(1200), 1.0);
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetrySplitter Node {width:px(1008),height:px(100)} },
    )
    .unwrap();
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.7).unwrap());
    WidgetrySplitter::set_split_ratio(&mut app.world_mut().commands(), root, 0.2);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.7
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.2
    );
}
