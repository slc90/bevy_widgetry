//! State 维度为祖先 disabled、本地请求和 hierarchy membership。
//! Insert/Remove、flush、Commands、late plugin 与 reparent 驱动 state transitions。
//! 有效值始终为 ancestry 与本地原因的 OR，官方输入投影一致。
//! 继承期间更新本地请求不会被投影覆盖，脱离祖先后恢复自己的 state。

// 测试通过断言验证 contract，仅在本文件允许测试所需的 panic lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::disabled::{WidgetryEffectiveDisabled, set_intrinsic_disabled};
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_test_utils::scene_app;

fn assert_disabled(world: &World, entity: Entity, disabled: bool) {
    assert_eq!(world.get::<InteractionDisabled>(entity).is_some(), disabled);
    assert_eq!(
        world
            .get::<WidgetryEffectiveDisabled>(entity)
            .unwrap()
            .is_disabled(),
        disabled
    );
}

#[test]
fn ancestor_request_projects_through_layout_after_flush_and_restores_child() {
    let mut app = scene_app();
    if !app.is_plugin_added::<WidgetryUiPlugin>() {
        app.add_plugins(WidgetryUiPlugin);
    }
    let root = app.world_mut().spawn(Node::default()).id();
    let bridge = app.world_mut().spawn(ChildOf(root)).id();
    let child = app
        .world_mut()
        .spawn((Node::default(), ChildOf(bridge)))
        .id();
    app.world_mut().flush();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert_disabled(app.world(), root, true);
    assert_disabled(app.world(), child, true);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), child, false);
}

#[test]
fn insert_and_remove_during_inheritance_preserve_local_intent() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let child = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(child)
        .insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), child, true);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(child)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), child, true);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), child, false);
}

#[test]
fn queued_remove_insert_and_nested_ancestors_use_final_requests() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let child = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled, ChildOf(root)))
        .id();
    let leaf = app
        .world_mut()
        .spawn((Node::default(), ChildOf(child)))
        .id();
    app.world_mut().flush();
    app.world_mut()
        .commands()
        .entity(child)
        .remove::<InteractionDisabled>()
        .insert(InteractionDisabled);
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), leaf, true);
    app.world_mut()
        .entity_mut(child)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), leaf, false);
}

#[test]
fn late_plugin_new_child_reparent_detach_and_despawn_follow_ancestry() {
    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let bridge = app.world_mut().spawn(ChildOf(root)).id();
    let child = app
        .world_mut()
        .spawn((Node::default(), ChildOf(bridge)))
        .id();
    app.add_plugins(WidgetryUiPlugin);
    assert_disabled(app.world(), child, true);
    let added = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    app.world_mut().flush();
    assert_disabled(app.world(), added, true);
    let enabled = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(bridge).insert(ChildOf(enabled));
    app.world_mut().flush();
    assert_disabled(app.world(), child, false);
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.world_mut().flush();
    assert_disabled(app.world(), child, true);
    app.world_mut().entity_mut(child).remove::<ChildOf>();
    app.world_mut().flush();
    assert_disabled(app.world(), child, false);
    app.world_mut().commands().entity(root).despawn();
    app.world_mut().flush();
    assert!(!app.world().entities().contains(root));
    assert!(!app.world().entities().contains(added));
    assert_disabled(app.world(), child, false);
}

#[derive(Resource, Default)]
struct Writes(usize);

fn count_changes(
    changed: Query<(), Changed<WidgetryEffectiveDisabled>>,
    mut writes: ResMut<Writes>,
) {
    writes.0 += changed.iter().count();
}

#[test]
fn stable_frames_do_not_rewrite_disabled_outputs() {
    let mut app = scene_app();
    app.init_resource::<Writes>()
        .add_systems(Update, count_changes);
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let child = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    app.update();
    let writes = app.world().resource::<Writes>().0;
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(writes, app.world().resource::<Writes>().0);
    assert_disabled(app.world(), child, true);
}

#[test]
fn data_entities_do_not_receive_ui_projection() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let model = app
        .world_mut()
        .spawn((Name::new("offscreen model"), ChildOf(root)))
        .id();
    app.world_mut().flush();
    assert!(app.world().get::<InteractionDisabled>(model).is_none());
    assert!(
        app.world()
            .get::<WidgetryEffectiveDisabled>(model)
            .is_none()
    );
}

#[test]
fn intrinsic_and_explicit_requests_do_not_clear_each_other() {
    let mut app = scene_app();
    let entity = app.world_mut().spawn(Node::default()).id();
    set_intrinsic_disabled(app.world_mut(), entity, true);
    assert_disabled(app.world(), entity, true);
    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), entity, true);
    set_intrinsic_disabled(app.world_mut(), entity, false);
    assert_disabled(app.world(), entity, false);
    set_intrinsic_disabled(app.world_mut(), entity, true);
    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);
    set_intrinsic_disabled(app.world_mut(), entity, false);
    assert_disabled(app.world(), entity, true);
}

#[derive(Resource)]
struct Cascade {
    trigger: Entity,
    receiver: Entity,
}

fn request_other(
    event: On<Insert<InteractionDisabled>>,
    cascade: Res<Cascade>,
    mut commands: Commands,
) {
    if event.entity == cascade.trigger {
        commands
            .entity(cascade.receiver)
            .insert(InteractionDisabled);
    }
}

#[test]
fn projection_guard_preserves_requests_for_other_entities_in_callback_chain() {
    let mut app = scene_app();
    let root = app.world_mut().spawn(Node::default()).id();
    let trigger = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    let receiver = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    app.world_mut().flush();
    app.insert_resource(Cascade { trigger, receiver })
        .add_observer(request_other);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_disabled(app.world(), trigger, false);
    assert_disabled(app.world(), receiver, true);
}

#[test]
fn materialized_children_are_projected_before_content_consumers() {
    let mut app = scene_app();
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    app.add_systems(
        PostUpdate,
        (move |mut commands: Commands| {
            commands.spawn((Node::default(), ChildOf(root)));
        })
        .in_set(WidgetryUiSystems::Materialize),
    );
    app.add_systems(
        PostUpdate,
        (move |children: Query<&Children>,
               states: Query<(&WidgetryEffectiveDisabled, Has<InteractionDisabled>)>| {
            for child in children.get(root).unwrap().iter() {
                let (state, official) = states.get(child).unwrap();
                assert!(state.is_disabled() && official);
            }
        })
        .after(WidgetryUiSystems::Disabled)
        .before(bevy::ui::UiSystems::Content),
    );
    app.update();
}

#[test]
fn lifecycle_consumers_can_despawn_the_projected_entity() {
    for initial in [false, true] {
        let mut app = scene_app();
        let root = app.world_mut().spawn(Node::default()).id();
        let child = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
        if initial {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        }
        app.world_mut().flush();
        app.world_mut().entity_mut(child).observe(
            |event: On<Insert<WidgetryEffectiveDisabled>>, mut commands: Commands| {
                commands.entity(event.entity).despawn();
            },
        );
        if initial {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        } else {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        }
        app.world_mut().flush();
        assert!(!app.world().entities().contains(child));
        assert_disabled(app.world(), root, !initial);
    }
}

#[test]
fn wheel_marker_removal_consumers_can_despawn_the_projected_entity() {
    use bevy::ui_widgets::ScrollArea;
    let mut app = scene_app();
    let root = app.world_mut().spawn(Node::default()).id();
    let child = app
        .world_mut()
        .spawn((Node::default(), ScrollArea, ChildOf(root)))
        .id();
    app.world_mut().flush();
    app.world_mut().entity_mut(child).observe(
        |event: On<Remove<ScrollArea>>, mut commands: Commands| {
            commands.entity(event.entity).despawn();
        },
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(!app.world().entities().contains(child));
    assert_disabled(app.world(), root, true);
}
