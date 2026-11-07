use crate::{
    DatePickerError, DatePickerFailure, WidgetryDatePicker, WidgetryDatePickerNavigation as Nav,
    colors::ColorState,
    error::{core, failure, missing},
    model::State,
    navigation::{DatePickerView as View, Navigation, navigation},
};
use bevy::{
    ecs::entity::EntityHashSet,
    picking::{events::PointerClick, hover::Hovered, pointer::PointerButton},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
};
use bevy_widgetry_core::{
    color::WidgetryStyleOwner, disabled::WidgetryEffectiveDisabled, icon::WidgetryIcon,
    pointer::WidgetryPointerPressed, ui::WidgetryUiSystems,
};
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_theme::WidgetryTheme;

#[derive(Resource, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Resource)]
pub enum WidgetryDatePickerLanguage {
    #[default]
    English,
    SimplifiedChinese,
}

impl WidgetryDatePickerLanguage {
    fn year(self, year: i32) -> String {
        match self {
            Self::English => year.to_string(),
            Self::SimplifiedChinese => format!("{year}年"),
        }
    }

    fn month(self, index: usize) -> String {
        match self {
            Self::English => [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ][index]
                .into(),
            Self::SimplifiedChinese => format!("{}月", index + 1),
        }
    }

    fn weekday(self, index: usize) -> &'static str {
        match self {
            Self::English => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][index],
            Self::SimplifiedChinese => {
                ["周一", "周二", "周三", "周四", "周五", "周六", "周日"][index]
            }
        }
    }
}

#[derive(Component, Clone, Copy)]
enum Part {
    Previous,
    Next,
    Title,
    Weekday(usize),
    Day(usize),
    Clear,
    Candidate(usize),
}

#[derive(Component, Clone, Copy)]
enum Panel {
    Days,
    Candidates,
}

#[derive(Component)]
struct TodayRing(usize);

pub(crate) fn scene() -> impl Scene {
    let weekdays: Vec<_> = (0..7)
        .map(|index| cell(Part::Weekday(index), format!("DatePickerWeekday{index}")))
        .collect();
    let days: Vec<_> = (0..42)
        .map(|index| cell(Part::Day(index), format!("DatePickerDay{index}")))
        .collect();
    let candidates: Vec<_> = (0..12)
        .map(|index| {
            cell(
                Part::Candidate(index),
                format!("DatePickerCandidate{index}"),
            )
        })
        .collect();
    bsn! {
        @Nav
        Node { width: px(304), height: px(352), padding: UiRect::all(px(10)), flex_direction: FlexDirection::Column, flex_shrink: 0.0, border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(8)) }
        BackgroundColor BorderColor
        Children [
            Node { height: px(36), flex_shrink: 0.0, justify_content: JustifyContent::SpaceBetween } Children [
                @cell(Part::Previous, "DatePickerPrevious".into()) Node {width: px(32)}--
                @cell(Part::Title, "DatePickerTitle".into()) Node {flex_grow: 1.0}--
                @cell(Part::Next, "DatePickerNext".into()) Node {width: px(32)}
            ]--
            template(|_| Ok(Panel::Days)) Name("DatePickerDays")
            Node {height: px(268), flex_direction: FlexDirection::Column, flex_shrink: 0.0} Children [
                Node {height: px(28), display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::flex(7, 1.0)]}, flex_shrink: 0.0} Children [{weekdays}]--
                Node {height: px(240), display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::flex(7, 1.0)]}, grid_template_rows: {vec![RepeatedGridTrack::flex(6, 1.0)]}, flex_shrink: 0.0} Children [{days}]
            ]--
            template(|_| Ok(Panel::Candidates)) Name("DatePickerCandidates")
            Node {height: px(268), display: Display::None, grid_template_columns: {vec![RepeatedGridTrack::flex(3, 1.0)]}, grid_template_rows: {vec![RepeatedGridTrack::flex(4, 1.0)]}, flex_shrink: 0.0} Children [{candidates}]--
            @cell(Part::Clear, "DatePickerClear".into()) Node {height: px(26), flex_shrink: 0.0}
        ]
    }
}

fn cell(part: Part, name: String) -> impl Scene {
    // Rasterize above the displayed size to retain brush detail under UI scaling.
    let today = match part {
        Part::Day(index) => Some(bsn! {
            template(move |_| Ok(TodayRing(index))) Name({format!("DatePickerToday{index}")})
            @bevy_widgetry_core::icon::WidgetryIcon {
                @path: {bevy_widgetry_asset::BuiltinIcon::DatePickerToday.path()},
                @max_size: {Some(UVec2::splat(152))},
                @colors: {bevy_widgetry_core::icon::WidgetryIconColorOverrides {
                    normal: bevy_widgetry_core::icon::WidgetryIconStateColorOverrides {foreground: Some(Color::WHITE)},
                    disabled: bevy_widgetry_core::icon::WidgetryIconStateColorOverrides {foreground: Some(Color::WHITE)},
                }},
            }
            Node {position_type: PositionType::Absolute}
            UiTransform {scale: Vec2::splat(0.25)}
            Pickable::IGNORE Visibility::Hidden ZIndex(1)
        }),
        _ => None,
    };
    bsn! {
        template(move |_| Ok(part)) Name(name)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center, min_width: px(0), border_radius: BorderRadius::all(px(6)) }
        BackgroundColor
        Hovered(false) WidgetryPointerPressed
        @{(!matches!(part, Part::Weekday(_))).then_some(bsn! { bevy::ui_widgets::Button bevy::input_focus::tab_navigation::TabIndex(-1) })}
        Children [Text("") TextFont {font_size: px(14)} Pickable::IGNORE-- {today}]
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<WidgetryDatePickerLanguage>()
        .register_type::<WidgetryDatePickerLanguage>()
        .add_observer(own_today)
        .add_observer(click)
        .add_systems(PostUpdate, update.in_set(WidgetryUiSystems::Colors));
}

fn own_today(
    event: On<Add<TodayRing>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryDatePicker>>,
    mut commands: Commands,
) {
    if let Some(root) = owner(event.entity, &parents, &roots) {
        commands
            .entity(event.entity)
            .insert(WidgetryStyleOwner::<WidgetryIcon>::new(root));
    }
}

fn owner(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<(), With<WidgetryDatePicker>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if roots.contains(current) {
            return Some(current);
        }
        current = parents.get(current).ok()?.parent();
    }
}

fn update(
    mode: Res<WidgetryTheme>,
    today: Res<crate::today::Today>,
    language: Res<WidgetryDatePickerLanguage>,
    roots: Query<(Entity, Ref<State>, Ref<ColorState>, Ref<Navigation>), With<WidgetryDatePicker>>,
    root_markers: Query<(), With<WidgetryDatePicker>>,
    changed: Query<
        Entity,
        (
            With<Part>,
            Or<(
                Changed<Hovered>,
                Changed<Pressed>,
                Changed<WidgetryEffectiveDisabled>,
            )>,
        ),
    >,
    mut removed: RemovedComponents<Pressed>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let mut pending = EntityHashSet::default();
    for (root, state, colors, navigation) in &roots {
        if mode.is_changed()
            || state.is_changed()
            || colors.is_changed()
            || navigation.is_changed()
            || today.is_changed()
            || language.is_changed()
        {
            pending.insert(root);
        }
    }
    for entity in changed.iter().chain(removed.read()) {
        if let Some(root) = owner(entity, &parents, &root_markers) {
            pending.insert(root);
        }
    }
    for root in pending {
        commands.queue(move |world: &mut World| {
            apply(world, root).map_err(|error| {
                error
                    .with_context(ErrorFrame::new("date_picker.apply").field("root", root))
                    .into_bevy_error()
            })
        });
    }
}

fn click(mut event: On<PointerClick>, parts: Query<&Part>, mut commands: Commands) {
    if parts.get(event.entity).is_err() {
        return;
    }
    event.propagate(false);
    if event.button != PointerButton::Primary {
        return;
    }
    let target = event.entity;
    commands.queue(move |world: &mut World| {
        activate(world, target).map_err(|error| {
            error
                .with_context(ErrorFrame::new("date_picker.activate").field("target", target))
                .into_bevy_error()
        })
    });
}

fn root(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryDatePicker>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn activate(world: &mut World, target: Entity) -> Result<(), DatePickerFailure> {
    let Some(root) = root(world, target) else {
        return Ok(());
    };
    if world.get::<InteractionDisabled>(root).is_some()
        || world.get::<InteractionDisabled>(target).is_some()
    {
        return Ok(());
    }
    let Some(part) = world.get::<Part>(target).copied() else {
        return Ok(());
    };
    let view = navigation(world, root)?;
    match part {
        Part::Previous | Part::Next => {
            Nav::browse_in_world(
                world,
                root,
                if matches!(part, Part::Previous) {
                    -1
                } else {
                    1
                },
            )?;
        }
        Part::Title => {
            Nav::enter_parent_in_world(world, root)?;
        }
        Part::Candidate(index) => {
            if let Some(candidate) = Nav::candidates(world, root)?.get(index) {
                match candidate.value {
                    crate::DatePickerCandidateValue::Month(month) => {
                        Nav::choose_month_in_world(world, root, month.month)?;
                    }
                    crate::DatePickerCandidateValue::Year(year) => {
                        Nav::choose_year_in_world(world, root, year)?;
                    }
                }
            }
        }
        Part::Clear if view == View::Days => {
            crate::WidgetryDatePickerHeadless::choose_date_in_world(world, root, None)?;
        }
        Part::Day(index) if view == View::Days => {
            let day = WidgetryDatePicker::days(world, root, None)?[index];
            if day.is_selectable {
                crate::WidgetryDatePickerHeadless::choose_date_in_world(
                    world,
                    root,
                    Some(day.date),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn apply(world: &mut World, root: Entity) -> Result<(), DatePickerFailure> {
    if !world.entities().contains(root) {
        return Ok(());
    }
    let state = crate::model::state(world, root)?;
    let language = *world.resource::<WidgetryDatePickerLanguage>();
    let displayed = state.displayed;
    let selected = state.selected.is_some();
    let week_start = state.week_start.number_days_from_monday() as usize;
    let view = navigation(world, root)?;
    let previous = Nav::can_browse(world, root, -1)?;
    let next = Nav::can_browse(world, root, 1)?;
    let candidates = Nav::candidates(world, root)?;
    let days = WidgetryDatePicker::days(world, root, world.resource::<crate::today::Today>().0)?;
    let colors = crate::WidgetryDatePickerColorOverrides::get(world, root)?
        .resolve(&world.resource::<WidgetryTheme>().date_picker);
    world.entity_mut(root).insert((
        BackgroundColor(colors.background),
        BorderColor::all(colors.border),
    ));
    let mut pending = world
        .get::<Children>(root)
        .ok_or_else(|| missing(root, root, "Children", "date_picker.apply"))?
        .to_vec();
    let mut day_count = 0;
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        if let Some(ring) = world.get::<TodayRing>(entity) {
            let visible = days[ring.0].is_today && view == View::Days;
            world.entity_mut(entity).insert(if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
        if let Some(panel) = world.get::<Panel>(entity).copied() {
            let display = match (panel, view) {
                (Panel::Days, View::Days) => Display::Flex,
                (Panel::Candidates, View::Months(_) | View::Years(_)) => Display::Grid,
                _ => Display::None,
            };
            let mut node = world
                .get_mut::<Node>(entity)
                .ok_or_else(|| missing(root, entity, "Node", "date_picker.panel"))?;
            if node.display != display {
                node.display = display;
            }
        }
        let Some(part) = world.get::<Part>(entity).copied() else {
            continue;
        };
        let (text, selectable, chosen, adjacent) = match part {
            Part::Previous => ("‹".into(), previous, false, false),
            Part::Next => ("›".into(), next, false, false),
            Part::Title => (
                match view {
                    View::Days => match language {
                        WidgetryDatePickerLanguage::English => {
                            format!("{} / {:02}", displayed.year, displayed.month as u8)
                        }
                        WidgetryDatePickerLanguage::SimplifiedChinese => {
                            format!("{}年{}月", displayed.year, displayed.month as u8)
                        }
                    },
                    View::Months(year) => language.year(year),
                    View::Years(start) => {
                        format!("{} – {}", language.year(start), language.year(start + 11))
                    }
                },
                !matches!(view, View::Years(_)),
                false,
                false,
            ),
            Part::Clear => {
                world.entity_mut(entity).insert(if view == View::Days {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                (
                    match language {
                        WidgetryDatePickerLanguage::English => "Clear",
                        WidgetryDatePickerLanguage::SimplifiedChinese => "清空",
                    }
                    .into(),
                    selected && view == View::Days,
                    false,
                    false,
                )
            }
            Part::Candidate(index) => match view {
                View::Months(_) => (
                    language.month(index),
                    candidates[index].is_selectable,
                    candidates[index].is_current,
                    false,
                ),
                View::Years(start) => (
                    language.year(start + index as i32),
                    candidates[index].is_selectable,
                    candidates[index].is_current,
                    false,
                ),
                View::Days => (String::new(), false, false, false),
            },
            Part::Weekday(index) => (
                language.weekday((week_start + index) % 7).into(),
                false,
                false,
                true,
            ),
            Part::Day(index) => {
                day_count += 1;
                let day = days[index];
                (
                    day.date.day().to_string(),
                    day.is_selectable,
                    day.is_selected,
                    !day.is_current_month,
                )
            }
        };
        let interactive = !matches!(part, Part::Weekday(_));
        bevy_widgetry_core::disabled::set_intrinsic_disabled(
            world,
            entity,
            !selectable && interactive,
        )
        .map_err(|error| core(error, root, "date_picker.project_disabled"))?;
        let disabled = world.get::<InteractionDisabled>(entity).is_some();
        let hovered = world.get::<Hovered>(entity).is_some_and(|hover| hover.0);
        let pressed = world.get::<Pressed>(entity).is_some();
        let background = if chosen {
            colors.selected
        } else if interactive && !disabled && pressed {
            colors.pressed
        } else if interactive && !disabled && hovered {
            colors.hovered
        } else {
            Color::NONE
        };
        let foreground = if disabled {
            colors.disabled
        } else if adjacent {
            colors.adjacent
        } else {
            colors.foreground
        };
        world.entity_mut(entity).insert(BackgroundColor(background));
        let text_entity = world
            .get::<Children>(entity)
            .and_then(|children| children.first())
            .copied()
            .ok_or_else(|| missing(root, entity, "label child", "date_picker.label"))?;
        let mut label = world
            .get_mut::<Text>(text_entity)
            .ok_or_else(|| missing(root, text_entity, "Text", "date_picker.label"))?;
        if label.0 != text {
            label.0 = text;
        }
        world.entity_mut(text_entity).insert(TextColor(foreground));
    }
    if day_count != 42 {
        return Err(failure(
            DatePickerError::DayCount {
                root,
                actual: day_count,
            },
            ErrorFrame::new("date_picker.apply"),
        ));
    }
    Ok(())
}
