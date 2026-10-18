#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::scene_app;

fn view(app: &mut App, source: Entity) -> Entity {
    app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @item_height: 10.0,
            @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list!{Text({value.clone()})})},
        }
    }).unwrap().id()
}

#[test]
fn projections_are_independent_and_follow_shared_model_changes() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let a = model.push("alpha".to_owned()).unwrap();
    let b = model.push("beta".to_owned()).unwrap();
    let c = model.push("amber".to_owned()).unwrap();
    let source = app.world_mut().spawn(model).id();
    let all = view(&mut app, source);
    let filtered = view(&mut app, source);
    WidgetryListView::<String>::set_projection_in_world(
        app.world_mut(),
        filtered,
        Some(WidgetryListProjectionPolicy::new(|world, source, ids| {
            let model = world.get::<WidgetryListModel<String>>(source).unwrap();
            ids.retain(|id| model.get_by_id(*id).unwrap().starts_with('a'));
            ids.reverse();
        })),
    )
    .unwrap();
    let viewports = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    for viewport in viewports {
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::new(100.0, 30.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    }
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(all)
            .unwrap()
            .ordered_ids(),
        &[a, b, c]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(filtered)
            .unwrap()
            .ordered_ids(),
        &[c, a]
    );
    let rows = app
        .world_mut()
        .query::<(&WidgetryListViewItem, &ChildOf)>()
        .iter(app.world())
        .filter(|(_, parent)| {
            std::iter::successors(Some(parent.parent()), |entity| {
                app.world().get::<ChildOf>(*entity).map(ChildOf::parent)
            })
            .any(|entity| entity == filtered)
        })
        .map(|(item, _)| (item.index, item.id))
        .collect::<Vec<_>>();
    assert!(rows.contains(&(0, c)) && rows.contains(&(1, a)));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(0);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(all)
            .unwrap()
            .ordered_ids(),
        &[b, c]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(filtered)
            .unwrap()
            .ordered_ids(),
        &[c]
    );
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
        .unwrap()
        .unwrap() = "apricot".to_owned();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(filtered)
            .unwrap()
            .ordered_ids(),
        &[c, b]
    );
}

#[derive(Component)]
struct ExternalRows(Vec<Entity>);
#[derive(Component)]
struct ExternalValue(u32);
struct ExternalSource;
impl WidgetryListDataSource<String, Entity> for ExternalSource {
    fn changed_at(world: &World, source: Entity) -> Option<bevy::ecs::change_detection::Tick> {
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<ExternalRows>()?
                .last_changed(),
        )
    }
    fn ids(world: &World, source: Entity) -> Option<Vec<Entity>> {
        Some(world.get::<ExternalRows>(source)?.0.clone())
    }
    fn row(world: &World, source: Entity, id: Entity) -> Option<WidgetryListData<'_, String>> {
        if !world.get::<ExternalRows>(source)?.0.contains(&id) {
            return None;
        }
        Some(WidgetryListData {
            value: WidgetryListValue::Owned(world.get::<ExternalValue>(id)?.0.to_string()),
            disabled: false,
            revision: None,
        })
    }
}

#[test]
fn entity_key_adapter_supports_owned_unversioned_rows_and_style() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_source::<String, Entity, ExternalSource>()
        .unwrap();
    let node = app.world_mut().spawn(ExternalValue(7)).id();
    let source = app.world_mut().spawn(ExternalRows(vec![node])).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String, Entity> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list!{Text({value.clone()})})},
        }
    }).unwrap().id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 64.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection<Entity>>(root)
            .unwrap()
            .ordered_ids(),
        &[node]
    );
    let (row, _) = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem<Entity>)>()
        .single(app.world())
        .unwrap();
    let old = app.world().get::<Children>(row).unwrap()[0];
    WidgetryListView::<String, Entity>::select_display_index(
        &mut app.world_mut().commands(),
        root,
        0,
    );
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<bevy::ui::Selected>(row).is_some());
    app.world_mut().get_mut::<ExternalValue>(node).unwrap().0 = 8;
    app.world_mut()
        .get_mut::<ExternalRows>(source)
        .unwrap()
        .set_changed();
    app.update();
    let child = app.world().get::<Children>(row).unwrap()[0];
    assert_ne!(old, child);
    assert_eq!(app.world().get::<Text>(child).unwrap().0, "8");
}

#[test]
fn filtering_preserves_selection_and_original_index_api_while_deletion_repairs_it() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let first = model.push("first".to_owned()).unwrap();
    let hidden = model.push("hidden".to_owned()).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = view(&mut app, source);
    WidgetryListView::<String>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(|_, _, ids| {
            ids.truncate(1);
        })),
    )
    .unwrap();
    WidgetryListView::<String>::select_index(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    let state = list_snapshot(app.world(), root);
    assert_eq!(state.selected, Some(hidden));
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(root)
            .unwrap()
            .ordered_ids(),
        &[first]
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(1);
    app.update();
    assert_eq!(
        app.world()
            .get::<bevy_widgetry_list_view::WidgetryListSelectionState>(root)
            .unwrap()
            .selected_id(),
        None
    );
}

#[test]
fn projection_changes_coalesce_and_invalid_or_duplicate_ids_never_create_rows() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let stale = model.push("retired".to_owned()).unwrap();
    model.remove(0);
    let live = model.push("live".to_owned()).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = view(&mut app, source);
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..3 {
        let calls = calls.clone();
        WidgetryListView::<String>::set_projection_in_world(
            app.world_mut(),
            root,
            Some(WidgetryListProjectionPolicy::new(move |_, _, ids| {
                calls.fetch_add(1, Ordering::Relaxed);
                ids.extend([live, stale]);
            })),
        )
        .unwrap();
    }
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(root)
            .unwrap()
            .ordered_ids(),
        &[live]
    );
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn unavailable_source_cannot_navigate_a_stale_projection_and_recovers_with_state() {
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy_widgetry_test_utils::{ErrorCapture, add_keyboard_dispatch, press_key};
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<bevy::ui::UiScale>();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    model.push("first".to_owned()).unwrap();
    model.push("second".to_owned()).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = view(&mut app, source);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    WidgetryListView::<String>::select_index(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    let before = list_snapshot(app.world(), root);
    let model = app
        .world_mut()
        .entity_mut(source)
        .take::<WidgetryListModel<String>>()
        .unwrap();
    let errors = ErrorCapture::default();
    errors.run(|| press_key(&mut app, window, KeyCode::Home));
    assert!(!errors.take().is_empty());
    assert_eq!(list_snapshot(app.world(), root), before);
    app.world_mut().entity_mut(source).insert(model);
    app.update();
    assert_eq!(list_snapshot(app.world(), root), before);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ListSnapshot {
    selected: Option<bevy_widgetry_list_view::WidgetryListItemId>,
    active: Option<bevy_widgetry_list_view::WidgetryListItemId>,
}
fn list_snapshot(world: &World, root: Entity) -> ListSnapshot {
    ListSnapshot {
        selected: world
            .get::<bevy_widgetry_list_view::WidgetryListSelectionState>(root)
            .unwrap()
            .selected_id(),
        active: world
            .get::<bevy_widgetry_list_view::WidgetryListViewState>(root)
            .unwrap()
            .active,
    }
}
