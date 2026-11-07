use crate::{Date, DatePickerDay, DatePickerFailure, DatePickerMonth, Weekday, WidgetryDatePicker};
use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryDatePickerHeadlessProps)]
#[require(Node)]
pub struct WidgetryDatePickerHeadless;

#[derive(Clone, Debug)]
pub struct WidgetryDatePickerHeadlessProps {
    pub selected_date: Option<Date>,
    pub displayed_month: Option<DatePickerMonth>,
    pub min_date: Option<Date>,
    pub max_date: Option<Date>,
    pub week_start: Weekday,
}
impl Default for WidgetryDatePickerHeadlessProps {
    fn default() -> Self {
        Self {
            selected_date: None,
            displayed_month: None,
            min_date: None,
            max_date: None,
            week_start: Weekday::Monday,
        }
    }
}

pub struct WidgetryDatePickerHeadlessPlugin;
impl Plugin for WidgetryDatePickerHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        crate::today::install(app);
        bevy_widgetry_internal_log::widgetry_debug!("WidgetryDatePickerHeadlessPlugin 注册完成");
    }
}

impl WidgetryDatePickerHeadless {
    fn scene(props: WidgetryDatePickerHeadlessProps) -> impl Scene {
        bsn! {
            template(move |context| {
                let root=context.entity.id();
                crate::model::State::new(props.clone(),||context.entity.world_scope(crate::today::local_date),||time::OffsetDateTime::now_utc().date())
                    .map_err(|error|error.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("date_picker.initialize").field("root",root)).into_bevy_error())
            })
        }
    }
    pub fn selected_date(world: &World, root: Entity) -> Result<Option<Date>, DatePickerFailure> {
        WidgetryDatePicker::selected_date(world, root)
    }
    pub fn displayed_month(
        world: &World,
        root: Entity,
    ) -> Result<DatePickerMonth, DatePickerFailure> {
        WidgetryDatePicker::displayed_month(world, root)
    }
    pub fn days(
        world: &World,
        root: Entity,
        today: Option<Date>,
    ) -> Result<[DatePickerDay; 42], DatePickerFailure> {
        WidgetryDatePicker::days(world, root, today)
    }
    pub fn set_selected_date_in_world(
        world: &mut World,
        root: Entity,
        date: Option<Date>,
    ) -> Result<bool, DatePickerFailure> {
        WidgetryDatePicker::set_selected_date_in_world(world, root, date)
    }
    pub fn set_selected_date(commands: &mut Commands, root: Entity, date: Option<Date>) {
        WidgetryDatePicker::set_selected_date(commands, root, date);
    }
    pub fn set_displayed_month_in_world(
        world: &mut World,
        root: Entity,
        month: DatePickerMonth,
    ) -> Result<bool, DatePickerFailure> {
        WidgetryDatePicker::set_displayed_month_in_world(world, root, month)
    }
    pub fn set_displayed_month(commands: &mut Commands, root: Entity, month: DatePickerMonth) {
        WidgetryDatePicker::set_displayed_month(commands, root, month);
    }
    pub fn shift_month_in_world(
        world: &mut World,
        root: Entity,
        delta: i32,
    ) -> Result<bool, DatePickerFailure> {
        WidgetryDatePicker::shift_month_in_world(world, root, delta)
    }
    pub fn shift_month(commands: &mut Commands, root: Entity, delta: i32) {
        WidgetryDatePicker::shift_month(commands, root, delta);
    }
    pub fn is_selectable(
        world: &World,
        root: Entity,
        date: Date,
    ) -> Result<bool, DatePickerFailure> {
        Ok(crate::model::state(world, root)?.selectable(date))
    }
    pub fn choose_date_in_world(
        world: &mut World,
        root: Entity,
        date: Option<Date>,
    ) -> Result<bool, DatePickerFailure> {
        let state = crate::model::state(world, root)?;
        if world.get::<bevy::ui::InteractionDisabled>(root).is_some()
            || date.is_some_and(|date| !state.selectable(date))
        {
            return Ok(false);
        }
        Self::set_selected_date_in_world(world, root, date)
    }
    pub fn choose_date(commands: &mut Commands, root: Entity, date: Option<Date>) {
        commands.queue(move |world: &mut World| {
            Self::choose_date_in_world(world, root, date)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }
    pub fn today(world: &World) -> Option<Date> {
        world
            .get_resource::<crate::today::Today>()
            .and_then(|today| today.0)
    }
}
