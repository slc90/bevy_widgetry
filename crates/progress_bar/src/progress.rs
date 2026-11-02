use crate::{
    ProgressBarError, ProgressBarFailure,
    error::{failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum WidgetryProgressState {
    Determinate { value: f32 },
    Indeterminate,
}

impl Default for WidgetryProgressState {
    fn default() -> Self {
        Self::Determinate { value: 0.0 }
    }
}

impl WidgetryProgressState {
    pub(crate) fn validate(self) -> Result<(), ProgressBarFailure> {
        if let Self::Determinate { value } = self
            && (!value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(failure(
                ProgressBarError::InvalidValue { value },
                ErrorFrame::new("progress_bar.validate_value"),
            ));
        }
        Ok(())
    }
}

fn state(world: &World, entity: Entity) -> Result<&ProgressState, ProgressBarFailure> {
    if world.get::<WidgetryProgressBar>(entity).is_none() {
        return Err(failure(
            ProgressBarError::InvalidRoot { root: entity },
            ErrorFrame::new("progress_bar.state"),
        ));
    }
    world
        .get::<ProgressState>(entity)
        .ok_or_else(|| missing(entity, "ProgressState"))
}

#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub(crate) struct ProgressState(pub(crate) WidgetryProgressState);

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryProgressBarProps)]
pub struct WidgetryProgressBar;

#[derive(Clone)]
pub struct WidgetryProgressBarProps {
    pub state: WidgetryProgressState,
    pub height: f32,
    pub colors: crate::WidgetryProgressBarColorOverrides,
}

impl Default for WidgetryProgressBarProps {
    fn default() -> Self {
        Self {
            state: default(),
            height: 12.0,
            colors: default(),
        }
    }
}

impl WidgetryProgressBar {
    fn scene(props: WidgetryProgressBarProps) -> impl Scene {
        crate::style::scene(props)
    }

    pub fn state(
        world: &World,
        entity: Entity,
    ) -> Result<WidgetryProgressState, ProgressBarFailure> {
        Ok(state(world, entity)?.0)
    }

    pub fn set_state_in_world(
        world: &mut World,
        entity: Entity,
        next: WidgetryProgressState,
    ) -> Result<bool, ProgressBarFailure> {
        let current = state(world, entity)?;
        next.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("progress_bar.set_state")
                    .field("root", entity)
                    .field("state", format_args!("{next:?}")),
            )
        })?;
        if current.0 == next {
            return Ok(false);
        }
        world
            .get_mut::<ProgressState>(entity)
            .ok_or_else(|| missing(entity, "ProgressState"))?
            .0 = next;
        Ok(true)
    }

    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryProgressState) {
        commands.queue(move |world: &mut World| {
            Self::set_state_in_world(world, entity, state)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn state_validation_accepts_boundaries_and_rejects_non_finite_values() {
        for value in [0.0, -0.0, f32::MIN_POSITIVE, 0.65, 1.0] {
            assert!(
                WidgetryProgressState::Determinate { value }
                    .validate()
                    .is_ok()
            );
        }
        assert!(WidgetryProgressState::Indeterminate.validate().is_ok());
        for value in [
            f32::NAN,
            f32::NEG_INFINITY,
            f32::INFINITY,
            -f32::MIN_POSITIVE,
            1.0 + f32::EPSILON,
        ] {
            assert_eq!(
                WidgetryProgressState::Determinate { value }
                    .validate()
                    .unwrap_err()
                    .into_bevy_error()
                    .severity(),
                bevy::ecs::error::Severity::Error
            );
        }
    }

    #[test]
    fn same_value_does_not_dirty_state_and_incomplete_targets_are_rejected() {
        let mut world = World::new();
        let entity = world
            .spawn((WidgetryProgressBar, ProgressState(default())))
            .id();
        world.clear_trackers();
        assert!(!WidgetryProgressBar::set_state_in_world(&mut world, entity, default()).unwrap());
        assert!(
            !world
                .entity(entity)
                .get_ref::<ProgressState>()
                .unwrap()
                .is_changed()
        );
        world.entity_mut(entity).remove::<ProgressState>();
        assert!(WidgetryProgressBar::state(&world, entity).is_err());
        assert!(WidgetryProgressBar::set_state_in_world(&mut world, entity, default()).is_err());
        world
            .entity_mut(entity)
            .insert(ProgressState(default()))
            .remove::<WidgetryProgressBar>();
        assert!(WidgetryProgressBar::state(&world, entity).is_err());
        assert!(WidgetryProgressBar::set_state_in_world(&mut world, entity, default()).is_err());
    }
}
