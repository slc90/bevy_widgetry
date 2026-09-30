//! core 基础 Coverage Map：ui_schedule 负责 Build / Materialize 的 deferred subtree 同帧准备；
//! default_font 负责显式字体、新增 TextFont fallback 与内建字体加载；
//! foreground_color 负责传播结果到 TextColor 的适配与动态文字 subtree；icon 负责图标 lifecycle。
//!
//! 本模块的 stimulus 是两个合法构造阶段的创建与重复替换，观察点为一次 update 后。
//! 前置字体必须已加载；新内容的 fallback、visibility、stack 与真实 measurement 必须当帧完成，旧 subtree 不得残留。

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::text::{FontSource, TextLayoutInfo};
use bevy::ui::{ComputedStackIndex, UiSystems};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, scene_app, spawn_ui_camera};
use std::time::Duration;

/// 每帧替换两种构造阶段的内容，验证 deferred Commands 也赶上渲染准备。
#[derive(Resource)]
struct ContentRoots([Entity; 2]);

/// 在 UI Prepare 前重建业务内容。
fn rebuild_model_content(mut commands: Commands, roots: Res<ContentRoots>) {
    commands.entity(roots.0[0]).despawn_children();
    commands
        .entity(roots.0[0])
        .apply_scene(bsn! { Children [(Node Visibility::Inherited)] });
}

/// 在 UI Prepare 后生成内容，模拟 Icon materialization 的时机。
fn materialize_content(mut commands: Commands, roots: Res<ContentRoots>) {
    commands.entity(roots.0[1]).despawn_children();
    commands
        .entity(roots.0[1])
        .apply_scene(bsn! { Children [(Node Visibility::Inherited)] });
}

/// 在 Prepare 前通过 deferred Commands 创建嵌套文字。
fn rebuild_text(mut commands: Commands, roots: Res<ContentRoots>) {
    commands.entity(roots.0[0]).despawn_children();
    commands.entity(roots.0[0]).apply_scene(bsn! {
        Children [(Node Visibility::Inherited Children [(Text("共享基础文字") Visibility::Inherited)])]
    });
}

/// 在 Prepare 后向已有 root materialize 嵌套文字，不创建新的 UI root。
fn materialize_text(mut commands: Commands, roots: Res<ContentRoots>) {
    commands.entity(roots.0[1]).despawn_children();
    commands.entity(roots.0[1]).apply_scene(bsn! {
        Children [(Node Visibility::Inherited Children [(Text("共享基础文字") Visibility::Inherited)])]
    });
}

/// 在刻意提前 visibility 和 stack 的 schedule 中，新建和替换内容首帧必须可见并位于 parent 之上。
#[test]
fn both_build_phases_prepare_replaced_content_in_same_frame() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    // 两个独立的 Bevy 消费阶段尽早运行，避免偶然排在构造之后掩盖缺失的约束。
    app.configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    );
    let first = app.world_mut().spawn_scene(bsn! { (Node) }).unwrap().id();
    let second = app.world_mut().spawn_scene(bsn! { (Node) }).unwrap().id();
    app.insert_resource(ContentRoots([first, second]))
        .add_systems(
            PostUpdate,
            (
                rebuild_model_content.in_set(WidgetryUiSystems::Build),
                materialize_content.in_set(WidgetryUiSystems::Materialize),
            ),
        );
    for _ in 0..3 {
        app.update();
        for root in [first, second] {
            let child = app.world().get::<Children>(root).unwrap()[0];
            assert!(app.world().get::<InheritedVisibility>(child).unwrap().get());
            assert!(
                app.world().get::<ComputedStackIndex>(child).unwrap().0
                    > app.world().get::<ComputedStackIndex>(root).unwrap().0
            );
        }
    }
}

/// 字体预加载后，在两种合法阶段重复替换嵌套文字，验证当帧 fallback、visibility、stack 与 measurement。
#[test]
fn both_build_phases_measure_deferred_text_in_same_frame() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    advance_until(
        &mut app,
        Duration::from_secs(10),
        &format!("调度测试 Font {:?}", font.id()),
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .expect("调度断言之前字体必须已加载");
    app.set_default_font(FontSource::Handle(font.clone()));
    spawn_ui_camera(&mut app, UVec2::splat(400), 2.0);
    app.configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    );
    let roots = [
        app.world_mut().spawn(Node::default()).id(),
        app.world_mut().spawn(Node::default()).id(),
    ];
    // Materialize 的 contract 要求已有 tree，先完成 root 的 Prepare。
    app.update();
    app.insert_resource(ContentRoots(roots)).add_systems(
        PostUpdate,
        (
            rebuild_text.in_set(WidgetryUiSystems::Build),
            materialize_text.in_set(WidgetryUiSystems::Materialize),
        ),
    );
    let mut previous = Vec::new();
    let mut measurements = Vec::new();
    for _ in 0..3 {
        app.update();
        for entity in previous.drain(..) {
            assert!(
                app.world().get_entity(entity).is_err(),
                "旧 subtree 不得残留"
            );
        }
        let mut current_measurements = Vec::new();
        for root in roots {
            let branch = app.world().get::<Children>(root).unwrap()[0];
            let text = app.world().get::<Children>(branch).unwrap()[0];
            assert_eq!(app.world().get::<ChildOf>(branch).unwrap().parent(), root);
            assert_eq!(app.world().get::<ChildOf>(text).unwrap().parent(), branch);
            assert_eq!(
                app.world().get::<TextFont>(text).unwrap().font,
                FontSource::Handle(font.clone())
            );
            for entity in [branch, text] {
                assert!(
                    app.world()
                        .get::<InheritedVisibility>(entity)
                        .unwrap()
                        .get()
                );
            }
            assert!(
                app.world().get::<ComputedStackIndex>(text).unwrap().0
                    > app.world().get::<ComputedStackIndex>(branch).unwrap().0
            );
            assert!(
                app.world().get::<ComputedStackIndex>(branch).unwrap().0
                    > app.world().get::<ComputedStackIndex>(root).unwrap().0
            );
            let layout = app.world().get::<TextLayoutInfo>(text).unwrap();
            assert!(
                !layout.glyphs.is_empty(),
                "真实字体应在创建当帧完成 glyph measurement"
            );
            assert!(layout.size.x > 0.0 && layout.size.y > 0.0);
            assert!(app.world().get::<ComputedNode>(text).unwrap().size().x > 0.0);
            current_measurements.push(layout.size);
            previous.extend([branch, text]);
        }
        if !measurements.is_empty() {
            assert_eq!(current_measurements, measurements);
        }
        measurements = current_measurements;
    }
}
