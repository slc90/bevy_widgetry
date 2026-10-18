#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_test_utils::scene_app;

#[derive(Resource, Default)]
struct Changes(Vec<(Vec<WidgetryListItemId>, Vec<WidgetryListItemId>)>);

#[test]
fn selection_api_commits_a_single_authority_before_collection_notifications() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<u32>()
        .unwrap()
        .init_resource::<Changes>();
    let mut model = WidgetryListModel::default();
    let a = model.push(10u32).unwrap();
    let b = model.push(20).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root=app.world_mut().spawn_scene(bsn! { @WidgetryListView::<u32> { @source:source, @renderer:{WidgetryListViewRenderer::new(|value:&u32| bsn_list!{Text({value.to_string()})})} } }).unwrap().id();
    app.add_observer(
        |event: On<WidgetryListSelectionChanged>,
         states: Query<&WidgetryListSelectionState>,
         mut changes: ResMut<Changes>| {
            assert_eq!(
                states.get(event.source).unwrap().selected_ids(),
                event.selected.as_ref()
            );
            changes.0.push((event.added.clone(), event.removed.clone()));
        },
    );
    assert!(WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, b).unwrap());
    assert!(!WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, b).unwrap());
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    assert!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .contains(b)
    );
    assert!(WidgetryListView::<u32>::select_index_in_world(app.world_mut(), root, 0).unwrap());
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(a)
    );
    assert!(WidgetryListView::<u32>::deselect_id_in_world(app.world_mut(), root, a).unwrap());
    assert!(!WidgetryListView::<u32>::clear_selection_in_world(app.world_mut(), root).unwrap());
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(vec![b], vec![]), (vec![a], vec![b]), (vec![], vec![a])]
    );
}

#[test]
fn original_and_display_indices_resolve_distinct_ids_and_queued_indices_resolve_at_execution() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<u32>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let a = model.push(10u32).unwrap();
    let b = model.push(20).unwrap();
    let c = model.push(30).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root=app.world_mut().spawn_scene(bsn! { @WidgetryListView::<u32> { @source:source, @renderer:{WidgetryListViewRenderer::new(|value:&u32| bsn_list!{Text({value.to_string()})})} } }).unwrap().id();
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(|_, _, ids| {
            ids.remove(1);
            ids.reverse();
        })),
    )
    .unwrap();
    WidgetryListView::<u32>::select_display_index_in_world(app.world_mut(), root, 0).unwrap();
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(c)
    );
    WidgetryListView::<u32>::select_index_in_world(app.world_mut(), root, 1).unwrap();
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(b)
    );
    app.update();
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(b)
    );
    WidgetryListView::<u32>::select_index(&mut app.world_mut().commands(), root, 0);
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .move_item(0, 1);
    app.world_mut().flush();
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(b)
    );
    assert!(WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, a).unwrap());
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .remove(1);
    let error = WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, a).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_id(),
        Some(a)
    );
    app.update();
    assert!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
}
