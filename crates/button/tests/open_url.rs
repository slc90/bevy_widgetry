#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry_button::{
    OpenUrlError, WidgetryButtonHeadless, WidgetryButtonPlugin, WidgetryOpenUrl,
};
use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

#[derive(Resource, Default)]
struct Activations(usize);

#[test]
fn custom_link_uses_only_behavior_and_preserves_application_content() {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin)
        .init_resource::<Activations>();
    app.add_observer(|_: On<Activate>, mut count: ResMut<Activations>| count.0 += 1);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButtonHeadless
            @WidgetryOpenUrl { @url: {"https://example.com".to_owned()} }
            Children [Text("Help card")]
        })
        .unwrap()
        .id();
    let label = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().entity_mut(root).remove::<ComputedNode>();
    for raw in [
        "HTTPS://Example.com/原文?q=%20#Top",
        "http://localhost:8080/help",
        "https://[::1]:8443/a?b=c#d",
    ] {
        assert!(WidgetryOpenUrl::set_url_in_world(app.world_mut(), root, raw).unwrap());
        assert_eq!(WidgetryOpenUrl::url(app.world(), root).unwrap(), raw);
        assert!(!WidgetryOpenUrl::set_url_in_world(app.world_mut(), root, raw).unwrap());
        app.update();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Help card");
        assert_eq!(app.world().get::<Children>(root).unwrap().len(), 1);
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            Color::NONE
        );
    }
    WidgetryOpenUrl::set_url(&mut app.world_mut().commands(), root, "https://bevy.org");
    assert_eq!(
        WidgetryOpenUrl::url(app.world(), root).unwrap(),
        "https://[::1]:8443/a?b=c#d"
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetryOpenUrl::url(app.world(), root).unwrap(),
        "https://bevy.org"
    );
    assert_eq!(app.world().resource::<Activations>().0, 0);
    assert_eq!(app.world().get::<Text>(label).unwrap().0, "Help card");
}

#[test]
fn invalid_capabilities_and_queued_updates_preserve_the_url() {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryOpenUrl { @url: {"https://example.com".to_owned()} }
        })
        .unwrap()
        .id();
    assert!(app.world().get::<Children>(root).is_none());
    let plain = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButtonHeadless })
        .unwrap()
        .id();
    assert!(
        matches!(WidgetryOpenUrl::set_url_in_world(app.world_mut(), plain, "https://bevy.org").unwrap_err().kind(), OpenUrlError::InvalidTarget { root } if *root == plain)
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<WidgetryButtonHeadless>();
    assert!(matches!(
        WidgetryOpenUrl::set_url_in_world(app.world_mut(), root, "https://bevy.org")
            .unwrap_err()
            .kind(),
        OpenUrlError::MissingPart {
            part: "WidgetryButtonHeadless",
            ..
        }
    ));
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryButtonHeadless);
    assert_eq!(
        WidgetryOpenUrl::url(app.world(), root).unwrap(),
        "https://example.com"
    );
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    WidgetryOpenUrl::set_url(&mut app.world_mut().commands(), root, "file:///C:/invalid");
    assert!(errors.take_records().is_empty());
    errors.run(|| app.world_mut().flush());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert_eq!(
        WidgetryOpenUrl::url(app.world(), root).unwrap(),
        "https://example.com"
    );
    assert!(app.world().get::<Children>(root).is_none());
}
