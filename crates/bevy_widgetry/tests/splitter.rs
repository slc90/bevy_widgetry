#![cfg(feature = "splitter")]
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry::{
    scene::spawn_scene,
    splitter::{WidgetrySplitter, WidgetrySplitterPlugin},
};

#[test]
fn facade_exposes_splitter_bsn_and_ratio() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetrySplitterPlugin);
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySplitter }).unwrap();
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
}
