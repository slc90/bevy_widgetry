//! State：viewport readiness/range、visible/offscreen 内容 revision 与 row lifecycle。
//! stimuli 为 scroll/resize/CRUD。
//! Invariant：无 overscan、重叠复用、identity/revision 驱动 subtree 重建。
//! 真实 Text/Icon 在生成帧完成 UI 消费准备。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui::{ComputedStackIndex, InteractionDisabled, UiSystems};
use bevy_widgetry_asset::{BuiltinFont, BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
    WidgetryListViewPlugin, WidgetryListViewRenderer,
};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, advance_until, scene_app, spawn_ui_camera,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn renderer_failure_preserves_ownership_and_recovers_on_retry() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    for value in ["first", "second", "third"] {
        model.push(value.to_owned()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let failing = Arc::new(AtomicBool::new(true));
    let factory_failure = failing.clone();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryListView::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(move |index, value: &String| {
                    let failure = factory_failure.clone();
                    let value = value.clone();
                    bsn_list!{template(move |_| {
                        if index == 1 && failure.load(Ordering::Relaxed) {
                            Err(BevyError::error("renderer rejected row"))
                        } else {
                            Ok(Text(value.clone()))
                        }
                    })}
                })},
            }
        })
        .unwrap()
        .id();
    let healthy = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
        }
    }).unwrap().id();
    let viewports = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    for viewport in viewports {
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::new(100.0, 96.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    // 先初始化 system state：Bevy 0.20 的 AssetChanged tracker 是 resource entity，
    // 应纳入 fixture，不能被当成本次 renderer 失败遗留的 subtree。
    app.world_mut()
        .schedule_scope(PostUpdate, |world, schedule| schedule.initialize(world))
        .unwrap();
    let before = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    for _ in 0..2 {
        errors.run(|| logs.run(|| app.update()));
        let added = app
            .world_mut()
            .query::<Entity>()
            .iter(app.world())
            .filter(|entity| !before.contains(entity))
            .collect::<Vec<_>>();
        assert!(
            added
                .iter()
                .all(|entity| std::iter::successors(Some(*entity), |entity| app
                    .world()
                    .get::<ChildOf>(*entity)
                    .map(ChildOf::parent))
                .any(|ancestor| ancestor == healthy)),
            "失败场景不能遗留任何未归属的 entity"
        );
        let rows = app
            .world_mut()
            .query::<(Entity, &WidgetryListViewItem, Option<&ChildOf>)>()
            .iter(app.world())
            .map(|(entity, _, parent)| (entity, parent.is_some()))
            .collect::<Vec<_>>();
        assert!(
            rows.iter().all(|(_, has_parent)| *has_parent),
            "失败不能留下无 parent 的 row"
        );
        assert_eq!(
            rows.iter()
                .filter(
                    |(entity, _)| std::iter::successors(Some(*entity), |entity| app
                        .world()
                        .get::<ChildOf>(*entity)
                        .map(ChildOf::parent))
                    .any(|ancestor| ancestor == healthy)
                )
                .count(),
            3
        );
    }
    let failures = errors.take();
    assert!(!failures.is_empty());
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        1,
        "持续失败只记录一次"
    );
    failing.store(false, Ordering::Relaxed);
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record
                .fields
                .get("message")
                .is_some_and(|message| message.contains("恢复")))
            .count(),
        1
    );
    let entities = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListViewItem>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(
        entities
            .iter()
            .filter(|&&entity| std::iter::successors(Some(entity), |entity| app
                .world()
                .get::<ChildOf>(*entity)
                .map(ChildOf::parent))
            .any(|ancestor| ancestor == root))
            .count(),
        3
    );
    assert_eq!(entities.len(), 6);
    app.world_mut().despawn(root);
    app.world_mut().despawn(healthy);
    let after = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    assert!(
        after.is_subset(&before),
        "恢复并销毁列表后不能遗留新 entity"
    );
}

fn fixture(
    len: usize,
) -> (
    App,
    Entity,
    Entity,
    Entity,
    Arc<Mutex<Vec<(usize, String)>>>,
) {
    fixture_in(scene_app(), len, bevy::text::FontSource::monospace())
}

fn fixture_in(
    mut app: App,
    len: usize,
    font: bevy::text::FontSource,
) -> (
    App,
    Entity,
    Entity,
    Entity,
    Arc<Mutex<Vec<(usize, String)>>>,
) {
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    for index in 0..len {
        model.push(index.to_string()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let history = calls.clone();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryListView::<String> {
                @source: source,
                @item_height: 10.0,
                @renderer: {WidgetryListViewRenderer::new(move |index, value: &String| {
                    history.lock().expect("测试记录锁应可用").push((index, value.clone()));
                    let font = font.clone();
                    bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText template(move |_| Ok(TextFont {font: font.clone(), font_size: FontSize::Px(12.0), ..default()}))}
                })},
            }
        })
        .expect("合法 fixture Scene 应成功展开")
        .id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .expect("fixture 应只有一个 viewport");
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 200.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    (app, source, root, viewport, calls)
}

fn rows(app: &mut App) -> Vec<(usize, Entity, Entity)> {
    let mut rows = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem, &Children)>()
        .iter(app.world())
        .map(|(entity, item, children)| (item.index, entity, children[0]))
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.0);
    rows
}

#[test]
fn large_list_reuses_index_overlap_without_overscan() {
    let (mut app, _, _, viewport, calls) = fixture(10_000);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1000.0;
    app.update();
    let before = rows(&mut app);
    assert_eq!(
        before.iter().map(|row| row.0).collect::<Vec<_>>(),
        (100..120).collect::<Vec<_>>()
    );
    assert_eq!(calls.lock().unwrap().len(), 20);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y += 10.0;
    app.update();
    let after = rows(&mut app);
    assert_eq!(&before[1..], &after[..19]);
    assert_eq!(after[19].0, 120);
    assert!(app.world().get_entity(before[0].1).is_err());
    assert_eq!(calls.lock().unwrap().len(), 21);
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    let children = app.world().get::<Children>(content).unwrap();
    assert_eq!(children.len(), 22);
    assert_eq!(
        &children[1..21],
        &after.iter().map(|row| row.1).collect::<Vec<_>>()
    );
}

#[test]
fn content_revisions_and_disabled_have_distinct_lifecycles() {
    let (mut app, source, _, viewport, calls) = fixture(100);
    app.update();
    let before = rows(&mut app);
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(5)
        .unwrap()
        .unwrap() = "updated".into();
    app.update();
    let after = rows(&mut app);
    assert_eq!(calls.lock().unwrap().len(), 21);
    for index in 0..20 {
        assert_eq!(before[index].1, after[index].1);
        assert_eq!(before[index].2 == after[index].2, index != 5);
    }
    assert!(app.world().get_entity(before[5].2).is_err());
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap();
        *model.get_mut(90).unwrap().unwrap() = "offscreen".into();
        model.set_disabled(5, true);
    }
    app.update();
    assert_eq!(calls.lock().unwrap().len(), 21);
    assert_eq!(after, rows(&mut app));
    assert!(app.world().get::<InteractionDisabled>(after[5].1).is_some());
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 800.0;
    app.update();
    assert!(calls.lock().unwrap().contains(&(90, "offscreen".into())));
    assert!(app.world().get_entity(after[5].1).is_err());
}

#[test]
fn resize_structural_changes_and_shrink_preserve_invariants() {
    let (mut app, source, _, viewport, calls) = fixture(100);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 200.0;
    app.update();
    let before = rows(&mut app);
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size
        .y = 210.0;
    app.update();
    let expanded = rows(&mut app);
    assert_eq!(&expanded[..20], &before);
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size
        .y = 190.0;
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 190.0;
    app.update();
    let backwards = rows(&mut app);
    assert_eq!(&backwards[1..], &before[..18]);
    let count = calls.lock().unwrap().len();
    let old_id = app
        .world()
        .get::<WidgetryListViewItem>(backwards[1].1)
        .unwrap()
        .id;
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(20, "inserted".into())
        .unwrap();
    app.update();
    let replaced = rows(&mut app);
    assert_eq!(replaced[1].1, backwards[1].1);
    assert_ne!(
        app.world()
            .get::<WidgetryListViewItem>(replaced[1].1)
            .unwrap()
            .id,
        old_id
    );
    assert_eq!(calls.lock().unwrap().len(), count + 18);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .move_item(20, 21);
    app.update();
    assert_eq!(calls.lock().unwrap().len(), count + 20);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 800.0;
    app.update();
    let far = rows(&mut app);
    assert!(
        replaced
            .iter()
            .all(|row| app.world().get_entity(row.1).is_err())
    );
    while app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .len()
        > 3
    {
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .remove(3);
    }
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    assert_eq!(
        rows(&mut app).iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(far.iter().all(|row| app.world().get_entity(row.1).is_err()));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.update();
    assert!(rows(&mut app).is_empty());
}

fn real_ui_app() -> (App, Handle<Font>) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin))
        .configure_sets(
            PostUpdate,
            (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
        );
    // 通过语义 asset 接口预加载，首次 row measurement 不依赖异步完成时机。
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    let warm = app.world_mut().spawn_scene(bsn! {
        @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(8))} }
    }).unwrap().id();
    advance_until(
        &mut app,
        Duration::from_secs(10),
        &format!("内建 Font {:?}", font.id()),
        |world| {
            world.resource::<Assets<Font>>().contains(&font)
                && world.get::<Children>(warm).is_some_and(|children| {
                    world.get::<ImageNode>(children[0]).is_some_and(|image| {
                        world.resource::<Assets<Image>>().contains(&image.image)
                    })
                })
        },
    )
    .expect("内建字体应在期限内加载");
    app.set_default_font(bevy::text::FontSource::Handle(font.clone()));
    // 保留预热 Icon 的 strong SVG handle，避免新 row 生成前 asset 因最后一个 handle 释放而卸载。
    (app, font)
}

#[test]
fn real_layout_bootstraps_visible_rows_and_full_content_height() {
    let (app, font) = real_ui_app();
    let (mut app, source, root, viewport, _) =
        fixture_in(app, 10_000, bevy::text::FontSource::default());
    app.world_mut()
        .entity_mut(viewport)
        .insert(ComputedNode::default());
    spawn_ui_camera(&mut app, UVec2::splat(400), 2.0);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(100);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(95);
    assert!(rows(&mut app).is_empty());
    app.update();
    assert!(rows(&mut app).is_empty());
    app.update();
    assert_eq!(rows(&mut app).len(), 10);
    for (_, row, text) in rows(&mut app) {
        assert!(app.world().get::<InheritedVisibility>(text).unwrap().get());
        assert!(
            app.world().get::<ComputedStackIndex>(text).unwrap().0
                > app.world().get::<ComputedStackIndex>(row).unwrap().0
        );
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font,
            bevy::text::FontSource::Handle(font.clone()),
            "初次生成的 Text 必须在本帧应用 fallback"
        );
        let computed = app.world().get::<ComputedNode>(row).unwrap();
        assert_eq!(computed.size().y, 20.0);
        assert_eq!(computed.inverse_scale_factor(), 0.5);
        assert!(app.world().get::<ComputedNode>(text).unwrap().size().x > 0.0);
    }
    let computed = app.world().get::<ComputedNode>(viewport).unwrap();
    assert_eq!(computed.size().y * computed.inverse_scale_factor(), 93.0);
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    let computed = app.world().get::<ComputedNode>(content).unwrap();
    assert_eq!(
        computed.size().y * computed.inverse_scale_factor(),
        100_000.0
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1001.0;
    app.update();
    let rendered = rows(&mut app);
    assert_eq!(rendered.first().unwrap().0, 100);
    assert_eq!(rendered.last().unwrap().0, 109);
    for (_, row, text) in rendered {
        assert!(app.world().get::<InheritedVisibility>(text).unwrap().get());
        assert!(
            app.world().get::<ComputedStackIndex>(text).unwrap().0
                > app.world().get::<ComputedStackIndex>(row).unwrap().0
        );
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font,
            bevy::text::FontSource::Handle(font.clone()),
            "滚入的新 Text 必须在本帧应用 fallback"
        );
        let computed = app.world().get::<ComputedNode>(row).unwrap();
        assert_eq!(computed.size().y * computed.inverse_scale_factor(), 10.0);
    }
    let children = app.world().get::<Children>(content).unwrap();
    assert_eq!(
        app.world().get::<Node>(children[0]).unwrap().height,
        px(1000)
    );
    assert_eq!(
        app.world()
            .get::<Node>(*children.last().unwrap())
            .unwrap()
            .height,
        px(98_900)
    );
    let before = rows(&mut app)[0];
    let width = app.world().get::<ComputedNode>(before.2).unwrap().size().x;
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(before.0)
        .unwrap()
        .unwrap();
    app.update();
    let rebuilt = rows(&mut app)[0];
    assert!(
        app.world()
            .get::<InheritedVisibility>(rebuilt.2)
            .unwrap()
            .get()
    );
    assert!(
        app.world().get::<ComputedStackIndex>(rebuilt.2).unwrap().0
            > app.world().get::<ComputedStackIndex>(rebuilt.1).unwrap().0
    );
    assert_eq!(rebuilt.1, before.1);
    assert_ne!(rebuilt.2, before.2);
    assert_eq!(
        app.world().get::<TextFont>(rebuilt.2).unwrap().font,
        bevy::text::FontSource::Handle(font)
    );
    assert_eq!(
        app.world().get::<ComputedNode>(rebuilt.2).unwrap().size().x,
        width
    );
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(rebuilt.2).unwrap().size().x,
        width
    );
}

#[test]
fn text_and_icon_renderer_materializes_in_the_generation_frame() {
    let (mut app, font) = real_ui_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    spawn_ui_camera(&mut app, UVec2::splat(400), 2.0);
    let mut model = WidgetryListModel::default();
    for index in 0..100 {
        model.push(format!("row {index}")).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 24.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list!{
                Text({value.clone()}) bevy_widgetry_core::text::WidgetryText TextFont {font_size: FontSize::Px(12.0)}--
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(8))} }
            })},
        }
        Node { width: px(100), height: px(95) }
    }).unwrap().id();
    let viewport = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    app.update();
    assert!(rows(&mut app).is_empty(), "bootstrap 尚无有效 viewport");
    app.update();
    assert_eq!(rows(&mut app).len(), 4);
    assert_text_icon_rows(&mut app, &font);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 240.0;
    app.update();
    assert_eq!(rows(&mut app)[0].0, 10);
    assert_text_icon_rows(&mut app, &font);
    let before = rows(&mut app)[0];
    let old_icon = app.world().get::<Children>(before.1).unwrap()[1];
    let old_image = app.world().get::<Children>(old_icon).unwrap()[0];
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(10)
        .unwrap()
        .unwrap() = "rebuilt".into();
    app.update();
    let rebuilt = rows(&mut app)[0];
    assert_eq!(rebuilt.1, before.1);
    assert_ne!(rebuilt.2, before.2);
    for old in [before.2, old_icon, old_image] {
        assert!(app.world().get_entity(old).is_err());
    }
    assert_eq!(app.world().get::<Text>(rebuilt.2).unwrap().0, "rebuilt");
    assert_text_icon_rows(&mut app, &font);
}

fn assert_text_icon_rows(app: &mut App, font: &Handle<Font>) {
    let rendered = rows(app);
    assert!(!rendered.is_empty());
    for (_, row, text) in rendered {
        let world = app.world();
        assert_eq!(
            world.get::<TextFont>(text).unwrap().font,
            bevy::text::FontSource::Handle(font.clone())
        );
        assert!(
            !world
                .get::<bevy::text::TextLayoutInfo>(text)
                .unwrap()
                .glyphs
                .is_empty()
        );
        let icon = world.get::<Children>(row).unwrap()[1];
        let image = world.get::<Children>(icon).unwrap()[0];
        let handle = &world.get::<ImageNode>(image).unwrap().image;
        assert!(world.resource::<Assets<Image>>().contains(handle));
        assert_eq!(
            world
                .resource::<Assets<Image>>()
                .get(handle)
                .unwrap()
                .size(),
            UVec2::splat(8)
        );
        for entity in [text, icon, image] {
            assert!(world.get::<InheritedVisibility>(entity).unwrap().get());
            assert!(
                world.get::<ComputedStackIndex>(entity).unwrap().0
                    > world.get::<ComputedStackIndex>(row).unwrap().0
            );
            let computed = world.get::<ComputedNode>(entity).unwrap();
            assert!(computed.size().x > 0.0 && computed.size().y > 0.0);
            assert_eq!(computed.inverse_scale_factor(), 0.5);
        }
    }
}
