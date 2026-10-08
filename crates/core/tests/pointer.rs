//! State：直接 / descendant hover、有效 / 失效 Pointer、不同 target 与 parent。
//! Stimuli：HoverMap 投影、reparent、despawn、Location 清空与 window 消失。
//! Invariants：所有有效身份共享官方 state，静止 frame 不产生 Changed。
//! 手工 HoverMap fixture 只覆盖投影，真实 backend 由 test_utils 的 pipeline 覆盖。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    ecs::entity::EntityHashMap,
    picking::{
        backend::HitData,
        hover::{DirectlyHovered, HoverMap, Hovered},
        pointer::*,
    },
    prelude::*,
    window::WindowRef,
};
use bevy_widgetry_core::pointer::WidgetryPointerPlugin;
use bevy_widgetry_test_utils::{pointer_ids, scene_app};

#[derive(Resource, Default)]
struct Changes(usize);

fn count_changes(
    changed: Query<(), Or<(Changed<Hovered>, Changed<DirectlyHovered>)>>,
    mut count: ResMut<Changes>,
) {
    count.0 += changed.iter().count();
}

#[test]
fn shared_hover_projects_live_hits_and_is_quiet_until_context_changes() {
    for id in pointer_ids() {
        let mut app = scene_app();
        if !app.is_plugin_added::<WidgetryPointerPlugin>() {
            app.add_plugins(WidgetryPointerPlugin);
        }
        app.init_resource::<HoverMap>()
            .init_resource::<Changes>()
            .add_systems(Update, count_changes);
        let window = app.world_mut().spawn(Window::default()).id();
        let location = Location {
            target: RenderTarget::Window(WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::ONE,
        };
        let pointer = app
            .world_mut()
            .spawn((id, PointerLocation::new(location.clone())))
            .id();
        let parent = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false)))
            .id();
        let other = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false)))
            .id();
        let child = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false), ChildOf(parent)))
            .id();
        let mut hits = EntityHashMap::default();
        hits.insert(child, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        app.world_mut().resource_mut::<HoverMap>().insert(id, hits);
        app.update();
        assert!(app.world().get::<Hovered>(parent).unwrap().0);
        assert!(!app.world().get::<DirectlyHovered>(parent).unwrap().0);
        assert!(app.world().get::<DirectlyHovered>(child).unwrap().0);
        let changes = app.world().resource::<Changes>().0;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, changes);
        app.world_mut().entity_mut(child).insert(ChildOf(other));
        app.update();
        assert!(!app.world().get::<Hovered>(parent).unwrap().0);
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = None;
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
        assert!(!app.world().get::<DirectlyHovered>(child).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = Some(location);
        app.update();
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut().despawn(window);
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = Some(Location {
            target: NormalizedRenderTarget::None {
                width: 100,
                height: 100,
            },
            position: Vec2::ZERO,
        });
        app.update();
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut().despawn(pointer);
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
    }
}

#[test]
fn concurrent_valid_hits_form_union_and_stale_entities_do_not_hover_ancestors() {
    let mut app = scene_app();
    if !app.is_plugin_added::<WidgetryPointerPlugin>() {
        app.add_plugins(WidgetryPointerPlugin);
    }
    app.init_resource::<HoverMap>();
    let root = app
        .world_mut()
        .spawn((Hovered(false), DirectlyHovered(false)))
        .id();
    let child = app.world_mut().spawn(ChildOf(root)).id();
    let pointers: Vec<_> = pointer_ids()
        .into_iter()
        .map(|id| {
            let pointer = app
                .world_mut()
                .spawn((
                    id,
                    PointerLocation::new(Location {
                        target: NormalizedRenderTarget::None {
                            width: 1,
                            height: 1,
                        },
                        position: Vec2::ZERO,
                    }),
                ))
                .id();
            let mut hits = EntityHashMap::default();
            hits.insert(child, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
            app.world_mut().resource_mut::<HoverMap>().insert(id, hits);
            pointer
        })
        .collect();
    app.update();
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    app.world_mut().despawn(pointers[0]);
    app.update();
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    app.world_mut().despawn(child);
    app.update();
    assert!(!app.world().get::<Hovered>(root).unwrap().0);
}
