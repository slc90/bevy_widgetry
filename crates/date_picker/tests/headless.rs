#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_date_picker::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use time::macros::date;

fn month(year: i32, month: Month) -> DatePickerMonth {
    DatePickerMonth { year, month }
}

fn spawn(app: &mut App, props: WidgetryDatePickerProps) -> Entity {
    app.world_mut()
        .spawn_scene(bsn! { @WidgetryDatePicker {
            @selected_date: {props.selected_date},
            @displayed_month: {props.displayed_month},
            @min_date: {props.min_date},
            @max_date: {props.max_date},
            @week_start: {props.week_start},
        } })
        .unwrap()
        .id()
}

#[derive(Resource, Default)]
struct Changes(Vec<(Option<Date>, DatePickerMonth, bool)>);

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<WidgetryDatePickerSelectionChanged>,
         roots: Query<&WidgetryDatePicker>,
         mut commands: Commands| {
            assert!(roots.contains(event.entity));
            let root = event.entity;
            let value = event.selected_date;
            commands.queue(move |world: &mut World| {
                assert_eq!(
                    WidgetryDatePicker::selected_date(world, root).unwrap(),
                    value
                );
                let month = WidgetryDatePicker::displayed_month(world, root).unwrap();
                world.resource_mut::<Changes>().0.push((value, month, true));
            });
        },
    );
    app.add_observer(
        |event: On<WidgetryDatePickerDisplayedMonthChanged>, mut commands: Commands| {
            let root = event.entity;
            let value = event.displayed_month;
            commands.queue(move |world: &mut World| {
                assert_eq!(
                    WidgetryDatePicker::displayed_month(world, root).unwrap(),
                    value
                );
                let selected = WidgetryDatePicker::selected_date(world, root).unwrap();
                world
                    .resource_mut::<Changes>()
                    .0
                    .push((selected, value, false));
            });
        },
    );
    app
}

#[test]
fn initialization_and_selection_browsing_clearing_have_distinct_semantics() {
    let mut app = app();
    let initial = date!(2026 - 10 - 15);
    let root = spawn(
        &mut app,
        WidgetryDatePickerProps {
            selected_date: Some(initial),
            displayed_month: Some(month(2026, Month::December)),
            ..default()
        },
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        month(2026, Month::December)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert!(
        !WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, Some(initial))
            .unwrap()
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        month(2026, Month::December)
    );
    let chosen = date!(2027 - 01 - 02);
    assert!(
        WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, Some(chosen))
            .unwrap()
    );
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![
            (Some(chosen), month(2027, Month::January), true),
            (Some(chosen), month(2027, Month::January), false)
        ]
    );
    assert!(WidgetryDatePicker::shift_month_in_world(app.world_mut(), root, -1).unwrap());
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(chosen)
    );
    assert!(WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, None).unwrap());
    assert!(!WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, None).unwrap());
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        month(2026, Month::December)
    );
}

#[test]
fn grids_are_continuous_and_respect_week_start_bounds_selection_and_today() {
    let mut app = app();
    for year in [2024, 2025, 2026] {
        for m in 1..=12 {
            for start in [Weekday::Monday, Weekday::Sunday, Weekday::Thursday] {
                let root = spawn(
                    &mut app,
                    WidgetryDatePickerProps {
                        displayed_month: Some(month(year, Month::try_from(m).unwrap())),
                        week_start: start,
                        min_date: Some(date!(2024 - 02 - 01)),
                        max_date: Some(date!(2026 - 10 - 15)),
                        ..default()
                    },
                );
                let grid = WidgetryDatePicker::days(app.world(), root, Some(date!(2024 - 02 - 29)))
                    .unwrap();
                assert_eq!(grid.len(), 42);
                assert_eq!(grid[0].date.weekday(), start);
                assert!(
                    grid.windows(2).all(
                        |pair| pair[1].date.to_julian_day() == pair[0].date.to_julian_day() + 1
                    )
                );
                for day in grid {
                    assert_eq!(
                        day.is_current_month,
                        day.date.year() == year && day.date.month() as u8 == m
                    );
                    assert_eq!(
                        day.is_selectable,
                        (date!(2024 - 02 - 01)..=date!(2026 - 10 - 15)).contains(&day.date)
                    );
                    assert_eq!(day.is_today, day.date == date!(2024 - 02 - 29));
                    assert!(!day.is_selected);
                }
                app.world_mut().despawn(root);
            }
        }
    }
}

#[test]
fn queued_calls_apply_in_order_and_errors_preserve_state_even_when_disabled() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let root = spawn(
        &mut app,
        WidgetryDatePickerProps {
            selected_date: Some(date!(2026 - 10 - 15)),
            min_date: Some(date!(2026 - 10 - 01)),
            max_date: Some(date!(2026 - 10 - 31)),
            ..default()
        },
    );
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    WidgetryDatePicker::set_selected_date(
        &mut app.world_mut().commands(),
        root,
        Some(date!(2026 - 10 - 31)),
    );
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 15))
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 31))
    );
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    WidgetryDatePicker::set_selected_date(
        &mut app.world_mut().commands(),
        root,
        Some(date!(2026 - 11 - 01)),
    );
    errors.run(|| logs.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 31))
    );
    for target in [root, Entity::PLACEHOLDER] {
        let err = WidgetryDatePicker::set_displayed_month_in_world(
            app.world_mut(),
            target,
            month(i32::MAX, Month::January),
        )
        .unwrap_err();
        assert_eq!(err.severity(), bevy::ecs::error::Severity::Error);
    }
    assert!(WidgetryDatePicker::shift_month_in_world(app.world_mut(), root, i32::MAX).is_err());
    WidgetryDatePicker::set_displayed_month(
        &mut app.world_mut().commands(),
        root,
        month(2000, Month::January),
    );
    WidgetryDatePicker::shift_month(&mut app.world_mut().commands(), root, -1);
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        month(1999, Month::December)
    );
}

#[test]
fn invalid_initial_configuration_is_rejected_and_selected_date_supplies_month() {
    let mut app = app();
    for (min, max, selected) in [
        (date!(2026 - 10 - 31), date!(2026 - 10 - 01), None),
        (
            date!(2026 - 10 - 01),
            date!(2026 - 10 - 31),
            Some(date!(2026 - 11 - 01)),
        ),
    ] {
        let err = app.world_mut().spawn_scene(bsn! { @WidgetryDatePicker { @min_date: {Some(min)}, @max_date: {Some(max)}, @selected_date: selected } }).err().unwrap();
        assert!(err.to_string().contains("date"));
    }
    let root = spawn(
        &mut app,
        WidgetryDatePickerProps {
            selected_date: Some(date!(2024 - 02 - 29)),
            ..default()
        },
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        month(2024, Month::February)
    );
    assert_eq!(
        WidgetryDatePicker::days(app.world(), root, None)
            .unwrap()
            .iter()
            .filter(|day| day.is_selected)
            .count(),
        1
    );
}
