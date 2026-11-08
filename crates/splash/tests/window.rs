#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::{
    prelude::*,
    window::{Monitor, PrimaryMonitor, WindowPosition},
};
use bevy_widgetry_splash::{
    Splash, SplashError, SplashPhase, SplashPlugin, SplashReady, SplashShown, SplashWindow,
};
use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin, WindowScaleExcluded};

const SPLASH_SIZE: UVec2 = UVec2::new(480, 270);
const MAIN_SIZE: UVec2 = UVec2::new(640, 400);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SplashPlugin));
    app
}

fn main_window(app: &mut App) -> Entity {
    app.world_mut()
        .spawn(Window {
            resolution: MAIN_SIZE.into(),
            position: WindowPosition::At(IVec2::new(-1900, 80)),
            ..default()
        })
        .id()
}

fn monitor(app: &mut App, position: IVec2, size: UVec2) -> Entity {
    app.world_mut()
        .spawn(Monitor {
            name: None,
            physical_width: size.x,
            physical_height: size.y,
            physical_position: position,
            scale_factor: 1.5,
            refresh_rate_millihertz: None,
            video_modes: vec![],
        })
        .id()
}

fn splash_config() -> Window {
    Window {
        resolution: SPLASH_SIZE.into(),
        position: WindowPosition::At(IVec2::splat(100)),
        ..default()
    }
}

#[test]
fn installing_the_plugin_does_not_start_or_hide_a_window() {
    let mut app = app();
    let main = main_window(&mut app);
    app.update();
    assert!(app.world().get::<Window>(main).unwrap().visible);
    assert_eq!(
        app.world_mut()
            .query::<&SplashWindow>()
            .iter(app.world())
            .count(),
        0
    );
}

#[derive(Resource, Default)]
struct ShownCount(usize);

#[test]
fn readiness_is_idempotent_and_never_substitutes_for_a_rendered_frame() {
    let mut app = app();
    app.init_resource::<ShownCount>()
        .add_observer(|_: On<SplashShown>, mut count: ResMut<ShownCount>| count.0 += 1);
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
    let main = main_window(&mut app);
    let first = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    let second = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    assert_eq!(
        Splash::phase(app.world(), first),
        Some(SplashPhase::WaitingForContent)
    );
    app.update();
    assert!(Splash::ready(app.world_mut(), first).unwrap());
    assert!(!Splash::ready(app.world_mut(), first).unwrap());
    app.world_mut().trigger(SplashReady { splash: first });
    app.update();
    assert_eq!(
        Splash::phase(app.world(), first),
        Some(SplashPhase::WaitingForFrame)
    );
    assert_eq!(
        Splash::phase(app.world(), second),
        Some(SplashPhase::WaitingForContent)
    );
    assert!(!app.world().get::<Window>(first).unwrap().visible);
    assert!(!app.world().get::<Window>(second).unwrap().visible);
    assert!(app.world().get::<Window>(main).unwrap().visible);
    assert_eq!(app.world().resource::<ShownCount>().0, 0);
    app.world_mut().trigger(SplashReady { splash: second });
    app.update();
    assert_eq!(
        Splash::phase(app.world(), second),
        Some(SplashPhase::WaitingForFrame)
    );
    assert!(!app.world().get::<Window>(second).unwrap().visible);
    app.world_mut().despawn(first);
    assert_eq!(Splash::phase(app.world(), first), None);
    assert!(
        matches!(Splash::ready(app.world_mut(), first).unwrap_err().kind(), SplashError::UnknownSplash { splash } if *splash == first)
    );
    assert!(app.world().get::<Window>(second).is_some());
}

#[test]
fn readiness_waits_for_metadata_and_rejects_invalid_direct_requests() {
    let mut world = World::new();
    assert!(matches!(
        Splash::ready(&mut world, Entity::PLACEHOLDER)
            .unwrap_err()
            .kind(),
        SplashError::PluginMissing
    ));
    let mut app = app();
    let main = main_window(&mut app);
    let splash = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    app.world_mut().trigger(SplashReady { splash });
    app.world_mut().trigger(SplashReady {
        splash: Entity::PLACEHOLDER,
    });
    app.update();
    assert_eq!(
        Splash::phase(app.world(), splash),
        Some(SplashPhase::WaitingForFrame)
    );
    assert!(
        !app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .is_positioned()
    );
    assert!(!app.world().get::<Window>(splash).unwrap().visible);
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
    app.update();
    assert!(
        app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .is_positioned()
    );
    assert!(!app.world().get::<Window>(splash).unwrap().visible);
    assert!(app.world().get::<Window>(main).unwrap().visible);
}

#[test]
fn explicit_start_is_an_independent_fixed_window_on_the_main_center_monitor() {
    let mut app = app();
    let primary = monitor(&mut app, IVec2::ZERO, UVec2::new(1920, 1080));
    app.world_mut().entity_mut(primary).insert(PrimaryMonitor);
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
    let main = main_window(&mut app);
    app.world_mut().get_mut::<Window>(main).unwrap().visible = false;
    let splash = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    assert_ne!(splash, main);
    let window = app.world().get::<Window>(splash).unwrap();
    assert_eq!(window.physical_size(), SPLASH_SIZE);
    assert_eq!(window.position, WindowPosition::At(IVec2::new(-1200, 405)));
    assert!(!window.decorations && !window.resizable && !window.visible);
    assert_eq!(window.resolution.scale_factor_override(), Some(1.0));
    assert!(app.world().get::<WindowScaleExcluded>(splash).is_some());
    assert_eq!(
        app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .main_window(),
        main
    );
    assert!(
        app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .is_positioned()
    );
    assert!(!app.world().get::<Window>(main).unwrap().visible);
}

#[test]
fn missing_metadata_waits_then_centers_without_changing_the_physical_design() {
    let mut app = app();
    let main = main_window(&mut app);
    let splash = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    assert!(
        !app.world()
            .get::<SplashWindow>(splash)
            .unwrap()
            .is_positioned()
    );
    assert!(!app.world().get::<Window>(splash).unwrap().visible);
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
    app.update();
    {
        let mut window = app.world_mut().get_mut::<Window>(splash).unwrap();
        window.resolution.set_scale_factor(2.0);
        window.resolution.set_scale_factor_override(Some(2.0));
        window.resolution.set_physical_resolution(960, 540);
        window.resizable = true;
        window.decorations = true;
    }
    app.update();
    let window = app.world().get::<Window>(splash).unwrap();
    assert_eq!(window.physical_size(), SPLASH_SIZE);
    assert_eq!(window.position, WindowPosition::At(IVec2::new(-1200, 405)));
    assert_eq!(window.resolution.scale_factor_override(), Some(1.0));
    assert!(!window.decorations && !window.resizable);
}

#[test]
fn global_scaling_cannot_rewrite_the_splash_even_during_creation() {
    let mut app = app();
    app.insert_resource(WidgetryUiScale::new(1.25).unwrap())
        .add_plugins(WidgetryUiScalePlugin);
    let main = main_window(&mut app);
    let splash = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    for scale in [1.0, 2.0] {
        app.world_mut()
            .resource_mut::<WidgetryUiScale>()
            .set(scale)
            .unwrap();
        app.update();
        let window = app.world().get::<Window>(splash).unwrap();
        assert_eq!(window.physical_size(), SPLASH_SIZE);
        assert_eq!(window.scale_factor(), 1.0);
    }
}

#[test]
fn invalid_inputs_do_not_create_a_partial_window_or_change_the_main() {
    let mut without_plugin = World::new();
    let target = without_plugin.spawn(Window::default()).id();
    assert!(matches!(
        Splash::start(&mut without_plugin, target, splash_config())
            .unwrap_err()
            .kind(),
        SplashError::PluginMissing
    ));
    let mut app = app();
    let main = main_window(&mut app);
    let mut config = splash_config();
    config.resolution.set_physical_resolution(0, SPLASH_SIZE.y);
    assert!(matches!(
        Splash::start(app.world_mut(), main, config)
            .unwrap_err()
            .kind(),
        SplashError::InvalidSize { .. }
    ));
    assert!(matches!(
        Splash::start(app.world_mut(), Entity::PLACEHOLDER, splash_config())
            .unwrap_err()
            .kind(),
        SplashError::MainWindowMissing { .. }
    ));
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(320, 240));
    assert!(matches!(
        Splash::start(app.world_mut(), main, splash_config())
            .unwrap_err()
            .kind(),
        SplashError::TooLarge { .. }
    ));
    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        1
    );
    assert!(app.world().get::<Window>(main).unwrap().visible);
}

#[test]
fn center_monitor_wins_over_primary_and_largest_overlap() {
    let mut app = app();
    let left = monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 2160));
    app.world_mut().entity_mut(left).insert(PrimaryMonitor);
    monitor(&mut app, IVec2::ZERO, UVec2::new(1920, 1080));
    let main = app
        .world_mut()
        .spawn(Window {
            resolution: (3200, 2000).into(),
            position: WindowPosition::At(IVec2::new(-1500, 0)),
            ..default()
        })
        .id();
    let splash = Splash::start(app.world_mut(), main, splash_config()).unwrap();
    assert_eq!(
        app.world().get::<Window>(splash).unwrap().position,
        WindowPosition::At(IVec2::new(720, 405))
    );
    app.world_mut().get_mut::<Window>(main).unwrap().position =
        WindowPosition::Centered(bevy::window::MonitorSelection::Entity(left));
    app.update();
    assert_eq!(
        app.world().get::<Window>(splash).unwrap().position,
        WindowPosition::At(IVec2::new(-1200, 945))
    );
}

#[derive(Resource, Default)]
struct ChangedWindows(usize);

#[test]
fn static_geometry_does_not_mark_window_changed_in_the_sync_frame() {
    let mut app = app();
    monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
    let main = main_window(&mut app);
    Splash::start(app.world_mut(), main, splash_config()).unwrap();
    app.init_resource::<ChangedWindows>().add_systems(
        PostUpdate,
        (|windows: Query<Entity, Changed<Window>>, mut count: ResMut<ChangedWindows>| {
            count.0 = windows.iter().count();
        })
        .after(bevy_widgetry_splash::SplashSystems::Geometry),
    );
    app.update();
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 0);
}

#[test]
fn domain_failure_keeps_identity_and_reaches_the_real_system_handler() {
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler()).add_systems(
        Startup,
        |world: &mut World| -> Result {
            Splash::start(world, Entity::PLACEHOLDER, splash_config())
                .map(|_| ())
                .map_err(|failure| failure.into_bevy_error())
        },
    );
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| app.update()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    let failure = (diagnostic as &dyn std::error::Error)
        .downcast_ref::<bevy_widgetry_splash::SplashFailure>()
        .unwrap();
    assert!(
        matches!(failure.kind(), SplashError::MainWindowMissing { main } if *main == Entity::PLACEHOLDER)
    );
    assert_eq!(
        logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}

#[test]
fn independent_failures_batch_once_and_waiting_is_not_recovery() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let first = main_window(&mut app);
    let second = main_window(&mut app);
    Splash::start(app.world_mut(), first, splash_config()).unwrap();
    Splash::start(app.world_mut(), second, splash_config()).unwrap();
    app.world_mut().entity_mut(first).remove::<Window>();
    app.world_mut().entity_mut(second).remove::<Window>();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.update();
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let old_ids = records[0]
                .managed()
                .unwrap()
                .diagnostics()
                .map(|diagnostic| diagnostic.origin().id().clone())
                .collect::<Vec<_>>();
            assert_eq!(old_ids.len(), 2);
            app.world_mut().entity_mut(first).insert(Window {
                resolution: MAIN_SIZE.into(),
                position: WindowPosition::At(IVec2::new(-1900, 80)),
                ..default()
            });
            app.update();
            app.world_mut().entity_mut(first).remove::<Window>();
            app.update();
            assert!(errors.take_records().is_empty());
            assert!(
                logs.records()
                    .iter()
                    .all(|record| !record.matches_field("phase", "recovered"))
            );
            monitor(&mut app, IVec2::new(-1920, 0), UVec2::new(1920, 1080));
            app.world_mut().entity_mut(first).insert(Window {
                resolution: MAIN_SIZE.into(),
                position: WindowPosition::At(IVec2::new(-1900, 80)),
                ..default()
            });
            app.update();
            app.world_mut().entity_mut(first).remove::<Window>();
            app.update();
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert!(!old_ids.contains(diagnostic.origin().id()));
        })
    });
}
