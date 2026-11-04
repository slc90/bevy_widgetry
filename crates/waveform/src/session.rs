use crate::{Waveform, WaveformCursor, WaveformRuntime, WaveformSource};
use bevy::prelude::*;
use bevy_widgetry_job::JobFailure;
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
    pub fn validate(&self) -> Result<(), crate::WaveformFailure> {
        if self.channel_count == 0
            || self.channel_labels.len() != self.channel_count
            || self
                .channel_labels
                .iter()
                .any(|label| label.trim().is_empty())
            || !(1..=1_000_000_000).contains(&self.sample_rate)
            || matches!(self.kind, WaveformSessionKind::Replay { total_frames: 0 })
        {
            return Err(crate::error::failure(
                crate::WaveformError::Metadata {
                    metadata: Box::new(self.clone()),
                },
                "waveform.metadata.validate",
            ));
        }
        Ok(())
    }

    pub fn total_duration(&self) -> Result<Option<std::time::Duration>, crate::WaveformFailure> {
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

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformDurationChanged {
    pub entity: Entity,
    pub visible_duration_ms: u32,
}

impl Waveform {
    pub fn grid_visible(world: &World, entity: Entity) -> Result<bool, JobFailure> {
        Self::validate_target(world, entity)?;
        world
            .get::<crate::time_axis::GridState>(entity)
            .map(|state| state.0)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::GridState)
                    .into_failure(Some(entity), "waveform.grid")
            })
    }

    pub fn set_grid_visible_in_world(
        world: &mut World,
        entity: Entity,
        visible: bool,
    ) -> Result<bool, JobFailure> {
        if Self::grid_visible(world, entity)? == visible {
            return Ok(false);
        }
        world
            .get_mut::<crate::time_axis::GridState>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::GridState)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .0 = visible;
        world.trigger(crate::WaveformGridChanged { entity, visible });
        Ok(true)
    }

    pub fn set_grid_visible(commands: &mut Commands, entity: Entity, visible: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_grid_visible_in_world(world, entity, visible)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }

    pub fn set_visible_duration_in_world(
        world: &mut World,
        entity: Entity,
        visible_duration_ms: u32,
    ) -> Result<bool, JobFailure> {
        Self::validate_target(world, entity)?;
        if let Some(choices) = world.get::<crate::controls::TimeChoices>(entity)
            && !choices.0.is_empty()
            && !choices.0.contains(&visible_duration_ms)
        {
            return Err(crate::error::request(
                crate::WaveformError::TimeChoices {
                    choices: choices.0.clone(),
                    duration: visible_duration_ms,
                },
                Some(entity),
                "waveform.duration.set",
            ));
        }
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .set_visible_duration(visible_duration_ms)
            .map_err(|error| {
                error.with_context(
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform.session.duration")
                        .field("root", entity),
                )
            })?;
        if changed {
            world.trigger(WaveformDurationChanged {
                entity,
                visible_duration_ms,
            });
        }
        Ok(changed)
    }

    pub fn set_visible_duration(commands: &mut Commands, entity: Entity, visible_duration_ms: u32) {
        commands.queue(move |world: &mut World| {
            Self::set_visible_duration_in_world(world, entity, visible_duration_ms)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }

    pub fn set_session_in_world(
        world: &mut World,
        entity: Entity,
        source: Arc<dyn WaveformSource>,
    ) -> Result<(), JobFailure> {
        Self::validate_target(world, entity)?;
        world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .replace_session(source)
            .map_err(|error| -> JobFailure {
                error.with_context(
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform.session.replace")
                        .field("entity", entity),
                )
            })?;
        *world.get_mut::<WaveformCursor>(entity).ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::Cursor)
                .at(entity)
                .into_failure(Some(entity), "waveform.request")
        })? = WaveformCursor::default();
        Self::notify_session(world, entity)?;
        Ok(())
    }

    pub fn set_session(commands: &mut Commands, entity: Entity, source: Arc<dyn WaveformSource>) {
        commands.queue(move |world: &mut World| {
            Self::set_session_in_world(world, entity, source)
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }

    pub fn clear_session_in_world(world: &mut World, entity: Entity) -> Result<bool, JobFailure> {
        Self::validate_target(world, entity)?;
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .clear_session()
            .map_err(|error| {
                error.with_context(
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform.session.clear")
                        .field("root", entity),
                )
            })?;
        if changed {
            *world.get_mut::<WaveformCursor>(entity).ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Cursor)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })? = WaveformCursor::default();
            Self::notify_session(world, entity)?;
        }
        Ok(changed)
    }

    pub fn clear_session(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| {
            Self::clear_session_in_world(world, entity)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }

    pub fn set_running_in_world(
        world: &mut World,
        entity: Entity,
        running: bool,
    ) -> Result<bool, JobFailure> {
        Self::validate_target(world, entity)?;
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .set_running(running)
            .map_err(|error| {
                error.with_context(
                    bevy_widgetry_diagnostics::ErrorFrame::new("waveform.session.running")
                        .field("root", entity),
                )
            })?;
        if changed {
            world.trigger(crate::WaveformRunningChanged { entity, running });
        }
        Ok(changed)
    }

    pub fn set_running(commands: &mut Commands, entity: Entity, running: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_running_in_world(world, entity, running)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }

    pub(crate) fn validate_target(world: &World, entity: Entity) -> Result<(), JobFailure> {
        let widget = world.get::<Waveform>(entity).is_some();
        let runtime = world.get::<WaveformRuntime>(entity).is_some();
        let cursor = world.get::<WaveformCursor>(entity).is_some();
        if !widget || !runtime || !cursor {
            return Err(crate::error::request(
                crate::WaveformError::Target {
                    entity,
                    widget,
                    runtime,
                    cursor,
                },
                Some(entity),
                "waveform.target",
            ));
        }
        Ok(())
    }

    fn notify_session(world: &mut World, entity: Entity) -> Result<(), JobFailure> {
        let generation = world
            .get::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .into_failure(Some(entity), "waveform.session.notify")
            })?
            .session_generation();
        if world.get::<crate::view::MeasurementParts>(entity).is_some() {
            Self::clear_measurement_in_world(world, entity)?;
        }
        world.trigger(WaveformSessionChanged { entity, generation });
        Ok(())
    }
}
