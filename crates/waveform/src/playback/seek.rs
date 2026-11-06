use super::{WaveformPlayback, WaveformPlaybackAction, WaveformPlaybackRequested};
use crate::view_error::ViewResult;
use crate::{Waveform, WaveformRuntime};
use bevy::{prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_slider::{
    SliderDragEnded, SliderDragStarted, WidgetrySlider, WidgetrySliderPlugin,
};
use std::time::Duration;

use bevy_widgetry_slider::WidgetrySliderHeadless;
#[derive(Component)]
pub(super) struct ProgressOwner {
    root: Entity,
    generation: u64,
    projecting: bool,
    dragging: bool,
    gesture_position: Option<Duration>,
}

impl ProgressOwner {
    pub(super) fn new(root: Entity, generation: u64) -> Self {
        Self {
            root,
            generation,
            projecting: false,
            dragging: false,
            gesture_position: None,
        }
    }
}

pub(super) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetrySliderPlugin>() {
        app.add_plugins(WidgetrySliderPlugin);
    }
    app.add_observer(started)
        .add_observer(changed)
        .add_observer(ended);
}

fn duration(world: &World, root: Entity) -> ViewResult<Duration> {
    world
        .get::<WaveformRuntime>(root)
        .and_then(WaveformRuntime::metadata)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Metadata).at(root))?
        .total_duration()
        .map_err(bevy_widgetry_job::JobFailure::from)?
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Duration))
}

pub(super) fn project(world: &mut World, slider: Entity, playback: WaveformPlayback) -> ViewResult {
    if world.get::<WidgetrySlider>(slider).is_none() {
        return Err(crate::view_error::missing(crate::WaveformPart::ProgressSlider).at(slider));
    }
    let owner = world
        .get::<ProgressOwner>(slider)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ProgressOwner).at(slider))?;
    if owner.dragging || owner.generation != playback.session_generation {
        return Ok(());
    }
    let total = duration(world, owner.root)?;
    let value = (playback.position.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0) as f32;
    world
        .get_mut::<ProgressOwner>(slider)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ProgressOwner).at(slider))?
        .projecting = true;
    let result = WidgetrySliderHeadless::set_value_in_world(world, slider, value);
    if let Some(mut owner) = world.get_mut::<ProgressOwner>(slider) {
        owner.projecting = false;
    }
    result
        .map(|_| ())
        .map_err(|error| -> crate::view_error::ViewFailure {
            error
                .with_context(
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform.project_progress")
                        .field("slider", slider),
                )
                .into()
        })
}

fn request(commands: &mut Commands, root: Entity, generation: u64, action: WaveformPlaybackAction) {
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<Waveform>(root).is_none() {
            return Ok(());
        }
        if Waveform::playback(world, root)
            .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?
            .is_some_and(|state| state.session_generation == generation)
        {
            world.trigger(WaveformPlaybackRequested {
                entity: root,
                session_generation: generation,
                action,
            });
        }
        Ok(())
    });
}

fn started(
    event: On<SliderDragStarted>,
    mut owners: Query<&mut ProgressOwner>,
    cursors: Query<(&crate::WaveformCursor, &WaveformRuntime)>,
    mut commands: Commands,
) {
    let Ok(mut owner) = owners.get_mut(event.entity) else {
        return;
    };
    let Ok((cursor, runtime)) = cursors.get(owner.root) else {
        return;
    };
    if runtime.session_generation() != owner.generation {
        return;
    }
    owner.dragging = true;
    owner.gesture_position = Some(cursor.position);
    request(
        &mut commands,
        owner.root,
        owner.generation,
        WaveformPlaybackAction::SeekStarted,
    );
}

fn position(total: Duration, value: f32) -> Result<Duration, bevy_widgetry_job::JobFailure> {
    if value <= 0.0 {
        return Ok(Duration::ZERO);
    }
    if value >= 1.0 {
        return Ok(total);
    }
    Duration::try_from_secs_f64(total.as_secs_f64() * f64::from(value))
        .map(|position| position.min(total))
        .map_err(|source| {
            crate::error::request(
                crate::WaveformError::SeekPosition {
                    total,
                    value,
                    source,
                },
                None,
                "waveform.playback.seek",
            )
        })
}

fn changed(
    event: On<ValueChange<f32>>,
    mut owners: Query<&mut ProgressOwner>,
    runtimes: Query<&WaveformRuntime>,
    mut commands: Commands,
) -> Result {
    let Ok(mut owner) = owners.get_mut(event.source) else {
        return Ok(());
    };
    if owner.projecting {
        return Ok(());
    }
    let Ok(runtime) = runtimes.get(owner.root) else {
        return Ok(());
    };
    if runtime.session_generation() != owner.generation {
        return Ok(());
    }
    let total = runtime
        .metadata()
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Metadata)
                .into_bevy_error(None, "waveform.request")
        })?
        .total_duration()
        .map_err(crate::WaveformFailure::into_bevy_error)?
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Duration)
                .into_bevy_error(None, "waveform.request")
        })?;
    let position =
        position(total, event.value).map_err(bevy_widgetry_job::JobFailure::into_bevy_error)?;
    if owner.dragging {
        owner.gesture_position = Some(position);
    }
    request(
        &mut commands,
        owner.root,
        owner.generation,
        WaveformPlaybackAction::Seek(position),
    );
    Ok(())
}

fn ended(
    event: On<SliderDragEnded>,
    mut owners: Query<&mut ProgressOwner>,
    runtimes: Query<&WaveformRuntime>,
    mut commands: Commands,
) -> Result {
    let Ok(mut owner) = owners.get_mut(event.entity) else {
        return Ok(());
    };
    owner.dragging = false;
    let Some(position) = owner.gesture_position.take() else {
        return Ok(());
    };
    let Ok(runtime) = runtimes.get(owner.root) else {
        return Ok(());
    };
    if runtime.session_generation() != owner.generation {
        return Ok(());
    }
    request(
        &mut commands,
        owner.root,
        owner.generation,
        WaveformPlaybackAction::SeekEnded {
            position,
            outcome: event.outcome,
        },
    );
    Ok(())
}
