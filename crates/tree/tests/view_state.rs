#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_tree::*;

#[test]
fn shared_tree_views_keep_expansion_selection_and_lifetime_independent() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTreePlugin);
    let root = app.world_mut().spawn_empty().id();
    let branch = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let child = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(branch)))
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
    WidgetryTreeView::expand(app.world_mut(), a, branch).unwrap();
    WidgetryTreeView::select(app.world_mut(), a, Some(child)).unwrap();
    WidgetryTreeView::select(app.world_mut(), b, Some(branch)).unwrap();
    app.update();
    assert!(
        WidgetryTreeView::state(app.world(), a)
            .unwrap()
            .is_expanded(branch)
    );
    assert!(
        !WidgetryTreeView::state(app.world(), b)
            .unwrap()
            .is_expanded(branch)
    );
    assert_eq!(
        WidgetryTreeView::state(app.world(), a)
            .unwrap()
            .visible_items()
            .len(),
        2
    );
    assert_eq!(
        WidgetryTreeView::state(app.world(), b)
            .unwrap()
            .visible_items()
            .len(),
        1
    );
    WidgetryTreeView::collapse(app.world_mut(), a, branch).unwrap();
    app.update();
    assert!(
        WidgetryTreeView::selection(app.world(), a)
            .unwrap()
            .contains(child)
    );
    assert!(
        WidgetryTreeView::selection(app.world(), b)
            .unwrap()
            .contains(branch)
    );
    app.world_mut().despawn(child);
    app.update();
    assert!(
        WidgetryTreeView::selection(app.world(), a)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
    app.world_mut().despawn(a);
    app.update();
    assert!(app.world().get::<WidgetryTreeModel>(source).is_some());
    assert!(
        WidgetryTreeView::selection(app.world(), b)
            .unwrap()
            .contains(branch)
    );
}

#[derive(Resource, Default)]
struct Requests(Vec<(Entity, Entity)>);

#[test]
fn shared_lazy_loading_requests_once_and_projects_only_expanded_views() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTreePlugin)
        .init_resource::<Requests>();
    app.add_observer(
        |event: On<WidgetryTreeEvent>,
         states: Query<&WidgetryTreeState>,
         loading: Query<&WidgetryTreeChildrenState>,
         mut requests: ResMut<Requests>| {
            if let WidgetryTreeEventKind::ChildrenRequested(node) = event.kind {
                assert!(states.get(event.entity).unwrap().is_expanded(node));
                assert_eq!(
                    loading.get(node).unwrap(),
                    &WidgetryTreeChildrenState::Loading
                );
                requests.0.push((event.entity, node));
            }
        },
    );
    let root = app.world_mut().spawn_empty().id();
    let node = app
        .world_mut()
        .spawn((
            WidgetryTreeNode,
            ChildOf(root),
            WidgetryTreeChildrenState::Unknown,
        ))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let a = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    let b = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } InteractionDisabled })
        .unwrap()
        .id();
    app.update();
    WidgetryTreeView::expand(app.world_mut(), a, node).unwrap();
    WidgetryTreeView::queue_expand(&mut app.world_mut().commands(), b, node);
    app.world_mut().flush();
    assert!(
        WidgetryTreeView::state(app.world(), b)
            .unwrap()
            .is_expanded(node)
    );
    assert_eq!(app.world().resource::<Requests>().0, vec![(a, node)]);
    WidgetryTreeView::collapse(app.world_mut(), a, node).unwrap();
    let child = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(node)))
        .id();
    WidgetryTreeChildrenState::set_loaded(app.world_mut(), node).unwrap();
    app.update();
    assert_eq!(
        WidgetryTreeView::state(app.world(), a)
            .unwrap()
            .visible_index(child),
        None
    );
    assert_eq!(
        WidgetryTreeView::state(app.world(), b)
            .unwrap()
            .visible_index(child),
        Some(1)
    );
    assert!(WidgetryTreeView::select(app.world_mut(), b, Some(child)).unwrap());
    assert!(
        WidgetryTreeView::selection(app.world(), a)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
    assert!(
        WidgetryTreeView::selection(app.world(), b)
            .unwrap()
            .contains(child)
    );
    app.world_mut()
        .entity_mut(child)
        .remove::<WidgetryTreeNode>();
    assert!(WidgetryTreeView::select(app.world_mut(), b, Some(child)).is_err());
    app.update();
    assert!(
        WidgetryTreeView::selection(app.world(), b)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
    assert_eq!(app.world().resource::<Requests>().0, vec![(a, node)]);
}
