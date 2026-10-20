#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input_focus::{FocusCause, InputFocus, InputFocusVisible},
    picking::pointer::*,
    prelude::*,
    ui::{InteractionDisabled, UiGlobalTransform},
};
use bevy_widgetry_slider::{
    WidgetrySlider, WidgetrySliderColorOverrides, WidgetrySliderOrientation, WidgetrySliderPlugin,
    WidgetrySliderStateColorOverrides,
};
use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};
use bevy_widgetry_theme::WidgetryThemeMode;

fn fixture(
    vertical: bool,
    diameter: f32,
    thickness: f32,
    dpi: f32,
    ui_scale: f32,
) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins(WidgetrySliderPlugin)
        .insert_resource(UiScale(ui_scale));
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(800, 800)
                    .with_scale_factor_override(dpi),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(800), dpi);
    let orientation = if vertical {
        WidgetrySliderOrientation::Vertical
    } else {
        WidgetrySliderOrientation::Horizontal
    };
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetrySlider { @max:10.0, @value:{Some(5.0)}, @orientation:orientation, @thumb_diameter:diameter, @track_thickness:thickness }
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40)}
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    let id = pointer_ids()[1];
    app.world_mut().spawn(id);
    app.update();
    app.update();
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::ZERO,
    };
    (app, root, window, at)
}

fn parts(world: &World, root: Entity) -> [Entity; 3] {
    let track = world.get::<Children>(root).unwrap()[0];
    let children = world.get::<Children>(track).unwrap();
    [track, children[0], children[1]]
}
fn size(world: &World, entity: Entity) -> Vec2 {
    let node = world.get::<ComputedNode>(entity).unwrap();
    node.size() * node.inverse_scale_factor
}
fn center(world: &World, entity: Entity) -> Vec2 {
    world.get::<UiGlobalTransform>(entity).unwrap().translation
}
fn frames(app: &mut App) {
    app.update();
    app.update();
}
fn moved(app: &mut App, at: &mut Location, position: Vec2) {
    let delta = position - at.position;
    at.position = position;
    queue_pointer(
        app,
        pointer_ids()[1],
        at.clone(),
        PointerAction::Move { delta },
    );
    frames(app);
}
fn press(app: &mut App, at: &Location, action: PointerAction) {
    queue_pointer(app, pointer_ids()[1], at.clone(), action);
    frames(app);
}

#[test]
fn bsn_geometry_projects_endpoints_and_vertical_fill_from_the_bottom() {
    for vertical in [false, true] {
        for (diameter, thickness) in [(20.0, 6.0), (28.0, 10.0)] {
            let (mut app, root, _, _) = fixture(vertical, diameter, thickness, 1.0, 1.0);
            let [track, fill, thumb] = parts(app.world(), root);
            assert_eq!(size(app.world(), thumb), Vec2::splat(diameter));
            assert_eq!(
                if vertical {
                    size(app.world(), track).x
                } else {
                    size(app.world(), track).y
                },
                thickness
            );
            for value in [0.0, 2.5, 10.0] {
                WidgetrySlider::set_value_in_world(app.world_mut(), root, value).unwrap();
                frames(&mut app);
                let (track_center, thumb_center) =
                    (center(app.world(), track), center(app.world(), thumb));
                let length = if vertical {
                    size(app.world(), track).y
                } else {
                    size(app.world(), track).x
                };
                let expected = (value / 10.0 - 0.5) * length;
                assert!(
                    (if vertical {
                        track_center.y - thumb_center.y
                    } else {
                        thumb_center.x - track_center.x
                    } - expected)
                        .abs()
                        < 0.02
                );
                let fill_length = if vertical {
                    size(app.world(), fill).y
                } else {
                    size(app.world(), fill).x
                };
                assert!((fill_length - length * value / 10.0).abs() < 1.1);
                let fill_center = center(app.world(), fill);
                if vertical {
                    assert!(
                        (fill_center.y + fill_length * 0.5 - (track_center.y + length * 0.5)).abs()
                            < 0.02
                    );
                }
            }
            if vertical {
                app.world_mut().get_mut::<Node>(root).unwrap().height = px(320);
            } else {
                app.world_mut().get_mut::<Node>(root).unwrap().width = px(320);
            }
            frames(&mut app);
            assert_eq!(size(app.world(), thumb), Vec2::splat(diameter));
            assert_eq!(
                if vertical {
                    size(app.world(), track).y
                } else {
                    size(app.world(), track).x
                },
                320.0 - diameter
            );
        }
    }
}

#[test]
fn actual_thumb_geometry_and_root_length_keep_input_aligned_at_scale() {
    for vertical in [false, true] {
        for (dpi, ui_scale) in [(1.0, 1.0), (1.5, 1.0), (1.5, 1.5)] {
            let (mut app, root, _, mut at) = fixture(vertical, 28.0, 10.0, dpi, ui_scale);
            if vertical {
                app.world_mut().get_mut::<Node>(root).unwrap().height = px(320);
            } else {
                app.world_mut().get_mut::<Node>(root).unwrap().width = px(320);
            }
            frames(&mut app);
            let [_, _, thumb] = parts(app.world(), root);
            let original = center(app.world(), thumb) / dpi;
            let offset = if vertical {
                Vec2::new(0.0, 9.0 * ui_scale)
            } else {
                Vec2::new(9.0 * ui_scale, 0.0)
            };
            moved(&mut app, &mut at, original + offset);
            press(&mut app, &at, PointerAction::Press(PointerButton::Primary));
            assert_eq!(WidgetrySlider::value(app.world(), root).unwrap(), 5.0);
            let delta = (320.0 - 28.0) * 0.2 * ui_scale;
            let target = original
                + offset
                + if vertical {
                    Vec2::new(0.0, -delta)
                } else {
                    Vec2::new(delta, 0.0)
                };
            moved(&mut app, &mut at, target);
            assert!((WidgetrySlider::value(app.world(), root).unwrap() - 7.0).abs() < 0.001);
            press(
                &mut app,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
        }
    }
}

#[test]
fn themes_overrides_and_interaction_feedback_preserve_geometry() {
    let (mut app, root, _, mut at) = fixture(false, 20.0, 6.0, 1.0, 1.0);
    let [track, fill, thumb] = parts(app.world(), root);
    let original_size = size(app.world(), thumb);
    let original_center = center(app.world(), thumb);
    let fixed = Color::srgb(0.2, 0.8, 0.4);
    let overrides = WidgetrySliderColorOverrides {
        normal: WidgetrySliderStateColorOverrides {
            thumb: Some(fixed),
            ..default()
        },
        ..default()
    };
    assert!(
        WidgetrySliderColorOverrides::set_in_world(app.world_mut(), root, overrides.clone())
            .unwrap()
    );
    assert!(!WidgetrySliderColorOverrides::set_in_world(app.world_mut(), root, overrides).unwrap());
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        frames(&mut app);
        assert_eq!(app.world().get::<BackgroundColor>(thumb).unwrap().0, fixed);
        assert_eq!(
            app.world().get::<BackgroundColor>(track).unwrap().0,
            mode.colors().slider.normal.track
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(fill).unwrap().0,
            mode.colors().slider.normal.fill
        );
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        frames(&mut app);
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            mode.colors().slider.disabled.thumb
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<InteractionDisabled>();
        frames(&mut app);
    }
    WidgetrySliderColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    frames(&mut app);
    let colors = WidgetryThemeMode::KamuriViolet.colors().slider;
    moved(&mut app, &mut at, original_center);
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb).unwrap().0,
        colors.hovered.thumb
    );
    press(&mut app, &at, PointerAction::Press(PointerButton::Primary));
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb).unwrap().0,
        colors.pressed.thumb
    );
    press(
        &mut app,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    frames(&mut app);
    assert_eq!(
        app.world().get::<Outline>(thumb).unwrap().color,
        colors.hovered.focus_ring
    );
    assert_eq!(size(app.world(), thumb), original_size);
    assert_eq!(center(app.world(), thumb), original_center);
}

#[test]
fn invalid_visual_props_fail_without_retaining_failed_roots() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let before = app.world().entities().len();
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! {@WidgetrySlider{@thumb_diameter:value}}
            )
            .is_err()
        );
        assert!(
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! {@WidgetrySlider{@track_thickness:value}}
            )
            .is_err()
        );
        assert_eq!(app.world().entities().len(), before);
    }
}

#[test]
fn color_commands_validate_at_execution_and_failed_requests_preserve_overrides() {
    let (mut app, root, _, _) = fixture(false, 20.0, 6.0, 1.0, 1.0);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let fixed = WidgetrySliderColorOverrides {
        normal: WidgetrySliderStateColorOverrides {
            fill: Some(Color::srgb(0.1, 0.7, 0.4)),
            ..default()
        },
        ..default()
    };
    let invalid = WidgetrySliderColorOverrides {
        normal: WidgetrySliderStateColorOverrides {
            thumb: Some(Color::srgba(0.1, 0.2, 0.3, 2.0)),
            ..default()
        },
        ..default()
    };
    let bare = app.world_mut().spawn_empty().id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetrySliderColorOverrides::set(&mut commands, root, fixed.clone());
        WidgetrySliderColorOverrides::set(&mut commands, root, invalid);
        WidgetrySliderColorOverrides::clear(&mut commands, bare);
    }
    assert_eq!(
        WidgetrySliderColorOverrides::get(app.world(), root).unwrap(),
        &WidgetrySliderColorOverrides::default()
    );
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert_eq!(
        WidgetrySliderColorOverrides::get(app.world(), root).unwrap(),
        &fixed
    );
    assert!(WidgetrySliderColorOverrides::get(app.world(), bare).is_err());
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetrySliderColorOverrides::clear(&mut Commands::new(&mut queue, app.world()), root);
    queue.apply(app.world_mut());
    assert_eq!(
        WidgetrySliderColorOverrides::get(app.world(), root).unwrap(),
        &WidgetrySliderColorOverrides::default()
    );
}

#[test]
fn track_click_uses_the_actual_center_after_fractional_scale_layout_rounding() {
    for vertical in [false, true] {
        let (mut app, root, _, mut at) = fixture(vertical, 20.0, 6.0, 1.25, 1.0);
        WidgetrySlider::set_value_in_world(app.world_mut(), root, 0.0).unwrap();
        frames(&mut app);
        let [track, _, _] = parts(app.world(), root);
        let midpoint = center(app.world(), track) / 1.25;
        moved(&mut app, &mut at, midpoint);
        press(&mut app, &at, PointerAction::Press(PointerButton::Primary));
        assert!((WidgetrySlider::value(app.world(), root).unwrap() - 5.0).abs() < 0.00001);
    }
}

#[test]
fn track_endpoint_reaches_max_even_when_the_domain_spans_very_different_magnitudes() {
    let (mut app, other, _, mut at) = fixture(false, 20.0, 6.0, 1.0, 1.0);
    let camera = app
        .world()
        .get::<bevy::ui::ComputedUiTargetCamera>(other)
        .unwrap()
        .get()
        .unwrap();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySlider {@min:-1e20_f32,@max:1.0}
            Node {position_type:PositionType::Absolute,left:px(40),top:px(400)}
            template(move |_| Ok(UiTargetCamera(camera)))
        })
        .unwrap()
        .id();
    frames(&mut app);
    let [track, _, _] = parts(app.world(), root);
    let endpoint = center(app.world(), track) + Vec2::new(size(app.world(), track).x * 0.5, 0.0);
    moved(&mut app, &mut at, endpoint);
    press(&mut app, &at, PointerAction::Press(PointerButton::Primary));
    assert_eq!(WidgetrySlider::value(app.world(), root).unwrap(), 1.0);
}
