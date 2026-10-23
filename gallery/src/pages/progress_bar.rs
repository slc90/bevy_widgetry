use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{
    LocalizedText, NavigationKey, ProgressBarKey, TextKey, localized, localized_args,
};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{button::WidgetryButton, progress_bar::*, text::WidgetryText};

pub(crate) struct ProgressBarDemoPlugin;

impl Plugin for ProgressBarDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::ProgressBar), enter)
            .add_systems(OnExit(GalleryPage::ProgressBar), exit)
            .add_systems(
                Update,
                refresh_readout.run_if(in_state(GalleryPage::ProgressBar)),
            );
    }
}

#[derive(Component, FromTemplate, Clone, Copy)]
struct Demo {
    bar: Entity,
    status: Entity,
}

#[derive(Component, Default)]
struct Simulation {
    ticks: u8,
    displayed: Option<WidgetryProgressState>,
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Increase,
    Decrease,
    Reset,
    Mode,
    Override,
    Clear,
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::ProgressBar, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::ProgressBar)
}

fn scene() -> impl Scene {
    let controls: Vec<_> = [
        (ProgressBarKey::Increase, Action::Increase),
        (ProgressBarKey::Decrease, Action::Decrease),
        (ProgressBarKey::Reset, Action::Reset),
        (ProgressBarKey::Mode, Action::Mode),
        (ProgressBarKey::Override, Action::Override),
        (ProgressBarKey::Clear, Action::Clear),
    ].into_iter().map(|(label, action)| bsn! {
        Name({format!("ProgressBar{action:?}")})
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::ProgressBar(label)) WidgetryText }) as Box<dyn SceneList>)} }
        template(move |_| Ok(action)) on(operate)
    }).collect();
    bsn! {
        Name::new("ProgressBarDemo") Demo { bar: #Progress, status: #Status }
        template(|_| Ok(Simulation::default()))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Navigation(NavigationKey::ProgressBar)) WidgetryText--
            @localized(TextKey::ProgressBar(ProgressBarKey::Description)) WidgetryText Node { max_width: percent(100) }--
            #Status Name::new("ProgressBarStatus")
            @localized_args(TextKey::ProgressBar(ProgressBarKey::Percentage), vec![("percent", "0".to_owned())]) WidgetryText--
            #Progress Name::new("ProgressBarMain") @WidgetryProgressBar--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap, max_width: percent(100) } Children [{controls}]
        ]
    }
}

fn invalid(reason: &str) -> BevyError {
    error!(reason, "ProgressBar 演示内部结构失效");
    BevyError::error(reason.to_owned())
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result {
        let (root, parts, ticks) = {
            let mut demos = world.query::<(Entity, &Demo, &Simulation)>();
            let (root, parts, simulation) = demos
                .single(world)
                .map_err(|_| invalid("expected one ProgressBar demo"))?;
            (root, *parts, simulation.ticks)
        };
        match action {
            Action::Override => {
                WidgetryProgressBarColorOverrides::set_in_world(
                    world,
                    parts.bar,
                    WidgetryProgressBarColorOverrides {
                        fill: Some(Color::srgb(0.15, 0.65, 0.4)),
                        ..default()
                    },
                )?;
                info!("应用进度条局部填充色");
                return Ok(());
            }
            Action::Clear => {
                WidgetryProgressBarColorOverrides::clear_in_world(world, parts.bar)?;
                info!("清除进度条局部配色");
                return Ok(());
            }
            _ => {}
        }
        let current = WidgetryProgressBar::state(world, parts.bar)?;
        let next_ticks = match action {
            Action::Increase => ticks.saturating_add(1).min(10),
            Action::Decrease => ticks.saturating_sub(1),
            Action::Reset => 0,
            _ => ticks,
        };
        let determinate = WidgetryProgressState::Determinate {
            value: f32::from(next_ticks) / 10.0,
        };
        let next = match action {
            Action::Reset => determinate,
            Action::Mode if current == WidgetryProgressState::Indeterminate => determinate,
            Action::Mode => WidgetryProgressState::Indeterminate,
            _ if current == WidgetryProgressState::Indeterminate => current,
            _ => determinate,
        };
        let changed = WidgetryProgressBar::set_state_in_world(world, parts.bar, next)?;
        world
            .get_mut::<Simulation>(root)
            .ok_or_else(|| invalid("simulation is missing"))?
            .ticks = next_ticks;
        info!(?action, ticks = next_ticks, ?next, changed, "调整演示进度");
        Ok(())
    });
}

fn refresh_readout(world: &mut World) -> Result {
    let demos: Vec<_> = world
        .query::<(Entity, &Demo, &Simulation)>()
        .iter(world)
        .map(|(root, parts, simulation)| (root, *parts, simulation.displayed))
        .collect();
    for (root, parts, previous) in demos {
        let current = WidgetryProgressBar::state(world, parts.bar)?;
        if previous == Some(current) {
            continue;
        }
        let binding = match current {
            WidgetryProgressState::Determinate { value } => LocalizedText::with_args(
                TextKey::ProgressBar(ProgressBarKey::Percentage),
                vec![("percent", format!("{:.0}", value * 100.0))],
            ),
            WidgetryProgressState::Indeterminate => {
                LocalizedText::new(TextKey::ProgressBar(ProgressBarKey::Running))
            }
        };
        *world
            .get_mut::<LocalizedText>(parts.status)
            .ok_or_else(|| invalid("status text is missing"))? = binding;
        world
            .get_mut::<Simulation>(root)
            .ok_or_else(|| invalid("simulation is missing"))?
            .displayed = Some(current);
    }
    Ok(())
}
