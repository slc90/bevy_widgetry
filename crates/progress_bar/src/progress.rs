use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

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
    pub(crate) fn validate(self) -> Result<(), BevyError> {
        if let Self::Determinate { value } = self
            && (!value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(invalid(
                "ProgressBar value must be finite and within [0, 1]",
            ));
        }
        Ok(())
    }
}

fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "ProgressBar 状态请求无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

fn state(world: &World, entity: Entity) -> Result<&ProgressState, BevyError> {
    if world.get::<WidgetryProgressBar>(entity).is_none() {
        return Err(invalid(
            "ProgressBar target does not exist or is not a WidgetryProgressBar",
        ));
    }
    world
        .get::<ProgressState>(entity)
        .ok_or_else(|| invalid("ProgressBar runtime state is missing"))
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

    pub fn state(world: &World, entity: Entity) -> Result<WidgetryProgressState, BevyError> {
        Ok(state(world, entity)?.0)
    }

    pub fn set_state_in_world(
        world: &mut World,
        entity: Entity,
        next: WidgetryProgressState,
    ) -> Result<bool, BevyError> {
        let current = state(world, entity)?;
        next.validate()?;
        if current.0 == next {
            return Ok(false);
        }
        world
            .get_mut::<ProgressState>(entity)
            .ok_or_else(|| invalid("ProgressBar runtime state is missing"))?
            .0 = next;
        Ok(true)
    }

    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryProgressState) {
        commands.queue(move |world: &mut World| {
            Self::set_state_in_world(world, entity, state).map(|_| ())
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
