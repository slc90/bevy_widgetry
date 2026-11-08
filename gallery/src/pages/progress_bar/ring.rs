const PROGRESS_STEP: f32 = 0.1;
const INDETERMINATE_ACTIVE_DOTS: usize = 18;

const DOT_COUNT: usize = 60;
const DOT_ORIGIN: f32 = 67.0;
const RING_RADIUS: f32 = 54.0;
const DOT_DIAMETER: f32 = 6.0;
const CONTROL_SIZE: f32 = 140.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const LABEL_MAX_WIDTH: f32 = 100.0;

const DETAIL_GAP: f32 = 12.0;
const CONTROL_GAP: f32 = 8.0;

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
    let dots=(0usize..DOT_COUNT).map(|index| {
        let angle=index as f32 / DOT_COUNT as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        bsn! {template(move |_|Ok(Dot(index))) Pickable::IGNORE BackgroundColor
            Node {position_type:PositionType::Absolute,width:px(DOT_DIAMETER),height:px(DOT_DIAMETER),left:{px(DOT_ORIGIN+angle.cos()*RING_RADIUS)},top:{px(DOT_ORIGIN+angle.sin()*RING_RADIUS)},border_radius:BorderRadius::MAX}}
    }).collect::<Vec<_>>();
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(DETAIL_GAP),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::ProgressBar(ProgressBarKey::Ring)) WidgetryText--
            #Root @Progress {@state:WidgetryProgressState::Determinate {value:0.65}} Ring Name::new("ProgressRing")
            Pickable::IGNORE Node {width:px(CONTROL_SIZE),height:px(CONTROL_SIZE),align_items:AlignItems::Center,justify_content:JustifyContent::Center}
            Children [{dots}--Label Text("") WidgetryText Name::new("ProgressRingStatus")
                TextFont {font_size:px(LABEL_FONT_SIZE)} TextLayout::justify(Justify::Center)
                Node {max_width:px(LABEL_MAX_WIDTH)} Pickable::IGNORE]--
            Node {column_gap:px(CONTROL_GAP)} Children [
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
                    value: (value + PROGRESS_STEP).min(1.0),
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
        * DOT_COUNT as f64) as usize;
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
                    WidgetryProgressState::Determinate { value } => {
                        (dot.0 as f32) < value * DOT_COUNT as f32
                    }
                    WidgetryProgressState::Indeterminate => {
                        (dot.0 + DOT_COUNT - phase) % DOT_COUNT < INDETERMINATE_ACTIVE_DOTS
                    }
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
