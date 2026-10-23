#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_progress_bar::{
    WidgetryProgressBar, WidgetryProgressBarPlugin, WidgetryProgressState,
};
use bevy_widgetry_test_utils::{picking_app, spawn_picking_camera};

fn fixture(value: f32) -> (App, Entity, Entity) {
    let mut app = picking_app();
    app.add_plugins(WidgetryProgressBarPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(800), 1.0);
    let root = spawn_scene(app.world_mut(), bsn! {
        Node { width: px(300), height: px(120), flex_direction: FlexDirection::Column }
        template(move |_| Ok(UiTargetCamera(camera)))
        Children [Name::new("CallerProgress") @WidgetryProgressBar { @state: {WidgetryProgressState::Determinate { value }} }]
    }).unwrap();
    let bar = app.world().get::<Children>(root).unwrap()[0];
    frames(&mut app);
    (app, root, bar)
}

fn frames(app: &mut App) {
    for _ in 0..12 {
        app.update();
    }
}
fn size(world: &World, entity: Entity) -> Vec2 {
    let n = world.get::<ComputedNode>(entity).unwrap();
    n.size() * n.inverse_scale_factor
}

#[test]
fn default_bsn_has_full_width_track_and_border_with_inset_fill() {
    for value in [0.0, 0.65, 1.0] {
        let (app, _, bar) = fixture(value);
        assert_eq!(size(app.world(), bar), Vec2::new(300.0, 12.0));
        let track = app.world().get::<Children>(bar).unwrap()[0];
        let clip = app.world().get::<Children>(track).unwrap()[0];
        let fill = app.world().get::<Children>(clip).unwrap()[0];
        assert_eq!(size(app.world(), track), Vec2::new(298.0, 10.0));
        assert!((size(app.world(), clip).x - 298.0 * value).abs() <= 1.0);
        assert_eq!(size(app.world(), fill).y, 10.0);
        assert_eq!(app.world().get::<Children>(bar).unwrap().len(), 1);
        assert_eq!(app.world().get::<Children>(track).unwrap().len(), 1);
        assert!(app.world().get::<Text>(fill).is_none());
        assert!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(bar)
                .is_none()
        );
        assert_eq!(
            app.world().get::<Name>(bar).unwrap().as_str(),
            "CallerProgress"
        );
    }
}

#[test]
fn updates_and_parent_resize_preserve_inset_ratio_and_custom_height() {
    let (mut app, parent, bar) = fixture(0.25);
    let track = app.world().get::<Children>(bar).unwrap()[0];
    let clip = app.world().get::<Children>(track).unwrap()[0];
    let fill = app.world().get::<Children>(clip).unwrap()[0];
    for value in [1.0, 0.65, 0.0] {
        WidgetryProgressBar::set_state_in_world(
            app.world_mut(),
            bar,
            WidgetryProgressState::Determinate { value },
        )
        .unwrap();
        frames(&mut app);
        assert!((size(app.world(), clip).x - 298.0 * value).abs() <= 1.0);
        assert_eq!(
            WidgetryProgressBar::state(app.world(), bar).unwrap(),
            WidgetryProgressState::Determinate { value }
        );
    }
    WidgetryProgressBar::set_state_in_world(
        app.world_mut(),
        bar,
        WidgetryProgressState::Determinate { value: 0.65 },
    )
    .unwrap();
    app.world_mut().get_mut::<Node>(parent).unwrap().width = px(500);
    app.world_mut().get_mut::<Node>(bar).unwrap().height = px(24);
    frames(&mut app);
    assert_eq!(size(app.world(), bar), Vec2::new(500.0, 24.0));
    assert_eq!(size(app.world(), track), Vec2::new(498.0, 22.0));
    assert!((size(app.world(), clip).x - 498.0 * 0.65).abs() <= 1.0);
    assert!((size(app.world(), fill).x - 498.0).abs() <= 1.0);
    assert_eq!(
        app.world().get::<Node>(track).unwrap().overflow,
        Overflow::clip()
    );
}

#[test]
fn themes_and_partial_overrides_refresh_without_resetting_state() {
    use bevy_widgetry_progress_bar::WidgetryProgressBarColorOverrides;
    use bevy_widgetry_theme::WidgetryThemeMode;
    let (mut app, _, bar) = fixture(0.65);
    let track = app.world().get::<Children>(bar).unwrap()[0];
    let clip = app.world().get::<Children>(track).unwrap()[0];
    let fill = app.world().get::<Children>(clip).unwrap()[0];
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        app.insert_resource(mode);
        WidgetryProgressBarColorOverrides::clear_in_world(app.world_mut(), bar).unwrap();
        frames(&mut app);
        let colors = &mode.colors().progress_bar;
        assert_eq!(
            app.world().get::<BackgroundColor>(track).unwrap().0,
            colors.track
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(fill).unwrap().0,
            colors.fill
        );
        assert_eq!(
            *app.world().get::<BorderColor>(bar).unwrap(),
            BorderColor::all(colors.border)
        );
        assert_ne!(colors.border, colors.track);
        assert_ne!(colors.border, colors.fill);
        let local = WidgetryProgressBarColorOverrides {
            fill: Some(Color::srgb(0.2, 0.8, 0.3)),
            ..default()
        };
        assert!(
            WidgetryProgressBarColorOverrides::set_in_world(app.world_mut(), bar, local.clone())
                .unwrap()
        );
        assert!(
            !WidgetryProgressBarColorOverrides::set_in_world(app.world_mut(), bar, local.clone())
                .unwrap()
        );
        frames(&mut app);
        assert_eq!(
            app.world().get::<BackgroundColor>(fill).unwrap().0,
            local.fill.unwrap()
        );
        assert_eq!(
            WidgetryProgressBar::state(app.world(), bar).unwrap(),
            WidgetryProgressState::Determinate { value: 0.65 }
        );
    }
    app.insert_resource(WidgetryThemeMode::Light);
    frames(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(track).unwrap().0,
        WidgetryThemeMode::Light.colors().progress_bar.track
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(fill).unwrap().0,
        Color::srgb(0.2, 0.8, 0.3)
    );
    WidgetryProgressBarColorOverrides::clear_in_world(app.world_mut(), bar).unwrap();
    frames(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(fill).unwrap().0,
        WidgetryThemeMode::Light.colors().progress_bar.fill
    );
}

#[test]
fn color_and_height_errors_preserve_valid_state_and_deferred_errors_reach_host() {
    use bevy_widgetry_progress_bar::WidgetryProgressBarColorOverrides;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let (mut app, _, bar) = fixture(0.65);
    app.set_error_handler(ErrorCapture::handler());
    let invalid = WidgetryProgressBarColorOverrides {
        track: Some(Color::srgba(f32::NAN, 0.0, 0.0, 1.0)),
        ..default()
    };
    let errors = ErrorCapture::default();
    let log = LogCapture::default();
    log.run(|| {
        assert!(
            WidgetryProgressBarColorOverrides::set_in_world(app.world_mut(), bar, invalid.clone())
                .is_err()
        );
        WidgetryProgressBarColorOverrides::set(&mut app.world_mut().commands(), bar, invalid);
        errors.run(|| app.world_mut().flush());
    });
    assert_eq!(log.records().len(), 2);
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(failures[0].to_string().contains("alpha"));
    assert_eq!(
        WidgetryProgressBarColorOverrides::get(app.world(), bar).unwrap(),
        &default()
    );
    let before = app.world().entities().len();
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            spawn_scene(
                app.world_mut(),
                bsn! { @WidgetryProgressBar { @height: height } }
            )
            .is_err()
        );
        assert_eq!(app.world().entities().len(), before);
    }
    let custom = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryProgressBar { @height: 24.0 } },
    )
    .unwrap();
    assert_eq!(app.world().get::<Node>(custom).unwrap().height, px(24));
}
