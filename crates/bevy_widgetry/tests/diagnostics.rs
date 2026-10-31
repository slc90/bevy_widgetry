#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry::{
    diagnostics::{
        ErrorFrame, WidgetryFailure,
        bevy::{WidgetryEcsError, into_bevy_error},
    },
    icon::WidgetryIconPlugin,
};
use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

#[test]
fn facade_diagnostics_and_plugin_preserve_the_hosts_handler() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_plugins(WidgetryIconPlugin);
    let capture = ErrorCapture::default();
    capture.run(|| {
        app.world_mut()
            .commands()
            .queue(|_: &mut World| -> Result<(), BevyError> {
                Err(into_bevy_error(WidgetryFailure::new(
                    std::io::Error::other("Facade 源头"),
                    ErrorFrame::new("命令"),
                )))
            });
        app.world_mut().flush();
    });
    let errors = capture.take();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].downcast_ref::<WidgetryEcsError>().is_some());
}
