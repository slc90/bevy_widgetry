#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit},
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::{input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Resource, Default)]
struct Count(usize);

#[test]
fn focus_displays_full_value_without_committing_rounded_text_or_erasing_pending_input() {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Count>();
    app.add_observer(|_: On<ValueChange<f64>>, mut count: ResMut<Count>| count.0 += 1);
    let value = 0.1_f64 + 0.2_f64;
    let root = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryNumberInput<f64>{@value:{Some(value)},@decimals:{Some(2)}}},
    )
    .unwrap();
    let field = app.world().get::<Children>(root).unwrap()[0];
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "0.30");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    assert_eq!(
        WidgetryTextField::text(app.world(), field).unwrap(),
        "0.30000000000000004"
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "0.30");
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root)
            .unwrap()
            .to_bits(),
        value.to_bits()
    );
    assert_eq!(app.world().resource::<Count>().0, 0);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root)
            .unwrap()
            .to_bits(),
        value.to_bits()
    );
    assert_eq!(app.world().resource::<Count>().0, 0);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Pressed);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .push(TextEdit::Insert("7".into()));
    app.update();
    assert!(
        WidgetryTextField::text(app.world(), field)
            .unwrap()
            .contains('7')
    );
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        value
    );
    assert_eq!(app.world().resource::<Count>().0, 0);
    WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 1.234567).unwrap();
    assert_eq!(
        WidgetryTextField::text(app.world(), field).unwrap(),
        "1.234567"
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "1.23");
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        1.234567
    );
    assert_eq!(app.world().resource::<Count>().0, 1);
}

#[test]
fn integer_decimals_configuration_is_rejected() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    assert!(
        spawn_scene(
            app.world_mut(),
            bsn! {@WidgetryNumberInput<i32>{@value:{Some(1)},@decimals:{Some(2)}}}
        )
        .is_err()
    );
}

#[test]
fn setter_after_focus_leaves_cannot_commit_its_rounded_projection() {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Count>();
    app.add_observer(|_: On<ValueChange<f64>>, mut count: ResMut<Count>| count.0 += 1);
    let root = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryNumberInput<f64>{@value:{Some(1.234567)},@decimals:{Some(2)}}},
    )
    .unwrap();
    let field = app.world().get::<Children>(root).unwrap()[0];
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for (value, changed, count) in [(1.234567, false, 0), (2.345678, true, 1)] {
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Navigated);
        app.update();
        app.world_mut().resource_mut::<InputFocus>().clear();
        assert_eq!(
            WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, value).unwrap(),
            changed
        );
        app.update();
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root)
                .unwrap()
                .to_bits(),
            value.to_bits()
        );
        assert_eq!(app.world().resource::<Count>().0, count);
    }
}

#[test]
fn extreme_floats_focus_and_blur_without_losing_bits_or_using_exponents() {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Count>();
    app.add_observer(|_: On<ValueChange<f64>>, mut count: ResMut<Count>| count.0 += 1);
    macro_rules! check {
        ($t:ty) => {
            for value in [
                <$t>::from_bits(1),
                <$t>::MIN_POSITIVE,
                <$t>::MAX,
                -<$t>::from_bits(1),
                -<$t>::MIN_POSITIVE,
                -<$t>::MAX,
                -0.0,
            ] {
                let root = spawn_scene(
                    app.world_mut(),
                    bsn! {@WidgetryNumberInput<$t>{@value:{Some(value)},@decimals:{Some(2)}}},
                )
                .unwrap();
                let field = app.world().get::<Children>(root).unwrap()[0];
                let editor = app
                    .world()
                    .get::<WidgetryInputTarget>(field)
                    .unwrap()
                    .target;
                app.world_mut()
                    .resource_mut::<InputFocus>()
                    .set(editor, FocusCause::Pressed);
                app.update();
                let text = WidgetryTextField::text(app.world(), field).unwrap();
                assert!(!text.contains(['e', 'E']));
                assert_eq!(text.parse::<$t>().unwrap().to_bits(), value.to_bits());
                app.world_mut().resource_mut::<InputFocus>().clear();
                app.update();
                assert_eq!(
                    WidgetryNumberInput::<$t>::value(app.world(), root)
                        .unwrap()
                        .to_bits(),
                    value.to_bits()
                );
            }
        };
    }
    check!(f32);
    check!(f64);
    assert_eq!(app.world().resource::<Count>().0, 0);
}
