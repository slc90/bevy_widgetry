use crate::i18n::{CommonKey, DatePickerKey, TextKey, localized};
use bevy::{
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonHeadless},
    date_picker::{
        Date, DatePickerCandidateValue, DatePickerMonth, DatePickerView, Month,
        WidgetryDatePickerHeadless as Picker, WidgetryDatePickerNavigation as Nav,
    },
    text::WidgetryText,
    theme::WidgetryTheme,
};

#[derive(Component, Default, Clone)]
struct DateRegion;
#[derive(Component, Default, Clone)]
struct Readout;
#[derive(Component, FromTemplate)]
struct Day {
    root: Entity,
    index: usize,
    date: Option<Date>,
    selectable: bool,
    text: Entity,
}
#[derive(Component, FromTemplate)]
struct Target {
    root: Entity,
}

#[derive(Component, FromTemplate)]
struct NavigationAction {
    root: Entity,
    step: i32,
}
#[derive(Component, FromTemplate)]
struct NavigationTitle {
    root: Entity,
}
#[derive(Component, FromTemplate)]
struct NavigationPanel {
    root: Entity,
    days: bool,
}
#[derive(Component, FromTemplate)]
struct Candidate {
    root: Entity,
    index: usize,
    value: Option<DatePickerCandidateValue>,
    selectable: bool,
    text: Entity,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        refresh
            .before(bevy_widgetry_core::ui::WidgetryUiSystems::Colors)
            .before(bevy::ui::UiSystems::Prepare)
            .run_if(any_with_component::<DateRegion>),
    );
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(10),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::DatePicker(DatePickerKey::DateOnly)) WidgetryText--
            @localized(TextKey::DatePicker(DatePickerKey::DateOnlyDescription)) WidgetryText--
            Node {column_gap:px(16),align_items:AlignItems::Start} Children [
                #Root @Picker {@displayed_month:{Some(DatePickerMonth {year:2026,month:Month::October})}}
                DateRegion Name::new("DateCustomRoot")
                Node {width:px(320),flex_direction:FlexDirection::Column,row_gap:px(8),padding:UiRect::all(px(8))}
                Children [
                    @grid(#Root)--
                    Target {root:#Root} Readout Text("") WidgetryText Name::new("DateCustomStatus")--
                    @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Common(CommonKey::ClearSelection)) WidgetryText}) as Box<dyn SceneList>)}}
                    Target {root:#Root} Name::new("DateCustomClear") on(clear)
                ]--
                @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Common(CommonKey::EnableDisable)) WidgetryText}) as Box<dyn SceneList>)}}
                Target {root:#Root} Name::new("DateCustomDisabled") on(toggle)
            ]
        ]
    }
}

pub(super) fn navigation_scene() -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(10),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::DatePicker(DatePickerKey::CustomNavigation)) WidgetryText--
            @localized(TextKey::DatePicker(DatePickerKey::CustomNavigationDescription)) WidgetryText--
            #Root @Picker {@displayed_month:{Some(DatePickerMonth {year:2026,month:Month::October})}}
            @Nav DateRegion Name::new("DateCustomNavigation")
            Node {width:px(320),flex_direction:FlexDirection::Column,row_gap:px(8),padding:UiRect::all(px(8))}
            Children [
                Node {width:percent(100),justify_content:JustifyContent::SpaceBetween} Children [
                    @WidgetryButtonHeadless NavigationAction {root:#Root,step:-1} Name::new("DateCustomPrevious")
                    Node {width:px(40),height:px(36),align_items:AlignItems::Center,justify_content:JustifyContent::Center} on(browse)
                    Children [Text("‹") WidgetryText Pickable::IGNORE]--
                    @WidgetryButtonHeadless NavigationTitle {root:#Root} Name::new("DateCustomTitle")
                    Node {flex_grow:1.0,height:px(36),align_items:AlignItems::Center,justify_content:JustifyContent::Center} on(enter_parent)
                    Children [Text("") WidgetryText Pickable::IGNORE]--
                    @WidgetryButtonHeadless NavigationAction {root:#Root,step:1} Name::new("DateCustomNext")
                    Node {width:px(40),height:px(36),align_items:AlignItems::Center,justify_content:JustifyContent::Center} on(browse)
                    Children [Text("›") WidgetryText Pickable::IGNORE]
                ]--
                @grid(#Root) NavigationPanel {root:#Root,days:true}--
                NavigationPanel {root:#Root,days:false}
                Node {display:Display::Grid,grid_template_columns:{vec![RepeatedGridTrack::px(3,90.0)]},column_gap:px(8),row_gap:px(8)}
                @candidate_cells(#Root)--
                Target {root:#Root} Readout Text("") WidgetryText--
                @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Common(CommonKey::ClearSelection)) WidgetryText}) as Box<dyn SceneList>)}}
                Target {root:#Root} on(clear)
            ]
        ]
    }
}

fn candidate_cells(root: bevy::ecs::template::EntityTemplate) -> impl Scene {
    let candidates = (0usize..12).map(|index| bsn! {
        @WidgetryButtonHeadless Candidate {root:{root},index:index,value:None,selectable:true,text:#Text}
        Name({format!("DateCustomCandidate{index}")})
        Node {width:px(90),height:px(40),align_items:AlignItems::Center,justify_content:JustifyContent::Center,border_radius:BorderRadius::all(px(8))}
        BackgroundColor on(confirm_candidate)
        Children [#Text Text("") WidgetryText TextFont {font_size:px(14)} Pickable::IGNORE]
    }).collect::<Vec<_>>();
    bsn! {Children [{candidates}]}
}

fn browse(event: On<Activate>, actions: Query<&NavigationAction>, mut commands: Commands) {
    if let Ok(action) = actions.get(event.entity) {
        Nav::browse(&mut commands, action.root, action.step);
        info!(root=?action.root,step=action.step,"请求浏览自定义日历");
    }
}
fn enter_parent(event: On<Activate>, titles: Query<&NavigationTitle>, mut commands: Commands) {
    if let Ok(title) = titles.get(event.entity) {
        Nav::enter_parent(&mut commands, title.root);
        info!(root=?title.root,"请求进入自定义日历上级视图");
    }
}
fn confirm_candidate(event: On<Activate>, candidates: Query<&Candidate>, mut commands: Commands) {
    let Ok(candidate) = candidates.get(event.entity) else {
        return;
    };
    match candidate.value {
        Some(DatePickerCandidateValue::Month(month)) => {
            Nav::choose_month(&mut commands, candidate.root, month.month)
        }
        Some(DatePickerCandidateValue::Year(year)) => {
            Nav::choose_year(&mut commands, candidate.root, year)
        }
        None => return,
    }
    info!(root=?candidate.root,value=?candidate.value,"请求确认自定义日历候选");
}

fn refresh_navigation(world: &mut World, root: Entity, language: crate::i18n::Language) -> Result {
    let view = Nav::view(world, root).map_err(|error| error.into_bevy_error())?;
    let month = Picker::displayed_month(world, root).map_err(|error| error.into_bevy_error())?;
    let title = match (view, language) {
        (DatePickerView::Days, _) => format!("{} / {:02}", month.year, month.month as u8),
        (DatePickerView::Months(year), crate::i18n::Language::SimplifiedChinese) => {
            format!("{year}年")
        }
        (DatePickerView::Months(year), _) => year.to_string(),
        (DatePickerView::Years(start), _) => format!("{start} – {}", start + 11),
    };
    let titles = world
        .query::<(Entity, &NavigationTitle, &Children)>()
        .iter(world)
        .filter(|(_, binding, _)| binding.root == root)
        .map(|(_, _, children)| children[0])
        .collect::<Vec<_>>();
    for entity in titles {
        if let Some(mut text) = world.get_mut::<Text>(entity) {
            text.set_if_neq(Text::new(title.clone()));
        }
    }
    let panels = world
        .query::<(Entity, &NavigationPanel)>()
        .iter(world)
        .filter(|(_, panel)| panel.root == root)
        .map(|(entity, panel)| (entity, panel.days))
        .collect::<Vec<_>>();
    for (entity, days) in panels {
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            let display = if days != (view == DatePickerView::Days) {
                Display::None
            } else {
                Display::Grid
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    let values = Nav::candidates(world, root).map_err(|error| error.into_bevy_error())?;
    let candidates = world
        .query::<(Entity, &Candidate)>()
        .iter(world)
        .filter(|(_, binding)| binding.root == root)
        .map(|(entity, binding)| (entity, binding.index, binding.text, binding.selectable))
        .collect::<Vec<_>>();
    let theme = world.resource::<WidgetryTheme>().date_picker;
    for (entity, index, text_entity, was_selectable) in candidates {
        let value = values.get(index);
        if let Some(mut binding) = world.get_mut::<Candidate>(entity) {
            binding.value = value.map(|value| value.value);
            binding.selectable = value.is_some_and(|value| value.is_selectable);
        }
        let text = match value.map(|value| value.value) {
            Some(DatePickerCandidateValue::Year(year)) => year.to_string(),
            Some(DatePickerCandidateValue::Month(month)) => match language {
                crate::i18n::Language::SimplifiedChinese => format!("{}月", month.month as u8),
                _ => month.month.to_string(),
            },
            None => String::new(),
        };
        if let Some(mut target) = world.get_mut::<Text>(text_entity) {
            target.set_if_neq(Text::new(text));
        }
        let selectable = value.is_some_and(|value| value.is_selectable);
        if was_selectable != selectable {
            if selectable {
                world.entity_mut(entity).remove::<InteractionDisabled>();
            } else {
                world.entity_mut(entity).insert(InteractionDisabled);
            }
        }
        if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
            background.set_if_neq(BackgroundColor(
                if value.is_some_and(|value| value.is_current) {
                    theme.selected
                } else {
                    Color::NONE
                },
            ));
        }
    }
    Ok(())
}

fn grid(root: bevy::ecs::template::EntityTemplate) -> impl Scene {
    let cells=(0usize..42).map(|index| {
        bsn! {
            @WidgetryButtonHeadless Day {root:{root},index:index,date:None,selectable:true,text:#Text}
            Name({format!("DateCustomDay{index}")})
            Node {width:px(40),height:px(34),align_items:AlignItems::Center,justify_content:JustifyContent::Center,border_radius:BorderRadius::all(px(17)),border:UiRect::all(px(1))}
            BackgroundColor BorderColor on(choose)
            Children [#Text Text("") WidgetryText TextFont {font_size:px(14)} TextLayout::justify(Justify::Center) Node {width:percent(100)} Pickable::IGNORE]
        }
    }).collect::<Vec<_>>();
    bsn! {
        Node {display:Display::Grid,grid_template_columns:{vec![RepeatedGridTrack::px(7,40.0)]},column_gap:px(3),row_gap:px(3)}
        Children [{cells}]
    }
}

fn choose(event: On<Activate>, days: Query<&Day>, mut commands: Commands) {
    let Ok(day) = days.get(event.entity) else {
        return;
    };
    let Some(date) = day.date else {
        return;
    };
    Picker::choose_date(&mut commands, day.root, Some(date));
    info!(root=?day.root,?date,"选择自定义日期");
}
fn clear(event: On<Activate>, targets: Query<&Target>, mut commands: Commands) {
    let Ok(target) = targets.get(event.entity) else {
        return;
    };
    Picker::choose_date(&mut commands, target.root, None);
    info!(root=?target.root,"请求清空自定义日期");
}
fn toggle(event: On<Activate>, targets: Query<&Target>, mut commands: Commands) {
    let Ok(target) = targets.get(event.entity) else {
        return;
    };
    let root = target.root;
    commands.queue(move |world: &mut World| -> Result {
        let disabled = world.get::<InteractionDisabled>(root).is_some();
        let mut entity = world
            .get_entity_mut(root)
            .map_err(|_| crate::diagnostics::error("自定义日期根已失效"))?;
        if disabled {
            entity.remove::<InteractionDisabled>();
        } else {
            entity.insert(InteractionDisabled);
        }
        info!(?root, disabled = !disabled, "切换自定义日期区域可用状态");
        Ok(())
    });
}

fn refresh(world: &mut World) -> Result {
    let theme = world.resource::<WidgetryTheme>().date_picker;
    let roots = world
        .query_filtered::<Entity, With<DateRegion>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let days = Picker::days(world, root, Picker::today(world))
            .map_err(|error| error.into_bevy_error())?;
        let selected =
            Picker::selected_date(world, root).map_err(|error| error.into_bevy_error())?;
        let language = *world.resource::<crate::i18n::Language>();
        if world.get::<Nav>(root).is_some() {
            refresh_navigation(world, root, language)?;
        }
        let readouts = world
            .query_filtered::<(Entity, &Target), With<Readout>>()
            .iter(world)
            .filter_map(|(entity, target)| (target.root == root).then_some(entity))
            .collect::<Vec<_>>();
        let value = selected.map(|date| date.to_string()).unwrap_or_else(|| {
            crate::i18n::tr(language, TextKey::DatePicker(DatePickerKey::None)).to_owned()
        });
        for entity in readouts {
            if let Some(mut text) = world.get_mut::<Text>(entity) {
                text.set_if_neq(Text::new(value.clone()));
            }
        }
        let cells = world
            .query::<(Entity, &Day)>()
            .iter(world)
            .filter(|(_, day)| day.root == root)
            .map(|(entity, day)| (entity, day.index, day.text, day.selectable))
            .collect::<Vec<_>>();
        for (entity, index, text, was_selectable) in cells {
            let day = days
                .get(index)
                .ok_or_else(|| crate::diagnostics::error("自定义日期格索引失效"))?;
            if let Some(mut binding) = world.get_mut::<Day>(entity) {
                binding.date = Some(day.date);
                binding.selectable = day.is_selectable;
            }
            if was_selectable != day.is_selectable {
                if day.is_selectable {
                    world.entity_mut(entity).remove::<InteractionDisabled>();
                } else {
                    world.entity_mut(entity).insert(InteractionDisabled);
                }
            }
            let disabled = world.get::<InteractionDisabled>(entity).is_some();
            let color = if disabled {
                theme.disabled
            } else if !day.is_current_month {
                theme.adjacent
            } else {
                theme.foreground
            };
            if let Some(mut value) = world.get_mut::<Text>(text) {
                value.set_if_neq(Text::new(day.date.day().to_string()));
            }
            let foreground = bevy_widgetry_core::foreground::ResolvedForeground(color);
            if world.get::<bevy_widgetry_core::foreground::ResolvedForeground>(entity)
                != Some(&foreground)
            {
                world.entity_mut(entity).insert(foreground);
            }
            let background = if disabled {
                Color::NONE
            } else if day.is_selected {
                theme.selected
            } else if world.get::<Pressed>(entity).is_some() {
                theme.pressed
            } else if world
                .get::<bevy::picking::hover::Hovered>(entity)
                .is_some_and(|hover| hover.0)
            {
                theme.hovered
            } else {
                Color::NONE
            };
            if let Some(mut value) = world.get_mut::<BackgroundColor>(entity) {
                value.set_if_neq(BackgroundColor(background));
            }
            if let Some(mut value) = world.get_mut::<BorderColor>(entity) {
                value.set_if_neq(BorderColor::all(if day.is_today {
                    theme.foreground
                } else {
                    Color::NONE
                }));
            }
        }
    }
    Ok(())
}
