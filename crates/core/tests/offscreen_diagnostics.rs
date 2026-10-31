#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_core::offscreen::{
    OffscreenError, OffscreenFailures, OffscreenTarget, OffscreenViewport, WidgetryOffscreenPlugin,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

#[allow(clippy::unwrap_used)]
fn bound(world: &mut World) -> (Entity, Entity, Entity, AssetId<Image>) {
    let owner = world
        .spawn_scene(bsn! { Node::default() template(|_| Ok(ViewportNode { camera: None })) })
        .unwrap()
        .id();
    let scene = world.spawn(Transform::default()).id();
    let camera = world.spawn((Camera2d, ChildOf(scene))).id();
    let target = OffscreenTarget::allocate(world, owner).unwrap();
    let image = target.image().id();
    let viewport = target.bind(world, owner, camera, scene).unwrap();
    world.entity_mut(owner).insert(viewport);
    (owner, scene, camera, image)
}

#[allow(clippy::unwrap_used)]
fn kind(error: &OffscreenFailures) -> &OffscreenError {
    let mut cause: &(dyn Error + 'static) = error.diagnostics().next().unwrap();
    loop {
        if let Some(kind) = cause.downcast_ref::<OffscreenError>() {
            return kind;
        }
        cause = cause.source().unwrap();
    }
}

#[test]
fn allocation_missing_resources_is_lazy_per_owner_and_keeps_ownership_fields() {
    let mut world = World::new();
    let owner = world.spawn_empty().id();
    let other = world.spawn_empty().id();
    let logs = LogCapture::default();
    logs.run(|| {
        let a = OffscreenTarget::allocate(&mut world, owner).err().unwrap();
        let b = OffscreenTarget::allocate(&mut world, owner).err().unwrap();
        let c = OffscreenTarget::allocate(&mut world, other).err().unwrap();
        assert!(matches!(kind(&a), OffscreenError::MissingResource { owner: actual, .. } if *actual == owner));
        let id = a.diagnostics().next().unwrap().origin().id();
        assert_eq!(id, b.diagnostics().next().unwrap().origin().id());
        assert_ne!(id, c.diagnostics().next().unwrap().origin().id());
        assert_eq!(logs.diagnostic_records(id, DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(id, DiagnosticPhase::Handled).is_empty());
    });
}

#[test]
fn explicit_release_reports_each_missing_resource_without_losing_target_identity() {
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    let owner = app.world_mut().spawn_empty().id();
    let target = OffscreenTarget::allocate(app.world_mut(), owner).unwrap();
    let image = target.image().id();
    app.world_mut().remove_resource::<Assets<Image>>();
    let failure = target.release(app.world_mut()).unwrap_err();
    assert!(
        matches!(kind(&failure), OffscreenError::MissingResource { owner: actual, target: Some(identity), .. } if *actual == owner && identity.image == image)
    );
}

#[test]
fn update_keeps_diagnostic_identity_and_discard_cleans_only_its_owned_resources() {
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let owner = app.world_mut().spawn(Node::default()).id();
    let scene = app.world_mut().spawn(Transform::default()).id();
    let camera = app.world_mut().spawn((Camera2d, ChildOf(scene))).id();
    let target = OffscreenTarget::allocate(app.world_mut(), owner).unwrap();
    let image = target.image().id();
    let bound = target.bind(app.world_mut(), owner, camera, scene).unwrap();
    app.world_mut().entity_mut(owner).insert(bound);
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(OffscreenViewport::update(app.world_mut(), owner).unwrap().is_none());
        assert!(logs.records().is_empty());
        app.world_mut().entity_mut(camera).remove::<Camera>();
        let a = OffscreenViewport::update(app.world_mut(), owner).unwrap_err();
        let b = OffscreenViewport::update(app.world_mut(), owner).unwrap_err();
        assert!(matches!(kind(&a), OffscreenError::MissingCamera { binding } if binding.owner == owner && binding.camera == camera && binding.target.image == image));
        assert_eq!(a.diagnostics().next().unwrap().origin().id(), b.diagnostics().next().unwrap().origin().id());
        app.world_mut().entity_mut(camera).insert(Camera::default());
        assert!(OffscreenViewport::update(app.world_mut(), owner).unwrap().is_none());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 1);
    });
    let errors = ErrorCapture::default();
    errors.run(|| {
        app.world_mut().despawn(owner);
        app.world_mut().flush();
    });
    assert!(errors.take_records().is_empty());
    assert!(app.world().get_entity(scene).is_err());
    assert!(app.world().resource::<Assets<Image>>().get(image).is_none());
}

#[test]
fn failed_binding_does_not_take_another_owners_scene_camera_or_viewport() {
    use bevy_widgetry_core::offscreen::OffscreenBindingRejection;
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    let world = app.world_mut();
    let (owner, scene, camera, image) = bound(world);
    let intruder = world.spawn_scene(bsn! { Node::default() }).unwrap().id();
    let target = OffscreenTarget::allocate(world, intruder).unwrap();
    let rejected_image = target.image().id();
    let error = target.bind(world, intruder, camera, scene).err().unwrap();
    assert!(matches!(
        kind(&error),
        OffscreenError::InvalidBinding {
            reason: OffscreenBindingRejection::SceneInUse,
            ..
        }
    ));
    assert!(world.get_entity(scene).is_ok());
    assert!(world.get_entity(camera).is_ok());
    assert_eq!(
        world.get::<ViewportNode>(owner).unwrap().camera,
        Some(camera)
    );
    assert!(world.resource::<Assets<Image>>().get(image).is_some());
    assert!(
        world
            .resource::<Assets<Image>>()
            .get(rejected_image)
            .is_none()
    );

    let fresh_scene = world.spawn(Transform::default()).id();
    let fresh_camera = world.spawn((Camera2d, ChildOf(fresh_scene))).id();
    let target = OffscreenTarget::allocate(world, intruder).unwrap();
    let error = target
        .bind(world, owner, fresh_camera, fresh_scene)
        .err()
        .unwrap();
    assert!(matches!(
        kind(&error),
        OffscreenError::InvalidBinding {
            reason: OffscreenBindingRejection::ViewportInUse,
            ..
        }
    ));
    assert!(world.get_entity(fresh_scene).is_err());
    assert_eq!(
        world.get::<ViewportNode>(owner).unwrap().camera,
        Some(camera)
    );
    OffscreenViewport::update(world, owner).unwrap();
}

#[test]
fn failed_binding_protects_nested_leased_scenes_and_unrelated_cameras() {
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    let world = app.world_mut();
    let (_, scene, _, _) = bound(world);
    let nested = world.spawn((Transform::default(), ChildOf(scene))).id();
    let camera = world.spawn((Camera2d, ChildOf(nested))).id();
    let owner = world.spawn_scene(bsn! { Node::default() }).unwrap().id();
    let target = OffscreenTarget::allocate(world, owner).unwrap();
    assert!(target.bind(world, owner, camera, nested).is_err());
    assert!(world.get_entity(nested).is_ok());
    assert!(world.get_entity(camera).is_ok());

    let dedicated = world.spawn(Transform::default()).id();
    let host_camera = world.spawn(Camera2d).id();
    let target = OffscreenTarget::allocate(world, owner).unwrap();
    assert!(target.bind(world, owner, host_camera, dedicated).is_err());
    assert!(world.get_entity(host_camera).is_ok());
    assert!(world.get_entity(dedicated).is_err());
}

#[test]
fn persistent_update_failure_changes_identity_only_when_reason_changes_or_recovers() {
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    let (owner, _, camera, image) = bound(app.world_mut());
    let world = app.world_mut();
    world.entity_mut(owner).remove::<ViewportNode>();
    let a = OffscreenViewport::update(world, owner).unwrap_err();
    let a_id = a.diagnostics().next().unwrap().origin().id();
    assert!(
        matches!(kind(&a), OffscreenError::AssociationBroken { binding, actual_camera: None } if binding.target.image == image)
    );
    let b = OffscreenViewport::update(world, owner).unwrap_err();
    assert_eq!(a_id, b.diagnostics().next().unwrap().origin().id());
    world
        .entity_mut(owner)
        .insert(ViewportNode::new(camera))
        .remove::<ComputedNode>();
    let c = OffscreenViewport::update(world, owner).unwrap_err();
    assert!(matches!(
        kind(&c),
        OffscreenError::MissingComputedNode { .. }
    ));
    assert_ne!(a_id, c.diagnostics().next().unwrap().origin().id());
    world.entity_mut(owner).insert(ComputedNode::default());
    assert!(OffscreenViewport::update(world, owner).unwrap().is_none());
    world.entity_mut(owner).remove::<ViewportNode>();
    let d = OffscreenViewport::update(world, owner).unwrap_err();
    assert_ne!(a_id, d.diagnostics().next().unwrap().origin().id());
}

#[test]
fn discard_cleanup_failure_reaches_real_command_boundary_after_other_cleanup() {
    use bevy::ecs::error::Severity;
    use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let (owner, scene, _, image) = bound(app.world_mut());
    app.world_mut().remove_resource::<Assets<Image>>();
    let errors = ErrorCapture::default();
    errors.run(|| {
        app.world_mut().despawn(owner);
        app.world_mut().flush();
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].error.severity(), Severity::Error);
    let managed = records[0].error.downcast_ref::<WidgetryEcsError>().unwrap();
    let diagnostic = managed.diagnostics().next().unwrap();
    assert!(
        diagnostic
            .contexts()
            .iter()
            .any(|frame| frame.operation() == "offscreen.discard")
    );
    let mut cause: &dyn Error = diagnostic;
    while !cause.is::<OffscreenError>() {
        cause = cause.source().unwrap();
    }
    assert!(
        matches!(cause.downcast_ref::<OffscreenError>().unwrap(), OffscreenError::MissingResource { target: Some(target), .. } if target.image == image)
    );
    assert!(app.world().get_entity(scene).is_err());
}

#[test]
fn repeated_create_destroy_recycles_only_offscreen_assets_without_error_logs() {
    let mut app = scene_app();
    app.add_plugins(WidgetryOffscreenPlugin);
    let world = app.world_mut();
    let host = world.resource_mut::<Assets<Image>>().add(Image::default());
    let logs = LogCapture::default();
    logs.run(|| {
        for _ in 0..30 {
            let (owner, scene, _, _) = bound(world);
            assert!(OffscreenViewport::update(world, owner).unwrap().is_none());
            world.despawn(owner);
            world.flush();
            assert!(world.get_entity(scene).is_err());
            assert_eq!(world.resource::<Assets<Image>>().len(), 1);
            assert!(world.resource::<Assets<Image>>().get(&host).is_some());
        }
    });
    assert!(logs.records().is_empty());
}
