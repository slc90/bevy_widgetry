#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{prelude::*, ui::InteractionDisabled};
use bevy_widgetry_date_picker::*;
use time::macros::date;

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture() -> (App, Entity) {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryDatePickerHeadlessPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |_: On<WidgetryDatePickerDisplayedMonthChanged>, mut changes: ResMut<Changes>| {
            changes.0 += 1
        },
    );
    app.add_observer(
        |_: On<WidgetryDatePickerSelectionChanged>, mut changes: ResMut<Changes>| changes.0 += 1,
    );
    let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryDatePickerHeadless {@selected_date:{Some(date!(2026-10-15))},@displayed_month:{Some(date!(2026-12-01).into())}}
        @WidgetryDatePickerNavigation
    }).unwrap();
    (app, root)
}

#[test]
fn independent_navigation_browses_candidates_and_only_commits_confirmed_month() {
    let (mut app, root) = fixture();
    type Nav = WidgetryDatePickerNavigation;
    assert_eq!(Nav::view(app.world(), root).unwrap(), DatePickerView::Days);
    assert!(Nav::enter_parent_in_world(app.world_mut(), root).unwrap());
    assert_eq!(
        Nav::view(app.world(), root).unwrap(),
        DatePickerView::Months(2026)
    );
    let candidates = Nav::candidates(app.world(), root).unwrap();
    assert!(candidates[11].is_current);
    assert!(!candidates[9].is_current);
    Nav::browse_in_world(app.world_mut(), root, 1).unwrap();
    Nav::enter_parent_in_world(app.world_mut(), root).unwrap();
    assert_eq!(
        Nav::view(app.world(), root).unwrap(),
        DatePickerView::Years(2016)
    );
    Nav::browse_in_world(app.world_mut(), root, 1).unwrap();
    assert_eq!(
        Nav::candidates(app.world(), root).unwrap()[0].value,
        DatePickerCandidateValue::Year(2028)
    );
    Nav::choose_year_in_world(app.world_mut(), root, 2029).unwrap();
    assert_eq!(app.world().resource::<Changes>().0, 0);
    assert_eq!(
        WidgetryDatePickerHeadless::displayed_month(app.world(), root).unwrap(),
        date!(2026 - 12 - 01).into()
    );
    Nav::choose_month(&mut app.world_mut().commands(), root, Month::February);
    app.world_mut().flush();
    assert_eq!(Nav::view(app.world(), root).unwrap(), DatePickerView::Days);
    assert_eq!(
        WidgetryDatePickerHeadless::displayed_month(app.world(), root).unwrap(),
        date!(2029 - 02 - 01).into()
    );
    assert_eq!(
        WidgetryDatePickerHeadless::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 15))
    );
    assert_eq!(app.world().resource::<Changes>().0, 1);
    app.update();
    assert!(app.world().get::<Children>(root).is_none());
}

#[test]
fn navigation_guards_current_view_effective_disabled_and_calendar_limits() {
    let (mut app, root) = fixture();
    type Nav = WidgetryDatePickerNavigation;
    assert!(!Nav::choose_year_in_world(app.world_mut(), root, 2026).unwrap());
    assert!(!Nav::choose_month_in_world(app.world_mut(), root, Month::January).unwrap());
    let parent = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(!Nav::enter_parent_in_world(app.world_mut(), root).unwrap());
    WidgetryDatePickerHeadless::set_displayed_month_in_world(
        app.world_mut(),
        root,
        DatePickerMonth {
            year: Date::MAX.year(),
            month: Month::November,
        },
    )
    .unwrap();
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert!(!Nav::browse_in_world(app.world_mut(), root, i32::MAX).unwrap());
    Nav::enter_parent_in_world(app.world_mut(), root).unwrap();
    assert!(!Nav::candidates(app.world(), root).unwrap()[11].is_selectable);
    assert!(!Nav::choose_month_in_world(app.world_mut(), root, Month::December).unwrap());
    Nav::enter_parent_in_world(app.world_mut(), root).unwrap();
    assert!(!Nav::browse_in_world(app.world_mut(), root, 1).unwrap());
    assert!(!Nav::choose_year_in_world(app.world_mut(), root, Date::MIN.year()).unwrap());
    assert!(!Nav::choose_year_in_world(app.world_mut(), root, i32::MAX).unwrap());
    assert!(Nav::view(app.world(), Entity::PLACEHOLDER).is_err());
    app.world_mut()
        .entity_mut(root)
        .remove::<WidgetryDatePickerNavigation>();
    assert!(Nav::view(app.world(), root).is_err());
}
