#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{cursor::EntityCursor, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, UiGlobalTransform},
    window::SystemCursorIcon,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_splitter::{
    WidgetrySplitter, WidgetrySplitterColorOverrides as Colors,
    WidgetrySplitterOrientation as Orientation, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, picking_app, queue_pointer, spawn_picking_camera,
};
use bevy_widgetry_theme::WidgetryThemeMode;

fn frames(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

fn fixture(vertical: bool, dpi: f32, thickness: f32, line: f32) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(1200), dpi);
    let root = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetrySplitter {
                @orientation: {if vertical {Orientation::Vertical} else {Orientation::Horizontal}},
                @handle_thickness: thickness, @line_thickness: line,
                @pane_a: bsn! { BackgroundColor(Color::srgb(0.12,0.25,0.37)) },
            }
            Node {width:px(408),height:px(208)}
            template(move |_| Ok(UiTargetCamera(camera)))
        },
    )
    .unwrap();
    frames(&mut app);
    let handle = app.world().get::<Children>(root).unwrap()[1];
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: app
            .world()
            .get::<UiGlobalTransform>(handle)
            .unwrap()
            .translation
            / dpi,
    };
    (app, root, handle, at)
}

fn line(world: &World, handle: Entity) -> Entity {
    world.get::<Children>(handle).unwrap()[0]
}
fn color(world: &World, handle: Entity) -> Color {
    world.get::<BackgroundColor>(line(world, handle)).unwrap().0
}
fn send(app: &mut App, at: &Location, action: PointerAction) {
    queue_pointer(app, PointerId::Mouse, at.clone(), action);
    app.update();
}

#[test]
fn centered_line_and_hit_region_keep_independent_sizes_in_both_axes_and_dpi() {
    for vertical in [false, true] {
        for dpi in [1.0, 1.25, 1.5] {
            for (thickness, visible) in [(8.0, 2.0), (12.0, 4.0)] {
                let (mut app, root, handle, at) = fixture(vertical, dpi, thickness, visible);
                let visual = line(app.world(), handle);
                let h = app.world().get::<ComputedNode>(handle).unwrap().size() / dpi;
                let l = app.world().get::<ComputedNode>(visual).unwrap().size() / dpi;
                let axis = if vertical { 1 } else { 0 };
                assert!((h[axis] - thickness).abs() <= 1.0 / dpi);
                assert!((l[axis] - visible).abs() <= 1.0 / dpi);
                assert!((h[1 - axis] - l[1 - axis]).abs() <= 1.0 / dpi);
                let center = app
                    .world()
                    .get::<UiGlobalTransform>(visual)
                    .unwrap()
                    .translation;
                assert!((center / dpi - at.position).length() <= 1.0 / dpi);
                assert_eq!(
                    *app.world().get::<Pickable>(visual).unwrap(),
                    Pickable::IGNORE
                );
                send(&mut app, &at, PointerAction::Move { delta: Vec2::ZERO });
                send(&mut app, &at, PointerAction::Press(PointerButton::Primary));
                assert_eq!(
                    app.world().get::<ComputedNode>(handle).unwrap().size() / dpi,
                    h
                );
                assert_eq!(
                    WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
                    0.5
                );
                send(
                    &mut app,
                    &at,
                    PointerAction::Release(PointerButton::Primary),
                );
            }
        }
    }
}

#[test]
fn all_themes_follow_real_hover_drag_disabled_and_restore_without_geometry_changes() {
    for vertical in [false, true] {
        let (mut app, root, handle, at) = fixture(vertical, 1.0, 8.0, 2.0);
        let pane = app.world().get::<Children>(root).unwrap()[0];
        let original = *app.world().get::<BackgroundColor>(pane).unwrap();
        for mode in [
            WidgetryThemeMode::Light,
            WidgetryThemeMode::Dark,
            WidgetryThemeMode::PinkDream,
            WidgetryThemeMode::KamuriViolet,
        ] {
            app.insert_resource(mode);
            let outside = Location {
                position: Vec2::splat(900.0),
                ..at.clone()
            };
            send(
                &mut app,
                &outside,
                PointerAction::Move { delta: Vec2::ZERO },
            );
            assert_eq!(color(app.world(), handle), mode.colors().splitter.normal);
            send(&mut app, &at, PointerAction::Move { delta: Vec2::ZERO });
            assert_eq!(color(app.world(), handle), mode.colors().splitter.hovered);
            assert!(
                matches!(app.world().get::<EntityCursor>(handle),Some(EntityCursor::System(c)) if *c == if vertical {SystemCursorIcon::NsResize} else {SystemCursorIcon::EwResize})
            );
            send(&mut app, &at, PointerAction::Press(PointerButton::Primary));
            assert_eq!(color(app.world(), handle), mode.colors().splitter.dragging);
            WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, false).unwrap();
            app.update();
            assert_eq!(color(app.world(), handle), mode.colors().splitter.disabled);
            assert!(app.world().get::<EntityCursor>(handle).is_none());
            send(
                &mut app,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            WidgetrySplitter::set_resizable_in_world(app.world_mut(), root, true).unwrap();
            app.update();
            assert_eq!(color(app.world(), handle), mode.colors().splitter.hovered);
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
            app.update();
            assert_eq!(color(app.world(), handle), mode.colors().splitter.disabled);
            assert!(app.world().get::<EntityCursor>(handle).is_none());
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
            app.update();
            assert_eq!(color(app.world(), handle), mode.colors().splitter.hovered);
            send(&mut app, &at, PointerAction::Press(PointerButton::Primary));
            send(&mut app, &at, PointerAction::Cancel);
            assert_ne!(color(app.world(), handle), mode.colors().splitter.dragging);
            assert_eq!(*app.world().get::<BackgroundColor>(pane).unwrap(), original);
            assert_eq!(
                WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
                0.5
            );
        }
    }
}

#[test]
fn partial_color_overrides_follow_theme_and_commands_validate_at_execution() {
    let (mut app, root, handle, at) = fixture(false, 1.0, 8.0, 2.0);
    let local = Colors {
        normal: Some(Color::srgb(0.2, 0.8, 0.3)),
        ..default()
    };
    assert!(Colors::set_in_world(app.world_mut(), root, local.clone()).unwrap());
    assert!(!Colors::set_in_world(app.world_mut(), root, local.clone()).unwrap());
    assert_eq!(Colors::get(app.world(), root).unwrap(), &local);
    app.update();
    assert_eq!(color(app.world(), handle), local.normal.unwrap());
    app.insert_resource(WidgetryThemeMode::Light);
    app.update();
    assert_eq!(color(app.world(), handle), local.normal.unwrap());
    send(&mut app, &at, PointerAction::Move { delta: Vec2::ZERO });
    assert_eq!(
        color(app.world(), handle),
        WidgetryThemeMode::Light.colors().splitter.hovered
    );
    let outside = Location {
        position: Vec2::splat(900.0),
        ..at
    };
    send(
        &mut app,
        &outside,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    assert_eq!(color(app.world(), handle), local.normal.unwrap());
    Colors::clear(&mut app.world_mut().commands(), root);
    assert_eq!(Colors::get(app.world(), root).unwrap(), &local);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        color(app.world(), handle),
        WidgetryThemeMode::Light.colors().splitter.normal
    );
    assert!(!Colors::clear_in_world(app.world_mut(), root).unwrap());
    Colors::set(&mut app.world_mut().commands(), root, local.clone());
    app.world_mut().flush();
    assert_eq!(Colors::get(app.world(), root).unwrap(), &local);
}

#[test]
fn invalid_sizes_colors_and_targets_report_errors_without_partial_updates() {
    let (mut app, root, _, _) = fixture(false, 1.0, 8.0, 2.0);
    app.set_error_handler(ErrorCapture::handler());
    let invalid = Colors {
        normal: Some(Color::srgba(f32::NAN, 0.0, 0.0, 1.0)),
        ..default()
    };
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(Colors::set_in_world(app.world_mut(), root, invalid.clone()).is_err());
        Colors::set(&mut app.world_mut().commands(), root, invalid);
        errors.run(|| app.world_mut().flush());
    });
    assert_eq!(logs.records().len(), 2);
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(Colors::get(app.world(), root).unwrap(), &Colors::default());
    assert!(Colors::get(app.world(), Entity::PLACEHOLDER).is_err());
    let before = app.world().entities().len();
    for visible in [0.0, -1.0, 9.0, f32::NAN, f32::INFINITY] {
        assert!(
            spawn_scene(
                app.world_mut(),
                bsn! {@WidgetrySplitter {@line_thickness:visible}}
            )
            .is_err()
        );
        assert_eq!(app.world().entities().len(), before);
    }
    let local = Colors {
        hovered: Some(Color::srgb(0.6, 0.2, 0.4)),
        ..default()
    };
    let custom = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetrySplitter {@colors:{local.clone()}}},
    )
    .unwrap();
    assert_eq!(Colors::get(app.world(), custom).unwrap(), &local);
}
