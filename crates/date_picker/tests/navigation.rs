#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_date_picker::*;
use bevy_widgetry_test_utils::{primary_click, scene_app};
use bevy_widgetry_theme::WidgetryThemeMode;
use time::macros::date;

#[derive(Resource, Default)]
struct Changes(usize, usize);

fn find(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, n)| n.as_str() == name)
        .unwrap()
        .0
}

fn click(app: &mut App, name: &str) {
    let entity = find(app, name);
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    app.update();
}

fn label(app: &mut App, name: &str) -> String {
    let entity = find(app, name);
    let text = app.world().get::<Children>(entity).unwrap()[0];
    app.world().get::<Text>(text).unwrap().0.clone()
}

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryDatePickerPlugin)
        .init_resource::<Changes>();
    app.add_observer(|_: On<WidgetryDatePickerSelectionChanged>, mut c: ResMut<Changes>| c.0 += 1);
    app.add_observer(
        |_: On<WidgetryDatePickerDisplayedMonthChanged>, mut c: ResMut<Changes>| c.1 += 1,
    );
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryDatePicker {
            @selected_date: {Some(date!(2026-10-15))},
            @displayed_month: {Some(DatePickerMonth {year:2026, month:Month::December})},
            @min_date: {Some(date!(2026-10-01))},
            @max_date: {Some(date!(2026-10-31))},
        } Name("NavigationRoot") })
        .unwrap()
        .id();
    app.update();
    (app, root)
}

#[test]
fn candidate_navigation_only_commits_on_month_confirmation() {
    let (mut app, root) = fixture();
    let size = app.world().get::<Node>(root).unwrap().clone();
    click(&mut app, "DatePickerTitle");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2026");
    let december = find(&mut app, "DatePickerCandidate11");
    let selected_color = app
        .world()
        .resource::<WidgetryThemeMode>()
        .colors()
        .date_picker
        .selected;
    assert_eq!(
        app.world().get::<BackgroundColor>(december).unwrap().0,
        selected_color
    );
    let october = find(&mut app, "DatePickerCandidate9");
    assert_eq!(
        app.world().get::<BackgroundColor>(october).unwrap().0,
        Color::NONE
    );
    click(&mut app, "DatePickerNext");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2027");
    assert_eq!(
        app.world().get::<BackgroundColor>(december).unwrap().0,
        Color::NONE
    );
    click(&mut app, "DatePickerTitle");
    assert_eq!(label(&mut app, "DatePickerCandidate0"), "2016");
    assert_eq!(label(&mut app, "DatePickerCandidate11"), "2027");
    let committed_year = find(&mut app, "DatePickerCandidate10");
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(committed_year)
            .unwrap()
            .0,
        selected_color
    );
    click(&mut app, "DatePickerNext");
    assert_eq!(label(&mut app, "DatePickerCandidate0"), "2028");
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(committed_year)
            .unwrap()
            .0,
        Color::NONE
    );
    click(&mut app, "DatePickerPrevious");
    click(&mut app, "DatePickerCandidate1");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2017");
    assert_eq!(app.world().resource::<Changes>().0, 0);
    assert_eq!(app.world().resource::<Changes>().1, 0);
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        DatePickerMonth {
            year: 2026,
            month: Month::December
        }
    );
    click(&mut app, "DatePickerCandidate0");
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        DatePickerMonth {
            year: 2017,
            month: Month::January
        }
    );
    assert_eq!(
        WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
        Some(date!(2026 - 10 - 15))
    );
    assert_eq!(app.world().resource::<Changes>().0, 0);
    assert_eq!(app.world().resource::<Changes>().1, 1);
    let node = app.world().get::<Node>(root).unwrap();
    assert_eq!((node.width, node.height), (size.width, size.height));
}

#[test]
fn same_month_confirmation_is_silent_and_disabled_guards_all_navigation() {
    let (mut app, root) = fixture();
    click(&mut app, "DatePickerTitle");
    click(&mut app, "DatePickerCandidate11");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2026 / 12");
    assert_eq!(app.world().resource::<Changes>().1, 0);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    click(&mut app, "DatePickerTitle");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2026 / 12");
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    click(&mut app, "DatePickerTitle");
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    click(&mut app, "DatePickerNext");
    click(&mut app, "DatePickerCandidate0");
    click(&mut app, "DatePickerTitle");
    assert_eq!(label(&mut app, "DatePickerTitle"), "2026");
    assert_eq!(app.world().resource::<Changes>().1, 0);
    WidgetryDatePicker::set_displayed_month_in_world(
        app.world_mut(),
        root,
        DatePickerMonth {
            year: 2028,
            month: Month::February,
        },
    )
    .unwrap();
    assert_eq!(app.world().resource::<Changes>().1, 1);
}

#[test]
fn unrepresentable_month_and_year_candidates_are_disabled() {
    let (mut app, root) = fixture();
    WidgetryDatePicker::set_displayed_month_in_world(
        app.world_mut(),
        root,
        DatePickerMonth {
            year: Date::MAX.year(),
            month: Month::November,
        },
    )
    .unwrap();
    app.update();
    let next = find(&mut app, "DatePickerNext");
    assert!(app.world().get::<InteractionDisabled>(next).is_some());
    click(&mut app, "DatePickerTitle");
    let december = find(&mut app, "DatePickerCandidate11");
    assert!(app.world().get::<InteractionDisabled>(december).is_some());
    click(&mut app, "DatePickerCandidate11");
    assert_eq!(
        label(&mut app, "DatePickerTitle"),
        Date::MAX.year().to_string()
    );
    click(&mut app, "DatePickerTitle");
    assert!(app.world().get::<InteractionDisabled>(next).is_some());
    let outside = find(&mut app, "DatePickerCandidate11");
    assert!(app.world().get::<InteractionDisabled>(outside).is_some());
    click(&mut app, "DatePickerNext");
    click(&mut app, "DatePickerCandidate11");
    assert_eq!(
        WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
        DatePickerMonth {
            year: Date::MAX.year(),
            month: Month::November
        }
    );
    assert_eq!(app.world().resource::<Changes>().1, 1);
}
