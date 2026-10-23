use crate::editing::StepButton;
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
pub(crate) struct Repeat {
    next: Option<Duration>,
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<RequestRedraw>().add_systems(Update, tick);
}

fn tick(
    real: Res<Time<Real>>,
    strategy: Res<TimeUpdateStrategy>,
    mut buttons: Query<(
        Entity,
        &StepButton,
        Has<Pressed>,
        Has<InteractionDisabled>,
        &mut Repeat,
    )>,
    roots: Query<Has<InteractionDisabled>>,
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
    for (entity, button, pressed, disabled, mut repeat) in &mut buttons {
        if !pressed || disabled || !roots.get(button.root).is_ok_and(|disabled| !disabled) {
            repeat.next = None;
            continue;
        }
        redraw.write(RequestRedraw);
        if presses.started_this_frame(entity) {
            repeat.next = Some(now + Duration::from_millis(400));
            continue;
        }
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
