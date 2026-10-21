use crate::{Waveform, WaveformCursor, WaveformRuntime, WaveformSource};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_internal_log::widgetry_error;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformUnit {
    Volts,
    Millivolts,
    Microvolts,
}

impl WaveformUnit {
    pub(crate) fn volts(self) -> f32 {
        match self {
            Self::Volts => 1.0,
            Self::Millivolts => 0.001,
            Self::Microvolts => 0.000001,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformSessionKind {
    Live,
    Replay { total_frames: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformMetadata {
    pub channel_count: usize,
    pub channel_labels: Vec<String>,
    pub sample_rate: u32,
    pub unit: WaveformUnit,
    pub kind: WaveformSessionKind,
}

impl WaveformMetadata {
    pub fn validate(&self) -> Result<(), BevyError> {
        if self.channel_count == 0
            || self.channel_labels.len() != self.channel_count
            || self
                .channel_labels
                .iter()
                .any(|label| label.trim().is_empty())
            || !(1..=1_000_000_000).contains(&self.sample_rate)
            || matches!(self.kind, WaveformSessionKind::Replay { total_frames: 0 })
        {
            widgetry_error!(?self, "Waveform 会话元信息无效");
            return Err(BevyError::error("Invalid WaveformMetadata"));
        }
        Ok(())
    }

    pub fn total_duration(&self) -> Result<Option<std::time::Duration>, BevyError> {
        match self.kind {
            WaveformSessionKind::Live => Ok(None),
            WaveformSessionKind::Replay { total_frames } => {
                crate::duration_from_frames(total_frames, self.sample_rate).map(Some)
            }
        }
    }
}

#[derive(EntityEvent, Clone, Debug)]
pub struct WaveformSessionChanged {
    pub entity: Entity,
    pub generation: u64,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformRunningChanged {
    pub entity: Entity,
    pub running: bool,
}

impl Waveform {
    pub fn set_session_in_world(
        world: &mut World,
        entity: Entity,
        source: Arc<dyn WaveformSource>,
    ) -> Result {
        Self::validate_target(world, entity)?;
        world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .replace_session(source)
            .map_err(|error| logged_error(format!("Waveform {entity:?}: {error}")))?;
        world
            .get_mut::<WaveformCursor>(entity)
            .ok_or_else(|| logged_error("Waveform cursor missing"))?
            .position = std::time::Duration::ZERO;
        Self::notify_session(world, entity)?;
        Ok(())
    }

    pub fn set_session(commands: &mut Commands, entity: Entity, source: Arc<dyn WaveformSource>) {
        commands.queue(move |world: &mut World| Self::set_session_in_world(world, entity, source));
    }

    pub fn clear_session_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::validate_target(world, entity)?;
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .clear_session()?;
        if changed {
            world
                .get_mut::<WaveformCursor>(entity)
                .ok_or_else(|| logged_error("Waveform cursor missing"))?
                .position = std::time::Duration::ZERO;
            Self::notify_session(world, entity)?;
        }
        Ok(changed)
    }

    pub fn clear_session(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| {
            Self::clear_session_in_world(world, entity).map(|_| ())
        });
    }

    pub fn set_running_in_world(
        world: &mut World,
        entity: Entity,
        running: bool,
    ) -> Result<bool, BevyError> {
        Self::validate_target(world, entity)?;
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .set_running(running)?;
        if changed {
            world.trigger(crate::WaveformRunningChanged { entity, running });
        }
        Ok(changed)
    }

    pub fn set_running(commands: &mut Commands, entity: Entity, running: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_running_in_world(world, entity, running).map(|_| ())
        });
    }

    fn validate_target(world: &World, entity: Entity) -> Result {
        if world.get::<Waveform>(entity).is_none()
            || world.get::<WaveformRuntime>(entity).is_none()
            || world.get::<WaveformCursor>(entity).is_none()
        {
            widgetry_error!(?entity, "Waveform 会话目标无效");
            return Err(logged_error("Waveform target or runtime is missing"));
        }
        Ok(())
    }

    fn notify_session(world: &mut World, entity: Entity) -> Result {
        let generation = world
            .get::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                widgetry_error!(?entity, "Waveform 通知所需运行时缺失");
                logged_error("Waveform runtime missing")
            })?
            .session_generation();
        world.trigger(WaveformSessionChanged { entity, generation });
        Ok(())
    }
}
