use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::{ComputedStackIndex, UiSystems};
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app};

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
