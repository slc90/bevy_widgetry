use crate::{
    gallery::{GalleryPage, mount_page, unmount_page},
    i18n::{DatePickerKey, Language, LocalizedText, NavigationKey, TextKey, localized, tr},
};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use bevy_widgetry::{button::WidgetryButton, date_picker::*, text::WidgetryText};
use time::macros::date;

pub(crate) struct DatePickerDemoPlugin;

impl Plugin for DatePickerDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WidgetryDatePickerPlugin)
            .register_type::<DatePickerDemoState>()
            .add_systems(OnEnter(GalleryPage::DatePicker), enter)
            .add_systems(OnExit(GalleryPage::DatePicker), exit)
            .add_systems(Update, sync_language)
            .add_systems(
                PostUpdate,
                refresh
                    .before(bevy::ui::UiSystems::Prepare)
                    .run_if(in_state(GalleryPage::DatePicker)),
            );
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct DatePickerDemoState {
    selection_events: u64,
    month_events: u64,
    history: Vec<String>,
    last_action: Option<DatePickerKey>,
    last_changed: bool,
}

impl DatePickerDemoState {
    fn record(&mut self, event: String) {
        if self.history.len() == 8 {
            self.history.remove(0);
        }
        self.history.push(event);
    }
}

#[derive(Component, FromTemplate)]
struct ExampleParts {
    picker: Entity,
    status: Entity,
    history: Entity,
    operation: Entity,
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    SetDate,
    Same,
    Clear,
    Previous,
    Next,
    Jump,
    SetDisabled,
}

impl Action {
    fn key(self) -> DatePickerKey {
        match self {
            Self::SetDate => DatePickerKey::SetDate,
            Self::Same => DatePickerKey::Same,
            Self::Clear => DatePickerKey::Clear,
            Self::Previous => DatePickerKey::Previous,
            Self::Next => DatePickerKey::Next,
            Self::Jump => DatePickerKey::Jump,
            Self::SetDisabled => DatePickerKey::SetDisabled,
        }
    }
}

fn sync_language(language: Res<Language>, mut picker: ResMut<WidgetryDatePickerLanguage>) {
    picker.set_if_neq(match *language {
        Language::English => WidgetryDatePickerLanguage::English,
        Language::SimplifiedChinese => WidgetryDatePickerLanguage::SimplifiedChinese,
    });
}

fn enter(world: &mut World) -> Result {
    let selected = match time::OffsetDateTime::now_local() {
        Ok(now) => Some(now.date()),
        Err(error) => {
            warn!(%error, "DatePicker 演示无法读取本地日期，主实例保持未选择");
            None
        }
    };
    mount_page(
        world,
        GalleryPage::DatePicker,
        bsn_list! { @scene(selected) },
    )?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::DatePicker)
}

fn label(key: DatePickerKey) -> impl Scene {
    bsn! { @localized(TextKey::DatePicker(key)) WidgetryText }
}

fn scene(selected: Option<Date>) -> impl Scene {
    bsn! {
        Name("DatePickerDemo")
        Node {width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(24), flex_shrink: 0.0}
        Children [
            @label(DatePickerKey::Description)--
            @example("DatePickerMain", DatePickerKey::Main, WidgetryDatePickerProps {selected_date: selected, ..default()},
                vec![Action::SetDate, Action::Same, Action::Clear, Action::Previous, Action::Next, Action::Jump], false, false)--
            Node {width: percent(100), column_gap: px(24), row_gap: px(24), flex_wrap: FlexWrap::Wrap, align_items: AlignItems::Start, flex_shrink: 0.0}
            Children [
                @example("DatePickerUnselected", DatePickerKey::Unselected, WidgetryDatePickerProps::default(), vec![], false, true)--
                @example("DatePickerBounded", DatePickerKey::Bounded, WidgetryDatePickerProps {
                    selected_date: Some(date!(2026-10-15)), min_date: Some(date!(2026-10-10)), max_date: Some(date!(2026-10-20)), ..default()
                }, vec![], false, true)--
                @example("DatePickerDisabled", DatePickerKey::Disabled, WidgetryDatePickerProps {selected_date: Some(date!(2026-10-15)), ..default()}, vec![Action::SetDisabled], true, true)
            ]
        ]
    }
}

fn example(
    name: &'static str,
    title: DatePickerKey,
    props: WidgetryDatePickerProps,
    actions: Vec<Action>,
    disabled: bool,
    compact: bool,
) -> impl Scene {
    let buttons: Vec<_> = actions.into_iter().map(|action| bsn! {
        Name({format!("{name}{action:?}")}) template(move |_| Ok(action))
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @label(action.key()) }) as Box<dyn SceneList>)} }
        on(operate)
    }).collect();
    let disabled = disabled.then_some(bsn! {InteractionDisabled});
    bsn! {
        ExampleParts {picker: #DatePickerExamplePicker, status: #DatePickerExampleStatus, history: #DatePickerExampleHistory, operation: #DatePickerExampleOperation}
        Name({format!("{name}Example")})
        Node {width: {if compact {px(304)} else {percent(100)}}, min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(10), flex_shrink: 0.0}
        Children [
            @label(title)--
            Node {width: percent(100), flex_direction: {if compact {FlexDirection::Column} else {FlexDirection::Row}}, flex_wrap: FlexWrap::Wrap, column_gap: px(24), row_gap: px(12), align_items: AlignItems::Start, flex_shrink: 0.0}
            Children [
                #DatePickerExamplePicker Name(name)
                @WidgetryDatePicker {@selected_date: {props.selected_date}, @displayed_month: {props.displayed_month}, @min_date: {props.min_date}, @max_date: {props.max_date}}
                @{disabled} template(|_| Ok(DatePickerDemoState::default())) on(selection_changed) on(month_changed)--
                Node {flex_direction: FlexDirection::Column, row_gap: px(10), flex_grow: 1.0, flex_basis: {if compact {Val::Auto} else {px(280)}}, min_width: {if compact {px(0)} else {px(280)}}, max_width: percent(100)}
                Children [
                    #DatePickerExampleStatus Name({format!("{name}Status")}) Text("") WidgetryText--
                    #DatePickerExampleHistory Name({format!("{name}History")}) Text("") WidgetryText--
                    Node {width: percent(100), flex_wrap: FlexWrap::Wrap, column_gap: px(6), row_gap: px(6)} Children [{buttons}]--
                    #DatePickerExampleOperation Name({format!("{name}Operation")}) Text("") WidgetryText
                ]
            ]
        ]
    }
}

fn selection_changed(
    event: On<WidgetryDatePickerSelectionChanged>,
    mut states: Query<&mut DatePickerDemoState>,
) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.selection_events += 1;
        state.record(format!(
            "SelectionChanged({})",
            event
                .selected_date
                .map_or_else(|| "None".into(), |date| format!("Some({date})"))
        ));
        info!(entity=?event.entity, selected=?event.selected_date, "DatePicker 选择已变化");
    }
}

fn month_changed(
    event: On<WidgetryDatePickerDisplayedMonthChanged>,
    mut states: Query<&mut DatePickerDemoState>,
) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.month_events += 1;
        let month = event.displayed_month;
        state.record(format!(
            "DisplayedMonthChanged({}-{:02})",
            month.year, month.month as u8
        ));
        info!(entity=?event.entity, month=?month, "DatePicker 显示月份已变化");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    parents: Query<&ChildOf>,
    parts: Query<&ExampleParts>,
    mut commands: Commands,
) -> Result {
    let Ok(action) = actions.get(event.entity).copied() else {
        return Ok(());
    };
    let root = parents
        .iter_ancestors(event.entity)
        .find_map(|ancestor| parts.get(ancestor).ok().map(|parts| parts.picker))
        .ok_or_else(|| {
            error!(entity=?event.entity, "DatePicker 操作按钮缺少示例归属");
            BevyError::error("DatePicker operation owner is missing")
        })?;
    commands.queue(move |world: &mut World| -> Result {
        let changed = match action {
            Action::SetDate => WidgetryDatePicker::set_selected_date_in_world(
                world,
                root,
                Some(date!(2026 - 10 - 26)),
            )?,
            Action::Same => {
                let value = WidgetryDatePicker::selected_date(world, root)?;
                WidgetryDatePicker::set_selected_date_in_world(world, root, value)?
            }
            Action::Clear => WidgetryDatePicker::set_selected_date_in_world(world, root, None)?,
            Action::Previous => WidgetryDatePicker::shift_month_in_world(world, root, -1)?,
            Action::Next => WidgetryDatePicker::shift_month_in_world(world, root, 1)?,
            Action::Jump => WidgetryDatePicker::set_displayed_month_in_world(
                world,
                root,
                DatePickerMonth {
                    year: 2027,
                    month: Month::January,
                },
            )?,
            Action::SetDisabled => WidgetryDatePicker::set_selected_date_in_world(
                world,
                root,
                Some(date!(2026 - 11 - 06)),
            )?,
        };
        let mut state = world.get_mut::<DatePickerDemoState>(root).ok_or_else(|| {
            error!(?root, "DatePicker 示例观测状态缺失");
            BevyError::error("DatePicker demo state is missing")
        })?;
        state.last_action = Some(action.key());
        state.last_changed = changed;
        info!(?root, ?action, changed, "通过公开 API 操作 DatePicker");
        Ok(())
    });
    Ok(())
}

fn refresh(world: &mut World) -> Result {
    let language = *world.resource::<Language>();
    let examples = world
        .query::<&ExampleParts>()
        .iter(world)
        .map(|p| (p.picker, p.status, p.history, p.operation))
        .collect::<Vec<_>>();
    let mut changed = false;
    for (picker, status, history, operation) in examples {
        let selected = WidgetryDatePicker::selected_date(world, picker)?;
        let month = WidgetryDatePicker::displayed_month(world, picker)?;
        let state = world.get::<DatePickerDemoState>(picker).ok_or_else(|| {
            error!(?picker, "DatePicker 示例观测状态缺失");
            BevyError::error("DatePicker demo state is missing")
        })?;
        let value = LocalizedText::with_args(
            TextKey::DatePicker(DatePickerKey::Status),
            vec![
                (
                    "selected",
                    selected.map_or_else(
                        || tr(language, TextKey::DatePicker(DatePickerKey::None)).into(),
                        |date| date.to_string(),
                    ),
                ),
                ("month", format!("{}-{:02}", month.year, month.month as u8)),
                ("selection_events", state.selection_events.to_string()),
                ("month_events", state.month_events.to_string()),
            ],
        )
        .render(language);
        let events = LocalizedText::with_args(
            TextKey::DatePicker(DatePickerKey::History),
            vec![(
                "events",
                if state.history.is_empty() {
                    "—".into()
                } else {
                    state.history.join("\n")
                },
            )],
        )
        .render(language);
        let result = if let Some(action) = state.last_action {
            LocalizedText::with_args(
                TextKey::DatePicker(DatePickerKey::Operation),
                vec![
                    ("action", tr(language, TextKey::DatePicker(action)).into()),
                    (
                        "changed",
                        tr(
                            language,
                            TextKey::DatePicker(if state.last_changed {
                                DatePickerKey::Changed
                            } else {
                                DatePickerKey::Unchanged
                            }),
                        )
                        .into(),
                    ),
                ],
            )
            .render(language)
        } else {
            tr(language, TextKey::DatePicker(DatePickerKey::Ready)).into()
        };
        for (entity, text) in [(status, value), (history, events), (operation, result)] {
            let mut current = world.get_mut::<Text>(entity).ok_or_else(|| {
                error!(?entity, "DatePicker 观测面板文字缺失");
                BevyError::error("DatePicker readout Text is missing")
            })?;
            changed |= current.set_if_neq(Text(text));
        }
    }
    if changed {
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}

pub(crate) fn color_examples() -> impl Scene {
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::DatePicker),
        TextKey::DatePicker(DatePickerKey::Overrides),
        bsn! {@WidgetryDatePicker},
        bsn! {@WidgetryDatePicker},
        apply_colors,
    )
}

fn apply_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool> {
    WidgetryDatePickerColorOverrides::set_in_world(
        world,
        root,
        if apply {
            WidgetryDatePickerColorOverrides {
                border: Some(crate::color_showcase::PURPLE),
                selected: Some(crate::color_showcase::TEAL),
                foreground: Some(crate::color_showcase::TEAL),
                ..default()
            }
        } else {
            default()
        },
    )
}
