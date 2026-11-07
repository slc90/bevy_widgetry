use crate::{
    Date, DatePickerFailure, DatePickerMonth, Month, WidgetryDatePickerHeadless as Picker,
};
use bevy::{prelude::*, ui::InteractionDisabled};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerView {
    #[default]
    Days,
    Months(i32),
    Years(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerCandidateValue {
    Month(DatePickerMonth),
    Year(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatePickerCandidate {
    pub value: DatePickerCandidateValue,
    pub is_selectable: bool,
    pub is_current: bool,
}

#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryDatePickerNavigation;

#[derive(Component, Default)]
pub(crate) struct Navigation(pub(crate) DatePickerView);

pub(crate) fn navigation(world: &World, root: Entity) -> Result<DatePickerView, DatePickerFailure> {
    crate::model::state(world, root)?;
    if world.get::<WidgetryDatePickerNavigation>(root).is_none() {
        return Err(crate::error::missing(
            root,
            root,
            "WidgetryDatePickerNavigation",
            "date_picker.navigation",
        ));
    }
    world
        .get::<Navigation>(root)
        .map(|state| state.0)
        .ok_or_else(|| crate::error::missing(root, root, "Navigation", "date_picker.navigation"))
}

fn shifted_month(month: DatePickerMonth, delta: i32) -> Option<DatePickerMonth> {
    let total = i64::from(month.year) * 12 + month.month as i64 - 1 + i64::from(delta);
    Some(DatePickerMonth {
        year: i32::try_from(total.div_euclid(12)).ok()?,
        month: Month::January.nth_next(total.rem_euclid(12) as u8),
    })
}

fn shifted_view(view: DatePickerView, delta: i32) -> Option<DatePickerView> {
    let (first, last, years) = match view {
        DatePickerView::Days => return None,
        DatePickerView::Months(year) => (
            i64::from(year) + i64::from(delta),
            i64::from(year) + i64::from(delta),
            false,
        ),
        DatePickerView::Years(start) => (
            i64::from(start) + i64::from(delta) * 12,
            i64::from(start) + i64::from(delta) * 12 + 11,
            true,
        ),
    };
    if last < i64::from(Date::MIN.year()) || first > i64::from(Date::MAX.year()) {
        return None;
    }
    let first = i32::try_from(first).ok()?;
    Some(if years {
        DatePickerView::Years(first)
    } else {
        DatePickerView::Months(first)
    })
}

impl WidgetryDatePickerNavigation {
    fn scene() -> impl Scene {
        bsn! {template(|_| Ok(Navigation::default()))}
    }

    pub fn view(world: &World, root: Entity) -> Result<DatePickerView, DatePickerFailure> {
        navigation(world, root)
    }

    pub fn candidates(
        world: &World,
        root: Entity,
    ) -> Result<Vec<DatePickerCandidate>, DatePickerFailure> {
        let state = crate::model::state(world, root)?;
        Ok(match navigation(world, root)? {
            DatePickerView::Days => Vec::new(),
            DatePickerView::Months(year) => (0..12)
                .map(|index| {
                    let month = DatePickerMonth {
                        year,
                        month: Month::January.nth_next(index),
                    };
                    DatePickerCandidate {
                        value: DatePickerCandidateValue::Month(month),
                        is_selectable: state.represents(month),
                        is_current: state.displayed == month,
                    }
                })
                .collect(),
            DatePickerView::Years(start) => (start..start + 12)
                .map(|year| DatePickerCandidate {
                    value: DatePickerCandidateValue::Year(year),
                    is_selectable: (Date::MIN.year()..=Date::MAX.year()).contains(&year),
                    is_current: state.displayed.year == year,
                })
                .collect(),
        })
    }

    pub fn can_browse(world: &World, root: Entity, delta: i32) -> Result<bool, DatePickerFailure> {
        let view = navigation(world, root)?;
        Ok(if view == DatePickerView::Days {
            let state = crate::model::state(world, root)?;
            shifted_month(state.displayed, delta).is_some_and(|month| state.represents(month))
        } else {
            shifted_view(view, delta).is_some()
        })
    }

    fn commit(
        world: &mut World,
        root: Entity,
        next: DatePickerView,
    ) -> Result<bool, DatePickerFailure> {
        if navigation(world, root)? == next {
            return Ok(false);
        }
        world
            .get_mut::<Navigation>(root)
            .ok_or_else(|| {
                crate::error::missing(root, root, "Navigation", "date_picker.navigation")
            })?
            .0 = next;
        Ok(true)
    }

    pub fn enter_parent_in_world(
        world: &mut World,
        root: Entity,
    ) -> Result<bool, DatePickerFailure> {
        let view = navigation(world, root)?;
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(false);
        }
        let next = match view {
            DatePickerView::Days => {
                DatePickerView::Months(Picker::displayed_month(world, root)?.year)
            }
            DatePickerView::Months(year) => DatePickerView::Years(year.div_euclid(12) * 12),
            DatePickerView::Years(_) => return Ok(false),
        };
        Self::commit(world, root, next)
    }

    pub fn browse_in_world(
        world: &mut World,
        root: Entity,
        delta: i32,
    ) -> Result<bool, DatePickerFailure> {
        let view = navigation(world, root)?;
        if world.get::<InteractionDisabled>(root).is_some()
            || !Self::can_browse(world, root, delta)?
        {
            return Ok(false);
        }
        if view == DatePickerView::Days {
            return Picker::shift_month_in_world(world, root, delta);
        }
        match shifted_view(view, delta) {
            Some(next) => Self::commit(world, root, next),
            None => Ok(false),
        }
    }

    pub fn choose_month_in_world(
        world: &mut World,
        root: Entity,
        month: Month,
    ) -> Result<bool, DatePickerFailure> {
        let view = navigation(world, root)?;
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(false);
        }
        let DatePickerView::Months(year) = view else {
            return Ok(false);
        };
        let month = DatePickerMonth { year, month };
        if !crate::model::state(world, root)?.represents(month) {
            return Ok(false);
        }
        Picker::set_displayed_month_in_world(world, root, month)?;
        // A committed-month observer may remove the navigation capability or its root.
        if world.get::<WidgetryDatePickerNavigation>(root).is_none()
            || world.get::<Navigation>(root).is_none()
        {
            return Ok(true);
        }
        Self::commit(world, root, DatePickerView::Days)?;
        Ok(true)
    }

    pub fn choose_year_in_world(
        world: &mut World,
        root: Entity,
        year: i32,
    ) -> Result<bool, DatePickerFailure> {
        let view = navigation(world, root)?;
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(false);
        }
        let DatePickerView::Years(start) = view else {
            return Ok(false);
        };
        if !(start..start + 12).contains(&year)
            || !(Date::MIN.year()..=Date::MAX.year()).contains(&year)
        {
            return Ok(false);
        }
        Self::commit(world, root, DatePickerView::Months(year))
    }

    pub fn enter_parent(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::enter_parent_in_world(world, root)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }
    pub fn browse(commands: &mut Commands, root: Entity, delta: i32) {
        commands.queue(move |world: &mut World| {
            Self::browse_in_world(world, root, delta)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }
    pub fn choose_month(commands: &mut Commands, root: Entity, month: Month) {
        commands.queue(move |world: &mut World| {
            Self::choose_month_in_world(world, root, month)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }
    pub fn choose_year(commands: &mut Commands, root: Entity, year: i32) {
        commands.queue(move |world: &mut World| {
            Self::choose_year_in_world(world, root, year)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }
}
