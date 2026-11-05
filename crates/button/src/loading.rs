use crate::{
    ButtonError, ButtonFailure, WidgetryButtonHeadless,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_cursor::internal::LoadingCursor;

#[derive(Component, Default, Clone, Debug, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryButtonLoading(bool);

#[derive(EntityEvent)]
pub struct WidgetryButtonLoadingChanged {
    pub entity: Entity,
    pub loading: bool,
}

fn missing(root: Entity, part: &'static str) -> ButtonFailure {
    failure(ButtonError::MissingPart { root, part }, "button.loading")
}

impl WidgetryButtonLoading {
    pub fn is_active(&self) -> bool {
        self.0
    }

    pub fn get(world: &World, entity: Entity) -> Result<&Self, ButtonFailure> {
        if world.get_entity(entity).is_err()
            || (world.get::<WidgetryButtonHeadless>(entity).is_none()
                && world.get::<Self>(entity).is_none())
        {
            return Err(failure(
                ButtonError::InvalidTarget { root: entity },
                "button.loading",
            ));
        }
        if world.get::<WidgetryButtonHeadless>(entity).is_none() {
            return Err(missing(entity, "WidgetryButtonHeadless"));
        }
        if world.get::<Node>(entity).is_none() {
            return Err(missing(entity, "Node"));
        }
        if world.get::<ComputedNode>(entity).is_none() {
            return Err(missing(entity, "ComputedNode"));
        }
        world
            .get::<Self>(entity)
            .ok_or_else(|| missing(entity, "WidgetryButtonLoading"))
    }

    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        active: bool,
    ) -> Result<bool, ButtonFailure> {
        if Self::get(world, entity)?.is_active() == active {
            return Ok(false);
        }
        bevy_widgetry_core::disabled::validate_intrinsic_disabled_target(world, entity)
            .map_err(|error| core(error, entity, "button.set_loading.disabled"))?;
        world.entity_mut(entity).insert(Self(active));
        if active {
            world.entity_mut(entity).insert(LoadingCursor);
        } else {
            world.entity_mut(entity).remove::<LoadingCursor>();
        }
        bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<Self>(world, entity, active)
            .map_err(|error| core(error, entity, "button.set_loading.disabled"))?;
        world.trigger(WidgetryButtonLoadingChanged {
            entity,
            loading: active,
        });
        Ok(true)
    }

    pub fn set(commands: &mut Commands, entity: Entity, active: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, active)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}

pub(crate) fn install(app: &mut App) {
    app.register_type::<WidgetryButtonLoading>();
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_core::diagnostics::CoreError;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    use std::error::Error;

    #[test]
    fn unavailable_disabled_runtime_preserves_state_and_original_core_diagnostic() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<bevy::scene::ScenePatch>();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButtonHeadless WidgetryButtonLoading::default() })
            .unwrap()
            .id();
        assert!(!WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap());
        let logs = LogCapture::default();
        let failure = logs
            .run(|| WidgetryButtonLoading::set_in_world(app.world_mut(), root, true))
            .unwrap_err();
        assert!(
            matches!(failure.kind(), ButtonError::Core { source: CoreError::MissingResource { entity, resource: "DisabledRuntime" } } if *entity == root)
        );
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        assert_eq!(report.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = report
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            !app.world()
                .get::<WidgetryButtonLoading>(root)
                .unwrap()
                .is_active()
        );
        assert!(app.world().get::<LoadingCursor>(root).is_none());
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(root)
                .is_none()
        );
    }
}
