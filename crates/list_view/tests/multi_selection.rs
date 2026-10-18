#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{primary_click, scene_app};
use std::collections::HashSet;

fn fixture(
    count: usize,
    mode: WidgetryListSelectionMode,
) -> (App, Entity, Entity, Vec<WidgetryListItemId>) {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<u32>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let ids = (0..count).map(|i| model.push(i as u32).unwrap()).collect();
    let source = app.world_mut().spawn(model).id();
    let root=app.world_mut().spawn_scene(bsn! { @WidgetryListView::<u32> { @source:source,@selection_mode:mode, @item_height:10.0, @renderer:{WidgetryListViewRenderer::new(|value:&u32| bsn_list!{Text({value.to_string()})})} } }).unwrap().id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 40.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    (app, source, root, ids)
}
fn click(app: &mut App, id: WidgetryListItemId, ctrl: bool, shift: bool) {
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, row)| row.id == id)
        .unwrap()
        .0;
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if ctrl {
        keys.press(KeyCode::ControlLeft);
    }
    if shift {
        keys.press(KeyCode::ShiftLeft);
    }
    app.world_mut().trigger(primary_click(row));
    app.update();
}
fn selected(app: &App, root: Entity) -> HashSet<WidgetryListItemId> {
    WidgetryListView::<u32>::selection(app.world(), root)
        .unwrap()
        .selected_ids()
        .clone()
}
#[test]
fn control_toggle_shift_range_and_plain_click_share_one_selection_engine() {
    let (mut app, source, root, ids) = fixture(8, WidgetryListSelectionMode::Multiple);
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .set_disabled(2, true);
    app.update();
    click(&mut app, ids[0], false, false);
    click(&mut app, ids[3], true, false);
    assert_eq!(selected(&app, root), HashSet::from([ids[0], ids[3]]));
    click(&mut app, ids[1], false, true);
    assert_eq!(selected(&app, root), HashSet::from([ids[1], ids[3]]));
    click(&mut app, ids[1], true, false);
    assert_eq!(selected(&app, root), HashSet::from([ids[3]]));
    click(&mut app, ids[0], false, false);
    assert_eq!(selected(&app, root), HashSet::from([ids[0]]));
}
#[test]
fn programmatic_sets_validate_atomically_and_single_mode_rejects_multiple_ids() {
    let (mut app, _, root, ids) = fixture(4, WidgetryListSelectionMode::Single);
    WidgetryListView::<u32>::replace_selection_in_world(app.world_mut(), root, [ids[1]]).unwrap();
    assert!(
        WidgetryListView::<u32>::replace_selection_in_world(
            app.world_mut(),
            root,
            [ids[0], ids[2]]
        )
        .is_err()
    );
    assert_eq!(selected(&app, root), HashSet::from([ids[1]]));
    click(&mut app, ids[3], true, false);
    assert_eq!(selected(&app, root), HashSet::from([ids[3]]));
}

#[derive(Resource, Default)]
struct Notifications(usize);
fn control_a(app: &mut App, root: Entity) {
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy_widgetry_test_utils::press_key;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    press_key(app, Entity::PLACEHOLDER, KeyCode::KeyA);
}
#[test]
fn control_a_selects_the_whole_source_once_and_keeps_virtual_rows_bounded() {
    let (mut app, source, root, ids) = fixture(10_000, WidgetryListSelectionMode::Multiple);
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.init_resource::<bevy::ui::UiScale>()
        .init_resource::<Notifications>();
    app.add_observer(
        |_: On<WidgetryListSelectionChanged>, mut count: ResMut<Notifications>| {
            count.0 += 1;
        },
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .set_disabled(9999, true);
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(|_, _, ids| {
            ids.truncate(3);
        })),
    )
    .unwrap();
    app.update();
    control_a(&mut app, root);
    let selection = WidgetryListView::<u32>::selection(app.world(), root).unwrap();
    assert_eq!(selection.selected_ids().len(), 9999);
    assert!(selection.contains(ids[9998]));
    assert!(!selection.contains(ids[9999]));
    assert_eq!(app.world().resource::<Notifications>().0, 1);
    control_a(&mut app, root);
    assert_eq!(app.world().resource::<Notifications>().0, 1);
    assert!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count()
            <= 4
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .remove(0);
    app.update();
    assert!(!selected(&app, root).contains(&ids[0]));
    assert_eq!(app.world().resource::<Notifications>().0, 1);
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    WidgetryListView::<u32>::clear_selection_in_world(app.world_mut(), root).unwrap();
    control_a(&mut app, root);
    assert!(selected(&app, root).is_empty());
    WidgetryListView::<u32>::replace_selection_in_world(app.world_mut(), root, [ids[1], ids[2]])
        .unwrap();
    assert_eq!(selected(&app, root).len(), 2);
}
#[test]
fn shift_follows_projection_order_skips_disabled_and_repairs_a_hidden_anchor() {
    let (mut app, source, root, ids) = fixture(8, WidgetryListSelectionMode::Multiple);
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .set_disabled(2, true);
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(|world, source, ids| {
            let model = world.get::<WidgetryListModel<u32>>(source).unwrap();
            ids.retain(|id| model.get_by_id(*id).unwrap() % 2 == 0);
            ids.reverse();
        })),
    )
    .unwrap();
    app.update();
    click(&mut app, ids[6], false, false);
    click(&mut app, ids[0], false, true);
    assert_eq!(
        selected(&app, root),
        HashSet::from([ids[6], ids[4], ids[0]])
    );
    let visible = [ids[0], ids[4]];
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(move |_, _, ids| {
            ids.retain(|id| visible.contains(id));
            ids.reverse();
        })),
    )
    .unwrap();
    app.update();
    assert!(selected(&app, root).contains(&ids[6]));
    click(&mut app, ids[0], false, true);
    assert_eq!(selected(&app, root), HashSet::from([ids[0]]));
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .anchor(),
        Some(ids[0])
    );
}
#[test]
fn multi_selection_and_anchors_are_view_local_for_shared_sources() {
    let (mut app, _, first, ids) = fixture(8, WidgetryListSelectionMode::Multiple);
    let source = app
        .world()
        .get::<WidgetryListView<u32>>(first)
        .unwrap()
        .source();
    let second=app.world_mut().spawn_scene(bsn! { @WidgetryListView::<u32> { @source:source,@item_height:10.0, @projection:{Some(WidgetryListProjectionPolicy::new(|_,_,ids|ids.reverse()))}, @renderer:{WidgetryListViewRenderer::new(|value:&u32|bsn_list!{Text({value.to_string()})})} } }).unwrap().id();
    let viewports = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    for viewport in viewports {
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::new(100.0, 40.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    }
    app.update();
    click(&mut app, ids[0], false, false);
    click(&mut app, ids[2], true, false);
    click(&mut app, ids[7], false, false);
    click(&mut app, ids[5], false, true);
    assert_eq!(selected(&app, first), HashSet::from([ids[0], ids[2]]));
    assert_eq!(
        selected(&app, second),
        HashSet::from([ids[7], ids[6], ids[5]])
    );
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), first)
            .unwrap()
            .anchor(),
        Some(ids[2])
    );
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), second)
            .unwrap()
            .anchor(),
        Some(ids[7])
    );
}

#[test]
fn bulk_updates_are_atomic_and_deleted_unselected_anchors_are_repaired() {
    let (mut app, source, root, ids) = fixture(4, WidgetryListSelectionMode::Multiple);
    WidgetryListView::<u32>::replace_selection_in_world(app.world_mut(), root, [ids[0], ids[1]])
        .unwrap();
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .remove(3);
    assert!(
        WidgetryListView::<u32>::replace_selection_in_world(
            app.world_mut(),
            root,
            [ids[2], ids[3]]
        )
        .is_err()
    );
    assert_eq!(selected(&app, root), HashSet::from([ids[0], ids[1]]));
    WidgetryListView::<u32>::add_id_in_world(app.world_mut(), root, ids[2]).unwrap();
    assert_eq!(selected(&app, root).len(), 3);
    WidgetryListView::<u32>::deselect_id_in_world(app.world_mut(), root, ids[0]).unwrap();
    app.update();
    click(&mut app, ids[1], true, false);
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .anchor(),
        Some(ids[1])
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .remove(1);
    app.update();
    assert_eq!(selected(&app, root), HashSet::from([ids[2]]));
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .anchor(),
        None
    );
}

#[test]
fn shift_range_crosses_unmaterialized_rows_and_reused_rows_project_membership() {
    let (mut app, source, root, ids) = fixture(40, WidgetryListSelectionMode::Multiple);
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .set_disabled(10, true);
    app.update();
    click(&mut app, ids[0], false, false);
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 160.0;
    app.update();
    click(&mut app, ids[19], false, true);
    assert_eq!(
        selected(&app, root),
        ids[..20]
            .iter()
            .copied()
            .filter(|id| *id != ids[10])
            .collect()
    );
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 80.0;
    app.update();
    let rows = app
        .world_mut()
        .query::<(&WidgetryListViewItem, Has<bevy::ui::Selected>)>()
        .iter(app.world())
        .map(|(row, selected)| (row.index, selected))
        .collect::<Vec<_>>();
    assert!(rows.len() <= 4);
    assert!(
        rows.iter()
            .all(|(index, selected)| *selected == (*index != 10))
    );
}

#[test]
fn adding_after_same_frame_deletion_repairs_old_members_silently() {
    for queued in [false, true] {
        let (mut app, source, root, ids) = fixture(3, WidgetryListSelectionMode::Multiple);
        app.init_resource::<Notifications>();
        app.add_observer(
            |event: On<WidgetryListSelectionChanged>, mut count: ResMut<Notifications>| {
                assert!(event.removed.is_empty());
                assert_eq!(event.added.len(), 1);
                count.0 += 1;
            },
        );
        WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, ids[0]).unwrap();
        app.world_mut()
            .get_mut::<WidgetryListModel<u32>>(source)
            .unwrap()
            .remove(0);
        let before = selected(&app, root);
        assert!(WidgetryListView::<u32>::add_id_in_world(app.world_mut(), root, ids[0]).is_err());
        assert_eq!(selected(&app, root), before);
        if queued {
            WidgetryListView::<u32>::add_id(&mut app.world_mut().commands(), root, ids[1]);
            app.world_mut().flush();
        } else {
            WidgetryListView::<u32>::add_id_in_world(app.world_mut(), root, ids[1]).unwrap();
        }
        assert_eq!(selected(&app, root), HashSet::from([ids[1]]));
        assert_eq!(app.world().resource::<Notifications>().0, 2);
    }
}
