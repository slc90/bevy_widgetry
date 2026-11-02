#![allow(clippy::disallowed_macros)]
#![cfg(feature = "window")]

use bevy::prelude::*;

#[test]
fn facade_embeds_the_calling_package_png() {
    let mut app = App::new();
    app.add_plugins(bevy_widgetry::window::taskbar_icon!("tests/fixtures/taskbar.png").unwrap());
}

#[test]
fn facade_rejects_embedded_non_png() {
    let error = bevy_widgetry::window::taskbar_icon!("tests/taskbar_icon.rs")
        .err()
        .unwrap();
    assert!(
        matches!(error.kind(), bevy_widgetry::window::WindowError::IconDecode { path, .. } if path == "tests/taskbar_icon.rs")
    );
}
