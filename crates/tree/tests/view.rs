//! State：visible rows、Entity selection 与 root disabled；stimuli：pointer row/expander、公开 model API。
//! Invariant：expander 不选择 row，disabled 不修改 model，ListView state 仅为 Entity selection projection。
//! GUI 验收由 05 Gallery/BRP 覆盖；本文件验证真实 BSN 和官方 Button/ListView observer 组合。

#![cfg(test)]

use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;
use bevy_widgetry_core::{ForegroundColor, ThemeMode};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewItem, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{press, primary_click, release, scene_app, switch_theme};
use bevy_widgetry_tree::{
    WidgetryTreeModel, WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeView,
    WidgetryTreeVisibleItem,
};

/// 创建实际 TreeView shell，用真实 viewport Component 给三行可见范围。
fn fixture() -> (App, Entity, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<UiScale>();
    app.add_plugins(WidgetryTreePlugin);
    let root = app.world_mut().spawn_empty().id();
    let a = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let b = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let c = app.world_mut().spawn((WidgetryTreeNode, ChildOf(a))).id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app.world_mut().spawn_scene(bsn! {
        @WidgetryTreeView { @source: source, @renderer: {WidgetryListViewRenderer::new(|_, item: &WidgetryTreeVisibleItem| bsn_list![(Text({format!("{:?}", item.entity)}))])} }
    }).unwrap().id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(400.0, 96.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    (app, source, view, a, b, c)
}

/// 定位真实 ListView row 的公开 index identity。
fn row(app: &mut App, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, item)| item.index == index)
        .unwrap()
        .0
}

/// 按 row hierarchy 查找真正的官方 Button，而非绕过 Tree observer 改 state。
fn expander(world: &World, row: Entity) -> Entity {
    let mut stack = vec![row];
    while let Some(entity) = stack.pop() {
        if world.get::<Button>(entity).is_some() {
            return entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    panic!("row 应持有 Button expander");
}

/// expand button 更新真实 datasource/row，click 不污染 selection，collapse 删除 descendant row。
#[test]
fn expander_uses_button_and_updates_real_list_rows() {
    let (mut app, source, view, a, _, c) = fixture();
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListView<WidgetryTreeVisibleItem>>>()
        .single(app.world())
        .unwrap();
    assert_eq!(app.world().get::<ChildOf>(list).unwrap().parent(), view);
    let first = row(&mut app, 0);
    let button = expander(app.world(), first);
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.world_mut().flush();
    release(&mut app, button);
    app.update();
    assert!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(a)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        None
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .unwrap()
            .get(1)
            .unwrap()
            .entity,
        c
    );
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        3
    );
    let first = row(&mut app, 0);
    let button = expander(app.world(), first);
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.world_mut().flush();
    release(&mut app, button);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        2
    );
}

/// row selection 与 model Entity 双向同步；disable 同帧抑制 row 与 Button，仍允许程序选择。
#[test]
fn selection_and_disabled_follow_tree_authority() {
    let (mut app, source, view, a, b, _) = fixture();
    let second = row(&mut app, 1);
    app.world_mut().trigger(primary_click(second));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(b)
    );
    app.world_mut().entity_mut(view).insert(InteractionDisabled);
    app.world_mut().flush();
    let first = row(&mut app, 0);
    app.world_mut().trigger(primary_click(first));
    let button = expander(app.world(), first);
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(b)
    );
    assert!(
        !app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(a)
    );
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(a)));
    app.update();
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListView<WidgetryTreeVisibleItem>>>()
        .single(app.world())
        .unwrap();
    let selected = app
        .world()
        .get::<WidgetryListViewState>(list)
        .unwrap()
        .selected
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .unwrap()
            .get_by_id(selected)
            .unwrap()
            .entity,
        a
    );
    app.world_mut()
        .entity_mut(view)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    let second = row(&mut app, 1);
    app.world_mut().trigger(primary_click(second));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(b)
    );
}

/// ListView 在 PostUpdate 生成的 expander 必须当帧取得完整 theme 配色，包括新创建的 disabled Button。
#[test]
fn dynamic_expanders_receive_theme_in_the_generation_frame() {
    for (mode, disabled) in [
        (ThemeMode::Dark, false),
        (ThemeMode::Light, false),
        (ThemeMode::Dark, true),
        (ThemeMode::Light, true),
    ] {
        let (mut app, source, view, a, _, _) = fixture();
        if mode == ThemeMode::Dark && !disabled {
            let first = row(&mut app, 0);
            let button = expander(app.world(), first);
            assert_eq!(
                app.world().get::<BackgroundColor>(button).unwrap().0,
                mode.colors().control_background
            );
        }
        switch_theme(&mut app, mode);
        if disabled {
            app.world_mut().entity_mut(view).insert(InteractionDisabled);
        }
        WidgetryTreeModel::expand(app.world_mut(), source, a);
        app.update();
        for button in app
            .world_mut()
            .query_filtered::<Entity, With<Button>>()
            .iter(app.world())
        {
            let colors = mode.colors();
            assert_eq!(
                app.world().get::<BackgroundColor>(button).unwrap().0,
                if disabled {
                    colors.control_background_disabled
                } else {
                    colors.control_background
                }
            );
            assert_eq!(
                *app.world().get::<BorderColor>(button).unwrap(),
                BorderColor::all(if disabled {
                    colors.control_border_disabled
                } else {
                    colors.control_border
                })
            );
            assert_eq!(
                app.world()
                    .get::<Propagate<ForegroundColor>>(button)
                    .unwrap()
                    .0
                    .0,
                if disabled {
                    colors.foreground_disabled
                } else {
                    colors.foreground
                }
            );
        }
    }
}
