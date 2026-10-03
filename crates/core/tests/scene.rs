//! State：预约 root/child、既有 entity、Scene 成功/失败与 deferred command。
//! Stimuli：spawn/apply Scene、nested template failure、entity index 复用与嵌套同步 boundary。
//! Guards：失败清理只回收本次新建且未写入 Component 的预约 entity。
//! Transitions：失败以 Severity::Error 交给宿主，预约 root/child 回收，既有 root 可重试。
//! Invariants：既有 entity、业务 Component 副作用及外层预约保留；连续失败不累计空 entity。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::{
    WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, apply_scene, spawn_scene,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};

#[test]
fn scene_command_failure_reaches_host_and_removes_reserved_root() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app
        .world_mut()
        .commands()
        .spawn_scene_with_error_handler(bsn! {
            template(|_| Err::<Node, _>(BevyError::error("invalid configuration")))
        })
        .id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(app.world().get_entity(root).is_err());
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR)
    );
}

#[test]
fn scene_application_failure_preserves_existing_entity_and_allows_retry() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().spawn(Name::new("owner")).id();
    app.world_mut()
        .commands()
        .entity(root)
        .apply_scene_with_error_handler(bsn! {
            template(|_| Err::<Node, _>(BevyError::error("invalid patch")))
        });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(app.world().get::<Name>(root).unwrap().as_str(), "owner");
    app.world_mut()
        .commands()
        .entity(root)
        .apply_scene_with_error_handler(bsn! { Node });
    app.world_mut().flush();
    assert!(app.world().get::<Node>(root).is_some());
}

#[test]
fn nested_scene_failure_does_not_leak_reservations() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let empty = app.world_mut().spawn_empty().id();
    let before = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    for _ in 0..2 {
        app.world_mut()
            .commands()
            .spawn_scene_with_error_handler(bsn! {
                Name("root") Children [(Name("branch") Children [(
                    template(|_| Err::<Node, _>(BevyError::error("nested failure")))
                )])]
            });
        errors.run(|| logs.run(|| app.world_mut().flush()));
        let after = app
            .world_mut()
            .query::<Entity>()
            .iter(app.world())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(after, before);
        assert!(app.world().get_entity(empty).is_ok());
    }
    assert_eq!(errors.take().len(), 2);
}

#[test]
fn failed_scene_patch_preserves_business_entities() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().spawn(Name::new("owner")).id();
    let empty = app.world_mut().spawn_empty().id();
    let previous = app.world_mut().spawn(Name::new("previous")).id();
    let before = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    app.world_mut()
        .commands()
        .entity(root)
        .apply_scene_with_error_handler(bsn! {
            Children [(
                template(move |context| {
                    context.entity.world_scope(|world| {
                        world.entity_mut(previous).remove::<Name>();
                        world.spawn(Name::new("independent"));
                    });
                    Err::<Node, _>(BevyError::error("child failure"))
                })
            )]
        });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert!(app.world().get_entity(empty).is_ok());
    assert!(app.world().get_entity(previous).is_ok());
    let entities = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(entities.len(), before.len() + 1);
    assert!(
        before
            .iter()
            .all(|entity| app.world().get_entity(*entity).is_ok())
    );
    assert!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .any(|name| name.as_str() == "independent")
    );
}

#[test]
fn synchronous_failure_handles_reused_indices_and_same_tick_entities() {
    let mut app = scene_app();
    let world = app.world_mut();
    let existing = world.spawn_empty().id();
    let released = world.spawn_empty().id();
    world.despawn(released);
    let before = world.entities().count_spawned();
    let result = spawn_scene(
        world,
        bsn! {
            Name("root") Children [(
                template(|_| Err::<Node, _>(BevyError::error("reservation failure")))
            )]
        },
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("reservation failure")
    );
    assert_eq!(world.entities().count_spawned(), before);
    assert!(world.entities().contains(existing));
}

#[test]
fn nested_synchronous_failure_preserves_outer_and_existing_entities() {
    let mut app = scene_app();
    let world = app.world_mut();
    let root = world.spawn(Name::new("owner")).id();
    let existing = world.spawn(Name::new("business")).id();
    let before = world.entities().count_spawned();
    let result = apply_scene(
        &mut world.entity_mut(root),
        bsn! {
            Children [(
                template(move |context| {
                    context.entity.world_scope(|world| {
                        let nested = spawn_scene(world, bsn! {
                            Children [(
                                template(|_| Err::<Node, _>(BevyError::error("nested failure")))
                            )]
                        });
                        assert!(nested.is_err());
                        world.entity_mut(existing).remove::<Name>();
                        world.spawn(Name::new("independent"));
                    });
                    Err::<Node, _>(BevyError::error("outer failure"))
                })
            )]
        },
    );
    assert!(result.unwrap_err().to_string().contains("outer failure"));
    assert_eq!(world.entities().count_spawned(), before + 1);
    assert!(world.entities().contains(root));
    assert!(world.entities().contains(existing));
}
