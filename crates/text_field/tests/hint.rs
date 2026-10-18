#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
};
use bevy_widgetry_text_field::{
    AutoSize, WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
    WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};
use bevy_widgetry_theme::WidgetryThemeMode;
use support::editing_app;

fn displays(world: &World, root: Entity) -> Vec<(Entity, &str)> {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .filter_map(|entity| {
            world
                .get::<Text>(entity)
                .map(|text| (entity, text.0.as_str()))
        })
        .collect()
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

#[test]
fn hint_is_optional_and_follows_the_frame_or_count_without_entering_content() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let absent = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryTextField})
        .unwrap()
        .id();
    assert!(displays(app.world(), absent).is_empty());
    for show in [false, true] {
        for hint in ["", "Always visible"] {
            let root = app
                .world_mut()
                .spawn_scene(bsn! {
                    @WidgetryTextField { @show_count: show, @hint_text: {Some(hint.to_owned())} }
                    Node {width:px(300)}
                })
                .unwrap()
                .id();
            settle(&mut app);
            let children = app.world().get::<Children>(root).unwrap();
            let display = displays(app.world(), root);
            assert_eq!(display.len(), if show { 2 } else { 1 });
            assert_eq!(display.last().unwrap().1, hint);
            assert_eq!(children.last().copied(), Some(display.last().unwrap().0));
            if show {
                assert_eq!(display[0].1, "0");
            }
            let entity = display.last().unwrap().0;
            assert!(!app.world().get::<Pickable>(entity).unwrap().is_hoverable);
            assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "");
            WidgetryTextField::set_text_in_world(app.world_mut(), root, "value").unwrap();
            app.update();
            assert_eq!(displays(app.world(), root).last().unwrap().1, hint);
        }
    }
}

#[test]
fn focus_preedit_readonly_and_disabled_keep_the_hint_visible() {
    use bevy::ui::InteractionDisabled;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    for readonly in [false, true] {
        let root = if readonly {
            app.world_mut().spawn_scene(bsn!{@WidgetryReadOnlyTextField {@hint_text:{Some("Persistent hint".to_owned())}}}).unwrap().id()
        } else {
            app.world_mut()
                .spawn_scene(
                    bsn! {@WidgetryTextField {@hint_text:{Some("Persistent hint".to_owned())}}},
                )
                .unwrap()
                .id()
        };
        settle(&mut app);
        if readonly {
            WidgetryReadOnlyTextField::focus_in_world(app.world_mut(), root).unwrap();
        } else {
            WidgetryTextField::focus_in_world(app.world_mut(), root).unwrap();
        }
        let editor = support::editor_entity(app.world(), root);
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .unwrap()
            .queue_edit(TextEdit::ImeSetCompose {
                value: "预编辑".into(),
                cursor: None,
            });
        app.update();
        assert_eq!(
            displays(app.world(), root).last().unwrap().1,
            "Persistent hint"
        );
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        app.update();
        let hint = displays(app.world(), root)[0].0;
        assert_eq!(
            app.world().get::<TextColor>(hint).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .text_field
                .hint_warning
        );
        assert_eq!(
            app.world().get::<Visibility>(hint),
            Some(&Visibility::Inherited)
        );
    }
}

#[test]
fn all_four_themes_refresh_warning_without_changing_editor_state() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root=app.world_mut().spawn_scene(bsn!{
        @WidgetryTextField { @value:{"selected".to_owned()}, @hint_text:{Some("Warning".to_owned())} }
    }).unwrap().id();
    settle(&mut app);
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
    app.update();
    let selection = WidgetryTextField::editor(app.world(), root)
        .unwrap()
        .editor()
        .raw_selection()
        .text_range();
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        *app.world_mut().resource_mut::<WidgetryThemeMode>() = mode;
        app.update();
        let (hint, text) = displays(app.world(), root)[0];
        assert_eq!(text, "Warning");
        assert_eq!(
            app.world().get::<TextColor>(hint).unwrap().0,
            mode.colors().text_field.hint_warning
        );
        assert_eq!(
            WidgetryTextField::text(app.world(), root).unwrap(),
            "selected"
        );
        assert_eq!(
            WidgetryTextField::editor(app.world(), root)
                .unwrap()
                .editor()
                .raw_selection()
                .text_range(),
            selection
        );
    }
}

#[test]
fn unbroken_urls_and_identifiers_wrap_within_the_available_width() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    for text in [
        "https://example.com/averylongunbrokensegment".repeat(4),
        "LongIdentifierWithoutAnySpaces".repeat(5),
    ] {
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryTextField {@hint_text:{Some(text.clone())}}
                Node {width:px(140)}
            })
            .unwrap()
            .id();
        settle(&mut app);
        let hint = displays(app.world(), root)[0].0;
        let node = app.world().get::<ComputedNode>(hint).unwrap();
        assert!(
            node.content_size().x <= node.size().x + 1.0,
            "content={:?}, available={:?}",
            node.content_size(),
            node.size()
        );
        assert!(node.size().y > 30.0);
        assert_eq!(displays(app.world(), root)[0].1, text);
    }
}

#[test]
fn owned_warning_colors_remain_owned_through_global_theme_updates() {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let owner = app.world_mut().spawn(Node::default()).id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField {@hint_text:{Some("Owned warning".to_owned())}}
            template(move |_|Ok(WidgetryStyleOwner::<WidgetryTextField>::new(owner)))
        })
        .unwrap()
        .id();
    let colors = WidgetryThemeMode::PinkDream.colors().text_field;
    bevy_widgetry_text_field::internal::apply_owned_text_field_colors(
        app.world_mut(),
        root,
        &colors,
    )
    .unwrap();
    *app.world_mut().resource_mut::<WidgetryThemeMode>() = WidgetryThemeMode::Light;
    app.update();
    let hint = displays(app.world(), root)[0].0;
    assert_eq!(
        app.world().get::<TextColor>(hint).unwrap().0,
        colors.hint_warning
    );
}

#[test]
fn all_five_features_keep_hint_wrapping_outside_auto_size_and_official_text() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let hint = "This is a long persistent warning that naturally wraps to multiple lines without adding any editor rows or any committed graphemes.";
    let mut roots = Vec::new();
    for enabled in [false, true] {
        roots.push(app.world_mut().spawn_scene(bsn! {
            @WidgetryTextField {
                @value:{"中e\u{301}👍🏽🇨🇳".to_owned()}, @max_length:{Some(3)}, @show_count:true,
                @multiline:true, @auto_size:{Some(AutoSize::default())},
                @hint_text:{enabled.then(||hint.to_owned())},
                @children:{Some(Box::new(bsn_list!{
                    WidgetryTextFieldSuffix Children[Text("suffix")]--WidgetryTextFieldPrefix Children[Text("prefix")]
                }) as Box<dyn SceneList>)},
            }
            Node {width:px(300)}
        }).unwrap().id());
    }
    settle(&mut app);
    let sizes: Vec<_> = roots
        .iter()
        .map(|&root| {
            app.world()
                .get::<ComputedNode>(support::editor_entity(app.world(), root))
                .unwrap()
                .size()
        })
        .collect();
    assert_eq!(sizes[0], sizes[1]);
    let root = roots[1];
    let display = displays(app.world(), root);
    assert_eq!(display[0].1, "3 / 3");
    assert_eq!(display[1].1, hint);
    assert_eq!(
        WidgetryTextField::text(app.world(), root).unwrap(),
        "中e\u{301}👍🏽"
    );
    let (_, initial_height) = (
        display[1].0,
        app.world()
            .get::<ComputedNode>(display[1].0)
            .unwrap()
            .size()
            .y,
    );
    assert!(initial_height > 30.0);
    let frame = support::frame_entity(app.world(), root);
    let frame_y = app
        .world()
        .get::<UiGlobalTransform>(frame)
        .unwrap()
        .translation
        .y;
    let count_y = app
        .world()
        .get::<UiGlobalTransform>(display[0].0)
        .unwrap()
        .translation
        .y;
    let hint_y = app
        .world()
        .get::<UiGlobalTransform>(display[1].0)
        .unwrap()
        .translation
        .y;
    assert!(frame_y < count_y && count_y < hint_y);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(220);
    settle(&mut app);
    let hint_entity = displays(app.world(), root)[1].0;
    assert!(
        app.world()
            .get::<ComputedNode>(hint_entity)
            .unwrap()
            .size()
            .y
            > initial_height
    );
    assert_eq!(
        WidgetryTextField::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .extend([
            TextEdit::SelectAll,
            TextEdit::ImeSetCompose {
                value: "input method preedit".into(),
                cursor: None,
            },
        ]);
    app.update();
    assert_eq!(displays(app.world(), root)[1].1, hint);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::ImeCommit {
            value: "甲乙丙丁".into(),
        });
    settle(&mut app);
    assert_eq!(
        WidgetryTextField::text(app.world(), root).unwrap(),
        "甲乙丙"
    );
    assert_eq!(displays(app.world(), root)[0].1, "3 / 3");
}
