use bevy::ecs::system::EntityCommands;
use bevy::ecs::world::EntityWorldMut;
use bevy::prelude::*;
#[doc(hidden)]
pub mod attempt;
mod diagnostics;
pub use bevy_widgetry_diagnostics::ErrorFrame;
pub use diagnostics::{
    SceneError, SceneFailure, SceneRejection, original_scene_error, source_error,
};

pub trait WidgetrySceneCommandsExt {
    fn spawn_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> EntityCommands<'_>;
}

pub trait WidgetrySceneEntityCommandsExt {
    fn apply_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> &mut Self;
}

impl WidgetrySceneCommandsExt for Commands<'_, '_> {
    fn spawn_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> EntityCommands<'_> {
        let mut commands = self.spawn_empty();
        let id = commands.id();
        commands
            .commands()
            .queue(move |world: &mut World| -> Result<(), BevyError> {
                let result = match world.get_entity_mut(id) {
                    Ok(mut entity) => apply_scene(&mut entity, scene),
                    // The caller may despawn the reserved root before deferred construction.
                    // Treat this as cancellation rather than reporting a Scene error for a normal lifecycle event.
                    Err(_) => return Ok(()),
                };
                if let Err(error) = result {
                    world.despawn(id);
                    return Err(error.into_bevy_error());
                }
                Ok(())
            });
        commands
    }
}

impl WidgetrySceneEntityCommandsExt for EntityCommands<'_> {
    fn apply_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> &mut Self {
        let id = self.id();
        // EntityCommands::queue wraps errors in EntityCommandError and loses the BevyError severity.
        // Use Commands::queue to preserve the original severity.
        self.commands()
            .queue(move |world: &mut World| -> Result<(), BevyError> {
                let Ok(mut entity) = world.get_entity_mut(id) else {
                    return Ok(());
                };
                apply_scene(&mut entity, scene).map_err(SceneFailure::into_bevy_error)
            });
        self
    }
}

#[track_caller]
pub fn apply_scene<S: Scene>(
    entity: &mut EntityWorldMut<'_>,
    scene: S,
) -> Result<(), SceneFailure> {
    let root = entity.id();
    let result = attempt::apply_scene(entity, scene);
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(SceneFailure::from_native(error, root, "apply_scene")),
    }
}

#[track_caller]
pub fn spawn_scene<S: Scene>(world: &mut World, scene: S) -> Result<Entity, SceneFailure> {
    let mut entity = world.spawn_empty();
    let id = entity.id();
    if let Err(error) = apply_scene(&mut entity, scene) {
        world.despawn(id);
        return Err(error.with_context(ErrorFrame::new("spawn_scene").field("root", id)));
    }
    Ok(id)
}
