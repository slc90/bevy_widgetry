//! State：viewport 未就绪/就绪、新建/重建 renderer subtree 与展开/收起 expander。
//! Stimuli：真实 UI layout、公开 expand/collapse、业务 Component mutation 与 theme 切换。
//! Invariant：同一次 update 内新 Text 完成 font、visibility、stack、measurement 与 layout 消费。
//! 仅准备字体/SVG 时允许条件等待，执行被测操作后不额外推进 frame。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::{ComputedStackIndex, UiSystems};
use bevy::ui_widgets::Button;
use bevy_widgetry_asset::{BuiltinFont, BuiltinIcon};
use bevy_widgetry_core::ThemeMode;
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::WidgetryListViewItem;
use bevy_widgetry_test_utils::{
    add_ui_plugins, advance_until, scene_app, spawn_ui_camera, switch_theme,
};
use bevy_widgetry_tree::{
    WidgetryTreeAppExt, WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeRenderer, WidgetryTreeView,
};
use std::time::Duration;

#[derive(Component)]
struct Label(String);

fn assert_text_ready(app: &mut App, font: &Handle<Font>, expected: &[&str]) {
    let texts = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .map(|(entity, text)| (entity, text.0.clone()))
        .collect::<Vec<_>>();
    assert_eq!(texts.len(), expected.len());
    for label in expected {
        assert!(texts.iter().any(|(_, text)| text == label));
    }
    for (entity, _) in texts {
        let world = app.world();
        assert_eq!(
            world.get::<TextFont>(entity).unwrap().font,
            bevy::text::FontSource::Handle(font.clone())
        );
        assert!(
            !world
                .get::<bevy::text::TextLayoutInfo>(entity)
                .unwrap()
                .glyphs
                .is_empty()
        );
        assert!(world.get::<InheritedVisibility>(entity).unwrap().get());
        let computed = world.get::<ComputedNode>(entity).unwrap();
        assert!(computed.size().x > 0.0 && computed.size().y > 0.0);
        assert_eq!(computed.inverse_scale_factor(), 0.5);
        let mut row = entity;
        while world.get::<WidgetryListViewItem>(row).is_none() {
            row = world.get::<ChildOf>(row).unwrap().parent();
        }
        assert!(
            world.get::<ComputedStackIndex>(entity).unwrap().0
                > world.get::<ComputedStackIndex>(row).unwrap().0
        );
    }
}

fn assert_expander_ready(world: &World, view: Entity, expected_background: Color) {
    let mut stack = vec![view];
    let mut buttons = 0;
    while let Some(entity) = stack.pop() {
        if world.get::<Button>(entity).is_some()
            && world.get::<InheritedVisibility>(entity).unwrap().get()
        {
            buttons += 1;
            assert_eq!(
                world.get::<BackgroundColor>(entity).unwrap().0,
                expected_background
            );
            let icon = world.get::<Children>(entity).unwrap()[0];
            let image = world.get::<Children>(icon).unwrap()[0];
            assert!(world.get::<InheritedVisibility>(image).unwrap().get());
            let image_node = world.get::<ImageNode>(image).unwrap();
            assert_eq!(
                world
                    .resource::<Assets<Image>>()
                    .get(&image_node.image)
                    .unwrap()
                    .size(),
                UVec2::splat(12)
            );
            assert!(world.get::<ComputedNode>(image).unwrap().size().x > 0.0);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    assert_eq!(buttons, 1);
}

#[test]
fn tree_renderer_content_and_expander_are_ready_in_the_generation_frame() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, label: &Label| bsn_list![(Text({label.0.clone()}) TextFont { font_size: FontSize::Px(18.0) })])).unwrap();
    app.configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    );
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    let mut warm = Vec::new();
    for icon in [BuiltinIcon::TreeExpand, BuiltinIcon::TreeCollapse] {
        warm.push(app.world_mut().spawn_scene(bsn! { @WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::splat(12))} } }).unwrap().id());
    }
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "Tree Font 与 expander SVG 前置就绪",
        |world| {
            world.resource::<Assets<Font>>().contains(&font)
                && warm.iter().all(|&entity| {
                    world.get::<Children>(entity).is_some_and(|children| {
                        world.get::<ImageNode>(children[0]).is_some_and(|image| {
                            world.resource::<Assets<Image>>().contains(&image.image)
                        })
                    })
                })
        },
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font.clone()));
    spawn_ui_camera(&mut app, UVec2::new(800, 600), 2.0);
    let root = app.world_mut().spawn_empty().id();
    let a = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("folder".into()), ChildOf(root)))
        .id();
    app.world_mut()
        .spawn((WidgetryTreeNode, Label("tail".into()), ChildOf(root)));
    let c = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("child".into()), ChildOf(a)))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryTreeView { @source: source } Node { width: px(240), height: px(128) } },
        )
        .unwrap()
        .id();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        0
    );
    app.update();
    assert_text_ready(&mut app, &font, &["folder", "tail"]);
    assert_expander_ready(
        app.world(),
        view,
        ThemeMode::Dark.colors().control_background,
    );
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    app.update();
    assert_text_ready(&mut app, &font, &["folder", "child", "tail"]);
    assert_expander_ready(
        app.world(),
        view,
        ThemeMode::Dark.colors().control_background,
    );
    let old_text = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(_, text)| text.0 == "child")
        .unwrap()
        .0;
    let old_width = app.world().get::<ComputedNode>(old_text).unwrap().size().x;
    app.world_mut().get_mut::<Label>(c).unwrap().0 = "longer child label".into();
    switch_theme(&mut app, ThemeMode::Light);
    app.update();
    assert!(app.world().get_entity(old_text).is_err());
    assert_text_ready(&mut app, &font, &["folder", "longer child label", "tail"]);
    let new_text = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(_, text)| text.0 == "longer child label")
        .unwrap()
        .0;
    assert!(app.world().get::<ComputedNode>(new_text).unwrap().size().x > old_width);
    assert_expander_ready(
        app.world(),
        view,
        ThemeMode::Light.colors().control_background,
    );
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, a).unwrap());
    app.update();
    assert_text_ready(&mut app, &font, &["folder", "tail"]);
    assert_expander_ready(
        app.world(),
        view,
        ThemeMode::Light.colors().control_background,
    );
}
