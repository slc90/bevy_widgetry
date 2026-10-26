use bevy::prelude::*;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_warn};
use time::{Date, Month, OffsetDateTime, Weekday};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatePickerMonth {
    pub year: i32,
    pub month: Month,
}

impl From<Date> for DatePickerMonth {
    fn from(date: Date) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WidgetryDatePickerProps {
    pub selected_date: Option<Date>,
    pub displayed_month: Option<DatePickerMonth>,
    pub min_date: Option<Date>,
    pub max_date: Option<Date>,
    pub week_start: Weekday,
    pub colors: crate::WidgetryDatePickerColorOverrides,
}

impl Default for WidgetryDatePickerProps {
    fn default() -> Self {
        Self {
            selected_date: None,
            displayed_month: None,
            min_date: None,
            max_date: None,
            week_start: Weekday::Monday,
            colors: default(),
        }
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryDatePickerProps)]
pub struct WidgetryDatePicker;

#[derive(Component)]
pub(crate) struct State {
    pub(crate) selected: Option<Date>,
    pub(crate) displayed: DatePickerMonth,
    pub(crate) min: Option<Date>,
    pub(crate) max: Option<Date>,
    pub(crate) week_start: Weekday,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatePickerDay {
    pub date: Date,
    pub is_current_month: bool,
    pub is_selectable: bool,
    pub is_selected: bool,
    pub is_today: bool,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryDatePickerSelectionChanged {
    pub entity: Entity,
    pub selected_date: Option<Date>,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryDatePickerDisplayedMonthChanged {
    pub entity: Entity,
    pub displayed_month: DatePickerMonth,
}

pub(crate) fn invalid(reason: impl std::fmt::Display) -> BevyError {
    widgetry_error!(%reason, "DatePicker 请求或配置无效");
    bevy_widgetry_core::scene::logged_error(reason.to_string())
}

pub(crate) fn state(world: &World, root: Entity) -> Result<&State, BevyError> {
    if world.get::<WidgetryDatePicker>(root).is_none() {
        return Err(invalid(format!(
            "WidgetryDatePicker root is missing: {root:?}"
        )));
    }
    world
        .get::<State>(root)
        .ok_or_else(|| invalid("DatePicker state is missing"))
}

impl State {
    fn new(
        props: WidgetryDatePickerProps,
        local: impl FnOnce() -> Option<Date>,
        utc: impl FnOnce() -> Date,
    ) -> Result<Self, BevyError> {
        if props
            .min_date
            .zip(props.max_date)
            .is_some_and(|(min, max)| min > max)
        {
            return Err(invalid("min_date exceeds max_date"));
        }
        let displayed = props
            .displayed_month
            .or_else(|| props.selected_date.map(Into::into))
            .unwrap_or_else(|| {
                local()
                    .unwrap_or_else(|| {
                        widgetry_warn!("DatePicker 本地日期读取失败，显示月份使用回退日期");
                        props.min_date.unwrap_or_else(utc)
                    })
                    .into()
            });
        let state = Self {
            selected: props.selected_date,
            displayed,
            min: props.min_date,
            max: props.max_date,
            week_start: props.week_start,
        };
        if state.selected.is_some_and(|date| !state.selectable(date)) {
            return Err(invalid("initial selected_date is outside date bounds"));
        }
        state.grid(None)?;
        Ok(state)
    }

    pub(crate) fn selectable(&self, date: Date) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    fn grid(&self, today: Option<Date>) -> Result<[DatePickerDay; 42], BevyError> {
        let first = Date::from_calendar_date(self.displayed.year, self.displayed.month, 1)
            .map_err(invalid)?;
        let offset = (i32::from(first.weekday().number_days_from_monday())
            - i32::from(self.week_start.number_days_from_monday()))
        .rem_euclid(7);
        let start = first.to_julian_day() - offset;
        let mut result = [DatePickerDay {
            date: first,
            is_current_month: false,
            is_selectable: false,
            is_selected: false,
            is_today: false,
        }; 42];
        for (index, day) in result.iter_mut().enumerate() {
            let date = Date::from_julian_day(start + index as i32).map_err(invalid)?;
            *day = DatePickerDay {
                date,
                is_current_month: DatePickerMonth::from(date) == self.displayed,
                is_selectable: self.selectable(date),
                is_selected: self.selected == Some(date),
                is_today: today == Some(date),
            };
        }
        Ok(result)
    }
}

impl WidgetryDatePicker {
    fn scene(props: WidgetryDatePickerProps) -> impl Scene {
        let colors = props.colors.clone();
        bsn! {
            template(move |_| colors.clone().initial())
            template(move |_| State::new(props.clone(), || OffsetDateTime::now_local().ok().map(|now| now.date()), || OffsetDateTime::now_utc().date()))
            @crate::view::scene()
        }
    }

    pub fn selected_date(world: &World, root: Entity) -> Result<Option<Date>, BevyError> {
        Ok(state(world, root)?.selected)
    }

    pub fn displayed_month(world: &World, root: Entity) -> Result<DatePickerMonth, BevyError> {
        Ok(state(world, root)?.displayed)
    }

    pub fn days(
        world: &World,
        root: Entity,
        today: Option<Date>,
    ) -> Result<[DatePickerDay; 42], BevyError> {
        state(world, root)?.grid(today)
    }

    pub fn set_selected_date(commands: &mut Commands, root: Entity, selected: Option<Date>) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_date_in_world(world, root, selected).map(|_| ())
        });
    }

    pub fn set_selected_date_in_world(
        world: &mut World,
        root: Entity,
        selected: Option<Date>,
    ) -> Result<bool, BevyError> {
        let current = state(world, root)?;
        if selected.is_some_and(|date| !current.selectable(date)) {
            return Err(invalid("selected_date is outside date bounds"));
        }
        if current.selected == selected {
            return Ok(false);
        }
        let month = selected.map(Into::into).unwrap_or(current.displayed);
        let probe = State {
            selected,
            displayed: month,
            min: current.min,
            max: current.max,
            week_start: current.week_start,
        };
        probe.grid(None)?;
        let month_changed = month != current.displayed;
        *world
            .get_mut::<State>(root)
            .ok_or_else(|| invalid("DatePicker state is missing"))? = probe;
        world.trigger(WidgetryDatePickerSelectionChanged {
            entity: root,
            selected_date: selected,
        });
        if month_changed {
            world.trigger(WidgetryDatePickerDisplayedMonthChanged {
                entity: root,
                displayed_month: month,
            });
        }
        Ok(true)
    }

    pub fn set_displayed_month(commands: &mut Commands, root: Entity, month: DatePickerMonth) {
        commands.queue(move |world: &mut World| {
            Self::set_displayed_month_in_world(world, root, month).map(|_| ())
        });
    }

    pub fn set_displayed_month_in_world(
        world: &mut World,
        root: Entity,
        month: DatePickerMonth,
    ) -> Result<bool, BevyError> {
        let current = state(world, root)?;
        let probe = State {
            selected: current.selected,
            displayed: month,
            min: current.min,
            max: current.max,
            week_start: current.week_start,
        };
        probe.grid(None)?;
        if current.displayed == month {
            return Ok(false);
        }
        world
            .get_mut::<State>(root)
            .ok_or_else(|| invalid("DatePicker state is missing"))?
            .displayed = month;
        world.trigger(WidgetryDatePickerDisplayedMonthChanged {
            entity: root,
            displayed_month: month,
        });
        Ok(true)
    }

    pub fn shift_month(commands: &mut Commands, root: Entity, delta: i32) {
        commands.queue(move |world: &mut World| {
            Self::shift_month_in_world(world, root, delta).map(|_| ())
        });
    }

    pub fn shift_month_in_world(
        world: &mut World,
        root: Entity,
        delta: i32,
    ) -> Result<bool, BevyError> {
        let month = state(world, root)?.displayed;
        let total =
            i64::from(month.year) * 12 + i64::from(month.month as u8) - 1 + i64::from(delta);
        let year = i32::try_from(total.div_euclid(12)).map_err(invalid)?;
        let month = Month::try_from((total.rem_euclid(12) + 1) as u8).map_err(invalid)?;
        Self::set_displayed_month_in_world(world, root, DatePickerMonth { year, month })
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::LogCapture;
    use time::macros::date;

    #[test]
    fn initialization_priority_and_local_failure_do_not_select_or_invent_today() {
        for (explicit, selected, min, expected) in [
            (
                Some(date!(2020 - 03 - 01).into()),
                Some(date!(2021 - 04 - 02)),
                None,
                date!(2020 - 03 - 01),
            ),
            (
                None,
                Some(date!(2021 - 04 - 02)),
                None,
                date!(2021 - 04 - 01),
            ),
            (
                None,
                None,
                Some(date!(2022 - 05 - 02)),
                date!(2022 - 05 - 01),
            ),
            (None, None, None, date!(2023 - 06 - 01)),
        ] {
            let logs = LogCapture::default();
            let state = logs
                .run(|| {
                    State::new(
                        WidgetryDatePickerProps {
                            displayed_month: explicit,
                            selected_date: selected,
                            min_date: min,
                            ..default()
                        },
                        || None,
                        || date!(2023 - 06 - 20),
                    )
                })
                .unwrap();
            assert_eq!(state.displayed, expected.into());
            assert_eq!(state.selected, selected);
            assert!(state.grid(None).unwrap().iter().all(|day| !day.is_today));
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|r| r.level == bevy::log::Level::WARN)
                    .count(),
                usize::from(explicit.is_none() && selected.is_none())
            );
        }
        let state = State::new(
            default(),
            || Some(date!(2024 - 07 - 05)),
            || date!(2023 - 06 - 20),
        )
        .unwrap();
        assert_eq!(state.displayed, date!(2024 - 07 - 01).into());
        assert_eq!(state.selected, None);
    }

    #[test]
    fn unrepresentable_grid_is_rejected_without_partial_state_change() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryDatePicker { @selected_date: {Some(date!(2026-10-15))} } })
            .unwrap()
            .id();
        {
            let date = Date::MAX;
            let before = WidgetryDatePicker::displayed_month(app.world(), root).unwrap();
            assert!(
                WidgetryDatePicker::set_selected_date_in_world(app.world_mut(), root, Some(date))
                    .is_err()
            );
            assert_eq!(
                WidgetryDatePicker::displayed_month(app.world(), root).unwrap(),
                before
            );
            assert_eq!(
                WidgetryDatePicker::selected_date(app.world(), root).unwrap(),
                Some(date!(2026 - 10 - 15))
            );
        }
    }
}
