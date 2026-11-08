#![cfg(feature = "splash")]
#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry::{
    WindowScaleExcluded,
    splash::{Splash, SplashPhase, SplashPlugin, SplashReady, SplashWindow},
};

#[test]
fn facade_can_explicitly_start_an_excluded_splash_without_scaling_or_window_features() {
    let mut app = App::new();
    app.add_plugins(SplashPlugin);
    let main = app.world_mut().spawn(Window::default()).id();
    let splash = Splash::start(app.world_mut(), main, Window::default()).unwrap();
    assert!(app.world().get::<WindowScaleExcluded>(splash).is_some());
    assert_eq!(
        app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .main_window(),
        main
    );
    assert!(app.world().get::<Window>(main).unwrap().visible);
    app.world_mut().trigger(SplashReady { splash });
    app.update();
    assert_eq!(
        Splash::phase(app.world(), splash),
        Some(SplashPhase::WaitingForFrame)
    );
    assert!(!app.world().get::<Window>(splash).unwrap().visible);
}
