use crate::{Waveform, WaveformRuntime, WaveformSessionKind};
use bevy::prelude::*;
use bevy_widgetry_job::JobFailure;
use std::time::Duration;
mod seek;
mod ui;
pub(crate) use ui::PlaybackUi;
pub(crate) use ui::install;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformPlayback {
    pub session_generation: u64,
    pub position: Duration,
    pub playing: bool,
    pub speed: f64,
    pub preview: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformPlaybackAction {
    Playing(bool),
    Speed(f64),
    SeekStarted,
    Seek(std::time::Duration),
    SeekEnded {
        position: std::time::Duration,
        outcome: bevy_widgetry_slider::SliderDragOutcome,
    },
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformPlaybackRequested {
    pub entity: Entity,
    pub session_generation: u64,
    pub action: WaveformPlaybackAction,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformPlaybackChanged {
    pub entity: Entity,
    pub playback: WaveformPlayback,
}

#[derive(Component)]
pub(crate) struct Playback {
    pub(crate) speed: f64,
    pub(crate) speeds: Vec<f64>,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            speed: 1.0,
            speeds: vec![0.25, 0.5, 1.0, 1.5, 2.0, 4.0],
        }
    }
}

impl Playback {
    pub(crate) fn new(speeds: Vec<f64>) -> Result<Self, JobFailure> {
        if !speeds.contains(&1.0)
            || speeds.iter().enumerate().any(|(index, speed)| {
                !speed.is_finite() || *speed <= 0.0 || speeds[..index].contains(speed)
            })
        {
            return Err(crate::error::request(
                crate::WaveformError::PlaybackSpeeds { speeds },
                None,
                "waveform.playback.speeds",
            ));
        }
        Ok(Self {
            speeds,
            ..default()
        })
    }
}

impl Waveform {
    pub fn playback(world: &World, entity: Entity) -> Result<Option<WaveformPlayback>, JobFailure> {
        Self::validate_target(world, entity)?;
        snapshot(world, entity)
            .map_err(|error| error.into_failure(Some(entity), "waveform.playback"))
    }

    pub fn set_playback_in_world(
        world: &mut World,
        entity: Entity,
        playback: WaveformPlayback,
    ) -> Result<bool, JobFailure> {
        let previous = Self::playback(world, entity)?.ok_or_else(|| {
            crate::error::request(
                crate::WaveformError::ReplaySession { entity },
                Some(entity),
                "waveform.playback.set",
            )
        })?;
        let runtime = world.get::<WaveformRuntime>(entity).ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Runtime)
                .at(entity)
                .into_failure(Some(entity), "waveform.request")
        })?;
        let total = runtime
            .metadata()
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Metadata)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .total_duration()
            .map_err(JobFailure::from)?
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Duration)
                    .into_failure(Some(entity), "waveform.request")
            })?;
        let state = world.get::<Playback>(entity).ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Playback)
                .at(entity)
                .into_failure(Some(entity), "waveform.request")
        })?;
        if playback.session_generation != previous.session_generation
            || !playback.speed.is_finite()
            || !state.speeds.contains(&playback.speed)
            || playback.position > total
            || playback.playing && playback.position == total
            || playback.preview && playback.playing
        {
            return Err(crate::error::request(
                crate::WaveformError::PlaybackState {
                    entity,
                    proposed: Box::new(playback),
                    previous: Box::new(previous),
                    total,
                },
                Some(entity),
                "waveform.playback.set",
            ));
        }
        if playback == previous {
            return Ok(false);
        }
        world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .set_preview(playback.preview)?;
        let running_changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .set_running(playback.playing)?;
        let explicit_position = playback.preview != previous.preview
            || !playback.playing && (playback.position != previous.position || previous.playing)
            || playback.position < previous.position;
        if explicit_position {
            world
                .get_mut::<WaveformRuntime>(entity)
                .ok_or_else(|| {
                    crate::view_error::missing(crate::WaveformPart::Runtime)
                        .at(entity)
                        .into_failure(Some(entity), "waveform.request")
                })?
                .request_position(playback.position)?;
            world
                .get_mut::<crate::WaveformCursor>(entity)
                .ok_or_else(|| {
                    crate::view_error::missing(crate::WaveformPart::Cursor)
                        .at(entity)
                        .into_failure(Some(entity), "waveform.request")
                })?
                .seek(playback.position);
        } else {
            world
                .get_mut::<crate::WaveformCursor>(entity)
                .ok_or_else(|| {
                    crate::view_error::missing(crate::WaveformPart::Cursor)
                        .at(entity)
                        .into_failure(Some(entity), "waveform.request")
                })?
                .position = playback.position;
        }
        let mut state = world.get_mut::<Playback>(entity).ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Playback)
                .at(entity)
                .into_failure(Some(entity), "waveform.request")
        })?;
        state.speed = playback.speed;
        if running_changed {
            world.trigger(crate::WaveformRunningChanged {
                entity,
                running: playback.playing,
            });
        }
        world.trigger(WaveformPlaybackChanged { entity, playback });
        Ok(true)
    }

    pub fn set_playback(commands: &mut Commands, entity: Entity, playback: WaveformPlayback) {
        commands.queue(move |world: &mut World| {
            Self::set_playback_in_world(world, entity, playback)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }
}

pub(crate) fn snapshot(
    world: &World,
    entity: Entity,
) -> crate::view_error::ViewResult<Option<WaveformPlayback>> {
    let runtime = world
        .get::<WaveformRuntime>(entity)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Runtime).at(entity))?;
    if !runtime
        .metadata()
        .is_some_and(|metadata| matches!(metadata.kind, WaveformSessionKind::Replay { .. }))
    {
        return Ok(None);
    }
    let playback = world
        .get::<Playback>(entity)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Playback).at(entity))?;
    Ok(Some(WaveformPlayback {
        session_generation: runtime.session_generation(),
        position: world
            .get::<crate::WaveformCursor>(entity)
            .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Cursor).at(entity))?
            .position,
        playing: runtime.is_running(),
        speed: playback.speed,
        preview: runtime.is_preview(),
    }))
}
