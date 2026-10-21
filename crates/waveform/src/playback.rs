use crate::{Waveform, WaveformRuntime, WaveformSessionKind};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_internal_log::widgetry_error;
use std::time::Duration;
mod seek;
mod ui;
pub(crate) use ui::PlaybackUi;
pub(crate) use ui::install;

fn missing(entity: Entity, reason: &str) -> BevyError {
    widgetry_error!(?entity, reason, "Waveform 回放内部状态缺失");
    logged_error(reason)
}

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
    pub(crate) fn new(speeds: Vec<f64>) -> Result<Self, BevyError> {
        if !speeds.contains(&1.0)
            || speeds.iter().enumerate().any(|(index, speed)| {
                !speed.is_finite() || *speed <= 0.0 || speeds[..index].contains(speed)
            })
        {
            widgetry_error!(?speeds, "Waveform 回放倍率候选无效");
            return Err(logged_error("Invalid replay speed choices"));
        }
        Ok(Self {
            speeds,
            ..default()
        })
    }
}

impl Waveform {
    pub fn playback(world: &World, entity: Entity) -> Result<Option<WaveformPlayback>, BevyError> {
        Self::validate_target(world, entity)?;
        let runtime = world
            .get::<WaveformRuntime>(entity)
            .ok_or_else(|| missing(entity, "Waveform runtime missing"))?;
        if !runtime
            .metadata()
            .is_some_and(|metadata| matches!(metadata.kind, WaveformSessionKind::Replay { .. }))
        {
            return Ok(None);
        }
        let playback = world
            .get::<Playback>(entity)
            .ok_or_else(|| missing(entity, "Waveform playback missing"))?;
        Ok(Some(WaveformPlayback {
            session_generation: runtime.session_generation(),
            position: world
                .get::<crate::WaveformCursor>(entity)
                .ok_or_else(|| missing(entity, "Waveform cursor missing"))?
                .position,
            playing: runtime.is_running(),
            speed: playback.speed,
            preview: runtime.is_preview(),
        }))
    }

    pub fn set_playback_in_world(
        world: &mut World,
        entity: Entity,
        playback: WaveformPlayback,
    ) -> Result<bool, BevyError> {
        let previous = Self::playback(world, entity)?.ok_or_else(|| {
            widgetry_error!(?entity, "Waveform 当前不是回放会话");
            logged_error("Waveform does not have a Replay session")
        })?;
        let runtime = world
            .get::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?;
        let total = runtime
            .metadata()
            .ok_or_else(|| logged_error("Waveform metadata missing"))?
            .total_duration()?
            .ok_or_else(|| logged_error("Replay duration missing"))?;
        let state = world
            .get::<Playback>(entity)
            .ok_or_else(|| logged_error("Waveform playback missing"))?;
        if playback.session_generation != previous.session_generation
            || !playback.speed.is_finite()
            || !state.speeds.contains(&playback.speed)
            || playback.position > total
            || playback.playing && playback.position == total
            || playback.preview && playback.playing
        {
            widgetry_error!(?entity, ?playback, "Waveform 回放状态提交无效或会话已过期");
            return Err(logged_error("Invalid or stale Waveform playback state"));
        }
        if playback == previous {
            return Ok(false);
        }
        world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .set_preview(playback.preview);
        let running_changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .set_running(playback.playing)?;
        let explicit_position = playback.preview != previous.preview
            || !playback.playing && (playback.position != previous.position || previous.playing)
            || playback.position < previous.position;
        if explicit_position {
            world
                .get_mut::<WaveformRuntime>(entity)
                .ok_or_else(|| logged_error("Waveform runtime missing"))?
                .request_position(playback.position);
            world
                .get_mut::<crate::WaveformCursor>(entity)
                .ok_or_else(|| logged_error("Waveform cursor missing"))?
                .seek(playback.position);
        } else {
            world
                .get_mut::<crate::WaveformCursor>(entity)
                .ok_or_else(|| logged_error("Waveform cursor missing"))?
                .position = playback.position;
        }
        let mut state = world
            .get_mut::<Playback>(entity)
            .ok_or_else(|| logged_error("Waveform playback missing"))?;
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
            Self::set_playback_in_world(world, entity, playback).map(|_| ())
        });
    }
}
