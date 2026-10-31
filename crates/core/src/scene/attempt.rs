use bevy::{ecs::world::EntityWorldMut, prelude::*, scene::SpawnSceneError};

#[track_caller]
pub fn into_bevy_error(error: SpawnSceneError, root: Entity, operation: &'static str) -> BevyError {
    if super::diagnostics::has_managed_diagnostics(&error) {
        super::SceneFailure::from_native(error, root, operation).into_bevy_error()
    } else {
        super::diagnostics::into_native_bevy_error(error)
    }
}

pub fn apply_scene<S: Scene>(
    entity: &mut EntityWorldMut<'_>,
    scene: S,
) -> Result<(), SpawnSceneError> {
    // The failure Owner decides whether this attempt is a new failure or another observation.
    // Keep native errors intact until that Owner creates or reuses its diagnostic.
    let before = entity.world_scope(World::increment_change_tick);
    let result = entity.apply_scene(scene);
    if result.is_err() {
        entity.world_scope(|world| {
            let current = world.change_tick();
            // ChildOf is written only after successful construction, so linked-tree cleanup misses reservations.
            let reservations = world
                .archetypes()
                .empty()
                .entities()
                .iter()
                .map(|entity| entity.id())
                .filter(|entity| {
                    world
                        .entities()
                        .entity_get_spawn_or_despawn_tick(*entity)
                        .is_some_and(|spawned| spawned.is_newer_than(before, current))
                })
                .collect::<Vec<_>>();
            for reservation in reservations {
                world.despawn(reservation);
            }
        });
    }
    result
}

pub fn spawn_scene<S: Scene>(world: &mut World, scene: S) -> Result<Entity, SpawnSceneError> {
    let mut entity = world.spawn_empty();
    let id = entity.id();
    if let Err(error) = apply_scene(&mut entity, scene) {
        world.despawn(id);
        return Err(error);
    }
    Ok(id)
}
