#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

use bevy::ecs::error::Severity;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Underline;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryLinkButtonColorOverrides;
use bevy_widgetry_button::{WidgetryButtonPlugin, WidgetryLinkButton};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_theme::WidgetryThemeMode;

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    app
}

fn text(world: &World, root: Entity) -> Entity {
    world.get::<Children>(root).unwrap()[0]
}

#[test]
fn url_updates_preserve_original_text_and_reject_invalid_input_atomically() {
    let mut app = app();
    let original = "HTTPS://Example.com/原文?q=%20#Top";
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryLinkButton { @url: {original.to_owned()} } })
        .unwrap()
        .id();
    app.update();
    let label = text(app.world(), root);
    assert_eq!(
        WidgetryLinkButton::url(app.world(), root).unwrap(),
        original
    );
    assert_eq!(app.world().get::<Text>(label).unwrap().0, original);
    assert!(!WidgetryLinkButton::set_url_in_world(app.world_mut(), root, original).unwrap());
    for invalid in [
        "",
        "example.com",
        "ftp://example.com",
        "javascript:alert(1)",
        "https://",
        "https://bad host",
        "https://example.com:99999",
        "https:example.com",
        "https://example.com/\0tail",
        " https://example.com",
        "https://example.com/\n",
        "https://example.com/%",
        "https://example.com/%GG",
    ] {
        let error =
            WidgetryLinkButton::set_url_in_world(app.world_mut(), root, invalid).unwrap_err();
        assert_eq!(error.severity(), Severity::Error);
        assert_eq!(
            WidgetryLinkButton::url(app.world(), root).unwrap(),
            original
        );
        assert_eq!(app.world().get::<Text>(label).unwrap().0, original);
    }
    assert!(
        WidgetryLinkButton::set_url_in_world(
            app.world_mut(),
            root,
            "http://localhost:8080/a?b=1&c=2"
        )
        .unwrap()
    );
    assert_eq!(
        app.world().get::<Text>(label).unwrap().0,
        "http://localhost:8080/a?b=1&c=2"
    );
    assert_eq!(
        WidgetryLinkButton::url(app.world(), root).unwrap(),
        app.world().get::<Text>(label).unwrap().0
    );
    let wrong = app.world_mut().spawn(Node::default()).id();
    assert!(WidgetryLinkButton::url(app.world(), wrong).is_err());
    assert!(WidgetryLinkButton::set_url_in_world(app.world_mut(), wrong, original).is_err());
    app.world_mut().despawn(root);
    assert!(WidgetryLinkButton::set_url_in_world(app.world_mut(), root, original).is_err());
}

#[test]
fn queued_setter_commits_when_commands_are_applied() {
    let mut app = app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryLinkButton { @url: {"https://www.baidu.com".to_owned()} } })
        .unwrap()
        .id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryLinkButton::set_url(
        &mut Commands::new(&mut queue, app.world()),
        root,
        "https://www.google.com",
    );
    assert_eq!(
        WidgetryLinkButton::url(app.world(), root).unwrap(),
        "https://www.baidu.com"
    );
    queue.apply(app.world_mut());
    assert_eq!(
        WidgetryLinkButton::url(app.world(), root).unwrap(),
        "https://www.google.com"
    );
    assert_eq!(
        app.world().get::<Text>(text(app.world(), root)).unwrap().0,
        "https://www.google.com"
    );
}

#[test]
fn long_url_wraps_without_truncating_content() {
    use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, spawn_ui_camera};
    let mut app = app();
    add_ui_plugins(&mut app);
    use bevy_widgetry_core::WidgetryAppExt;
    app.add_plugins(bevy_widgetry_asset::WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinFont::Default.path());
    app.set_default_font(bevy::text::FontSource::Handle(font));
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    let url = format!("https://example.com/{}?q=tail#end", "abcdefghij".repeat(12));
    let original = url.clone();
    let parent = app.world_mut().spawn_scene(bsn! { Node { width: px(120), flex_direction: FlexDirection::Column, align_items: AlignItems::Start } Children [@WidgetryLinkButton { @url: {url.clone()} }] }).unwrap().id();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    let label = text(app.world(), root);
    let outcome = advance_until(
        &mut app,
        std::time::Duration::from_secs(5),
        "wrapped URL layout",
        |world| {
            world
                .get::<ComputedNode>(label)
                .is_some_and(|node| node.size().y > 50.0 && node.size().x > 20.0)
        },
    );
    assert!(
        outcome.is_ok(),
        "{outcome:?}; root {:?}; text {:?}",
        app.world().get::<ComputedNode>(root),
        app.world().get::<ComputedNode>(label)
    );
    assert_eq!(app.world().get::<Text>(label).unwrap().0, original);
    assert!(app.world().get::<ComputedNode>(label).unwrap().size().x <= 120.5);
    assert_eq!(
        WidgetryLinkButton::url(app.world(), root).unwrap(),
        original
    );
}

#[test]
fn inherited_disabled_states_themes_and_overrides_preserve_url_and_layout() {
    let mut app = app();
    let parent = app.world_mut().spawn_scene(bsn! { Node Children [ @WidgetryLinkButton { @url: {"https://example.com".to_owned()} } Node { width: px(160) } ] }).unwrap().id();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    let label = text(app.world(), root);
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        for (hovered, pressed, disabled) in [
            (false, false, false),
            (true, false, false),
            (true, true, false),
            (false, true, false),
            (true, true, true),
            (true, false, false),
            (false, false, false),
        ] {
            app.world_mut().entity_mut(root).insert(Hovered(hovered));
            if pressed {
                app.world_mut().entity_mut(root).insert(Pressed);
            } else {
                app.world_mut().entity_mut(root).remove::<Pressed>();
            }
            if disabled {
                app.world_mut()
                    .entity_mut(parent)
                    .insert(InteractionDisabled);
            } else {
                app.world_mut()
                    .entity_mut(parent)
                    .remove::<InteractionDisabled>();
            }
            app.update();
            let c = mode.colors().link_button;
            let expected = if disabled {
                c.disabled
            } else if pressed {
                c.pressed
            } else if hovered {
                c.hovered
            } else {
                c.normal
            };
            assert_eq!(
                app.world().get::<ResolvedForeground>(root).unwrap().0,
                expected
            );
            assert_eq!(app.world().get::<TextColor>(label).unwrap().0, expected);
            assert_eq!(
                app.world().get::<Underline>(label).is_some(),
                !disabled && (hovered || pressed)
            );
            assert_eq!(
                app.world().get::<InteractionDisabled>(root).is_some(),
                disabled
            );
            assert_eq!(app.world().get::<Node>(root).unwrap().width, px(160));
            assert_eq!(
                app.world().get::<Text>(label).unwrap().0,
                WidgetryLinkButton::url(app.world(), root).unwrap()
            );
        }
    }
    let colors = WidgetryLinkButtonColorOverrides {
        hovered: Some(Color::BLACK),
        ..default()
    };
    assert!(
        WidgetryLinkButtonColorOverrides::set_in_world(app.world_mut(), root, colors.clone())
            .unwrap()
    );
    assert!(
        !WidgetryLinkButtonColorOverrides::set_in_world(app.world_mut(), root, colors.clone())
            .unwrap()
    );
    app.world_mut().entity_mut(root).insert(Hovered(true));
    app.update();
    assert_eq!(app.world().get::<TextColor>(label).unwrap().0, Color::BLACK);
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
    app.update();
    assert_eq!(app.world().get::<TextColor>(label).unwrap().0, Color::BLACK);
    let invalid = WidgetryLinkButtonColorOverrides {
        normal: Some(Color::srgba(f32::NAN, 0., 0., 1.)),
        ..default()
    };
    assert!(
        WidgetryLinkButtonColorOverrides::set_in_world(app.world_mut(), root, invalid).is_err()
    );
    assert_eq!(
        *WidgetryLinkButtonColorOverrides::get(app.world(), root).unwrap(),
        colors
    );
    WidgetryLinkButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        WidgetryThemeMode::Light.colors().link_button.hovered
    );
}

#[test]
fn deferred_errors_reach_host_handler_and_invalid_scenes_are_cleaned() {
    use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let logs = LogCapture::default();
    logs.run(|| {
        let mut app = app();
        let errors = ErrorCapture::default();
        app.set_error_handler(ErrorCapture::handler());
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryLinkButton { @url: {"https://example.com".to_owned()} } })
            .unwrap()
            .id();
        let label = text(app.world(), root);
        let count = app.world().entities().len();
        let invalid_root = app
            .world_mut()
            .commands()
            .spawn_scene_with_error_handler(
                bsn! { @WidgetryLinkButton { @url: {"file:///C:/secret".to_owned()} } },
            )
            .id();
        WidgetryLinkButton::set_url(&mut app.world_mut().commands(), root, "https://");
        errors.run(|| app.world_mut().flush());
        let captured = errors.take();
        assert_eq!(captured.len(), 2);
        assert!(
            captured
                .iter()
                .all(|error| error.severity() == Severity::Error)
        );
        assert!(
            captured
                .iter()
                .all(|error| error.to_string().contains("URL"))
        );
        assert!(app.world().get_entity(invalid_root).is_err());
        assert_eq!(app.world().entities().len(), count);
        assert_eq!(
            WidgetryLinkButton::url(app.world(), root).unwrap(),
            "https://example.com"
        );
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            "https://example.com"
        );
    });
    assert!(logs.records().iter().any(|record| {
        record.level == bevy::log::tracing::Level::ERROR
            && record
                .fields
                .get("message")
                .is_some_and(|message| message.contains("URL"))
    }));
}
