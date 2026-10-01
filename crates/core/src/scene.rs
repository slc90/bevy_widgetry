//! 保留 BSN composition 的 deferred Scene 错误适配；库不设置宿主 error handler。

use bevy::ecs::system::EntityCommands;
use bevy::ecs::world::EntityWorldMut;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::scene::{ApplySceneError, SpawnSceneError};
use bevy_widgetry_log::widgetry_error;
use std::fmt;

/// BSN 构造 command：返回预约 entity，失败以 Severity::Error 交给宿主并清理该 root。
pub trait WidgetrySceneCommandsExt {
    /// 与原生 spawn_scene 一样接收 Scene；预约的 id 只有 command 成功后才表示完整 Widget。
    /// 失败会销毁本次预约 root；随后操作应验证 entity 仍存在。
    fn spawn_scene_with_error_handler<S: Scene>(&mut self, scene: S) -> EntityCommands<'_>;
}

/// 对已有 entity 应用 BSN，失败交给宿主，不销毁调用方持有的 entity。
pub trait WidgetrySceneEntityCommandsExt {
    /// 保留 Scene 失败内容并显式使用 Severity::Error。
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
                    // 预约 root 在构造前被调用方销毁，是正常取消。
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
        // EntityCommands::queue 会再包一层 EntityCommandError，丢失 BevyError severity。
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

/// Workspace 内部：同步展开 Scene；日志与 severity 由拥有业务上下文的调用方负责。
/// 失败会清理展开中新建但尚未写入任何 Component 的预约 entity，保留原有 entity。
/// Template 新建的空 entity 属于本次 Scene 预约；独立业务 entity 应在创建时携带其 Component。
/// 已写入 Component 的业务副作用不做回滚；既有 root 的部分 Scene patch 也保留。
pub fn apply_scene<S: Scene>(
    entity: &mut EntityWorldMut<'_>,
    scene: S,
) -> Result<(), SpawnSceneError> {
    // Bevy 0.19 在 template 成功后才写 ChildOf；失败 child 和未使用的 forward reference
    // 仍停留在空 archetype，root despawn 无法触达。保留完整既有 identity，避免误删原有对象。
    let before =
        entity.world_scope(|world| world.query::<Entity>().iter(world).collect::<HashSet<_>>());
    let result = entity.apply_scene(scene);
    if result.is_err() {
        entity.world_scope(|world| {
            let reservations = world
                .archetypes()
                .empty()
                .entities()
                .iter()
                .map(|entity| entity.id())
                .filter(|entity| !before.contains(entity))
                .collect::<Vec<_>>();
            for reservation in reservations {
                world.despawn(reservation);
            }
        });
    }
    result
}

/// Workspace 内部：同步创建 Scene，失败清理 root 与未完成的空预约 entity。
/// 日志与 Severity::Error 转换仍由调用方负责；Template 的空预约规则同 apply_scene。
pub fn spawn_scene<S: Scene>(world: &mut World, scene: S) -> Result<Entity, SpawnSceneError> {
    let mut entity = world.spawn_empty();
    let id = entity.id();
    if let Err(error) = apply_scene(&mut entity, scene) {
        world.despawn(id);
        return Err(error);
    }
    Ok(id)
}

/// 产生处已报告的配置错误，避免在 Scene command 的传播层重复记录。
#[derive(Debug)]
struct LoggedError(&'static str);

impl fmt::Display for LoggedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for LoggedError {}

/// Workspace 内部：仅用于已通过 widgetry_error! 记录的 Scene 配置失败。
pub fn logged_error(message: &'static str) -> BevyError {
    BevyError::error(LoggedError(message))
}

/// Workspace 内部：识别嵌套 Scene 中已在产生处记录的 Widgetry 配置失败。
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
