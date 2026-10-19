#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::picking::events::{PointerDragDrop, PointerDragOver, PointerDragStart};
use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{primary_click, primary_drag_end, primary_press, scene_app};
use std::collections::HashSet;

fn fixture(enabled: bool, filtered: bool) -> (App, Entity, Entity, Vec<WidgetryListItemId>) {
    let mut app = scene_app();
    app.init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<u32>()
        .unwrap();
    app.world_mut().spawn(Camera {
        computed: bevy::camera::ComputedCameraValues {
            target_info: Some(bevy::camera::RenderTargetInfo {
                physical_size: UVec2::splat(1000),
                scale_factor: 1.0,
            }),
            ..default()
        },
        ..default()
    });
    let mut model = WidgetryListModel::<u32>::default();
    let ids = (0..5).map(|i| model.push(i).unwrap()).collect::<Vec<_>>();
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn!{
        @WidgetryListView::<u32> {
            @source: source, @draggable: enabled, @item_height: 10.0,
            @projection: {filtered.then(|| WidgetryListProjectionPolicy::new(|world, source, ids| {
                let model = world.get::<WidgetryListModel<u32>>(source).unwrap();
                ids.retain(|id| model.get_by_id(*id).unwrap() % 2 == 0);
            }))},
            @renderer: {WidgetryListViewRenderer::new(|i: &u32| bsn_list!{Text({i.to_string()})})},
        }
    }).unwrap().id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 50.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    (app, source, root, ids)
}
fn row(app: &mut App, id: WidgetryListItemId) -> Entity {
    let (e, index) = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find_map(|(e, i)| (i.id == id).then_some((e, i.index)))
        .unwrap();
    app.world_mut().entity_mut(e).insert((
        ComputedNode {
            size: Vec2::new(100.0, 10.0),
            inverse_scale_factor: 1.0,
            ..default()
        },
        UiGlobalTransform::from_xy(50.0, index as f32 * 10.0 + 5.0),
    ));
    e
}
fn start(app: &mut App, from: Entity) {
    let p = primary_press(from);
    app.world_mut().trigger(p.clone());
    app.world_mut().flush();
    app.world_mut().trigger(PointerDragStart {
        entity: from,
        pointer: p.pointer,
        button: p.button,
        hit: p.hit,
    });
    app.world_mut().flush();
}
fn over(app: &mut App, from: Entity, to: Entity, after: bool) {
    let mut p = primary_press(to);
    p.hit.camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let mut pointer = p.pointer;
    let center = app
        .world()
        .get::<UiGlobalTransform>(to)
        .unwrap()
        .translation;
    pointer.position = Vec2::new(center.x, center.y + if after { 2.0 } else { -2.0 });
    app.world_mut().trigger(PointerDragOver {
        entity: to,
        pointer,
        button: p.button,
        dragged: from,
        hit: p.hit,
    });
    app.world_mut().flush();
}
fn drop_on(app: &mut App, from: Entity, to: Entity, after: bool) {
    over(app, from, to, after);
    release_on(app, from, to, after);
}
fn release_on(app: &mut App, from: Entity, to: Entity, after: bool) {
    let mut p = primary_press(to);
    p.hit.camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let mut pointer = p.pointer;
    let center = app
        .world()
        .get::<UiGlobalTransform>(to)
        .unwrap()
        .translation;
    pointer.position = Vec2::new(center.x, center.y + if after { 2.0 } else { -2.0 });
    app.world_mut().trigger(PointerDragDrop {
        entity: to,
        pointer,
        button: p.button,
        dropped: from,
        hit: p.hit,
    });
    app.world_mut().flush();
    app.world_mut().trigger(primary_drag_end(from));
    app.world_mut().flush();
}

#[test]
fn no_op_and_invalid_drops_do_not_mark_the_model_changed() {
    use bevy::ui::InteractionDisabled;
    for case in [
        "self",
        "adjacent",
        "cancel",
        "root",
        "moving",
        "anchor",
        "content",
        "reuse_from",
        "reuse_to",
        "no_over",
    ] {
        let (mut app, source, root, ids) = fixture(true, false);
        let from = row(&mut app, ids[0]);
        let to = if case == "self" {
            from
        } else {
            row(&mut app, ids[1])
        };
        start(&mut app, from);
        over(&mut app, from, to, false);
        match case {
            "cancel" => {
                app.world_mut()
                    .trigger(bevy_widgetry_test_utils::primary_cancel(from));
                app.world_mut().flush();
            }
            "root" => {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
            }
            "moving" | "anchor" => {
                let index = usize::from(case == "anchor");
                app.world_mut()
                    .get_mut::<WidgetryListModel<u32>>(source)
                    .unwrap()
                    .set_disabled(index, true);
            }
            "content" => {
                app.world_mut().entity_mut(to).insert(InteractionDisabled);
            }
            "reuse_from" => {
                app.world_mut()
                    .get_mut::<WidgetryListViewItem>(from)
                    .unwrap()
                    .id = ids[2];
            }
            "reuse_to" => {
                app.world_mut()
                    .get_mut::<WidgetryListViewItem>(to)
                    .unwrap()
                    .id = ids[2];
            }
            "no_over" => {
                app.world_mut()
                    .trigger(bevy_widgetry_test_utils::primary_cancel(from));
                app.world_mut().flush();
                start(&mut app, from);
            }
            _ => {}
        }
        app.world_mut().clear_trackers();
        release_on(&mut app, from, to, false);
        assert_eq!(
            display_order(&app, source, root),
            vec![0, 1, 2, 3, 4],
            "{case}"
        );
        assert!(
            !app.world()
                .entity(source)
                .get_ref::<WidgetryListModel<u32>>()
                .unwrap()
                .is_changed(),
            "{case}"
        );
    }
}

#[test]
fn drop_resolves_current_stable_ids_after_insertion_and_rejects_deletions() {
    for removed in [None, Some(0), Some(3)] {
        let (mut app, source, root, ids) = fixture(true, false);
        let from = row(&mut app, ids[0]);
        let to = row(&mut app, ids[3]);
        start(&mut app, from);
        over(&mut app, from, to, true);
        let expected = {
            let mut model = app
                .world_mut()
                .get_mut::<WidgetryListModel<u32>>(source)
                .unwrap();
            if let Some(index) = removed {
                model.remove(index);
            }
            model.insert(0, 99).unwrap();
            (0..model.len())
                .map(|i| *model.get(i).unwrap())
                .collect::<Vec<_>>()
        };
        release_on(&mut app, from, to, true);
        assert_eq!(
            display_order(&app, source, root),
            if removed.is_none() {
                vec![99, 1, 2, 3, 0, 4]
            } else {
                expected
            }
        );
    }
}

#[test]
fn disabling_and_reenabling_during_a_drag_cancels_the_session() {
    use bevy::ui::InteractionDisabled;
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[0]);
    let to = row(&mut app, ids[4]);
    start(&mut app, from);
    over(&mut app, from, to, true);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    release_on(&mut app, from, to, true);
    assert_eq!(display_order(&app, source, root), vec![0, 1, 2, 3, 4]);
}

#[test]
fn every_before_after_pair_matches_a_single_item_reference_move() {
    for from_index in 0..5 {
        for anchor_index in 0..5 {
            for after in [false, true] {
                let (mut app, source, root, ids) = fixture(true, false);
                let from = row(&mut app, ids[from_index]);
                let to = row(&mut app, ids[anchor_index]);
                let mut expected = vec![0, 1, 2, 3, 4];
                if from_index != anchor_index {
                    let value = expected.remove(from_index);
                    let anchor = expected
                        .iter()
                        .position(|v| *v == anchor_index as u32)
                        .unwrap();
                    expected.insert(anchor + usize::from(after), value);
                }
                start(&mut app, from);
                drop_on(&mut app, from, to, after);
                assert_eq!(
                    display_order(&app, source, root),
                    expected,
                    "{from_index} {anchor_index} {after}"
                );
            }
        }
    }
}

#[test]
fn drag_reorders_only_its_view_and_preserves_model_selection_and_anchor() {
    let (mut app, source, root, ids) = fixture(true, true);
    let other = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<u32> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|i: &u32| bsn_list!{Text({i.to_string()})})},
        }
    }).unwrap().id();
    app.update();
    WidgetryListView::<u32>::select_id_in_world(app.world_mut(), other, ids[1]).unwrap();
    WidgetryListView::<u32>::replace_selection_in_world(app.world_mut(), root, [ids[2], ids[4]])
        .unwrap();
    let anchor = WidgetryListView::<u32>::selection(app.world(), root)
        .unwrap()
        .anchor();
    let active = app
        .world()
        .get::<WidgetryListViewState>(root)
        .unwrap()
        .active;
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    app.world_mut().clear_trackers();
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    assert_eq!(order(&app, source), vec![0, 1, 2, 3, 4]);
    assert!(
        !app.world()
            .entity(source)
            .get_ref::<WidgetryListModel<u32>>()
            .unwrap()
            .is_changed()
    );
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .anchor(),
        anchor
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        active
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(root)
            .unwrap()
            .ordered_ids(),
        &[ids[4], ids[0], ids[2]]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(other)
            .unwrap()
            .ordered_ids(),
        &[ids[0], ids[1], ids[2], ids[3], ids[4]]
    );
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), other)
            .unwrap()
            .selected_ids(),
        &HashSet::from([ids[1]])
    );
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_ids(),
        &HashSet::from([ids[2], ids[4]])
    );
}

#[test]
fn descendant_hits_use_the_rows_physical_geometry_at_scaled_dpi() {
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    app.world_mut()
        .get_mut::<ComputedNode>(to)
        .unwrap()
        .inverse_scale_factor = 0.5;
    app.world_mut()
        .query::<&mut Camera>()
        .single_mut(app.world_mut())
        .unwrap()
        .computed
        .target_info
        .as_mut()
        .unwrap()
        .scale_factor = 2.0;
    let from_child = app.world_mut().spawn(ChildOf(from)).id();
    let container = app.world_mut().spawn(ChildOf(to)).id();
    let to_child = app.world_mut().spawn(ChildOf(container)).id();
    start(&mut app, from_child);
    let mut p = primary_press(to_child);
    p.hit.camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let mut pointer = p.pointer;
    pointer.position = Vec2::new(25.0, 1.5);
    app.world_mut().trigger(PointerDragOver {
        entity: to_child,
        pointer: pointer.clone(),
        button: p.button,
        dragged: from_child,
        hit: p.hit.clone(),
    });
    app.world_mut().flush();
    app.world_mut().trigger(PointerDragDrop {
        entity: to_child,
        pointer,
        button: p.button,
        dropped: from_child,
        hit: p.hit,
    });
    app.world_mut().flush();
    assert_eq!(display_order(&app, source, root), vec![4, 0, 1, 2, 3]);
}
fn display_order(app: &App, source: Entity, root: Entity) -> Vec<u32> {
    let model = app.world().get::<WidgetryListModel<u32>>(source).unwrap();
    app.world()
        .get::<WidgetryListProjection>(root)
        .unwrap()
        .ordered_ids()
        .iter()
        .map(|id| *model.get_by_id(*id).unwrap())
        .collect()
}
fn order(app: &App, source: Entity) -> Vec<u32> {
    let m = app.world().get::<WidgetryListModel<u32>>(source).unwrap();
    (0..m.len()).map(|i| *m.get(i).unwrap()).collect()
}

#[test]
fn drop_geometry_uses_camera_scale_and_viewport_independently_of_ui_scale() {
    for (ui_scale, dpi, offset) in [(2.0, 1.0, UVec2::ZERO), (1.5, 2.0, UVec2::new(80, 120))] {
        for after in [false, true] {
            let (mut app, source, root, ids) = fixture(true, false);
            app.world_mut().resource_mut::<UiScale>().0 = ui_scale;
            let from = row(&mut app, ids[4]);
            let to = row(&mut app, ids[1]);
            app.world_mut()
                .get_mut::<ComputedNode>(to)
                .unwrap()
                .inverse_scale_factor = 1.0 / (dpi * ui_scale);
            let camera = app
                .world_mut()
                .query_filtered::<Entity, With<Camera>>()
                .single(app.world())
                .unwrap();
            {
                let mut camera = app.world_mut().get_mut::<Camera>(camera).unwrap();
                camera.computed.target_info.as_mut().unwrap().scale_factor = dpi;
                camera.viewport = Some(bevy::camera::Viewport {
                    physical_position: offset,
                    physical_size: UVec2::splat(500),
                    ..default()
                });
            }
            start(&mut app, from);
            let mut p = primary_press(to);
            p.hit.camera = camera;
            p.pointer.position =
                (Vec2::new(50.0, 15.0 + if after { 2.0 } else { -2.0 }) + offset.as_vec2()) / dpi;
            app.world_mut().trigger(PointerDragOver {
                entity: to,
                pointer: p.pointer.clone(),
                button: p.button,
                dragged: from,
                hit: p.hit.clone(),
            });
            app.world_mut().flush();
            app.world_mut().trigger(PointerDragDrop {
                entity: to,
                pointer: p.pointer,
                button: p.button,
                dropped: from,
                hit: p.hit,
            });
            app.world_mut().flush();
            assert_eq!(
                display_order(&app, source, root),
                if after {
                    vec![0, 1, 4, 2, 3]
                } else {
                    vec![0, 4, 1, 2, 3]
                },
                "ui={ui_scale}, dpi={dpi}, offset={offset:?}, after={after}"
            );
        }
    }
}

#[test]
fn cancelled_drag_still_suppresses_release_click_until_the_press_ends() {
    for disable in [false, true] {
        let (mut app, source, root, ids) = fixture(true, false);
        WidgetryListView::<u32>::replace_selection_in_world(
            app.world_mut(),
            root,
            [ids[2], ids[4]],
        )
        .unwrap();
        let from = row(&mut app, ids[0]);
        let to = row(&mut app, ids[4]);
        start(&mut app, from);
        over(&mut app, from, to, true);
        if disable {
            app.world_mut()
                .entity_mut(root)
                .insert(bevy::ui::InteractionDisabled);
            app.world_mut().flush();
            app.world_mut()
                .entity_mut(root)
                .remove::<bevy::ui::InteractionDisabled>();
            app.world_mut().flush();
        } else {
            let p = primary_press(from);
            app.world_mut().spawn((
                p.pointer.id,
                bevy::picking::pointer::PointerLocation {
                    location: Some(p.pointer.location().clone()),
                },
            ));
            app.world_mut()
                .get_mut::<WidgetryListModel<u32>>(source)
                .unwrap()
                .move_item(0, 4);
            app.update();
            assert_ne!(
                app.world().get::<WidgetryListViewItem>(from).unwrap().id,
                ids[0]
            );
        }
        app.world_mut().trigger(primary_click(from));
        app.world_mut().flush();
        assert_eq!(
            WidgetryListView::<u32>::selection(app.world(), root)
                .unwrap()
                .selected_ids(),
            &HashSet::from([ids[2], ids[4]])
        );
        app.world_mut().trigger(primary_drag_end(from));
        app.world_mut().flush();
        app.world_mut().trigger(primary_click(from));
        app.world_mut().flush();
        assert_eq!(
            WidgetryListView::<u32>::selection(app.world(), root)
                .unwrap()
                .selected_ids()
                .len(),
            1
        );
    }
}

#[test]
fn filtered_drag_moves_one_original_id_and_preserves_the_selection_set() {
    let (mut app, source, root, ids) = fixture(true, true);
    WidgetryListView::<u32>::replace_selection_in_world(app.world_mut(), root, [ids[2], ids[4]])
        .unwrap();
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    start(&mut app, from);
    app.world_mut().trigger(primary_click(from));
    app.world_mut().flush();
    drop_on(&mut app, from, to, false);
    assert_eq!(display_order(&app, source, root), vec![4, 0, 2]);
    assert_eq!(
        WidgetryListView::<u32>::selection(app.world(), root)
            .unwrap()
            .selected_ids(),
        &HashSet::from([ids[2], ids[4]])
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(root)
            .unwrap()
            .ordered_ids(),
        &[ids[4], ids[0], ids[2]]
    );
}

#[test]
fn disabled_dragging_is_off_by_default_and_before_after_reorder_display_ids() {
    let (mut app, source, root, ids) = fixture(false, false);
    let from = row(&mut app, ids[0]);
    let to = row(&mut app, ids[4]);
    start(&mut app, from);
    drop_on(&mut app, from, to, true);
    assert_eq!(display_order(&app, source, root), vec![0, 1, 2, 3, 4]);
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[0]);
    let to = row(&mut app, ids[3]);
    start(&mut app, from);
    drop_on(&mut app, from, to, true);
    assert_eq!(display_order(&app, source, root), vec![1, 2, 3, 0, 4]);
}

#[test]
fn manual_order_survives_refresh_and_drag_switch_and_appends_new_source_ids() {
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    assert!(WidgetryListView::<u32>::set_draggable_in_world(app.world_mut(), root, false).unwrap());
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryListModel<u32>>(source)
            .unwrap();
        model.insert(0, 5).unwrap();
        model.insert(0, 6).unwrap();
        *model.get_mut(2).unwrap().unwrap() = 10;
        model.remove(3);
    }
    app.update();
    assert_eq!(display_order(&app, source, root), vec![4, 10, 2, 3, 6, 5]);
    WidgetryListView::<u32>::set_draggable_in_world(app.world_mut(), root, true).unwrap();
    app.update();
    assert_eq!(display_order(&app, source, root), vec![4, 10, 2, 3, 6, 5]);
    assert!(WidgetryListView::<u32>::reset_view_order_in_world(app.world_mut(), root).unwrap());
    assert!(!WidgetryListView::<u32>::reset_view_order_in_world(app.world_mut(), root).unwrap());
    app.update();
    assert_eq!(display_order(&app, source, root), order(&app, source));
}

#[test]
fn filtered_items_keep_their_slots_and_before_after_move_only_the_dragged_id() {
    for after in [false, true] {
        let (mut app, source, root, ids) = fixture(true, false);
        let from = row(&mut app, ids[3]);
        let to = row(&mut app, ids[0]);
        start(&mut app, from);
        drop_on(&mut app, from, to, false);
        // Keep the same policy installed before establishing a new manual order.
        WidgetryListView::<u32>::set_projection_in_world(
            app.world_mut(),
            root,
            Some(WidgetryListProjectionPolicy::new(|world, source, ids| {
                let model = world.get::<WidgetryListModel<u32>>(source).unwrap();
                ids.retain(|id| *model.get_by_id(*id).unwrap() < 10);
            })),
        )
        .unwrap();
        app.update();
        let from = row(&mut app, ids[3]);
        let to = row(&mut app, ids[0]);
        start(&mut app, from);
        drop_on(&mut app, from, to, false);
        // Full order D,A,B,C,E. Hide D,B,E through data, not a policy replacement.
        {
            let mut model = app
                .world_mut()
                .get_mut::<WidgetryListModel<u32>>(source)
                .unwrap();
            for i in [1, 3, 4] {
                *model.get_mut(i).unwrap().unwrap() += 10;
            }
        }
        app.update();
        assert_eq!(display_order(&app, source, root), vec![0, 2]);
        let from = row(&mut app, ids[2]);
        let to = row(&mut app, ids[0]);
        start(&mut app, from);
        drop_on(&mut app, from, to, after);
        {
            let mut model = app
                .world_mut()
                .get_mut::<WidgetryListModel<u32>>(source)
                .unwrap();
            for i in [1, 3, 4] {
                *model.get_mut(i).unwrap().unwrap() -= 10;
            }
        }
        app.update();
        assert_eq!(
            display_order(&app, source, root),
            if after {
                vec![3, 0, 2, 1, 4]
            } else {
                vec![3, 2, 0, 1, 4]
            }
        );
    }
}

#[test]
fn explicit_policy_and_source_changes_reset_manual_order_even_with_matching_ids() {
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(|_, _, ids| ids.reverse())),
    )
    .unwrap();
    app.update();
    assert_eq!(display_order(&app, source, root), vec![4, 3, 2, 1, 0]);
    let from = row(&mut app, ids[0]);
    let to = row(&mut app, ids[4]);
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    let mut model = WidgetryListModel::<u32>::default();
    for i in 0..5 {
        model.push(i + 10).unwrap();
    }
    let old_rows = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListViewItem>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    let replacement = app.world_mut().spawn(model).id();
    assert!(
        WidgetryListView::<u32>::set_source_in_world(app.world_mut(), root, replacement).unwrap()
    );
    assert!(
        old_rows
            .iter()
            .all(|row| !app.world().entities().contains(*row))
    );
    app.update();
    assert_eq!(
        display_order(&app, replacement, root),
        vec![14, 13, 12, 11, 10]
    );
    let rendered = app
        .world_mut()
        .query::<(&WidgetryListViewItem, &Children)>()
        .iter(app.world())
        .map(|(item, children)| {
            (
                item.index,
                app.world().get::<Text>(children[0]).unwrap().0.clone(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        rendered.values().cloned().collect::<Vec<_>>(),
        vec!["14", "13", "12", "11", "10"]
    );

    WidgetryListView::<u32>::set_projection(&mut app.world_mut().commands(), root, None);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        display_order(&app, replacement, root),
        vec![10, 11, 12, 13, 14]
    );
}

#[test]
fn queued_order_reset_validates_at_execution_and_invalid_updates_leave_view_unchanged() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let (mut app, source, root, ids) = fixture(true, false);
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let log = LogCapture::default();
    log.run(|| {
        let error =
            WidgetryListView::<u32>::set_source_in_world(app.world_mut(), root, stale).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(
            WidgetryListView::<u32>::reset_view_order_in_world(app.world_mut(), stale).is_err()
        );
        assert!(
            WidgetryListView::<u32>::set_draggable_in_world(app.world_mut(), stale, true).is_err()
        );
    });
    assert_eq!(display_order(&app, source, root), vec![4, 0, 1, 2, 3]);
    WidgetryListView::<u32>::reset_view_order(&mut app.world_mut().commands(), root);
    assert_eq!(display_order(&app, source, root), vec![4, 0, 1, 2, 3]);
    app.world_mut().flush();
    app.update();
    assert_eq!(display_order(&app, source, root), vec![0, 1, 2, 3, 4]);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    log.run(|| {
        errors.run(|| {
            WidgetryListView::<u32>::reset_view_order(&mut app.world_mut().commands(), stale);
            app.world_mut().flush();
        })
    });
    assert_eq!(errors.take().len(), 1);
}

#[test]
fn view_reorder_does_not_reevaluate_unchanged_projection_eligibility() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let (mut app, source, root, ids) = fixture(true, false);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    WidgetryListView::<u32>::set_projection_in_world(
        app.world_mut(),
        root,
        Some(WidgetryListProjectionPolicy::new(move |_, _, _| {
            observed.fetch_add(1, Ordering::Relaxed);
        })),
    )
    .unwrap();
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let from = row(&mut app, ids[4]);
    let to = row(&mut app, ids[0]);
    start(&mut app, from);
    drop_on(&mut app, from, to, false);
    app.update();
    assert_eq!(display_order(&app, source, root), vec![4, 0, 1, 2, 3]);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .set_disabled(2, true);
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
