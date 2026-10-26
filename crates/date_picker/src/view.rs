use crate::{
    WidgetryDatePicker,
    colors::ColorState,
    model::{State, invalid},
};
use bevy::{
    ecs::entity::EntityHashSet,
    picking::{events::PointerClick, hover::Hovered, pointer::PointerButton},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
};
use bevy_widgetry_core::{
    disabled::WidgetryEffectiveDisabled, pointer::WidgetryPointerPressed, ui::WidgetryUiSystems,
};
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component, Clone, Copy)]
enum Part {
    Previous,
    Next,
    Title,
    Weekday(usize),
    Day(usize),
    Clear,
}

pub(crate) fn scene() -> impl Scene {
    let weekdays: Vec<_> = (0..7)
        .map(|index| cell(Part::Weekday(index), format!("DatePickerWeekday{index}")))
        .collect();
    let days: Vec<_> = (0..42)
        .map(|index| cell(Part::Day(index), format!("DatePickerDay{index}")))
        .collect();
    bsn! {
        Node { width: px(304), height: px(352), padding: UiRect::all(px(10)), flex_direction: FlexDirection::Column, flex_shrink: 0.0, border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(8)) }
        BackgroundColor BorderColor
        Children [
            Node { height: px(36), flex_shrink: 0.0, justify_content: JustifyContent::SpaceBetween } Children [
                @cell(Part::Previous, "DatePickerPrevious".into()) Node {width: px(32)}--
                @cell(Part::Title, "DatePickerTitle".into()) Node {flex_grow: 1.0}--
                @cell(Part::Next, "DatePickerNext".into()) Node {width: px(32)}
            ]--
            Node {height: px(28), display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::flex(7, 1.0)]}, flex_shrink: 0.0} Children [{weekdays}]--
            Node {height: px(240), display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::flex(7, 1.0)]}, grid_template_rows: {vec![RepeatedGridTrack::flex(6, 1.0)]}, flex_shrink: 0.0} Children [{days}]--
            @cell(Part::Clear, "DatePickerClear".into()) Node {height: px(26), flex_shrink: 0.0}
        ]
    }
}

fn cell(part: Part, name: String) -> impl Scene {
    bsn! {
        template(move |_| Ok(part)) Name(name)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center, min_width: px(0), border_radius: BorderRadius::all(px(6)) }
        BackgroundColor
        Hovered(false) WidgetryPointerPressed
        @{(!matches!(part, Part::Weekday(_) | Part::Title)).then_some(bsn! { bevy::ui_widgets::Button bevy::input_focus::tab_navigation::TabIndex(-1) })}
        Children [Text("") TextFont {font_size: px(14)} Pickable::IGNORE]
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(click)
        .add_systems(PostUpdate, update.in_set(WidgetryUiSystems::Colors));
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
    mode: Res<WidgetryThemeMode>,
    roots: Query<(Entity, Ref<State>, Ref<ColorState>)>,
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
    for (root, state, colors) in &roots {
        if mode.is_changed() || state.is_changed() || colors.is_changed() {
            pending.insert(root);
        }
    }
    for entity in changed.iter().chain(removed.read()) {
        if let Some(root) = owner(entity, &parents, &root_markers) {
            pending.insert(root);
        }
    }
    for root in pending {
        commands.queue(move |world: &mut World| apply(world, root));
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
    commands.queue(move |world: &mut World| activate(world, target));
}

fn root(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryDatePicker>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn activate(world: &mut World, target: Entity) -> Result<(), BevyError> {
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
    match part {
        Part::Previous => {
            WidgetryDatePicker::shift_month_in_world(world, root, -1)?;
        }
        Part::Next => {
            WidgetryDatePicker::shift_month_in_world(world, root, 1)?;
        }
        Part::Clear => {
            WidgetryDatePicker::set_selected_date_in_world(world, root, None)?;
        }
        Part::Day(index) => {
            let day = WidgetryDatePicker::days(world, root, None)?[index];
            if day.is_selectable {
                WidgetryDatePicker::set_selected_date_in_world(world, root, Some(day.date))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn apply(world: &mut World, root: Entity) -> Result<(), BevyError> {
    if !world.entities().contains(root) {
        return Ok(());
    }
    let state = crate::model::state(world, root)?;
    let displayed = state.displayed;
    let selected = state.selected.is_some();
    let week_start = state.week_start.number_days_from_monday() as usize;
    let days = WidgetryDatePicker::days(world, root, None)?;
    let colors = crate::WidgetryDatePickerColorOverrides::get(world, root)?
        .resolve(&world.resource::<WidgetryThemeMode>().colors().date_picker);
    world.entity_mut(root).insert((
        BackgroundColor(colors.background),
        BorderColor::all(colors.border),
    ));
    let mut pending = world
        .get::<Children>(root)
        .ok_or_else(|| invalid("DatePicker children are missing"))?
        .to_vec();
    let mut day_count = 0;
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        let Some(part) = world.get::<Part>(entity).copied() else {
            continue;
        };
        let (text, selectable, chosen, adjacent) = match part {
            Part::Previous => ("‹".into(), true, false, false),
            Part::Next => ("›".into(), true, false, false),
            Part::Title => (
                format!("{} / {:02}", displayed.year, displayed.month as u8),
                false,
                false,
                false,
            ),
            Part::Clear => ("清空".into(), selected, false, true),
            Part::Weekday(index) => (
                ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][(week_start + index) % 7].into(),
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
        let interactive = !matches!(part, Part::Weekday(_) | Part::Title);
        bevy_widgetry_core::disabled::set_intrinsic_disabled(
            world,
            entity,
            !selectable && interactive,
        );
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
            .ok_or_else(|| invalid("DatePicker label is missing"))?;
        let mut label = world
            .get_mut::<Text>(text_entity)
            .ok_or_else(|| invalid("DatePicker label text is missing"))?;
        if label.0 != text {
            label.0 = text;
        }
        world.entity_mut(text_entity).insert(TextColor(foreground));
    }
    if day_count != 42 {
        return Err(invalid("DatePicker requires 42 day cells"));
    }
    Ok(())
}
