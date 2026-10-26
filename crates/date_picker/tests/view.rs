#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_date_picker::*;
use bevy_widgetry_test_utils::{primary_click, scene_app};
use time::macros::date;

fn find(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, n)| n.as_str() == name)
        .unwrap()
        .0
}

#[derive(Resource, Default)]
struct Changes(Vec<Option<Date>>, usize);

fn observe(app: &mut App) {
    app.init_resource::<Changes>();
    app.add_observer(
        |event: On<WidgetryDatePickerSelectionChanged>, mut changes: ResMut<Changes>| {
            changes.0.push(event.selected_date);
        },
    );
    app.add_observer(
        |_: On<WidgetryDatePickerDisplayedMonthChanged>, mut changes: ResMut<Changes>| {
            changes.1 += 1;
        },
    );
}

#[test]
fn pointer_press_release_and_disabled_have_real_visual_feedback() {
    use bevy::ui::Pressed;
    use bevy_widgetry_test_utils::{primary_press, primary_release};
    use bevy_widgetry_theme::WidgetryThemeMode;
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryDatePicker {
            @selected_date: {Some(date!(2026-10-15))},
        } })
        .unwrap()
        .id();
    app.update();
    for name in ["DatePickerDay3", "DatePickerNext", "DatePickerClear"] {
        let target = find(&mut app, name);
        app.world_mut().trigger(primary_press(target));
        app.world_mut().flush();
        assert!(app.world().get::<Pressed>(target).is_some());
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(target).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .date_picker
                .pressed
        );
        app.world_mut().trigger(primary_release(target));
        app.world_mut().flush();
        app.update();
        assert!(app.world().get::<Pressed>(target).is_none());
        assert_eq!(
            app.world().get::<BackgroundColor>(target).unwrap().0,
            Color::NONE
        );
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    let day = find(&mut app, "DatePickerDay3");
    app.world_mut().trigger(primary_press(day));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(day).is_none());
}

#[test]
fn arrows_same_value_ancestor_disabled_and_color_overrides_remain_independent() {
    use bevy_widgetry_theme::WidgetryThemeMode;
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin);
    let parent = app
        .world_mut()
        .spawn_scene(bsn! { Node Children [@WidgetryDatePicker {
            @selected_date: {Some(date!(2026-10-15))},
        }] })
        .unwrap()
        .id();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    app.update();
    let next = find(&mut app, "DatePickerNext");
    app.world_mut().trigger(primary_click(next));
    app.world_mut().flush();
    app.update();
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root)
            .unwrap()
            .month,
        Month::November
    );
    assert!(
        !WidgetryDatePicker::set_selected_date_in_world(
            app.world_mut(),
            root,
            Some(date!(2026 - 10 - 15))
        )
        .unwrap()
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root)
            .unwrap()
            .month,
        Month::November
    );
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.world_mut().trigger(primary_click(next));
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root)
            .unwrap()
            .month,
        Month::November
    );
    let custom = Color::srgb_u8(100, 130, 150);
    WidgetryDatePickerColorOverrides::set_in_world(
        app.world_mut(),
        root,
        WidgetryDatePickerColorOverrides {
            background: Some(custom),
            ..default()
        },
    )
    .unwrap();
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        assert_eq!(app.world().get::<BackgroundColor>(root).unwrap().0, custom);
        assert_eq!(
            WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
            Some(date!(2026 - 10 - 15))
        );
    }
    WidgetryDatePickerColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        WidgetryThemeMode::KamuriViolet
            .colors()
            .date_picker
            .background
    );
    assert!(
        WidgetryDatePickerColorOverrides::set_in_world(
            app.world_mut(),
            root,
            WidgetryDatePickerColorOverrides {
                background: Some(Color::srgba(f32::NAN, 0.0, 0.0, 1.0)),
                ..default()
            }
        )
        .is_err()
    );
    assert_eq!(
        WidgetryDatePickerColorOverrides::get(app.world(), root).unwrap(),
        &WidgetryDatePickerColorOverrides::default()
    );
}

#[test]
fn day_and_clear_clicks_share_state_api_and_disabled_guards() {
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin);
    observe(&mut app);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryDatePicker {
            @displayed_month: {Some(DatePickerMonth {year: 2026, month: Month::October})},
            @min_date: {Some(date!(2026-10-01))},
            @max_date: {Some(date!(2026-11-01))},
        } })
        .unwrap()
        .id();
    app.update();
    let rejected = find(&mut app, "DatePickerDay0");
    app.world_mut().trigger(primary_click(rejected));
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        None
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    let adjacent = find(&mut app, "DatePickerDay34");
    app.world_mut().trigger(primary_click(adjacent));
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 11 - 01))
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        [Some(date!(2026 - 11 - 01))]
    );
    assert_eq!(app.world().resource::<Changes>().1, 1);
    app.update();
    let selected = find(&mut app, "DatePickerDay6");
    app.world_mut().trigger(primary_click(selected));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    assert_eq!(app.world().resource::<Changes>().1, 1);
    let clear = find(&mut app, "DatePickerClear");
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().trigger(primary_click(clear));
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 11 - 01))
    );
    WidgetryDatePicker::set_selected_date_in_world(
        app.world_mut(),
        root,
        Some(date!(2026 - 10 - 01)),
    )
    .unwrap();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_click(clear));
    app.world_mut().flush();
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        None
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root)
            .unwrap()
            .month,
        Month::October
    );
    assert_eq!(app.world().resource::<Changes>().0.last(), Some(&None));
    assert_eq!(app.world().resource::<Changes>().1, 2);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(clear).is_some());
    app.world_mut().trigger(primary_click(clear));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0.len(), 3);
}
