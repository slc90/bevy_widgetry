#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_collapsible::{
    WidgetryCollapsible, WidgetryCollapsibleColorOverrides, WidgetryCollapsiblePlugin,
};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_theme::WidgetryThemeMode;

fn parts(world: &World, root: Entity) -> (Entity, Entity, Entity) {
    let children = world.get::<Children>(root).unwrap();
    let header = children[0];
    (
        header,
        children[1],
        world.get::<Children>(header).unwrap()[1],
    )
}

fn icon(world: &World, button: Entity) -> Entity {
    let mut pending = vec![button];
    let mut found = None;
    while let Some(entity) = pending.pop() {
        if world.get::<WidgetryIcon>(entity).is_some() {
            found = Some(entity);
            break;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    found.unwrap()
}

#[test]
fn default_card_and_svg_follow_expansion_without_replacing_entities() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    for expanded in [true, false] {
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryCollapsible { @expanded: expanded } })
            .unwrap()
            .id();
        app.update();
        let (header, content, button) = parts(app.world(), root);
        assert_eq!(
            app.world().get::<Node>(root).unwrap().border,
            UiRect::all(px(1))
        );
        assert_eq!(
            app.world().get::<Node>(root).unwrap().border_radius,
            BorderRadius::all(px(6))
        );
        assert_eq!(app.world().get::<Node>(header).unwrap().height, Val::Auto);
        assert_eq!(
            app.world().get::<Node>(header).unwrap().padding,
            UiRect::axes(px(12), px(8))
        );
        assert_eq!(
            app.world().get::<Node>(content).unwrap().padding,
            UiRect::all(px(12))
        );
        assert_eq!(
            app.world().get::<Node>(content).unwrap().border,
            UiRect::top(px(1))
        );
        assert_eq!(app.world().get::<Node>(button).unwrap().width, px(32));
        assert_eq!(app.world().get::<Node>(button).unwrap().height, px(32));
        let arrow = icon(app.world(), button);
        assert_eq!(app.world().get::<Node>(arrow).unwrap().width, px(16));
        let expected = if expanded {
            BuiltinIcon::ChevronUp
        } else {
            BuiltinIcon::ChevronDown
        };
        assert_eq!(
            app.world().get::<WidgetryIcon>(arrow).unwrap().path(),
            Some(&expected.path())
        );
        for expanded in [!expanded, expanded] {
            WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, expanded).unwrap();
            let expected = if expanded {
                BuiltinIcon::ChevronUp
            } else {
                BuiltinIcon::ChevronDown
            };
            assert_eq!(
                app.world().get::<WidgetryIcon>(arrow).unwrap().path(),
                Some(&expected.path())
            );
            assert_eq!(parts(app.world(), root).1, content);
            assert_eq!(icon(app.world(), button), arrow);
            assert_eq!(
                app.world().get::<Node>(content).unwrap().display,
                if expanded {
                    Display::Flex
                } else {
                    Display::None
                }
            );
        }
    }
}

#[test]
fn disabled_arrow_inherits_button_foreground_without_changing_direction() {
    use bevy_widgetry_core::foreground::{InheritedForeground, ResolvedForeground};
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    app.update();
    let (_, _, button) = parts(app.world(), root);
    let arrow = icon(app.world(), button);
    for disabled in [true, false] {
        WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, disabled).unwrap();
        app.update();
        let inherited = app.world().get::<InheritedForeground>(arrow).unwrap();
        assert_eq!(inherited.source, button);
        assert_eq!(inherited.disabled, disabled);
        assert_eq!(
            inherited.color,
            app.world().get::<ResolvedForeground>(button).unwrap().0
        );
        assert_eq!(
            app.world().get::<WidgetryIcon>(arrow).unwrap().path(),
            Some(&BuiltinIcon::ChevronUp.path())
        );
    }
}

#[test]
fn invalid_colors_preserve_overrides_and_queued_updates_validate_at_execution() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    app.set_error_handler(ErrorCapture::handler());
    let capture = ErrorCapture::default();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    let colors = WidgetryCollapsibleColorOverrides {
        border: Some(Color::srgb(0.2, 0.3, 0.4)),
        ..default()
    };
    WidgetryCollapsibleColorOverrides::set(&mut app.world_mut().commands(), root, colors.clone());
    app.world_mut().flush();
    assert_eq!(
        WidgetryCollapsibleColorOverrides::get(app.world(), root).unwrap(),
        &colors
    );
    assert!(
        !WidgetryCollapsibleColorOverrides::set_in_world(app.world_mut(), root, colors.clone())
            .unwrap()
    );
    let invalid = WidgetryCollapsibleColorOverrides {
        border: Some(Color::srgba(0.0, 0.0, 0.0, f32::NAN)),
        ..default()
    };
    assert_eq!(
        WidgetryCollapsibleColorOverrides::set_in_world(app.world_mut(), root, invalid.clone())
            .unwrap_err()
            .severity(),
        bevy::ecs::error::Severity::Error
    );
    assert_eq!(
        WidgetryCollapsibleColorOverrides::get(app.world(), root).unwrap(),
        &colors
    );
    WidgetryCollapsibleColorOverrides::set(&mut app.world_mut().commands(), root, invalid);
    capture.run(|| app.world_mut().flush());
    assert_eq!(capture.take().len(), 1);
    assert_eq!(
        WidgetryCollapsibleColorOverrides::get(app.world(), root).unwrap(),
        &colors
    );
    assert!(WidgetryCollapsibleColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
    assert!(!WidgetryCollapsibleColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
    let wrong = app.world_mut().spawn_empty().id();
    assert!(WidgetryCollapsibleColorOverrides::get(app.world(), wrong).is_err());
}

#[test]
fn all_themes_and_local_overrides_only_change_managed_colors() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryCollapsible { @children: {Some(Box::new(bsn! { Node BackgroundColor(Color::srgb(0.13, 0.27, 0.39)) }) as Box<dyn SceneList>)} }
    }).unwrap().id();
    app.update();
    let (header, content, _) = parts(app.world(), root);
    let child = app.world().get::<Children>(content).unwrap()[0];
    let local = Color::srgb(0.17, 0.31, 0.59);
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        WidgetryCollapsibleColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
        app.update();
        let base = mode.colors().collapsible;
        assert_eq!(base.header_background, base.content_background);
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            base.background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(header).unwrap().0,
            base.header_background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(content).unwrap().0,
            base.content_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(base.border)
        );
        WidgetryCollapsibleColorOverrides::set_in_world(
            app.world_mut(),
            root,
            WidgetryCollapsibleColorOverrides {
                header_background: Some(local),
                separator: Some(local),
                ..default()
            },
        )
        .unwrap();
        app.update();
        assert_eq!(app.world().get::<BackgroundColor>(header).unwrap().0, local);
        assert_eq!(
            app.world().get::<BackgroundColor>(content).unwrap().0,
            base.content_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(content).unwrap(),
            BorderColor::all(local)
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(child).unwrap().0,
            Color::srgb(0.13, 0.27, 0.39)
        );
    }
}
