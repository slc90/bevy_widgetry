use crate::WidgetryNumberInputStep;
use crate::{WidgetryNumberInputEditor, editing::ReadOnly, input::StepTarget};
use bevy::{
    prelude::*,
    time::TimeUpdateStrategy,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
    window::RequestRedraw,
};
use bevy_widgetry_core::pointer::WidgetryPointerPressQuery;
use std::time::Duration;

#[derive(Component, Default, Clone)]
#[require(RepeatTimer, bevy_widgetry_core::pointer::WidgetryPointerPressed)]
pub struct WidgetryNumberInputRepeat;

#[derive(Component, Default)]
struct RepeatTimer {
    next: Option<Duration>,
    canceled: bool,
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<RequestRedraw>()
        .add_systems(Update, tick)
        .add_observer(cancel_disabled);
}

fn cancel_disabled(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    states: Query<&bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>,
    mut repeats: Query<
        (Entity, Option<&WidgetryNumberInputStep>, &mut RepeatTimer),
        With<WidgetryNumberInputRepeat>,
    >,
    roots: Query<&WidgetryNumberInputEditor>,
    fields: Query<&bevy_widgetry_core::input_target::WidgetryInputTarget>,
    mut commands: Commands,
) {
    if !states
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        return;
    }
    for (entity, step, mut repeat) in &mut repeats {
        let affected = entity == event.entity
            || step.is_some_and(|step| {
                step.root == event.entity
                    || roots.get(step.root).is_ok_and(|binding| {
                        binding.field == event.entity
                            || fields
                                .get(binding.field)
                                .is_ok_and(|target| target.target == event.entity)
                    })
            });
        if affected {
            repeat.next = None;
            repeat.canceled = true;
            // End ownership even when the first-step observer disables and re-enables the root.
            // The frame-level "started" flag still describes that canceled press.
            commands.entity(entity).remove::<Pressed>();
        }
    }
}

fn tick(
    real: Res<Time<Real>>,
    strategy: Res<TimeUpdateStrategy>,
    mut buttons: Query<(
        Entity,
        Option<&WidgetryNumberInputStep>,
        Has<Pressed>,
        Has<InteractionDisabled>,
        Ref<WidgetryNumberInputRepeat>,
        &mut RepeatTimer,
    )>,
    roots: Query<
        (
            &WidgetryNumberInputEditor,
            Has<ReadOnly>,
            Has<InteractionDisabled>,
        ),
        With<StepTarget>,
    >,
    fields: Query<(
        &bevy_widgetry_core::input_target::WidgetryInputTarget,
        Has<InteractionDisabled>,
    )>,
    editors: Query<Has<InteractionDisabled>, With<bevy::text::EditableText>>,
    mut commands: Commands,
    mut redraw: MessageWriter<RequestRedraw>,
    presses: WidgetryPointerPressQuery,
) {
    // Render-channel time can include idle time preceding the press.
    // Use the actual input interval in applications and Bevy's explicit manual time in tests.
    let now = match *strategy {
        TimeUpdateStrategy::Automatic => real.startup().elapsed(),
        _ => real.elapsed(),
    };
    for (entity, button, pressed, disabled, capability, mut repeat) in &mut buttons {
        if !pressed {
            repeat.next = None;
            repeat.canceled = false;
            continue;
        }
        let started = presses.started_this_frame(entity);
        let allowed = !disabled
            && button.is_some_and(|button| {
                roots
                    .get(button.root)
                    .is_ok_and(|(binding, readonly, disabled)| {
                        !readonly
                            && !disabled
                            && fields.get(binding.field).is_ok_and(|(target, disabled)| {
                                !disabled
                                    && editors.get(target.target).is_ok_and(|disabled| !disabled)
                            })
                    })
            });
        if !allowed || (capability.is_added() && !started) {
            repeat.next = None;
            repeat.canceled = true;
            continue;
        }
        if started {
            repeat.canceled = false;
            repeat.next = Some(now + Duration::from_millis(400));
            redraw.write(RequestRedraw);
            continue;
        }
        if repeat.canceled {
            continue;
        }
        redraw.write(RequestRedraw);
        let Some(next) = repeat.next else {
            repeat.next = Some(now + Duration::from_millis(400));
            continue;
        };
        if now >= next {
            repeat.next = Some(now + Duration::from_millis(100));
            commands.trigger(Activate { entity });
        }
    }
}
