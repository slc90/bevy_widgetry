#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

const EXPECTED_DAY_CELLS: usize = 42;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_date_picker::*;
use time::macros::date;

#[test]
fn independent_date_model_has_no_default_parts_and_separate_month_selection_states() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryDatePickerHeadlessPlugin);
    let root=bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn! {
        @WidgetryDatePickerHeadless {@displayed_month:{Some(date!(2026-10-01).into())},@week_start:Weekday::Sunday}
    }).unwrap();
    assert_eq!(
        WidgetryDatePickerHeadless::selected_date(app.world(), root).unwrap(),
        None
    );
    assert!(app.world().get::<Children>(root).is_none());
    assert!(
        WidgetryDatePickerHeadless::set_selected_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 15))
        )
        .unwrap()
    );
    WidgetryDatePickerHeadless::shift_month_in_world(app.world_mut(), root, 1).unwrap();
    assert!(
        !WidgetryDatePickerHeadless::set_selected_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 15))
        )
        .unwrap()
    );
    assert_eq!(
        WidgetryDatePickerHeadless::displayed_month(app.world(), root).unwrap(),
        date!(2026 - 11 - 01).into()
    );
    WidgetryDatePickerHeadless::set_selected_date_in_world(app.world_mut(), root, None).unwrap();
    let days =
        WidgetryDatePickerHeadless::days(app.world(), root, Some(date!(2026 - 11 - 07))).unwrap();
    assert_eq!(days[0].date.weekday(), Weekday::Sunday);
    assert_eq!(days.len(), EXPECTED_DAY_CELLS);
    assert_eq!(days.iter().filter(|day| day.is_today).count(), 1);
    app.update();
    assert!(app.world().get::<Children>(root).is_none());
}

#[test]
fn independent_date_user_operation_obeys_bounds_and_disabled_while_program_api_remains_available() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryDatePickerHeadlessPlugin);
    let root=bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn! {
        @WidgetryDatePickerHeadless {@displayed_month:{Some(date!(2026-10-01).into())},@min_date:{Some(date!(2026-10-10))},@max_date:{Some(date!(2026-10-20))}}
    }).unwrap();
    assert!(
        !WidgetryDatePickerHeadless::choose_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 09))
        )
        .unwrap()
    );
    assert!(
        WidgetryDatePickerHeadless::set_selected_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 09))
        )
        .is_err()
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(
        !WidgetryDatePickerHeadless::choose_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 15))
        )
        .unwrap()
    );
    WidgetryDatePickerHeadless::set_selected_date(
        &mut app.world_mut().commands(),
        root,
        Some(date!(2026 - 10 - 15)),
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePickerHeadless::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 15))
    );
}
