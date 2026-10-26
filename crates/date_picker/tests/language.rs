#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_date_picker::*;
use bevy_widgetry_test_utils::{primary_click, scene_app};
use time::macros::date;

fn find(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, value)| value.as_str() == name)
        .unwrap()
        .0
}
fn text(app: &mut App, name: &str) -> String {
    let root = find(app, name);
    let child = app.world().get::<Children>(root).unwrap()[0];
    app.world().get::<Text>(child).unwrap().0.clone()
}
fn click(app: &mut App, name: &str) {
    let target = find(app, name);
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    app.update();
}
#[derive(Resource, Default)]
struct Events(usize);

#[test]
fn language_switch_preserves_selection_candidate_navigation_and_entity_identity() {
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin)
        .init_resource::<Events>();
    app.add_observer(
        |_: On<WidgetryDatePickerSelectionChanged>, mut count: ResMut<Events>| count.0 += 1,
    );
    app.add_observer(
        |_: On<WidgetryDatePickerDisplayedMonthChanged>, mut count: ResMut<Events>| count.0 += 1,
    );
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryDatePicker {
            @selected_date: {Some(date!(2026-10-26))}, @week_start: Weekday::Sunday,
        } })
        .unwrap()
        .id();
    app.update();
    let day = find(&mut app, "DatePickerDay0");
    let candidate = find(&mut app, "DatePickerCandidate0");
    click(&mut app, "DatePickerTitle");
    click(&mut app, "DatePickerNext");
    app.insert_resource(WidgetryDatePickerLanguage::SimplifiedChinese);
    app.update();
    assert_eq!(text(&mut app, "DatePickerTitle"), "2027年");
    assert_eq!(text(&mut app, "DatePickerCandidate0"), "1月");
    assert_eq!(app.world().resource::<Events>().0, 0);
    click(&mut app, "DatePickerTitle");
    click(&mut app, "DatePickerNext");
    app.insert_resource(WidgetryDatePickerLanguage::English);
    app.update();
    assert_eq!(text(&mut app, "DatePickerTitle"), "2028 – 2039");
    app.insert_resource(WidgetryDatePickerLanguage::SimplifiedChinese);
    app.update();
    assert_eq!(text(&mut app, "DatePickerCandidate0"), "2028年");
    click(&mut app, "DatePickerCandidate0");
    click(&mut app, "DatePickerCandidate9");
    assert_eq!(text(&mut app, "DatePickerTitle"), "2028年10月");
    assert_eq!(text(&mut app, "DatePickerWeekday0"), "周日");
    assert_eq!(text(&mut app, "DatePickerClear"), "清空");
    app.insert_resource(WidgetryDatePickerLanguage::English);
    app.update();
    assert_eq!(text(&mut app, "DatePickerClear"), "Clear");
    assert_eq!(text(&mut app, "DatePickerWeekday0"), "Sun");
    assert_eq!(find(&mut app, "DatePickerDay0"), day);
    assert_eq!(find(&mut app, "DatePickerCandidate0"), candidate);
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 26))
    );
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        DatePickerMonth {
            year: 2028,
            month: Month::October
        }
    );
    assert_eq!(app.world().resource::<Events>().0, 1);
}
