//! State：内容标记、显式覆盖、最近 foreground scope、有效 Disabled 与 Theme。
//! Stimuli：构造、reparent、移除 scope/标记、覆盖替换/清除和 Theme 切换。
//! Invariants：原生 Text 不被修改，局部 Disabled 不消费启用来源，同值不写输出。
//! Couplings：scope identity 与 hierarchy、Disabled 与内容 fallback。

// 测试需要在 contract 不满足时立即失败，生产 panic lint 不适用于断言。
#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::text::{WidgetryTextColorOverrides, WidgetryTextStateColorOverrides};
use bevy_widgetry_core::{
    foreground::ResolvedForeground, text::WidgetryText, ui::WidgetryUiPlugin,
};
use bevy_widgetry_theme::WidgetryThemeMode;

#[test]
fn text_marker_supports_component_and_scene_construction_without_errors() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    let (direct, scene) = logs.run(|| {
        let direct = app
            .world_mut()
            .spawn((Text::new("direct"), WidgetryText))
            .id();
        let scene = app
            .world_mut()
            .spawn_scene(bsn! {
                Text("scene")
                @WidgetryText { @colors: {WidgetryTextColorOverrides {
                    normal: WidgetryTextStateColorOverrides { foreground: Some(Color::NONE) },
                    ..default()
                }} }
            })
            .unwrap()
            .id();
        app.world_mut().flush();
        app.update();
        (direct, scene)
    });
    assert!(
        !logs
            .records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR)
    );
    assert_eq!(
        app.world().get::<TextColor>(direct).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.normal.foreground
    );
    assert_eq!(app.world().get::<TextColor>(scene).unwrap().0, Color::NONE);
}

#[test]
fn managed_content_inherits_while_native_text_keeps_its_color() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let parent = app
        .world_mut()
        .spawn((Node::default(), ResolvedForeground(Color::WHITE)))
        .id();
    let branch = app
        .world_mut()
        .spawn((Node::default(), ChildOf(parent)))
        .id();
    let managed = app
        .world_mut()
        .spawn((Text::new("managed"), WidgetryText, ChildOf(branch)))
        .id();
    let native = app
        .world_mut()
        .spawn((
            Text::new("native"),
            TextColor(Color::BLACK),
            ChildOf(branch),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(managed).unwrap().0,
        Color::WHITE
    );
    assert_eq!(
        app.world().get::<TextColor>(native).unwrap().0,
        Color::BLACK
    );
    app.world_mut()
        .entity_mut(branch)
        .insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(managed).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.disabled.foreground
    );
    app.world_mut()
        .entity_mut(branch)
        .remove::<InteractionDisabled>();
    app.world_mut()
        .entity_mut(parent)
        .remove::<ResolvedForeground>();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(managed).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.normal.foreground
    );
    app.world_mut().entity_mut(managed).remove::<WidgetryText>();
    app.world_mut()
        .entity_mut(managed)
        .insert(TextColor(Color::BLACK));
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(managed).unwrap().0,
        Color::BLACK
    );
}

#[test]
fn overrides_are_atomic_state_specific_and_clear_to_current_theme() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let text = app
        .world_mut()
        .spawn((Text::new("value"), WidgetryText))
        .id();
    let normal = WidgetryTextColorOverrides {
        normal: WidgetryTextStateColorOverrides {
            foreground: Some(Color::NONE),
        },
        ..default()
    };
    assert!(
        WidgetryTextColorOverrides::set_in_world(app.world_mut(), text, normal.clone()).unwrap()
    );
    assert!(
        !WidgetryTextColorOverrides::set_in_world(app.world_mut(), text, normal.clone()).unwrap()
    );
    app.update();
    assert_eq!(app.world().get::<TextColor>(text).unwrap().0, Color::NONE);
    app.world_mut().entity_mut(text).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.disabled.foreground
    );
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
    app.world_mut()
        .entity_mut(text)
        .remove::<InteractionDisabled>();
    WidgetryTextColorOverrides::clear_in_world(app.world_mut(), text).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        WidgetryThemeMode::Light.colors().text.normal.foreground
    );
    let invalid = WidgetryTextColorOverrides {
        normal: WidgetryTextStateColorOverrides {
            foreground: Some(Color::linear_rgba(1.0, 0.0, 0.0, 2.0)),
        },
        ..default()
    };
    assert!(WidgetryTextColorOverrides::set_in_world(app.world_mut(), text, invalid).is_err());
    assert_eq!(
        *WidgetryTextColorOverrides::get(app.world(), text).unwrap(),
        WidgetryTextColorOverrides::default()
    );
}

#[test]
fn nearest_scope_reparent_and_rich_text_keep_source_identity() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let a = app
        .world_mut()
        .spawn((Node::default(), ResolvedForeground(Color::WHITE)))
        .id();
    let b = app
        .world_mut()
        .spawn((Node::default(), ResolvedForeground(Color::BLACK)))
        .id();
    let text = app
        .world_mut()
        .spawn((Text::new("root"), WidgetryText, ChildOf(a)))
        .id();
    let span = app
        .world_mut()
        .spawn((TextSpan::new("span"), WidgetryText, ChildOf(text)))
        .id();
    app.update();
    assert_eq!(app.world().get::<TextColor>(span).unwrap().0, Color::WHITE);
    WidgetryText::set_color_in_world(app.world_mut(), text, Color::NONE).unwrap();
    app.world_mut().entity_mut(text).insert(ChildOf(b));
    app.update();
    assert_eq!(app.world().get::<TextColor>(text).unwrap().0, Color::NONE);
    assert_eq!(app.world().get::<TextColor>(span).unwrap().0, Color::BLACK);
    app.world_mut().entity_mut(b).remove::<ResolvedForeground>();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(span).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.normal.foreground
    );
}

#[test]
fn disabled_spans_use_ui_context_and_text2d_is_not_managed() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let root = app
        .world_mut()
        .spawn((Text::new("root"), WidgetryText, InteractionDisabled))
        .id();
    let span = app
        .world_mut()
        .spawn((TextSpan::new("span"), WidgetryText, ChildOf(root)))
        .id();
    let native = app
        .world_mut()
        .spawn((
            TextSpan::new("native"),
            TextColor(Color::WHITE),
            ChildOf(root),
        ))
        .id();
    let world_text = app
        .world_mut()
        .spawn((Text2d::new("2d"), WidgetryText, TextColor(Color::WHITE)))
        .id();
    let world_span = app
        .world_mut()
        .spawn((
            TextSpan::new("2d span"),
            WidgetryText,
            TextColor(Color::WHITE),
            ChildOf(world_text),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(span).unwrap().0,
        WidgetryThemeMode::Dark.colors().text.disabled.foreground
    );
    for entity in [native, world_text, world_span] {
        assert_eq!(
            app.world().get::<TextColor>(entity).unwrap().0,
            Color::WHITE
        );
    }
}

#[test]
fn queued_colors_validate_at_execution_and_hdr_is_allowed() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let text = app
        .world_mut()
        .spawn((Text::new("value"), WidgetryText))
        .id();
    let ordinary = app.world_mut().spawn_empty().id();
    assert!(WidgetryTextColorOverrides::get(app.world(), ordinary).is_err());
    let hdr = Color::linear_rgba(4.0, -0.25, 2.0, 0.5);
    for invalid in [
        Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0),
        Color::linear_rgba(0.0, f32::INFINITY, 0.0, 1.0),
        Color::linear_rgba(0.0, 0.0, 0.0, -0.01),
    ] {
        let mut colors = WidgetryTextColorOverrides::default();
        colors.normal.foreground = Some(hdr);
        colors.disabled.foreground = Some(invalid);
        assert!(WidgetryTextColorOverrides::set_in_world(app.world_mut(), text, colors).is_err());
        assert_eq!(
            WidgetryTextColorOverrides::get(app.world(), text).unwrap(),
            &WidgetryTextColorOverrides::default()
        );
    }
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryText::set_color(&mut Commands::new(&mut queue, app.world()), text, hdr);
    assert_eq!(
        WidgetryTextColorOverrides::get(app.world(), text).unwrap(),
        &WidgetryTextColorOverrides::default()
    );
    queue.apply(app.world_mut());
    app.update();
    assert_eq!(app.world().get::<TextColor>(text).unwrap().0, hdr);
    app.world_mut().entity_mut(text).remove::<WidgetryText>();
    assert!(WidgetryTextColorOverrides::get(app.world(), text).is_err());
    assert!(WidgetryTextColorOverrides::clear_in_world(app.world_mut(), text).is_err());
}

#[test]
fn sources_with_equal_colors_still_track_identity_and_new_ui_text() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let a = app
        .world_mut()
        .spawn((Node::default(), ResolvedForeground(Color::WHITE)))
        .id();
    let b = app
        .world_mut()
        .spawn((Node::default(), ResolvedForeground(Color::WHITE)))
        .id();
    let content = app
        .world_mut()
        .spawn((Node::default(), WidgetryText, ChildOf(a)))
        .id();
    app.update();
    app.world_mut()
        .entity_mut(content)
        .insert((Text::new("late"), ChildOf(b)));
    app.update();
    assert_eq!(
        app.world()
            .get::<bevy_widgetry_core::foreground::InheritedForeground>(content)
            .unwrap()
            .source,
        b
    );
    assert_eq!(
        app.world().get::<TextColor>(content).unwrap().0,
        Color::WHITE
    );
    app.world_mut().entity_mut(b).despawn();
    assert!(app.world().get_entity(content).is_err());
    app.update();
}
