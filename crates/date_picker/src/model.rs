pub(crate) const DAYS_PER_WEEK: usize = 7;
pub(crate) const GRID_WEEK_COUNT: usize = 6;
pub(crate) const GRID_CELL_COUNT: usize = DAYS_PER_WEEK * GRID_WEEK_COUNT;
pub(crate) const MONTHS_PER_YEAR: i64 = 12;

use crate::{
    DatePickerError, DatePickerFailure,
    error::{failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::WidgetryEcsError};
use time::{Date, Month, Weekday};

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

pub(crate) fn state(world: &World, root: Entity) -> Result<&State, DatePickerFailure> {
    if world
        .get::<crate::WidgetryDatePickerHeadless>(root)
        .is_none()
    {
        return Err(failure(
            DatePickerError::InvalidRoot { root },
            ErrorFrame::new("date_picker.state"),
        ));
    }
    world
        .get::<State>(root)
        .ok_or_else(|| missing(root, root, "State", "date_picker.state"))
}

impl State {
    pub(crate) fn new(
        props: crate::WidgetryDatePickerHeadlessProps,
        local: impl FnOnce() -> Result<Date, WidgetryEcsError>,
        utc: impl FnOnce() -> Date,
    ) -> Result<Self, DatePickerFailure> {
        if let Some((min, max)) = props.min_date.zip(props.max_date)
            && min > max
        {
            return Err(failure(
                DatePickerError::ReversedBounds { min, max },
                ErrorFrame::new("date_picker.initialize"),
            ));
        }
        let displayed = props
            .displayed_month
            .or_else(|| props.selected_date.map(Into::into))
            .unwrap_or_else(|| match local() {
                Ok(date) => date.into(),
                Err(error) => {
                    let fallback = props.min_date.unwrap_or_else(utc);
                    for diagnostic in error.diagnostics() {
                        diagnostic.record_handled(
                            "initialize displayed month",
                            &format!(
                                "use {} month: {fallback}",
                                if props.min_date.is_some() {
                                    "min_date"
                                } else {
                                    "UTC"
                                }
                            ),
                        );
                    }
                    fallback.into()
                }
            });
        let state = Self {
            selected: props.selected_date,
            displayed,
            min: props.min_date,
            max: props.max_date,
            week_start: props.week_start,
        };
        if let Some(selected) = state.selected.filter(|date| !state.selectable(*date)) {
            return Err(state.selection_error(selected, "date_picker.initialize"));
        }
        state.grid(None)?;
        Ok(state)
    }

    fn selection_error(&self, selected: Date, operation: &'static str) -> DatePickerFailure {
        failure(
            DatePickerError::SelectionOutOfRange {
                selected,
                min: self.min,
                max: self.max,
            },
            ErrorFrame::new(operation),
        )
    }

    fn calendar_error(
        &self,
        source: time::error::ComponentRange,
        grid_index: Option<usize>,
        julian_day: Option<i32>,
    ) -> DatePickerFailure {
        failure(
            DatePickerError::Calendar {
                displayed: self.displayed,
                week_start: self.week_start,
                grid_index,
                julian_day,
                source,
            },
            ErrorFrame::new("date_picker.grid"),
        )
    }

    pub(crate) fn selectable(&self, date: Date) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    pub(crate) fn represents(&self, month: DatePickerMonth) -> bool {
        let Ok(first) = Date::from_calendar_date(month.year, month.month, 1) else {
            return false;
        };
        let offset = (i32::from(first.weekday().number_days_from_monday())
            - i32::from(self.week_start.number_days_from_monday()))
        .rem_euclid(DAYS_PER_WEEK as i32);
        let start = first.to_julian_day() - offset;
        Date::from_julian_day(start).is_ok()
            && Date::from_julian_day(start + (GRID_CELL_COUNT as i32 - 1)).is_ok()
    }

    fn grid(
        &self,
        today: Option<Date>,
    ) -> Result<[DatePickerDay; GRID_CELL_COUNT], DatePickerFailure> {
        let first = Date::from_calendar_date(self.displayed.year, self.displayed.month, 1)
            .map_err(|source| self.calendar_error(source, None, None))?;
        let offset = (i32::from(first.weekday().number_days_from_monday())
            - i32::from(self.week_start.number_days_from_monday()))
        .rem_euclid(DAYS_PER_WEEK as i32);
        let start = first.to_julian_day() - offset;
        let mut result = [DatePickerDay {
            date: first,
            is_current_month: false,
            is_selectable: false,
            is_selected: false,
            is_today: false,
        }; GRID_CELL_COUNT];
        for (index, day) in result.iter_mut().enumerate() {
            let date = Date::from_julian_day(start + index as i32).map_err(|source| {
                self.calendar_error(source, Some(index), Some(start + index as i32))
            })?;
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
            template(move |context| colors.clone().initial(context.entity.id()).map_err(DatePickerFailure::into_bevy_error))
            @crate::WidgetryDatePickerHeadless {
                @selected_date:{props.selected_date},@displayed_month:{props.displayed_month},
                @min_date:{props.min_date},@max_date:{props.max_date},@week_start:{props.week_start},
            }
            @crate::view::scene()
        }
    }

    pub fn selected_date(world: &World, root: Entity) -> Result<Option<Date>, DatePickerFailure> {
        Ok(state(world, root)?.selected)
    }

    pub fn displayed_month(
        world: &World,
        root: Entity,
    ) -> Result<DatePickerMonth, DatePickerFailure> {
        Ok(state(world, root)?.displayed)
    }

    pub fn days(
        world: &World,
        root: Entity,
        today: Option<Date>,
    ) -> Result<[DatePickerDay; GRID_CELL_COUNT], DatePickerFailure> {
        state(world, root)?.grid(today).map_err(|error| {
            error.with_context(ErrorFrame::new("date_picker.days").field("root", root))
        })
    }

    pub fn set_selected_date(commands: &mut Commands, root: Entity, selected: Option<Date>) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_date_in_world(world, root, selected)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }

    pub fn set_selected_date_in_world(
        world: &mut World,
        root: Entity,
        selected: Option<Date>,
    ) -> Result<bool, DatePickerFailure> {
        let current = state(world, root)?;
        if let Some(date) = selected.filter(|date| !current.selectable(*date)) {
            return Err(current
                .selection_error(date, "date_picker.set_selected_date")
                .with_context(
                    ErrorFrame::new("date_picker.set_selected_date").field("root", root),
                ));
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
        probe.grid(None).map_err(|error| {
            error.with_context(
                ErrorFrame::new("date_picker.validate_month")
                    .field("root", root)
                    .field("selected", format_args!("{selected:?}")),
            )
        })?;
        let month_changed = month != current.displayed;
        *world
            .get_mut::<State>(root)
            .ok_or_else(|| missing(root, root, "State", "date_picker.commit"))? = probe;
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
            Self::set_displayed_month_in_world(world, root, month)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }

    pub fn set_displayed_month_in_world(
        world: &mut World,
        root: Entity,
        month: DatePickerMonth,
    ) -> Result<bool, DatePickerFailure> {
        let current = state(world, root)?;
        let probe = State {
            selected: current.selected,
            displayed: month,
            min: current.min,
            max: current.max,
            week_start: current.week_start,
        };
        probe.grid(None).map_err(|error| {
            error.with_context(
                ErrorFrame::new("date_picker.set_displayed_month").field("root", root),
            )
        })?;
        if current.displayed == month {
            return Ok(false);
        }
        world
            .get_mut::<State>(root)
            .ok_or_else(|| missing(root, root, "State", "date_picker.commit"))?
            .displayed = month;
        world.trigger(WidgetryDatePickerDisplayedMonthChanged {
            entity: root,
            displayed_month: month,
        });
        Ok(true)
    }

    pub fn shift_month(commands: &mut Commands, root: Entity, delta: i32) {
        commands.queue(move |world: &mut World| {
            Self::shift_month_in_world(world, root, delta)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }

    pub fn shift_month_in_world(
        world: &mut World,
        root: Entity,
        delta: i32,
    ) -> Result<bool, DatePickerFailure> {
        let current = state(world, root)?;
        let month = current.displayed;
        let total = i64::from(month.year) * MONTHS_PER_YEAR + i64::from(month.month as u8) - 1
            + i64::from(delta);
        let year = i32::try_from(total.div_euclid(MONTHS_PER_YEAR)).map_err(|source| {
            failure(
                DatePickerError::MonthShift {
                    displayed: month,
                    delta,
                    source,
                },
                ErrorFrame::new("date_picker.shift_month").field("root", root),
            )
        })?;
        let shifted_month = Month::try_from((total.rem_euclid(MONTHS_PER_YEAR) + 1) as u8)
            .map_err(|source| {
                failure(
                    DatePickerError::Calendar {
                        displayed: month,
                        week_start: current.week_start,
                        grid_index: None,
                        julian_day: None,
                        source,
                    },
                    ErrorFrame::new("date_picker.shift_month")
                        .field("root", root)
                        .field("delta", delta),
                )
            })?;
        let shifted = DatePickerMonth {
            year,
            month: shifted_month,
        };
        Self::set_displayed_month_in_world(world, root, shifted).map_err(|error| {
            error.with_context(
                ErrorFrame::new("date_picker.shift_month")
                    .field("root", root)
                    .field("delta", delta),
            )
        })
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
                        crate::WidgetryDatePickerHeadlessProps {
                            displayed_month: explicit,
                            selected_date: selected,
                            min_date: min,
                            ..default()
                        },
                        || {
                            Err(WidgetryEcsError::new(failure(
                                DatePickerError::LocalTime {
                                    source: time::error::IndeterminateOffset,
                                },
                                ErrorFrame::new("date_picker.local_date"),
                            )))
                        },
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
                    .filter(|r| r.level == bevy::log::Level::ERROR)
                    .count(),
                usize::from(explicit.is_none() && selected.is_none())
            );
            let sources = logs
                .records()
                .into_iter()
                .filter(|r| r.phase() == Some(bevy_widgetry_test_utils::DiagnosticPhase::Source))
                .collect::<Vec<_>>();
            let handled = logs
                .records()
                .into_iter()
                .filter(|r| r.phase() == Some(bevy_widgetry_test_utils::DiagnosticPhase::Handled))
                .collect::<Vec<_>>();
            assert_eq!(handled.len(), sources.len());
            if let Some(source) = sources.first() {
                assert_eq!(source.fields["error_id"], handled[0].fields["error_id"]);
                assert!(handled[0].fields["outcome"].contains(if min.is_some() {
                    "min_date"
                } else {
                    "UTC"
                }));
            }
        }
        let state = State::new(
            crate::WidgetryDatePickerHeadlessProps::default(),
            || Ok(date!(2024 - 07 - 05)),
            || date!(2023 - 06 - 20),
        )
        .unwrap();
        assert_eq!(state.displayed, date!(2024 - 07 - 01).into());
        assert_eq!(state.selected, None);
    }

    #[test]
    fn unrepresentable_grid_is_rejected_without_partial_state_change() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryDatePickerPlugin);
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
