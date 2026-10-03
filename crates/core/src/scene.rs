use bevy::ecs::system::EntityCommands;
use bevy::ecs::world::EntityWorldMut;
use bevy::prelude::*;
use bevy::scene::{ApplySceneError, SpawnSceneError};
use bevy_widgetry_log::widgetry_error;
use std::fmt;

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
                    // 调用方可能在 deferred 构造前销毁预约 root；将其视为取消，避免为正常 lifecycle 向宿主报告 Scene 错误。
                    Err(_) => return Ok(()),
                };
                if let Err(error) = result {
                    if !scene_error_logged(&error) {
                        widgetry_error!(entity = ?id, %error, "Scene 构造失败");
                    }
                    world.despawn(id);
                    return Err(BevyError::error(error));
                }
                Ok(())
            });
        commands
    }
}

impl WidgetrySceneEntityCommandsExt for EntityCommands<'_> {
    fn apply_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> &mut Self {
        let id = self.id();
        // EntityCommands::queue 会再包一层 EntityCommandError，丢失 BevyError severity；改用 Commands::queue 直接返回原 severity 的错误。
        self.commands()
            .queue(move |world: &mut World| -> Result<(), BevyError> {
                let Ok(mut entity) = world.get_entity_mut(id) else {
                    return Ok(());
                };
                apply_scene(&mut entity, scene).map_err(|error| {
                    if !scene_error_logged(&error) {
                        widgetry_error!(entity = ?id, %error, "Scene 应用失败");
                    }
                    BevyError::error(error)
                })
            });
        self
    }
}

pub fn apply_scene<S: Scene>(
    entity: &mut EntityWorldMut<'_>,
    scene: S,
) -> Result<(), SpawnSceneError> {
    // Scene 失败后可能留下空预约，单凭空 archetype 会误删既有 entity；先推进 tick，再按 spawn tick 区分本次新建的 generation。
    let before = entity.world_scope(World::increment_change_tick);
    let result = entity.apply_scene(scene);
    if result.is_err() {
        entity.world_scope(|world| {
            let current = world.change_tick();
            // Bevy 在 template 成功后才写 ChildOf，递归销毁 root 无法清理尚未关联的 child 和 forward reference；额外回收本次新建的空 reservation。
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

#[derive(Debug)]
struct LoggedError(String);

impl fmt::Display for LoggedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LoggedError {}

pub fn logged_error(message: impl Into<String>) -> BevyError {
    BevyError::error(LoggedError(message.into()))
}

pub fn scene_error_logged(error: &SpawnSceneError) -> bool {
    let SpawnSceneError::ApplySceneError(error) = error else {
        return false;
    };
    let mut error = error;
    loop {
        match error {
            ApplySceneError::TemplateBuildError(error) => return error.is::<LoggedError>(),
            ApplySceneError::CachedSceneApplyError { error: inner, .. }
            | ApplySceneError::RelatedSceneError { error: inner, .. } => error = inner,
            _ => return false,
        }
    }
}
