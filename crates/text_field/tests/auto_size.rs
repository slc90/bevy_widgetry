#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{
    prelude::*,
    text::{EditableText, LineHeight, TextEdit},
};
use bevy_widgetry_text_field::{
    AutoSize, WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
    WidgetryTextFieldPrefix,
};
use support::editing_app;

fn settle(app: &mut App, root: Entity) -> f32 {
    let editor = support::editor_entity(app.world(), root);
    let mut heights = Vec::new();
    for _ in 0..10 {
        app.update();
        heights.push(app.world().get::<ComputedNode>(editor).unwrap().size().y);
    }
    assert!(
        heights[6..].windows(2).all(|pair| pair[0] == pair[1]),
        "heights: {heights:?}"
    );
    heights[9]
}

#[test]
fn default_rows_grow_cap_scroll_and_shrink_after_setters_and_edits() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @multiline: true, @auto_size: {Some(AutoSize::default())}, @show_count: true }
        Node {width:px(300)}
    }).unwrap().id();
    let min = settle(&mut app, root);
    assert_eq!(
        WidgetryTextField::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    WidgetryTextField::set_text_in_world(app.world_mut(), root, "one\ntwo\nthree").unwrap();
    let middle = settle(&mut app, root);
    assert!(middle > min);
    assert_eq!(
        WidgetryTextField::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(3.0)
    );
    WidgetryTextField::set_text_in_world(app.world_mut(), root, "1\n2\n3\n4\n5\n6\n7\n8").unwrap();
    WidgetryTextField::focus_in_world(app.world_mut(), root).unwrap();
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::TextEnd(false));
    let max = settle(&mut app, root);
    let edit = WidgetryTextField::editor(app.world(), root).unwrap();
    assert_eq!(edit.visible_lines, Some(6.0));
    assert!(max > middle);
    assert!(
        edit.viewport.offset.y > 0.0,
        "viewport: {:?}",
        edit.viewport
    );
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .extend([TextEdit::SelectAll, TextEdit::Insert("short".into())]);
    assert_eq!(settle(&mut app, root), min);
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "short");
}

#[test]
fn actual_wrapping_reacts_to_width_slots_font_and_line_height_and_converges() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @multiline: true, @auto_size: {Some(AutoSize {min_rows:1,max_rows:20})}, @value: {"one two three four five six seven eight nine ten eleven twelve".to_owned()}, @children: {Some(Box::new(bsn_list! {WidgetryTextFieldPrefix Node {width:px(10)}}) as Box<dyn SceneList>)} }
        Node {width:px(500)}
    }).unwrap().id();
    let wide = settle(&mut app, root);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(170);
    let narrow = settle(&mut app, root);
    assert!(narrow > wide);
    let frame = support::frame_entity(app.world(), root);
    let prefix = app.world().get::<Children>(frame).unwrap()[0];
    app.world_mut().get_mut::<Node>(prefix).unwrap().width = px(65);
    let slotted = settle(&mut app, root);
    assert!(slotted > narrow);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(500);
    let recovered = settle(&mut app, root);
    assert!(recovered < slotted);
    app.world_mut().entity_mut(root).insert(TextFont {
        font_size: FontSize::Px(30.0),
        ..default()
    });
    let larger_font = settle(&mut app, root);
    assert!(larger_font > recovered);
    app.world_mut()
        .entity_mut(root)
        .insert(LineHeight::Px(45.0));
    assert!(settle(&mut app, root) > larger_font);
}

#[test]
fn readonly_custom_rows_and_count_do_not_change_the_editable_height_policy() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let mut heights = Vec::new();
    for show in [false, true] {
        let root=app.world_mut().spawn_scene(bsn! {
            @WidgetryReadOnlyTextField { @multiline: true, @auto_size: {Some(AutoSize {min_rows:1,max_rows:3})}, @show_count: show, @value: {"a\nb\nc\nd\ne".to_owned()} }
            Node {width:px(250)}
        }).unwrap().id();
        heights.push(settle(&mut app, root));
        assert_eq!(
            WidgetryReadOnlyTextField::editor(app.world(), root)
                .unwrap()
                .visible_lines,
            Some(3.0)
        );
        WidgetryReadOnlyTextField::set_text_in_world(app.world_mut(), root, "").unwrap();
        settle(&mut app, root);
        assert_eq!(
            WidgetryReadOnlyTextField::editor(app.world(), root)
                .unwrap()
                .visible_lines,
            Some(1.0)
        );
    }
    assert_eq!(heights[0], heights[1]);
}

#[test]
fn default_single_line_and_fixed_multiline_keep_their_sizing() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let single = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryTextField Node {width:px(200)}})
        .unwrap()
        .id();
    let fixed = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @multiline: true, @visible_lines: {Some(4.0)} }
            Node {width:px(200),height:px(140)}
        })
        .unwrap()
        .id();
    settle(&mut app, single);
    let height = settle(&mut app, fixed);
    WidgetryTextField::set_text_in_world(app.world_mut(), fixed, "a\nb\nc\nd\ne\nf\ng").unwrap();
    assert_eq!(settle(&mut app, fixed), height);
    assert_eq!(
        WidgetryTextField::editor(app.world(), single)
            .unwrap()
            .visible_lines,
        Some(1.0)
    );
    assert!(
        !WidgetryTextField::editor(app.world(), single)
            .unwrap()
            .allow_newlines
    );
    assert!(
        WidgetryTextField::editor(app.world(), fixed)
            .unwrap()
            .allow_newlines
    );
    assert_eq!(
        WidgetryTextField::editor(app.world(), fixed)
            .unwrap()
            .visible_lines,
        Some(4.0)
    );
}

#[test]
fn tall_slot_content_does_not_stretch_the_auto_sized_editor() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @multiline: true, @auto_size: {Some(AutoSize {min_rows:1,max_rows:2})}, @children: {Some(Box::new(bsn_list! {WidgetryTextFieldPrefix Node {width:px(30),height:px(120)}}) as Box<dyn SceneList>)} }
        Node {width:px(250)}
    }).unwrap().id();
    let height = settle(&mut app, root);
    let frame = support::frame_entity(app.world(), root);
    assert!(height < 120.0);
    assert!(app.world().get::<ComputedNode>(frame).unwrap().size().y >= 120.0);
}

#[test]
fn invalid_rows_and_single_line_auto_size_are_reported_during_scene_creation() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    for (multiline, config) in [
        (false, AutoSize::default()),
        (
            true,
            AutoSize {
                min_rows: 0,
                max_rows: 6,
            },
        ),
        (
            true,
            AutoSize {
                min_rows: 3,
                max_rows: 2,
            },
        ),
    ] {
        let result = app.world_mut().spawn_scene(bsn! {
            @WidgetryTextField { @multiline: multiline, @auto_size: {Some(config)} }
        });
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("AutoSize"));
    }
}

#[test]
fn fixed_root_height_conflict_reaches_the_host_once_and_recovers_when_auto() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::default());
    });
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @multiline: true, @auto_size: {Some(AutoSize::default())} }
            Node {width:px(250),height:px(80)}
        })
        .unwrap()
        .id();
    logs.run(|| errors.run(|| app.update()));
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
    for width in [260.0, 270.0, 280.0] {
        let mut node = app.world_mut().get_mut::<Node>(root).unwrap();
        node.width = px(width);
        node.margin = UiRect::top(px(width / 10.0));
        logs.run(|| errors.run(|| app.update()));
        assert!(
            errors.take().is_empty(),
            "the same height conflict must not be reported again"
        );
    }
    let failures = logs
        .records()
        .iter()
        .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
        .count();
    assert_eq!(failures, 1);
    app.world_mut().get_mut::<Node>(root).unwrap().height = Val::Auto;
    logs.run(|| settle(&mut app, root));
    assert_eq!(
        WidgetryTextField::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    let recovered = logs
        .records()
        .iter()
        .filter(|record| {
            record
                .fields
                .get("message")
                .is_some_and(|message| message.contains("AutoSize") && message.contains("恢复"))
        })
        .count();
    assert_eq!(recovered, 1);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(90);
    logs.run(|| errors.run(|| app.update()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        2
    );
    app.world_mut().entity_mut(root).despawn();
    logs.run(|| app.update());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record
                .fields
                .get("message")
                .is_some_and(|message| message.contains("恢复")))
            .count(),
        1
    );
}
