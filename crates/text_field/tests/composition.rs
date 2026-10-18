#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{prelude::*, text::EditableText};
use bevy_widgetry_text_field::{
    WidgetryTextField, WidgetryTextFieldPlugin, WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};
use support::editing_app;

#[test]
fn fields_without_slots_keep_the_editors_intrinsic_line_height() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @value: {"Visible text".to_owned()} }
            Node {width:px(400)}
        })
        .unwrap()
        .id();
    for _ in 0..5 {
        app.update();
    }
    let editor = support::editor_entity(app.world(), root);
    assert!(app.world().get::<ComputedNode>(editor).unwrap().size().y > 10.0);
}

#[test]
fn callers_can_name_the_composite_before_or_after_its_scene() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let before = app
        .world_mut()
        .spawn_scene(bsn! {Name("Before") @WidgetryTextField})
        .unwrap()
        .id();
    let after = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryTextField Name("After")})
        .unwrap()
        .id();
    assert_eq!(app.world().get::<Name>(before).unwrap().as_str(), "Before");
    assert_eq!(app.world().get::<Name>(after).unwrap().as_str(), "After");
    assert!(WidgetryTextField::editor(app.world(), before).is_ok());
    assert!(WidgetryTextField::editor(app.world(), after).is_ok());
}

#[test]
fn removing_a_root_filter_restores_the_official_input_pipeline() {
    use bevy::text::{EditableTextFilter, TextEdit};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .entity_mut(root)
        .insert(EditableTextFilter::new(|c| c.is_ascii_digit()));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::Insert("x".into()));
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "");
    app.world_mut()
        .entity_mut(root)
        .remove::<EditableTextFilter>();
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::Insert("x".into()));
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "x");
}

#[test]
fn replacing_a_root_filter_in_one_frame_keeps_the_new_predicate() {
    use bevy::text::{EditableTextFilter, TextEdit};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(root)
        .insert(EditableTextFilter::new(|c| c.is_ascii_digit()));
    app.update();
    app.world_mut()
        .entity_mut(root)
        .remove::<EditableTextFilter>()
        .insert(EditableTextFilter::new(|c| c.is_ascii_alphabetic()));
    let editor = support::editor_entity(app.world(), root);
    for expected in ["a", "aa"] {
        for value in ["a", "1"] {
            app.world_mut()
                .get_mut::<EditableText>(editor)
                .unwrap()
                .queue_edit(TextEdit::Insert(value.into()));
        }
        app.update();
        assert_eq!(
            WidgetryTextField::text(app.world(), root).unwrap(),
            expected
        );
    }
}

#[test]
fn readding_select_all_in_one_frame_preserves_focus_selection_and_later_removal() {
    use bevy::ui_widgets::SelectAllOnFocus;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @value: {"Select this".to_owned()} }
            SelectAllOnFocus
        })
        .unwrap()
        .id();
    app.update();
    app.world_mut()
        .entity_mut(root)
        .remove::<SelectAllOnFocus>()
        .insert(SelectAllOnFocus);
    app.update();
    WidgetryTextField::focus_in_world(app.world_mut(), root).unwrap();
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(
        WidgetryTextField::editor(app.world(), root)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range(),
        0.."Select this".len()
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<SelectAllOnFocus>();
    app.update();
    let editor = support::editor_entity(app.world(), root);
    assert!(app.world().get::<SelectAllOnFocus>(editor).is_none());
}

#[test]
fn remaining_width_and_oversized_slots_do_not_shrink_user_content() {
    for slot_width in [80.0, 200.0] {
        let mut app = editing_app();
        app.add_plugins(WidgetryTextFieldPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryTextField { @children: {Some(Box::new(bsn_list! {
                    WidgetryTextFieldSuffix Node {width:px(slot_width)}--
                    WidgetryTextFieldPrefix Node {width:px(slot_width)}
                }) as Box<dyn SceneList>)} }
                Node {width:px(300),height:px(50)}
            })
            .unwrap()
            .id();
        for _ in 0..3 {
            app.update();
        }
        let frame = support::frame_entity(app.world(), root);
        let children = app.world().get::<Children>(frame).unwrap();
        let prefix = app
            .world()
            .get::<ComputedNode>(children[0])
            .unwrap()
            .size()
            .x;
        let input = app
            .world()
            .get::<ComputedNode>(children[1])
            .unwrap()
            .size()
            .x;
        let suffix = app
            .world()
            .get::<ComputedNode>(children[2])
            .unwrap()
            .size()
            .x;
        let available = app
            .world()
            .get::<ComputedNode>(frame)
            .unwrap()
            .content_box()
            .size()
            .x;
        assert_eq!(prefix, slot_width);
        assert_eq!(suffix, slot_width);
        assert!((input - (available - prefix - suffix).max(0.0)).abs() <= 1.0);
    }
}

#[test]
fn a_late_prefix_is_positioned_on_the_left_and_invalid_revisions_are_reported_once() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::default());
    });
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.update();
    let frame = support::frame_entity(app.world(), root);
    let prefix = app
        .world_mut()
        .spawn_scene(bsn! { WidgetryTextFieldPrefix Children [Text("前")] })
        .unwrap()
        .id();
    app.world_mut().entity_mut(frame).add_child(prefix);
    app.update();
    assert_eq!(app.world().get::<Children>(frame).unwrap()[0], prefix);
    let duplicate = app
        .world_mut()
        .spawn_scene(bsn! { WidgetryTextFieldPrefix })
        .unwrap()
        .id();
    app.world_mut().entity_mut(frame).add_child(duplicate);
    errors.run(|| app.update());
    assert_eq!(errors.take().len(), 1);
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    app.world_mut().entity_mut(duplicate).despawn();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    assert_eq!(app.world().get::<Children>(frame).unwrap()[0], prefix);
}

#[test]
fn missing_and_single_slots_keep_only_their_own_content() {
    for kind in ["none", "prefix", "suffix"] {
        let mut app = editing_app();
        app.add_plugins(WidgetryTextFieldPlugin);
        let children: Option<Box<dyn SceneList>> = match kind {
            "prefix" => Some(Box::new(
                bsn_list! { WidgetryTextFieldPrefix Children [Text("前")] },
            )),
            "suffix" => Some(Box::new(
                bsn_list! { WidgetryTextFieldSuffix Children [Text("后")] },
            )),
            _ => None,
        };
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTextField { @children: children } })
            .unwrap()
            .id();
        app.update();
        assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "");
        let frame = support::frame_entity(app.world(), root);
        assert_eq!(
            app.world().get::<Children>(frame).unwrap().len(),
            if kind == "none" { 1 } else { 2 }
        );
    }
}

#[test]
fn duplicate_and_ambiguous_slot_configuration_reaches_the_host_error_handler() {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::ErrorCapture;
    for kind in ["prefix", "suffix", "both"] {
        let mut app = editing_app();
        app.add_plugins(WidgetryTextFieldPlugin);
        app.edit_schedule(PostUpdate, |s| {
            s.set_executor(SingleThreadedExecutor::default());
        });
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let children: Box<dyn SceneList> = match kind {
            "prefix" => Box::new(bsn_list! { WidgetryTextFieldPrefix--WidgetryTextFieldPrefix }),
            "suffix" => Box::new(bsn_list! { WidgetryTextFieldSuffix--WidgetryTextFieldSuffix }),
            _ => Box::new(bsn_list! { WidgetryTextFieldPrefix WidgetryTextFieldSuffix }),
        };
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryTextField { @children: {Some(children)} } })
            .unwrap();
        errors.run(|| app.update());
        let observed = errors.take();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].severity(), bevy::ecs::error::Severity::Error);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
    }
}

#[test]
fn slot_focus_requests_do_not_select_the_editor_and_root_requests_do() {
    use bevy::input_focus::{AcquireFocus, InputFocus};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @children: {Some(Box::new(bsn_list! {
            WidgetryTextFieldSuffix Children [Node bevy::input_focus::tab_navigation::TabIndex(0)]
        }) as Box<dyn SceneList>)} }
        bevy::input_focus::tab_navigation::TabIndex(0)
    }).unwrap().id();
    app.update();
    app.world_mut().resource_mut::<InputFocus>().clear();
    let frame = support::frame_entity(app.world(), root);
    let slot = app.world().get::<Children>(frame).unwrap()[1];
    let button = app.world().get::<Children>(slot).unwrap()[0];
    app.world_mut().trigger(AcquireFocus {
        focused_entity: button,
        window,
    });
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    app.world_mut().trigger(AcquireFocus {
        focused_entity: root,
        window,
    });
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(support::editor_entity(app.world(), root))
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.world_mut().trigger(AcquireFocus {
        focused_entity: slot,
        window,
    });
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

#[test]
fn tab_moves_from_editor_to_slot_control_then_to_the_next_field() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::{
            InputFocus,
            tab_navigation::{TabGroup, TabIndex, handle_tab_navigation},
        },
    };
    let mut app = editing_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .observe(handle_tab_navigation)
        .id();
    let group = app
        .world_mut()
        .spawn(Node::default())
        .insert(TabGroup::new(0))
        .id();
    let first = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @children: {Some(Box::new(bsn_list! {
                WidgetryTextFieldSuffix Children [ Node TabIndex(2) ]
            }) as Box<dyn SceneList>)} } TabIndex(1)
        })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField TabIndex(3) })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(group)
        .add_children(&[first, second]);
    app.update();
    WidgetryTextField::focus_in_world(app.world_mut(), first).unwrap();
    let frame = support::frame_entity(app.world(), first);
    let slot = app.world().get::<Children>(frame).unwrap()[1];
    let control = app.world().get::<Children>(slot).unwrap()[0];
    for expected in [control, support::editor_entity(app.world(), second)] {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Tab,
                logical_key: Key::Tab,
                state,
                text: None,
                repeat: false,
                window,
            });
            app.update();
        }
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(expected));
    }
}

#[test]
fn slots_have_fixed_order_and_content_api_keeps_the_root_identity() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField {
                @value: {"正文".to_owned()}, @max_length: {Some(3)},
                @children: {Some(Box::new(bsn_list! {
                    WidgetryTextFieldSuffix Name("Suffix") Children[ Text("单位") ]--
                    WidgetryTextFieldPrefix Name("Prefix") Children[ Text("前")--Text("缀") ]
                }) as Box<dyn SceneList>)},
            }
            Node { width: px(240) }
        })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<EditableText>(root).is_none());
    assert_eq!(WidgetryTextField::text(app.world(), root).unwrap(), "正文");
    assert!(WidgetryTextField::set_text_in_world(app.world_mut(), root, "新正文尾").unwrap());
    assert_eq!(
        WidgetryTextField::text(app.world(), root).unwrap(),
        "新正文"
    );
    let frame = app.world().get::<Children>(root).unwrap()[0];
    let children = app.world().get::<Children>(frame).unwrap();
    assert_eq!(children.len(), 3);
    assert!(
        app.world()
            .get::<WidgetryTextFieldPrefix>(children[0])
            .is_some()
    );
    assert!(app.world().get::<EditableText>(children[1]).is_some());
    assert!(
        app.world()
            .get::<WidgetryTextFieldSuffix>(children[2])
            .is_some()
    );
    assert_eq!(app.world().get::<Children>(children[0]).unwrap().len(), 2);
}
