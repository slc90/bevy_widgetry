// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! State：visible/offscreen rows、Entity selection/active、focus、root disabled 与 shared source。
//! Stimuli：真实 pointer/keyboard/wheel、公开 model API、业务 Component mutation、view spawn/despawn。
//! Guard：disabled 限制用户输入；重复选择不重发 Selected；业务 state 独立于 row lifecycle。
//! Invariant：expander 不选择 row，disabled 不修改 model，ListView state 仅为 Entity selection projection。
//! GUI 验收由 05 Gallery/BRP 覆盖；本文件验证真实 BSN 和官方 Button/ListView observer 组合。

#![cfg(test)]

use bevy::app::Propagate;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition, Selected};
use bevy::ui_widgets::Button;
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::{ForegroundColor, ThemeMode};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewItem, WidgetryListViewState,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, press, press_key, primary_click, release, scene_app, switch_theme,
};
use bevy_widgetry_tree::{
    WidgetryTreeAppExt, WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeModel,
    WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeRenderer, WidgetryTreeView,
    WidgetryTreeVisibleItem,
};

/// 单一业务类型，不复制 node UI state。
#[derive(Component)]
struct Label(String);

/// 异构 Folder 业务 Component，由独立 factory 渲染。
#[derive(Component)]
struct Folder(String);

/// 异构 File 业务 Component，不使用 fallback。
#[derive(Component)]
struct File(String);

/// 捕获 source/node identity；程序投影与重复用户确认不得多发通知。
#[derive(Resource, Default)]
struct Events(Vec<(Entity, WidgetryTreeEventKind)>);

/// 创建实际 TreeView shell，用真实 viewport Component 给三行可见范围。
fn fixture() -> (App, Entity, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<UiScale>();
    app.add_plugins(WidgetryTreePlugin)
        .init_resource::<Events>();
    app.add_observer(|event: On<WidgetryTreeEvent>, mut events: ResMut<Events>| {
        events.0.push((event.entity, event.kind));
    });
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, label: &Label| {
        bsn_list![(Text({ label.0.clone() }))]
    }))
    .unwrap();
    let root = app.world_mut().spawn_empty().id();
    let a = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("a".into()), ChildOf(root)))
        .id();
    let b = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("b".into()), ChildOf(root)))
        .id();
    let c = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("c".into()), ChildOf(a)))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let group = app
        .world_mut()
        .spawn_scene(bsn! {
            TabGroup::default()
            Node
            Children [(@WidgetryTreeView { @source: source })]
        })
        .unwrap()
        .id();
    let view = app.world().get::<Children>(group).unwrap()[0];
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

/// 使用固定的公开组合关系定位内部 ListView，测试不读取私有 Tree state。
fn list(world: &World, view: Entity) -> Entity {
    world
        .get::<Children>(view)
        .unwrap()
        .iter()
        .find(|&child| {
            world
                .get::<WidgetryListView<WidgetryTreeVisibleItem>>(child)
                .is_some()
        })
        .unwrap()
}

/// 从 ListView 公开 shell 定位原生 ScrollPosition 所在 viewport。
fn view_viewport(world: &World, view: Entity) -> Entity {
    world
        .get::<Children>(list(world, view))
        .unwrap()
        .iter()
        .find(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())
        .unwrap()
}

/// 遍历完整 descendant tree，以验证销毁的是 UI ownership tree 而非业务 hierarchy。
fn subtree(world: &World, root: Entity) -> Vec<Entity> {
    let mut entities = vec![root];
    let mut index = 0;
    while index < entities.len() {
        if let Some(children) = world.get::<Children>(entities[index]) {
            entities.extend(children.iter());
        }
        index += 1;
    }
    entities
}

/// 将 row 的用户内容作为实际 click target，避免仅验证 wrapper observer。
fn text_entity(world: &World, row: Entity) -> Entity {
    subtree(world, row)
        .into_iter()
        .find(|&entity| world.get::<Text>(entity).is_some())
        .unwrap()
}

/// 通过公开 row identity 与 ancestor ownership 收集某个 view 的 physical projection。
fn rows(app: &mut App, view: Entity) -> Vec<(usize, Entity)> {
    let owned = subtree(app.world(), view);
    let mut rows = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .filter(|(entity, _)| owned.contains(entity))
        .map(|(entity, item)| (item.index, entity))
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.0);
    rows
}

/// 从 pointer event 的相同 location 发送 wheel，进入真正的 ScrollArea observer。
fn wheel(app: &mut App, target: Entity, y: f32) {
    let click = primary_click(target);
    app.world_mut().trigger(Pointer::new(
        click.pointer_id,
        click.pointer_location.clone(),
        Scroll {
            x: 0.0,
            y,
            unit: MouseScrollUnit::Pixel,
            hit: click.hit.clone(),
            phase: bevy::input::touch::TouchPhase::Moved,
        },
        target,
    ));
    app.world_mut().flush();
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
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(a)).unwrap());
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
        WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap();
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

/// 实际异构 renderer 按 Component dispatch；内容 mutation、type 切换和重复注册都在同帧刷新。
#[test]
fn heterogeneous_renderers_follow_component_mutation_and_registration() {
    let (mut app, _, _, a, b, _) = fixture();
    app.register_renderer::<Folder>(WidgetryTreeRenderer::new(|_, node: &Folder| {
        bsn_list![(Text({ format!("folder:{}", node.0) }))]
    }))
    .unwrap();
    app.register_renderer::<File>(WidgetryTreeRenderer::new(|_, node: &File| {
        bsn_list![(Text({ format!("file:{}", node.0) }))]
    }))
    .unwrap();
    app.world_mut()
        .entity_mut(a)
        .remove::<Label>()
        .insert(Folder("A".into()));
    app.world_mut()
        .entity_mut(b)
        .remove::<Label>()
        .insert(File("B".into()));
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "folder:A")
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "file:B")
    );
    let before = row(&mut app, 0);
    app.world_mut().get_mut::<Folder>(a).unwrap().0 = "A2".into();
    app.update();
    assert_eq!(row(&mut app, 0), before);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "folder:A2")
    );
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "folder:A")
    );
    app.register_renderer::<Folder>(WidgetryTreeRenderer::new(|_, node: &Folder| {
        bsn_list![(Text({ format!("new:{}", node.0) }))]
    }))
    .unwrap();
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "new:A2")
    );
    app.world_mut()
        .entity_mut(a)
        .remove::<Folder>()
        .insert(File("was-folder".into()));
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "file:was-folder")
    );
}

/// content pointer 真正取得 ListView focus；Arrow 只移动 active，Space/Enter 将 Entity selection 通知一次。
#[test]
fn keyboard_after_pointer_focus_selects_entities_and_obeys_disabled() {
    let (mut app, source, view, a, b, c) = fixture();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    app.update();
    let first = row(&mut app, 0);
    let content = text_entity(app.world(), first);
    press(&mut app, content);
    app.world_mut().trigger(primary_click(content));
    release(&mut app, content);
    app.update();
    let list = list(app.world(), view);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    app.world_mut().resource_mut::<Events>().0.clear();
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(a)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .active,
        app.world()
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .unwrap()
            .id(1)
    );
    press_key(&mut app, window, KeyCode::Space);
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(c)
    );
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![(source, WidgetryTreeEventKind::Selected(c))]
    );
    press_key(&mut app, window, KeyCode::End);
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(b)
    );
    app.world_mut().entity_mut(view).insert(InteractionDisabled);
    let state = *app.world().get::<WidgetryListViewState>(list).unwrap();
    press_key(&mut app, window, KeyCode::Home);
    press_key(&mut app, window, KeyCode::Space);
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(list).unwrap(),
        state
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
    app.world_mut()
        .entity_mut(view)
        .remove::<InteractionDisabled>();
    press_key(&mut app, window, KeyCode::Home);
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(a)
    );
    assert_eq!(
        app.world().resource::<Events>().0.last(),
        Some(&(source, WidgetryTreeEventKind::Selected(a)))
    );
}

/// 同 source 的 enabled/disabled view 同步展开和隐藏 selection，disabled 边界互不传播。
#[test]
fn shared_source_projects_selection_and_expansion_to_independent_views() {
    let (mut app, source, first, a, _, c) = fixture();
    let second = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTreeView { @source: source, @item_height: 48.0 }
            InteractionDisabled
        })
        .unwrap()
        .id();
    let second_viewport = view_viewport(app.world(), second);
    app.world_mut()
        .entity_mut(second_viewport)
        .insert(ComputedNode {
            size: Vec2::new(400.0, 144.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    app.update();
    for view in [first, second] {
        assert_eq!(rows(&mut app, view).len(), 3);
    }
    let target = rows(&mut app, first)[1].1;
    let content = text_entity(app.world(), target);
    app.world_mut().trigger(primary_click(content));
    app.update();
    let selected = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap()
        .id(1);
    for view in [first, second] {
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list(app.world(), view))
                .unwrap()
                .selected,
            selected
        );
        let selected_row = rows(&mut app, view)[1].1;
        assert!(app.world().get::<Selected>(selected_row).is_some());
        assert_eq!(
            app.world()
                .get::<InteractionDisabled>(list(app.world(), view))
                .is_some(),
            view == second
        );
    }
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            (source, WidgetryTreeEventKind::Expanded(a)),
            (source, WidgetryTreeEventKind::Selected(c))
        ]
    );
    let disabled_row = rows(&mut app, second)[0].1;
    let button = expander(app.world(), disabled_row);
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    let content = text_entity(app.world(), disabled_row);
    app.world_mut().trigger(primary_click(content));
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(c)
    );
    assert!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(a)
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, a).unwrap());
    app.update();
    for view in [first, second] {
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list(app.world(), view))
                .unwrap()
                .selected,
            None
        );
        assert_eq!(rows(&mut app, view).len(), 2);
    }
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(c)
    );
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    app.update();
    for view in [first, second] {
        let id = app
            .world()
            .get::<WidgetryListViewState>(list(app.world(), view))
            .unwrap()
            .selected
            .unwrap();
        assert_eq!(
            app.world()
                .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
                .unwrap()
                .get_by_id(id)
                .unwrap()
                .entity,
            c
        );
    }
    assert_eq!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .filter(|(_, kind)| matches!(kind, WidgetryTreeEventKind::Selected(_)))
            .count(),
        1
    );
}

/// 万级 hierarchy 仅实例化 viewport rows；滚动复用 overlap，offscreen mutation 与 UI 销毁/重建保留业务 state。
#[test]
fn virtual_rows_and_view_lifecycle_preserve_business_nodes() {
    let (mut app, source, view, a, b, c) = fixture();
    for index in 0..9999 {
        app.world_mut()
            .spawn((WidgetryTreeNode, Label(format!("file {index}")), ChildOf(a)));
    }
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    app.update();
    let viewport = view_viewport(app.world(), view);
    let before = rows(&mut app, view);
    assert_eq!(
        before.iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_items()
            .len(),
        10002
    );
    let old_subtree = subtree(app.world(), before[0].1);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 32.0;
    app.update();
    let after = rows(&mut app, view);
    assert_eq!(&before[1..], &after[..2]);
    assert!(
        old_subtree
            .iter()
            .all(|&entity| app.world().get_entity(entity).is_err())
    );
    let c_row = after[0].1;
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 3200.0;
    app.update();
    assert_eq!(
        rows(&mut app, view)
            .iter()
            .map(|row| row.0)
            .collect::<Vec<_>>(),
        vec![100, 101, 102]
    );
    assert!(app.world().get_entity(c_row).is_err());
    app.world_mut().get_mut::<Label>(c).unwrap().0 = "updated while offscreen".into();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(c)
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 0.0;
    app.update();
    let restored = rows(&mut app, view)[1].1;
    assert_eq!(
        app.world()
            .get::<Text>(text_entity(app.world(), restored))
            .unwrap()
            .0,
        "updated while offscreen"
    );
    assert!(app.world().get::<Selected>(restored).is_some());
    let owned = subtree(app.world(), view);
    app.world_mut().despawn(view);
    app.update();
    assert!(
        owned
            .iter()
            .all(|&entity| app.world().get_entity(entity).is_err())
    );
    for node in [source, a, b, c] {
        assert!(app.world().get_entity(node).is_ok());
    }
    let rebuilt = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    let viewport = view_viewport(app.world(), rebuilt);
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(400.0, 96.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    assert_eq!(rows(&mut app, rebuilt).len(), 3);
    let restored = rows(&mut app, rebuilt)[1].1;
    assert!(app.world().get::<Selected>(restored).is_some());
    assert_eq!(
        app.world()
            .get::<Text>(text_entity(app.world(), restored))
            .unwrap()
            .0,
        "updated while offscreen"
    );
    assert!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .all(|(_, event)| !matches!(event, WidgetryTreeEventKind::Selected(_)))
    );
}

/// root disabled 同帧清理 expander pressed 并关闭 wheel；恢复后 wheel 继续使用原生 ScrollPosition。
#[test]
fn disabled_clears_pressed_and_blocks_wheel_until_reenabled() {
    let (mut app, source, view, a, _, _) = fixture();
    for index in 0..10 {
        app.world_mut()
            .spawn((WidgetryTreeNode, Label(index.to_string()), ChildOf(a)));
    }
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    app.update();
    let viewport = view_viewport(app.world(), view);
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .content_size = Vec2::new(400.0, 416.0);
    let first = row(&mut app, 0);
    let button = expander(app.world(), first);
    press(&mut app, button);
    assert!(app.world().get::<Pressed>(button).is_some());
    app.world_mut().entity_mut(view).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(button).is_none());
    wheel(&mut app, first, -32.0);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    app.world_mut()
        .entity_mut(view)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    wheel(&mut app, first, -32.0);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        32.0
    );
    app.update();
    assert_eq!(rows(&mut app, view)[0].0, 1);
    assert!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .all(|(_, event)| !matches!(event, WidgetryTreeEventKind::Selected(_)))
    );
}
