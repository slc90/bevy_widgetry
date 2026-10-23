#![cfg(feature = "progress_bar")]
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry::{
    progress_bar::{WidgetryProgressBar, WidgetryProgressBarPlugin, WidgetryProgressState},
    scene::spawn_scene,
};

#[test]
fn facade_progress_bar_feature_exposes_bsn_and_state_updates() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryProgressBarPlugin);
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryProgressBar }).unwrap();
    WidgetryProgressBar::set_state(
        &mut app.world_mut().commands(),
        root,
        WidgetryProgressState::Indeterminate,
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetryProgressBar::state(app.world(), root).unwrap(),
        WidgetryProgressState::Indeterminate
    );
}
