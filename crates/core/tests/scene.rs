//! State：预约 root/child、既有 entity、Scene 成功/失败与 deferred command。
//! Stimuli：spawn/apply Scene、named/forward reference、root 取消、nested template failure 与嵌套同步 boundary。
//! Guards：失败清理只回收本次新建且未写入 Component 的预约 entity。
//! Transitions：失败以 Severity::Error 交给宿主，预约 root/child 回收，既有 root 可重试。
//! Invariants：既有 entity、业务 Component 副作用及外层预约保留，已记录错误不重复输出日志。
//! 连续失败不累计空 entity。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::{
    WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, apply_scene, logged_error,
    spawn_scene,
};
use bevy_widgetry_log::widgetry_error;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};

#[derive(Component, FromTemplate)]
struct SceneReferences {
    first: Entity,
    second: Entity,
}

fn referenced_content() -> impl Scene {
    bsn! {
        SceneReferences { first: #First, second: #Second }
        Children [#First Node--#Second Node]
    }
}

#[test]
fn scene_adapter_preserves_composed_references_children_and_field_patches() {
    let mut app = scene_app();
    let root = spawn_scene(
        app.world_mut(),
        bsn! {
            @referenced_content()
            Node { width: px(120), height: px(80) }
            Node { height: px(60) }
            Children [@referenced_content()]
        },
    )
    .unwrap();
    let references = app.world().get::<SceneReferences>(root).unwrap();
    let children = app.world().get::<Children>(root).unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(&children[..2], &[references.first, references.second]);
    let nested = children[2];
    let nested_references = app.world().get::<SceneReferences>(nested).unwrap();
    let nested_children = app.world().get::<Children>(nested).unwrap();
    assert_eq!(
        &nested_children[..],
        &[nested_references.first, nested_references.second]
    );
    for entity in nested_children.iter() {
        assert_ne!(entity, references.first);
        assert_ne!(entity, references.second);
        assert_eq!(app.world().get::<ChildOf>(entity).unwrap().parent(), nested);
    }
    let node = app.world().get::<Node>(root).unwrap();
    assert_eq!(node.width, px(120));
    assert_eq!(node.height, px(60));
}

#[test]
fn failed_forward_reference_patch_reclaims_only_new_reservations() {
    let mut app = scene_app();
    let root = app.world_mut().spawn(Name::new("owner")).id();
    let existing = app.world_mut().spawn_empty().id();
    let before = app.world().entities().count_spawned();
    let result = apply_scene(
        &mut app.world_mut().entity_mut(root),
        bsn! {
            SceneReferences { first: #First, second: #Second }
            Children [
                #First template(|_| Err::<Node, _>(BevyError::error("forward failure")))
                --
                #Second Node
            ]
        },
    );
    assert!(result.unwrap_err().to_string().contains("forward failure"));
    assert_eq!(app.world().entities().count_spawned(), before);
    assert!(app.world().entities().contains(existing));
    assert_eq!(app.world().get::<Name>(root).unwrap().as_str(), "owner");
}

#[test]
fn cancelled_queued_root_does_not_construct_or_report_failure() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().commands().spawn_empty().id();
    app.world_mut().commands().entity(root).despawn();
    app.world_mut()
        .commands()
        .entity(root)
        .apply_scene_with_error_handler(bsn! {
            template(|_| Err::<Node, _>(BevyError::error("cancelled construction")))
        });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    assert!(errors.take().is_empty());
    assert!(logs.records().is_empty());
    assert!(!app.world().entities().contains(root));
}

#[test]
fn nested_logged_scene_failure_reaches_host_without_duplicate_diagnostics() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app
        .world_mut()
        .commands()
        .spawn_scene_with_error_handler(bsn! {
            Children [template(|_| {
                widgetry_error!("已记录的 Scene failure");
                Err::<Node, _>(logged_error("logged nested failure"))
            })]
        })
        .id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(errors[0].to_string().contains("logged nested failure"));
    assert_eq!(logs.records().len(), 1);
    assert_eq!(logs.records()[0].level, bevy::log::Level::ERROR);
    assert!(!app.world().entities().contains(root));
}

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
                Name("root") Children [Name("branch") Children [
                    template(|_| Err::<Node, _>(BevyError::error("nested failure")))
                ]]
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
            Children [
                template(move |context| {
                    context.entity.world_scope(|world| {
                        world.entity_mut(previous).remove::<Name>();
                        world.spawn(Name::new("independent"));
                    });
                    Err::<Node, _>(BevyError::error("child failure"))
                })
            ]
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
            Name("root") Children [
                template(|_| Err::<Node, _>(BevyError::error("reservation failure")))
            ]
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
            Children [
                template(move |context| {
                    context.entity.world_scope(|world| {
                        let nested = spawn_scene(world, bsn! {
                            Children [
                                template(|_| Err::<Node, _>(BevyError::error("nested failure")))
                            ]
                        });
                        assert!(nested.is_err());
                        world.entity_mut(existing).remove::<Name>();
                        world.spawn(Name::new("independent"));
                    });
                    Err::<Node, _>(BevyError::error("outer failure"))
                })
            ]
        },
    );
    assert!(result.unwrap_err().to_string().contains("outer failure"));
    assert_eq!(world.entities().count_spawned(), before + 1);
    assert!(world.entities().contains(root));
    assert!(world.entities().contains(existing));
}
