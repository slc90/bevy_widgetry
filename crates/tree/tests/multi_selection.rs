#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_tree::*;

#[derive(Resource, Default)]
struct Events(Vec<WidgetryTreeEventKind>);

#[derive(Component)]
struct Label;

#[test]
fn consecutive_selection_and_expansion_apis_refresh_projection_within_one_world_tick() {
    for queued in [false, true] {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryTreePlugin);
        let root = app.world_mut().spawn_empty().id();
        let branch = app
            .world_mut()
            .spawn((WidgetryTreeNode, ChildOf(root)))
            .id();
        let leaf = app
            .world_mut()
            .spawn((WidgetryTreeNode, ChildOf(branch)))
            .id();
        let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
        let view = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
            .unwrap()
            .id();
        app.update();
        WidgetryTreeView::select_visible(app.world_mut(), view).unwrap();
        WidgetryTreeView::expand_all(app.world_mut(), view).unwrap();
        WidgetryTreeView::collapse_all(app.world_mut(), view).unwrap();
        let operation = move |world: &mut World| -> Result {
            WidgetryTreeView::select_visible(world, view)?;
            WidgetryTreeView::expand_all(world, view)?;
            WidgetryTreeView::select_visible(world, view)?;
            assert_eq!(
                WidgetryTreeView::selection(world, view)?.selected_ids(),
                &std::collections::HashSet::from([branch, leaf])
            );
            WidgetryTreeView::collapse_all(world, view)?;
            WidgetryTreeView::select(world, view, None)?;
            WidgetryTreeView::select_visible(world, view)?;
            assert_eq!(
                WidgetryTreeView::selection(world, view)?.selected_ids(),
                &std::collections::HashSet::from([branch])
            );
            Ok(())
        };
        if queued {
            app.world_mut().commands().queue(operation);
            app.world_mut().flush();
        } else {
            operation(app.world_mut()).unwrap();
        }
    }
}

fn click(app: &mut App, node: Entity, control: bool, shift: bool) {
    use bevy_widgetry_list_view::WidgetryListViewItem;
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem<Entity>)>()
        .iter(app.world())
        .find(|(_, row)| row.id == node)
        .unwrap()
        .0;
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if control {
        keys.press(KeyCode::ControlLeft);
    }
    if shift {
        keys.press(KeyCode::ShiftLeft);
    }
    app.world_mut()
        .trigger(bevy_widgetry_test_utils::primary_click(row));
    app.update();
}

#[test]
fn real_modifiers_use_visible_ranges_repair_hidden_anchor_and_union_control_a() {
    use bevy_widgetry_list_view::WidgetryListViewItem;
    use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<UiScale>();
    app.add_plugins(WidgetryTreePlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| bsn_list! { Node }))
        .unwrap();
    let root = app.world_mut().spawn_empty().id();
    let branch = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(root)))
        .id();
    let child1 = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(branch)))
        .id();
    let child2 = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(branch)))
        .id();
    let next = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(root)))
        .id();
    let disabled = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(root), InteractionDisabled))
        .id();
    let last = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(root)))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    let list = app.world().get::<Children>(view).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(400.0, 320.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    WidgetryTreeView::expand(app.world_mut(), view, branch).unwrap();
    app.update();
    click(&mut app, child1, false, false);
    click(&mut app, last, true, false);
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids()
            .len(),
        2
    );
    click(&mut app, last, true, false);
    click(&mut app, child1, false, false);
    click(&mut app, last, false, true);
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids(),
        &std::collections::HashSet::from([child1, child2, next, last])
    );
    assert!(
        !WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .contains(disabled)
    );
    WidgetryTreeView::collapse(app.world_mut(), view, branch).unwrap();
    app.update();
    click(&mut app, next, false, true);
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids(),
        &std::collections::HashSet::from([next])
    );
    WidgetryTreeView::select(app.world_mut(), view, Some(child1)).unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(list, bevy::input_focus::FocusCause::Navigated);
    bevy_widgetry_test_utils::press_key(&mut app, window, KeyCode::KeyA);
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids(),
        &std::collections::HashSet::from([branch, child1, next, last])
    );
    WidgetryTreeView::expand(app.world_mut(), view, branch).unwrap();
    app.update();
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem<Entity>)>()
        .iter(app.world())
        .find(|(_, row)| row.id == child1)
        .unwrap()
        .0;
    assert!(app.world().get::<bevy::ui::Selected>(row).is_some());
}

#[test]
fn visible_selection_unions_hidden_nodes_and_skips_disabled_without_affecting_other_views() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryTreePlugin);
    let root = app.world_mut().spawn_empty().id();
    let branch = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let hidden = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(branch)))
        .id();
    let disabled = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root), InteractionDisabled))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let a = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    let b = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    app.update();
    WidgetryTreeView::select(app.world_mut(), a, Some(hidden)).unwrap();
    WidgetryTreeView::expand(app.world_mut(), b, branch).unwrap();
    assert!(WidgetryTreeView::select_visible(app.world_mut(), a).unwrap());
    let selection = WidgetryTreeView::selection(app.world(), a).unwrap();
    assert_eq!(
        selection.selected_ids(),
        &std::collections::HashSet::from([branch, hidden])
    );
    assert!(!selection.contains(disabled));
    assert!(!WidgetryTreeView::select_visible(app.world_mut(), a).unwrap());
    assert!(
        WidgetryTreeView::selection(app.world(), b)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
    assert!(
        WidgetryTreeView::state(app.world(), b)
            .unwrap()
            .is_expanded(branch)
    );
    app.world_mut().despawn(hidden);
    app.update();
    assert_eq!(
        WidgetryTreeView::selection(app.world(), a)
            .unwrap()
            .selected_ids(),
        &std::collections::HashSet::from([branch])
    );
}

#[test]
fn batch_expansion_stops_at_unloaded_boundaries_and_keeps_shared_loading_and_selection() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryTreePlugin);
    app.init_resource::<Events>();
    app.add_observer(
        |event: On<WidgetryTreeEvent>,
         states: Query<&WidgetryTreeState>,
         mut events: ResMut<Events>| {
            if let WidgetryTreeEventKind::ExpansionChanged {
                expanded,
                collapsed,
            } = &event.kind
            {
                let state = states.get(event.entity).unwrap();
                assert!(expanded.iter().all(|node| state.is_expanded(*node)));
                assert!(collapsed.iter().all(|node| !state.is_expanded(*node)));
            }
            events.0.push(event.kind.clone());
        },
    );
    let root = app.world_mut().spawn_empty().id();
    let local = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let leaf = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(local)))
        .id();
    let unknown = app
        .world_mut()
        .spawn((
            WidgetryTreeNode,
            ChildOf(root),
            WidgetryTreeChildrenState::Unknown,
        ))
        .id();
    let loading = app
        .world_mut()
        .spawn((
            WidgetryTreeNode,
            ChildOf(root),
            WidgetryTreeChildrenState::Loading,
        ))
        .id();
    let unloaded_child = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(unknown)))
        .id();
    app.world_mut()
        .spawn((WidgetryTreeNode, ChildOf(unloaded_child)));
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } InteractionDisabled })
        .unwrap()
        .id();
    app.update();
    WidgetryTreeView::replace_selection(app.world_mut(), view, [leaf, unknown]).unwrap();
    app.world_mut().resource_mut::<Events>().0.clear();
    assert!(WidgetryTreeView::expand_all(app.world_mut(), view).unwrap());
    assert!(!WidgetryTreeView::expand_all(app.world_mut(), view).unwrap());
    let state = WidgetryTreeView::state(app.world(), view).unwrap();
    assert_eq!(
        state.expanded_entities(),
        &std::collections::HashSet::from([local])
    );
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(unknown),
        Some(&WidgetryTreeChildrenState::Unknown)
    );
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(loading),
        Some(&WidgetryTreeChildrenState::Loading)
    );
    assert!(WidgetryTreeView::collapse_all(app.world_mut(), view).unwrap());
    assert!(!WidgetryTreeView::collapse_all(app.world_mut(), view).unwrap());
    assert!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .contains(leaf)
    );
    assert_eq!(app.world().resource::<Events>().0.len(), 2);
    assert!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .all(|kind| matches!(kind, WidgetryTreeEventKind::ExpansionChanged { .. }))
    );
}

#[test]
fn ten_thousand_offscreen_nodes_can_be_selected_on_a_disabled_view_with_bounded_rows() {
    use bevy_widgetry_list_view::WidgetryListViewItem;
    use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.init_resource::<UiScale>()
        .add_plugins(WidgetryTreePlugin);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| bsn_list! { Node }))
        .unwrap();
    let root = app.world_mut().spawn_empty().id();
    let branch = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label, ChildOf(root)))
        .id();
    let nodes = (0..10_000)
        .map(|_| {
            app.world_mut()
                .spawn((WidgetryTreeNode, Label, ChildOf(branch)))
                .id()
        })
        .collect::<Vec<_>>();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } InteractionDisabled })
        .unwrap()
        .id();
    let list = app.world().get::<Children>(view).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(400.0, 96.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    WidgetryTreeView::expand_all(app.world_mut(), view).unwrap();
    app.update();
    assert!(WidgetryTreeView::select_visible(app.world_mut(), view).unwrap());
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids()
            .len(),
        10_001
    );
    assert!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .contains(nodes[9999])
    );
    app.update();
    assert!(
        app.world_mut()
            .query::<&WidgetryListViewItem<Entity>>()
            .iter(app.world())
            .count()
            <= 3
    );
    WidgetryTreeView::collapse_all(app.world_mut(), view).unwrap();
    app.update();
    assert!(!WidgetryTreeView::select_visible(app.world_mut(), view).unwrap());
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids()
            .len(),
        10_001
    );
}
