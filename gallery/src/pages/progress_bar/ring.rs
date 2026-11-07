use crate::i18n::{Language, ProgressBarKey, TextKey, localized, tr, tr_args};
use bevy::{prelude::*, ui_widgets::Activate, window::RequestRedraw};
use bevy_widgetry::{
    button::WidgetryButton,
    progress_bar::{WidgetryProgressBarHeadless as Progress, WidgetryProgressState},
    text::WidgetryText,
    theme::WidgetryTheme,
};

#[derive(Component, Default, Clone)]
struct Ring;
#[derive(Component)]
struct Dot(usize);
#[derive(Component, Default, Clone)]
struct Label;
#[derive(Component, FromTemplate)]
struct Control {
    root: Entity,
    action: Action,
}
#[derive(Clone, Copy, Debug, Default)]
enum Action {
    #[default]
    Increase,
    Reset,
    Mode,
}

pub(super) fn scene() -> impl Scene {
    let dots=(0usize..60).map(|index| {
        let angle=index as f32 / 60.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        bsn! {template(move |_|Ok(Dot(index))) Pickable::IGNORE BackgroundColor
            Node {position_type:PositionType::Absolute,width:px(6),height:px(6),left:{px(67.0+angle.cos()*54.0)},top:{px(67.0+angle.sin()*54.0)},border_radius:BorderRadius::MAX}}
    }).collect::<Vec<_>>();
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(12),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::ProgressBar(ProgressBarKey::Ring)) WidgetryText--
            #Root @Progress {@state:WidgetryProgressState::Determinate {value:0.65}} Ring Name::new("ProgressRing")
            Pickable::IGNORE Node {width:px(140),height:px(140),align_items:AlignItems::Center,justify_content:JustifyContent::Center}
            Children [{dots}--Label Text("") WidgetryText Name::new("ProgressRingStatus")
                TextFont {font_size:px(14)} TextLayout::justify(Justify::Center)
                Node {max_width:px(100)} Pickable::IGNORE]--
            Node {column_gap:px(8)} Children [
                @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::ProgressBar(ProgressBarKey::Increase)) WidgetryText}) as Box<dyn SceneList>)}}
                Control {root:#Root,action:Action::Increase} Name::new("ProgressRingIncrease") on(operate)--
                @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::ProgressBar(ProgressBarKey::Reset)) WidgetryText}) as Box<dyn SceneList>)}}
                Control {root:#Root,action:Action::Reset} Name::new("ProgressRingReset") on(operate)--
                @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::ProgressBar(ProgressBarKey::Mode)) WidgetryText}) as Box<dyn SceneList>)}}
                Control {root:#Root,action:Action::Mode} Name::new("ProgressRingMode") on(operate)
            ]
        ]
    }
}

fn operate(event: On<Activate>, controls: Query<&Control>, mut commands: Commands) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let root = control.root;
    let action = control.action;
    commands.queue(move |world: &mut World| -> Result {
        let current = Progress::state(world, root).map_err(|error| error.into_bevy_error())?;
        let state = match (action, current) {
            (Action::Reset, _) => WidgetryProgressState::default(),
            (Action::Mode, WidgetryProgressState::Indeterminate) => {
                WidgetryProgressState::Determinate { value: 0.65 }
            }
            (Action::Mode, _) => WidgetryProgressState::Indeterminate,
            (Action::Increase, WidgetryProgressState::Determinate { value }) => {
                WidgetryProgressState::Determinate {
                    value: (value + 0.1).min(1.0),
                }
            }
            _ => current,
        };
        let changed = Progress::set_state_in_world(world, root, state)
            .map_err(|error| error.into_bevy_error())?;
        info!(?root, ?action, ?state, changed, "调整自定义圆环进度");
        Ok(())
    });
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update
            .before(bevy::ui::UiSystems::Content)
            .run_if(any_with_component::<Ring>),
    );
}

fn update(world: &mut World) -> Result {
    let theme = world.resource::<WidgetryTheme>().progress_bar;
    let language = *world.resource::<Language>();
    let phase = (world
        .resource::<Time<Real>>()
        .elapsed_secs_f64()
        .rem_euclid(1.0)
        * 60.0) as usize;
    let roots = world
        .query_filtered::<(Entity, &Children), With<Ring>>()
        .iter(world)
        .map(|(entity, children)| (entity, children.to_vec()))
        .collect::<Vec<_>>();
    for (root, children) in roots {
        let state = Progress::state(world, root).map_err(|error| error.into_bevy_error())?;
        if state == WidgetryProgressState::Indeterminate {
            world.write_message(RequestRedraw);
        }
        let label = match state {
            WidgetryProgressState::Determinate { value } => tr_args(
                language,
                TextKey::ProgressBar(ProgressBarKey::Percentage),
                &[("percent", format!("{:.0}", value * 100.0))],
            ),
            WidgetryProgressState::Indeterminate => {
                tr(language, TextKey::ProgressBar(ProgressBarKey::Running)).to_owned()
            }
        };
        for child in children {
            if let Some(dot) = world.get::<Dot>(child) {
                let active = match state {
                    WidgetryProgressState::Determinate { value } => (dot.0 as f32) < value * 60.0,
                    WidgetryProgressState::Indeterminate => (dot.0 + 60 - phase) % 60 < 18,
                };
                if let Some(mut background) = world.get_mut::<BackgroundColor>(child) {
                    background.set_if_neq(BackgroundColor(if active {
                        theme.fill
                    } else {
                        theme.track
                    }));
                }
            }
            if world.get::<Label>(child).is_some()
                && let Some(mut text) = world.get_mut::<Text>(child)
            {
                text.set_if_neq(Text::new(label.clone()));
            }
        }
    }
    Ok(())
}
