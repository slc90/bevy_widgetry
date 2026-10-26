use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_widgetry_internal_log::{widgetry_info, widgetry_warn};
use std::time::{Duration, Instant};
use time::Date;

const CHECK_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Resource, Default, PartialEq)]
pub(crate) struct Today(pub(crate) Option<Date>);

#[derive(Resource, Default)]
struct Clock {
    next: Option<Instant>,
    failed: bool,
    focused_wait: Option<(Duration, Duration)>,
    unfocused_wait: Option<(Duration, Duration)>,
    settings: Option<(bevy::ecs::change_detection::Tick, UpdateMode, UpdateMode)>,
}

fn check(world: &mut World, now: Instant, read: impl FnOnce() -> Result<Date, String>) {
    if world
        .resource::<Clock>()
        .next
        .is_some_and(|next| now < next)
    {
        return;
    }
    let date = {
        let mut clock = world.resource_mut::<Clock>();
        clock.next = Some(now + CHECK_INTERVAL);
        match read() {
            Ok(date) => {
                if clock.failed {
                    widgetry_info!("DatePicker 本地日期读取已恢复");
                }
                clock.failed = false;
                Some(date)
            }
            Err(error) => {
                if !clock.failed {
                    widgetry_warn!(%error, "DatePicker 本地日期读取失败，暂不显示今天标识");
                }
                clock.failed = true;
                None
            }
        }
    };
    if world.resource_mut::<Today>().set_if_neq(Today(date)) {
        world.write_message(bevy::window::RequestRedraw);
    }
}

fn limit_wait(
    mode: &mut UpdateMode,
    remaining: Duration,
    previous: &mut Option<(Duration, Duration)>,
) {
    if let UpdateMode::Reactive { wait, .. } = mode {
        let original = (*previous)
            .filter(|(_, applied)| applied == wait)
            .map_or(*wait, |(original, _)| original);
        // Bevy 0.20 on Windows can spend one timeout requesting a redraw and
        // another before updating the app, so budget for both waits.
        let remaining = remaining.min(CHECK_INTERVAL) / 2;
        // Winit forces another frame whenever UpdateMode changes.
        // Whole-second bounds let that follow-up frame settle instead of continually shortening the wait.
        let bound =
            Duration::from_secs(remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0));
        let applied = original.min(bound);
        *wait = applied;
        *previous = Some((original, applied));
    } else {
        *previous = None;
    }
}

fn limit_settings(world: &mut World) {
    world.resource_scope(|world, mut clock: Mut<Clock>| {
        let Some(mut settings) = world.get_resource_mut::<WinitSettings>() else {
            clock.focused_wait = None;
            clock.unfocused_wait = None;
            clock.settings = None;
            return;
        };
        // Changed waits are merged independently by limit_wait, preserving the
        // other mode's original wait and any event-flag-only edits.
        // Resource-level change ticks can only identify a same-value write to
        // both modes, which reasserts their current values as the host policy.
        if clock.settings.is_some_and(|(tick, focused, unfocused)| {
            tick != settings.last_changed()
                && focused == settings.focused_mode
                && unfocused == settings.unfocused_mode
        }) {
            clock.focused_wait = None;
            clock.unfocused_wait = None;
        }
        let remaining = clock.next.map_or(CHECK_INTERVAL, |next| {
            next.saturating_duration_since(Instant::now())
        });
        let mut focused = settings.focused_mode;
        let mut unfocused = settings.unfocused_mode;
        limit_wait(&mut focused, remaining, &mut clock.focused_wait);
        limit_wait(&mut unfocused, remaining, &mut clock.unfocused_wait);
        if focused != settings.focused_mode || unfocused != settings.unfocused_mode {
            settings.focused_mode = focused;
            settings.unfocused_mode = unfocused;
        }
        clock.settings = Some((settings.last_changed(), focused, unfocused));
    });
}

fn tick(world: &mut World) {
    check(world, Instant::now(), || {
        time::OffsetDateTime::now_local()
            .map(|now| now.date())
            .map_err(|error| error.to_string())
    });
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Today>()
        .init_resource::<Clock>()
        .add_message::<bevy::window::RequestRedraw>()
        .add_systems(First, tick)
        .add_systems(Last, limit_settings);
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::LogCapture;
    use time::macros::date;

    #[derive(Resource, Default)]
    struct Changes(usize);

    fn rings(app: &mut App, root: Entity) -> Vec<Entity> {
        let mut pending = vec![root];
        let mut visible = Vec::new();
        while let Some(entity) = pending.pop() {
            if let Some(children) = app.world().get::<Children>(entity) {
                pending.extend(children.iter());
            }
            if app
                .world()
                .get::<Name>(entity)
                .is_some_and(|name| name.as_str().starts_with("DatePickerToday"))
                && app.world().get::<Visibility>(entity) == Some(&Visibility::Inherited)
            {
                visible.push(entity);
            }
        }
        visible
    }

    #[test]
    fn today_rejects_public_icon_color_changes_without_losing_fixed_tint() {
        use crate::*;
        use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconColorOverrides};
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryDatePickerPlugin);
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryDatePicker {
                @selected_date: {Some(date!(2026-10-26))},
            } })
            .unwrap();
        app.world_mut().flush();
        let ring = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "DatePickerToday28")
            .unwrap()
            .0;
        let before = WidgetryIconColorOverrides::get(app.world(), ring)
            .unwrap()
            .clone();
        let logs = LogCapture::default();
        let results = logs.run(|| {
            [
                WidgetryIcon::set_color_in_world(app.world_mut(), ring, Color::NONE),
                WidgetryIcon::clear_color_in_world(app.world_mut(), ring),
            ]
        });
        for result in results {
            let error = result.unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert!(error.to_string().contains("托管部件"));
        }
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            2
        );
        assert_eq!(
            WidgetryIconColorOverrides::get(app.world(), ring).unwrap(),
            &before
        );
        app.update();
        assert_eq!(
            WidgetryIconColorOverrides::get(app.world(), ring).unwrap(),
            &before
        );
        assert_eq!(before.normal.foreground, Some(Color::WHITE));
        assert_eq!(before.disabled.foreground, Some(Color::WHITE));
    }

    #[test]
    fn shared_today_moves_or_disappears_without_business_events_and_keeps_fixed_icon_tint() {
        use crate::*;
        use bevy_widgetry_core::icon::WidgetryIconColorOverrides;
        use bevy_widgetry_theme::WidgetryThemeMode;
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryDatePickerPlugin)
            .init_resource::<Changes>();
        app.add_observer(
            |_: On<WidgetryDatePickerSelectionChanged>, mut count: ResMut<Changes>| count.0 += 1,
        );
        app.add_observer(
            |_: On<WidgetryDatePickerDisplayedMonthChanged>, mut count: ResMut<Changes>| {
                count.0 += 1
            },
        );
        let start = Instant::now();
        check(app.world_mut(), start, || Ok(date!(2026 - 10 - 26)));
        let selected = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryDatePicker { @selected_date: {Some(date!(2026-10-26))} } })
            .unwrap()
            .id();
        let bounded = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryDatePicker {
                @displayed_month: {Some(DatePickerMonth{year:2026,month:Month::October})},
                @min_date: {Some(date!(2026-10-27))},
            } })
            .unwrap()
            .id();
        app.update();
        for root in [selected, bounded] {
            let ring = rings(&mut app, root);
            assert_eq!(ring.len(), 1);
            assert_eq!(
                app.world().get::<Name>(ring[0]).unwrap().as_str(),
                "DatePickerToday28"
            );
        }
        app.world_mut()
            .entity_mut(bounded)
            .insert(bevy::ui::InteractionDisabled);
        for mode in [
            WidgetryThemeMode::Dark,
            WidgetryThemeMode::Light,
            WidgetryThemeMode::PinkDream,
            WidgetryThemeMode::KamuriViolet,
        ] {
            WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
            app.update();
            for root in [selected, bounded] {
                let ring = rings(&mut app, root)[0];
                let colors = WidgetryIconColorOverrides::get(app.world(), ring).unwrap();
                assert_eq!(colors.normal.foreground, Some(Color::WHITE));
                assert_eq!(colors.disabled.foreground, Some(Color::WHITE));
            }
        }
        for (step, date, expected) in [
            (1, Ok(date!(2026 - 10 - 27)), Some("DatePickerToday29")),
            (2, Err("offset unavailable".to_owned()), None),
            (3, Ok(date!(2026 - 10 - 28)), Some("DatePickerToday30")),
        ] {
            check(app.world_mut(), start + CHECK_INTERVAL * step, || date);
            app.update();
            for root in [selected, bounded] {
                let visible = rings(&mut app, root);
                assert_eq!(visible.len(), usize::from(expected.is_some()));
                if let Some(name) = expected {
                    assert_eq!(app.world().get::<Name>(visible[0]).unwrap().as_str(), name);
                }
                assert_eq!(
                    WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
                    DatePickerMonth {
                        year: 2026,
                        month: Month::October
                    }
                );
            }
            assert_eq!(
                WidgetryDatePicker::selected_date(app.world(), selected).unwrap(),
                Some(date!(2026 - 10 - 26))
            );
            assert_eq!(
                WidgetryDatePicker::selected_date(app.world(), bounded).unwrap(),
                None
            );
            assert_eq!(app.world().resource::<Changes>().0, 0);
        }
    }

    #[test]
    fn low_frequency_checks_clear_failed_today_and_recover_without_inventing_dates() {
        let mut app = App::new();
        install(&mut app);
        let start = Instant::now();
        let logs = LogCapture::default();
        logs.run(|| check(app.world_mut(), start, || Ok(date!(2026 - 10 - 26))));
        assert_eq!(
            app.world().resource::<Today>().0,
            Some(date!(2026 - 10 - 26))
        );
        check(app.world_mut(), start + Duration::from_secs(59), || {
            panic!("clock read too early")
        });
        let next = start + CHECK_INTERVAL;
        check(app.world_mut(), next, || Ok(date!(2026 - 10 - 27)));
        assert_eq!(
            app.world().resource::<Today>().0,
            Some(date!(2026 - 10 - 27))
        );
        logs.run(|| {
            check(app.world_mut(), next + CHECK_INTERVAL, || {
                Err("local offset unavailable".into())
            })
        });
        assert_eq!(app.world().resource::<Today>().0, None);
        logs.run(|| {
            check(app.world_mut(), next + CHECK_INTERVAL * 2, || {
                Err("still unavailable".into())
            })
        });
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.level == bevy::log::Level::WARN)
                .count(),
            1
        );
        logs.run(|| {
            check(app.world_mut(), next + CHECK_INTERVAL * 3, || {
                Ok(date!(2026 - 10 - 28))
            })
        });
        assert_eq!(
            app.world().resource::<Today>().0,
            Some(date!(2026 - 10 - 28))
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.level == bevy::log::Level::INFO)
                .count(),
            1
        );
    }

    #[test]
    fn rasterized_today_stays_inside_its_day_at_normal_and_double_scale() {
        use crate::*;
        use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, scene_app, spawn_ui_camera};
        for scale in [1.0, 2.0] {
            let mut app = scene_app();
            add_ui_plugins(&mut app);
            spawn_ui_camera(&mut app, UVec2::new(1000, 1000), scale);
            app.add_plugins(WidgetryDatePickerPlugin);
            check(app.world_mut(), Instant::now(), || {
                Ok(date!(2026 - 10 - 26))
            });
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryDatePicker {
                    @selected_date: {Some(date!(2026-10-26))},
                } })
                .unwrap()
                .id();
            app.update();
            let ring = rings(&mut app, root)[0];
            advance_until(
                &mut app,
                Duration::from_secs(5),
                "Today raster and layout",
                |world| {
                    world.get::<Children>(ring).is_some_and(|children| {
                        children.iter().any(|child| {
                            world.get::<ImageNode>(child).is_some()
                                && world
                                    .get::<ComputedNode>(child)
                                    .is_some_and(|node| node.size().min_element() > 0.0)
                        })
                    })
                },
            )
            .unwrap();
            let image = app
                .world()
                .get::<Children>(ring)
                .unwrap()
                .iter()
                .find(|child| app.world().get::<ImageNode>(*child).is_some())
                .unwrap();
            let world = app.world();
            let day = world.get::<ChildOf>(ring).unwrap().parent();
            let image_transform = world.get::<UiGlobalTransform>(image).unwrap();
            let (image_scale, _, image_center) = image_transform.to_scale_angle_translation();
            let image_size = world.get::<ComputedNode>(image).unwrap().size() * image_scale.abs();
            let (_, _, day_center) = world
                .get::<UiGlobalTransform>(day)
                .unwrap()
                .to_scale_angle_translation();
            let day_size = world.get::<ComputedNode>(day).unwrap().size();
            assert!(
                image_size.cmple(day_size + Vec2::splat(0.5)).all(),
                "{image_size:?} outside {day_size:?}"
            );
            assert!(image_center.distance(day_center) <= 1.0);
        }
    }

    #[test]
    fn reactive_wait_is_bounded_while_shorter_waits_and_event_policy_are_preserved() {
        let mut settings = WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::MAX),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(5)),
        };
        limit_wait(&mut settings.focused_mode, CHECK_INTERVAL, &mut None);
        limit_wait(&mut settings.unfocused_mode, CHECK_INTERVAL, &mut None);
        assert_eq!(
            settings.focused_mode,
            UpdateMode::reactive(CHECK_INTERVAL / 2)
        );
        assert_eq!(
            settings.unfocused_mode,
            UpdateMode::reactive_low_power(Duration::from_secs(5))
        );
        let mut continuous = UpdateMode::Continuous;
        limit_wait(&mut continuous, CHECK_INTERVAL, &mut None);
        assert_eq!(continuous, UpdateMode::Continuous);
    }

    #[test]
    fn reactive_wait_leaves_time_for_redraw_and_the_following_update() {
        let mut mode = UpdateMode::reactive_low_power(Duration::MAX);
        limit_wait(&mut mode, CHECK_INTERVAL, &mut None);
        let UpdateMode::Reactive { wait, .. } = mode else {
            panic!("reactive host policy must be retained")
        };
        assert!(wait * 2 <= CHECK_INTERVAL);
    }

    #[test]
    fn recent_input_aligns_wake_to_check_deadline_then_restores_host_wait() {
        let mut mode = UpdateMode::reactive_low_power(Duration::MAX);
        let mut previous = None;
        limit_wait(&mut mode, CHECK_INTERVAL, &mut previous);
        assert_eq!(mode, UpdateMode::reactive_low_power(CHECK_INTERVAL / 2));
        limit_wait(&mut mode, Duration::from_millis(2500), &mut previous);
        assert_eq!(mode, UpdateMode::reactive_low_power(Duration::from_secs(2)));
        limit_wait(&mut mode, CHECK_INTERVAL, &mut previous);
        assert_eq!(mode, UpdateMode::reactive_low_power(CHECK_INTERVAL / 2));
        mode = UpdateMode::reactive(Duration::from_secs(5));
        limit_wait(&mut mode, Duration::from_secs(2), &mut previous);
        assert_eq!(mode, UpdateMode::reactive(Duration::from_secs(1)));
        limit_wait(&mut mode, CHECK_INTERVAL, &mut previous);
        assert_eq!(mode, UpdateMode::reactive(Duration::from_secs(5)));
    }

    #[test]
    fn explicit_host_write_equal_to_temporary_limit_becomes_the_new_policy() {
        let mut app = App::new();
        install(&mut app);
        app.insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::MAX),
            unfocused_mode: UpdateMode::Continuous,
        });
        app.world_mut().resource_mut::<Clock>().next =
            Some(Instant::now() + Duration::from_secs(3));
        limit_settings(app.world_mut());
        assert_eq!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::reactive(Duration::from_secs(2))
        );
        app.world_mut().increment_change_tick();
        app.world_mut().resource_mut::<WinitSettings>().focused_mode =
            UpdateMode::reactive(Duration::from_secs(2));
        app.world_mut().resource_mut::<Clock>().next = Some(Instant::now() + CHECK_INTERVAL);
        limit_settings(app.world_mut());
        assert_eq!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::reactive(Duration::from_secs(2))
        );
    }

    #[test]
    fn changing_one_host_mode_preserves_the_other_modes_original_wait() {
        for change_focused in [true, false] {
            let mut app = App::new();
            install(&mut app);
            let focused = UpdateMode::reactive(Duration::from_secs(20));
            let unfocused = UpdateMode::reactive_low_power(Duration::from_secs(25));
            app.insert_resource(WinitSettings {
                focused_mode: focused,
                unfocused_mode: unfocused,
            });
            app.world_mut().resource_mut::<Clock>().next =
                Some(Instant::now() + Duration::from_secs(1));
            limit_settings(app.world_mut());
            app.world_mut().increment_change_tick();
            let mut settings = app.world_mut().resource_mut::<WinitSettings>();
            if change_focused {
                settings.focused_mode = UpdateMode::Continuous;
            } else {
                settings.unfocused_mode = UpdateMode::Continuous;
            }
            app.world_mut().resource_mut::<Clock>().next = Some(Instant::now() + CHECK_INTERVAL);
            limit_settings(app.world_mut());
            let settings = app.world().resource::<WinitSettings>();
            assert_eq!(
                settings.focused_mode,
                if change_focused {
                    UpdateMode::Continuous
                } else {
                    focused
                }
            );
            assert_eq!(
                settings.unfocused_mode,
                if change_focused {
                    unfocused
                } else {
                    UpdateMode::Continuous
                }
            );
        }
    }

    #[test]
    fn changing_event_flags_preserves_both_original_waits() {
        let mut app = App::new();
        install(&mut app);
        app.insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::from_secs(20)),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(25)),
        });
        app.world_mut().resource_mut::<Clock>().next =
            Some(Instant::now() + Duration::from_secs(1));
        limit_settings(app.world_mut());
        app.world_mut().increment_change_tick();
        if let UpdateMode::Reactive {
            react_to_device_events,
            ..
        } = &mut app.world_mut().resource_mut::<WinitSettings>().focused_mode
        {
            *react_to_device_events = false;
        }
        app.world_mut().resource_mut::<Clock>().next = Some(Instant::now() + CHECK_INTERVAL);
        limit_settings(app.world_mut());
        let settings = app.world().resource::<WinitSettings>();
        assert_eq!(
            settings.focused_mode,
            UpdateMode::reactive_low_power(Duration::from_secs(20))
        );
        assert_eq!(
            settings.unfocused_mode,
            UpdateMode::reactive_low_power(Duration::from_secs(25))
        );
    }

    #[test]
    fn late_host_settings_are_limited_before_returning_to_the_event_loop() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        install(&mut app);
        app.add_systems(Startup, |mut commands: Commands| {
            commands.insert_resource(WinitSettings {
                focused_mode: UpdateMode::reactive(Duration::MAX),
                unfocused_mode: UpdateMode::reactive_low_power(Duration::MAX),
            });
        });
        app.update();
        let settings = app.world().resource::<WinitSettings>();
        assert_eq!(
            settings.focused_mode,
            UpdateMode::reactive(CHECK_INTERVAL / 2)
        );
        assert_eq!(
            settings.unfocused_mode,
            UpdateMode::reactive_low_power(CHECK_INTERVAL / 2)
        );
        app.world_mut().resource_mut::<WinitSettings>().focused_mode =
            UpdateMode::reactive(Duration::MAX);
        app.update();
        assert_eq!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::reactive(CHECK_INTERVAL / 2)
        );
    }
}
