//! Coverage Map：公开 Scene 的 hover/timing→popup、factory 重建、despawn、theme 和实际 Text/Icon UI 准备由本文件负责。
//! headless.rs 保留精确 timing、pointer reset 和 front hit；style.rs 保留私有 guard、shell 与诊断。
//! State：无候选/等待/显示/warm，anchor identity；stimuli 为 HoverMap、受控 Real time、despawn 与新增内容。
//! Guards：disabled ancestor 仍可显示；同 anchor 不重建；invariant 为唯一 popup、无孤儿 tree 与消费前 IGNORE。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::visibility::VisibilitySystems,
    ecs::entity::EntityHashMap,
    picking::{PickingSystems, backend::HitData, hover::HoverMap},
    prelude::*,
    time::TimeUpdateStrategy,
    ui::{ComputedStackIndex, InteractionDisabled, OverrideClip, UiSystems},
    ui_widgets::popover::Popover,
};
use bevy_widgetry_asset::{BuiltinFont, BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_core::{ThemeMode, WidgetryAppExt, z_index};
use bevy_widgetry_test_utils::{
    add_ui_plugins, advance_until, scene_app, spawn_ui_camera, switch_theme,
};
use bevy_widgetry_tooltip::{TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Resource, Default)]
struct HoverTarget(Option<Entity>);

#[derive(Component, Clone, Default)]
struct Body(usize);

fn inject_hover(target: Res<HoverTarget>, mut map: ResMut<HoverMap>) {
    map.clear();
    if let Some(entity) = target.0 {
        let mut hits = EntityHashMap::default();
        hits.insert(entity, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        map.insert(bevy::picking::pointer::PointerId::Mouse, hits);
    }
}

fn app() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((
        WidgetryAssetPlugin,
        WidgetryIconPlugin,
        WidgetryTooltipPlugin,
    ))
    .init_resource::<HoverTarget>()
    .configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    )
    .add_systems(
        PreUpdate,
        inject_hover
            .after(PickingSystems::Hover)
            .before(PickingSystems::PostHover),
    );
    spawn_ui_camera(&mut app, UVec2::splat(600), 1.0);
    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::ZERO);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    let warm = app.world_mut().spawn_scene(bsn! { @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(16))} } }).unwrap().id();
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "Tooltip font/icon 前置资源",
        |world| {
            world.resource::<Assets<Font>>().contains(&font)
                && world.get::<Children>(warm).is_some()
        },
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app
}

fn anchor_scene(calls: Arc<AtomicUsize>) -> impl Scene {
    bsn! {
        @WidgetryTooltip { @content: {TooltipContentFactory::new(move || {
            let generation = calls.fetch_add(1, Ordering::SeqCst) + 1;
            bsn_list![(Body(generation) Node Children [Text(format!("details {generation}")), @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(16))} }])]
        })} }
        Node { width: px(60), height: px(25), position_type: PositionType::Absolute, left: px(480), top: px(480) }
        Children [Text("label"), (Node Children [Text("nested label")])]
    }
}

fn advance(app: &mut App, millis: u64) {
    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::from_millis(millis));
    app.update();
}

fn popups(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<Popover>>()
        .iter(app.world())
        .collect()
}

fn descendants(app: &mut App, root: Entity) -> Vec<Entity> {
    app.world_mut()
        .query::<&Children>()
        .query(app.world())
        .iter_descendants(root)
        .collect()
}

#[test]
fn public_hover_lifecycle_rebuilds_content_and_cleans_tree() {
    let mut app = app();
    let calls = Arc::new(AtomicUsize::new(0));
    let a = app
        .world_mut()
        .spawn_scene(bsn! { anchor_scene(calls.clone()) InteractionDisabled })
        .unwrap()
        .id();
    let b = app
        .world_mut()
        .spawn_scene(anchor_scene(calls.clone()))
        .unwrap()
        .id();
    let label = app.world().get::<Children>(a).unwrap()[0];
    let nested = app.world().get::<Children>(a).unwrap()[1];
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(label);
    advance(&mut app, 0);
    advance(&mut app, 199);
    assert!(popups(&mut app).is_empty());
    advance(&mut app, 1);
    let popup = popups(&mut app);
    assert_eq!(popup.len(), 1);
    let popup = popup[0];
    assert_eq!(app.world().get::<ChildOf>(popup).unwrap().parent(), a);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let old = descendants(&mut app, popup);
    let body = old
        .iter()
        .find(|entity| app.world().get::<Body>(**entity).is_some())
        .unwrap();
    assert_eq!(app.world().get::<Body>(*body).unwrap().0, 1);
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(nested);
    advance(&mut app, 1);
    assert_eq!(popups(&mut app), vec![popup]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    app.world_mut().resource_mut::<HoverTarget>().0 = None;
    advance(&mut app, 0);
    assert!(popups(&mut app).is_empty());
    assert!(
        old.iter()
            .all(|entity| app.world().get_entity(*entity).is_err())
    );
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(b);
    advance(&mut app, 0);
    advance(&mut app, 50);
    assert_eq!(popups(&mut app).len(), 1);
    app.world_mut().despawn(b);
    advance(&mut app, 0);
    assert!(popups(&mut app).is_empty());
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(a);
    advance(&mut app, 0);
    advance(&mut app, 50);
    let popup = popups(&mut app)[0];
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(app.world().get_entity(old[0]).is_err());
    assert!(descendants(&mut app, popup).iter().any(|entity| {
        app.world()
            .get::<Body>(*entity)
            .is_some_and(|body| body.0 == 3)
    }));
    app.world_mut().despawn(a);
    advance(&mut app, 500);
    assert!(popups(&mut app).is_empty());
    let waiting = app
        .world_mut()
        .spawn_scene(anchor_scene(calls.clone()))
        .unwrap()
        .id();
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(waiting);
    advance(&mut app, 0);
    app.world_mut().despawn(waiting);
    advance(&mut app, 500);
    assert!(popups(&mut app).is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn popup_prepares_content_and_ignores_new_descendants_before_picking() {
    let mut app = app();
    let a = app
        .world_mut()
        .spawn_scene(anchor_scene(Arc::new(AtomicUsize::new(0))))
        .unwrap()
        .id();
    app.world_mut().resource_mut::<HoverTarget>().0 = Some(a);
    advance(&mut app, 0);
    advance(&mut app, 200);
    let popup = popups(&mut app)[0];
    assert!(app.world().get::<OverrideClip>(popup).is_some());
    assert_eq!(
        app.world().get::<GlobalZIndex>(popup).unwrap().0,
        z_index::TOOLTIP
    );
    let placement = app.world().get::<Popover>(popup).unwrap();
    assert_eq!(placement.positions.len(), 4);
    assert_eq!(placement.window_margin, 8.0);
    let content = descendants(&mut app, popup);
    assert_eq!(
        content
            .iter()
            .filter(|entity| app.world().get::<Text>(**entity).is_some())
            .count(),
        1
    );
    assert_eq!(
        content
            .iter()
            .filter(|entity| app.world().get::<ImageNode>(**entity).is_some())
            .count(),
        1
    );
    let popup_size = app.world().get::<ComputedNode>(popup).unwrap().size();
    assert!(popup_size.x > 0.0 && popup_size.y > 0.0);

    for entity in &content {
        assert!(
            app.world()
                .get::<InheritedVisibility>(*entity)
                .unwrap()
                .get()
        );
        assert!(
            app.world().get::<ComputedStackIndex>(*entity).unwrap().0
                > app.world().get::<ComputedStackIndex>(popup).unwrap().0
        );
        if let Some(text) = app.world().get::<Text>(*entity) {
            assert_eq!(text.0, "details 1");
            assert!(
                !app.world()
                    .get::<bevy::text::TextLayoutInfo>(*entity)
                    .unwrap()
                    .glyphs
                    .is_empty()
            );
            assert_eq!(
                app.world().get::<TextColor>(*entity).unwrap().0,
                ThemeMode::Dark.colors().foreground
            );
        }
        if let Some(image) = app.world().get::<ImageNode>(*entity) {
            assert!(
                app.world()
                    .resource::<Assets<Image>>()
                    .contains(&image.image)
            );
            assert_eq!(image.color, ThemeMode::Dark.colors().foreground);
        }
    }
    let added = app
        .world_mut()
        .spawn_scene(bsn! { Node Children [(Node Children [Text("late")])] })
        .unwrap()
        .id();
    app.world_mut().entity_mut(popup).add_child(added);
    app.add_systems(
        PreUpdate,
        (move |children: Query<&Children>, pickable: Query<&Pickable>| {
            assert_eq!(*pickable.get(popup).unwrap(), Pickable::IGNORE);
            for child in children.iter_descendants(popup) {
                assert_eq!(*pickable.get(child).unwrap(), Pickable::IGNORE);
            }
        })
        .in_set(PickingSystems::Backend),
    );
    advance(&mut app, 0);
    switch_theme(&mut app, ThemeMode::Light);
    advance(&mut app, 0);
    for entity in descendants(&mut app, popup) {
        if let Some(text) = app.world().get::<TextColor>(entity) {
            assert_eq!(text.0, ThemeMode::Light.colors().foreground);
        }
        if let Some(image) = app.world().get::<ImageNode>(entity) {
            assert_eq!(image.color, ThemeMode::Light.colors().foreground);
        }
    }
}
