#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{
    prelude::*,
    text::{EditableText, LineHeight, TextEdit},
};
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;
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
fn optional_adapter_sizes_a_nested_editor_without_default_frame_or_root_geometry() {
    use bevy_widgetry_text_field::{
        WidgetryTextFieldAutoSize, WidgetryTextFieldAutoSizePlugin, WidgetryTextFieldEditor,
    };
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldAutoSizePlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            #Root @WidgetryTextFieldHeadless {editor:#Editor}
            @WidgetryTextFieldAutoSize { @rows:{AutoSize {min_rows:1,max_rows:3}} }
            Node {width:px(400),height:px(350),flex_direction:FlexDirection::Column}
            Children [
                Node {width:percent(100),align_items:AlignItems::Center}
                Children [
                    Node {width:px(25),height:px(150),flex_shrink:0.0}--
                    Node {flex_grow:1.0,min_width:px(0),align_items:AlignItems::Center}
                    Children [
                        #Editor @WidgetryTextFieldEditor {root:#Root,@multiline:true}
                        Node {width:percent(100),min_width:px(0),align_self:AlignSelf::Center}
                    ]
                ]
            ]
        })
        .unwrap()
        .id();
    let min = settle(&mut app, root);
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(1.0)
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "one\ntwo\nthree\nfour")
        .unwrap();
    let max = settle(&mut app, root);
    assert!(max > min);
    assert!(max < 150.0);
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(3.0)
    );
    assert_eq!(app.world().get::<Node>(root).unwrap().height, px(350));
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "").unwrap();
    assert_eq!(settle(&mut app, root), min);
    assert!(app.world().get::<WidgetryTextField>(root).is_none());
    assert!(!app.is_plugin_added::<WidgetryTextFieldPlugin>());
}

fn custom(app: &mut App, rows: Option<AutoSize>, multiline: bool) -> Entity {
    use bevy_widgetry_text_field::{WidgetryTextFieldAutoSize, WidgetryTextFieldEditor};
    let adapter = rows.map(|rows| bsn! {@WidgetryTextFieldAutoSize {@rows:rows}});
    app.world_mut().spawn_scene(bsn! {
        #Root @WidgetryTextFieldHeadless {editor:#Editor} @{adapter}
        Node {width:px(420),flex_direction:FlexDirection::Column}
        Children [
            Node {width:percent(100),align_items:AlignItems::Center}
            Children [
                #Editor @WidgetryTextFieldEditor {root:#Root,@multiline:multiline,@visible_lines:{Some(4.0)}}
                Node {width:percent(100),min_width:px(0),align_self:AlignSelf::Center}
            ]
        ]
    }).unwrap().id()
}

#[test]
fn custom_wrapping_preedit_and_font_changes_converge_while_opted_out_editors_stay_fixed() {
    use bevy_widgetry_text_field::WidgetryTextFieldAutoSizePlugin;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldAutoSizePlugin);
    let root = custom(
        &mut app,
        Some(AutoSize {
            min_rows: 1,
            max_rows: 20,
        }),
        true,
    );
    let fixed = custom(&mut app, None, true);
    let value = "one two three four five six seven eight nine ten eleven twelve";
    for target in [root, fixed] {
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), target, value).unwrap();
    }
    let wide = settle(&mut app, root);
    let fixed_height = settle(&mut app, fixed);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(120);
    assert!(settle(&mut app, root) > wide);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(420);
    assert_eq!(settle(&mut app, root), wide);
    app.world_mut().entity_mut(root).insert(TextFont {
        font_size: FontSize::Px(30.0),
        ..default()
    });
    let large = settle(&mut app, root);
    assert!(large > wide);
    app.world_mut()
        .entity_mut(root)
        .insert(LineHeight::Px(60.0));
    assert!(settle(&mut app, root) > large);
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "").unwrap();
    let empty = settle(&mut app, root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::ImeSetCompose {
            value: "one\ntwo\nthree".into(),
            cursor: None,
        });
    assert!(settle(&mut app, root) > empty);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        ""
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "").unwrap();
    assert_eq!(settle(&mut app, root), empty);
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "theme selection").unwrap();
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
    settle(&mut app, root);
    let selection = WidgetryTextFieldHeadless::editor(app.world(), root)
        .unwrap()
        .editor()
        .raw_selection()
        .text_range();
    for color in [Color::BLACK, Color::WHITE] {
        app.world_mut()
            .resource_mut::<bevy_widgetry_theme::WidgetryTheme>()
            .text_field
            .hint_warning = color;
        settle(&mut app, root);
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
            "theme selection"
        );
        assert_eq!(
            WidgetryTextFieldHeadless::editor(app.world(), root)
                .unwrap()
                .editor()
                .raw_selection()
                .text_range(),
            selection
        );
    }
    assert_eq!(settle(&mut app, fixed), fixed_height);
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), fixed)
            .unwrap()
            .visible_lines,
        Some(4.0)
    );
    assert_eq!(app.world().get::<Node>(fixed).unwrap().height, Val::Auto);
}

#[test]
fn adapter_binding_errors_are_deduplicated_recover_and_do_not_block_healthy_roots() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    use bevy_widgetry_text_field::{WidgetryTextFieldAutoSizePlugin, WidgetryTextFieldEditor};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldAutoSizePlugin);
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::default());
    });
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let bad = custom(&mut app, Some(AutoSize::default()), true);
    let good = custom(&mut app, Some(AutoSize::default()), true);
    let single = custom(&mut app, Some(AutoSize::default()), false);
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), bad).unwrap();
    let parent = app.world().get::<ChildOf>(editor).unwrap().parent();
    app.world_mut().entity_mut(editor).remove::<ChildOf>();
    logs.run(|| errors.run(|| app.update()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert!(records[0].managed().is_some());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::tracing::Level::ERROR)
            .count(),
        2
    );
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), good)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    app.world_mut().entity_mut(editor).insert(ChildOf(parent));
    let single_editor = WidgetryTextFieldHeadless::editor_entity(app.world(), single).unwrap();
    app.world_mut()
        .get_mut::<EditableText>(single_editor)
        .unwrap()
        .allow_newlines = true;
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.matches_field("phase", "recovered"))
            .count(),
        2
    );
    app.world_mut()
        .entity_mut(editor)
        .insert(WidgetryTextFieldEditor { root: good });
    logs.run(|| errors.run(|| app.update()));
    assert_eq!(errors.take_records().len(), 1);
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    app.world_mut()
        .entity_mut(editor)
        .insert(WidgetryTextFieldEditor { root: bad });
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), bad)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
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
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "one\ntwo\nthree").unwrap();
    let middle = settle(&mut app, root);
    assert!(middle > min);
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(3.0)
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "1\n2\n3\n4\n5\n6\n7\n8")
        .unwrap();
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), root).unwrap();
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::TextEnd(false));
    let max = settle(&mut app, root);
    let edit = WidgetryTextFieldHeadless::editor(app.world(), root).unwrap();
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
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "short"
    );
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
            WidgetryTextFieldHeadless::editor(app.world(), root)
                .unwrap()
                .visible_lines,
            Some(3.0)
        );
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "").unwrap();
        settle(&mut app, root);
        assert_eq!(
            WidgetryTextFieldHeadless::editor(app.world(), root)
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
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), fixed, "a\nb\nc\nd\ne\nf\ng")
        .unwrap();
    assert_eq!(settle(&mut app, fixed), height);
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), single)
            .unwrap()
            .visible_lines,
        Some(1.0)
    );
    assert!(
        !WidgetryTextFieldHeadless::editor(app.world(), single)
            .unwrap()
            .allow_newlines
    );
    assert!(
        WidgetryTextFieldHeadless::editor(app.world(), fixed)
            .unwrap()
            .allow_newlines
    );
    assert_eq!(
        WidgetryTextFieldHeadless::editor(app.world(), fixed)
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
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .visible_lines,
        Some(2.0)
    );
    let recovered = logs
        .records()
        .iter()
        .filter(|record| record.matches_field("phase", "recovered"))
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
