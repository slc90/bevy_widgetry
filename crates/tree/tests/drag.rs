#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::events::{PointerDragDrop, PointerDragOver, PointerDragStart},
    prelude::*,
    ui::InteractionDisabled,
};
use bevy_widgetry_list_view::{WidgetryListView, WidgetryListViewItem};
use bevy_widgetry_test_utils::{primary_drag_end, primary_press, scene_app};
use bevy_widgetry_tree::*;

#[derive(Component)]
struct Label;

fn fixture(enabled: bool, count: usize) -> (App, Entity, Entity, Vec<Entity>) {
    let mut app = scene_app();
    app.init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(WidgetryTreePlugin);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| bsn_list! { Text("Node") }))
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
    let hierarchy = app.world_mut().spawn_empty().id();
    let nodes = (0..count)
        .map(|_| {
            app.world_mut()
                .spawn((WidgetryTreeNode, Label, ChildOf(hierarchy)))
                .id()
        })
        .collect();
    let model = app
        .world_mut()
        .spawn(WidgetryTreeModel::new(hierarchy))
        .id();
    let view = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryTreeView { @source:model, @draggable:enabled, @item_height:30.0 } },
        )
        .unwrap()
        .id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<bevy_widgetry_scroll_area::WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 150.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    (app, hierarchy, view, nodes)
}

fn row(app: &mut App, node: Entity) -> Entity {
    let (row, index) = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem<Entity>)>()
        .iter(app.world())
        .find_map(|(e, i)| (i.id == node).then_some((e, i.index)))
        .unwrap();
    let scroll=app.world_mut().query_filtered::<&ScrollPosition,With<bevy_widgetry_scroll_area::WidgetryScrollAreaViewport>>().single(app.world()).unwrap().0.y;
    app.world_mut().entity_mut(row).insert((
        ComputedNode {
            size: Vec2::new(100.0, 30.0),
            inverse_scale_factor: 1.0,
            ..default()
        },
        UiGlobalTransform::from_xy(50.0, index as f32 * 30.0 - scroll + 15.0),
    ));
    row
}

fn start(app: &mut App, from: Entity) {
    let p = primary_press(from);
    app.world_mut().spawn((
        p.pointer.id,
        bevy::picking::pointer::PointerLocation {
            location: Some(p.pointer.location().clone()),
        },
    ));
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

fn pointer(
    app: &mut App,
    to: Entity,
    offset: f32,
) -> (
    bevy::picking::events::Pointer,
    bevy::picking::backend::HitData,
) {
    let mut p = primary_press(to);
    p.hit.camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let center = app
        .world()
        .get::<UiGlobalTransform>(to)
        .unwrap()
        .translation;
    p.pointer.position = Vec2::new(center.x, center.y + offset);
    (p.pointer, p.hit)
}

fn over(app: &mut App, from: Entity, to: Entity, offset: f32) {
    let (pointer, hit) = pointer(app, to, offset);
    app.world_mut().trigger(PointerDragOver {
        entity: to,
        pointer,
        button: bevy::picking::pointer::PointerButton::Primary,
        dragged: from,
        hit,
    });
    app.world_mut().flush();
}

fn drop_on(app: &mut App, from: Entity, to: Entity, offset: f32) {
    let (pointer, hit) = pointer(app, to, offset);
    app.world_mut().trigger(PointerDragDrop {
        entity: to,
        pointer,
        button: bevy::picking::pointer::PointerButton::Primary,
        dropped: from,
        hit,
    });
    app.world_mut().flush();
    app.world_mut().trigger(primary_drag_end(from));
    app.world_mut().flush();
}

fn order(app: &App, parent: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(parent)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

#[test]
fn tree_pointer_uses_three_zones_and_moves_only_one_of_multiple_selected_entities() {
    for offset in [-12.0, 0.0, 12.0] {
        let (mut app, hierarchy, view, nodes) = fixture(true, 4);
        WidgetryTreeView::replace_selection(app.world_mut(), view, [nodes[2], nodes[3]]).unwrap();
        let from = row(&mut app, nodes[3]);
        let to = row(&mut app, nodes[0]);
        start(&mut app, from);
        over(&mut app, from, to, offset);
        assert_eq!(order(&app, hierarchy), nodes);
        app.world_mut()
            .trigger(bevy_widgetry_test_utils::primary_click(from));
        app.world_mut().flush();
        drop_on(&mut app, from, to, offset);
        assert_eq!(
            WidgetryTreeView::selection(app.world(), view)
                .unwrap()
                .selected_ids(),
            &std::collections::HashSet::from([nodes[2], nodes[3]])
        );
        if offset == 0.0 {
            assert_eq!(order(&app, nodes[0]), vec![nodes[3]]);
            assert_eq!(order(&app, hierarchy), nodes[..3]);
        } else if offset < 0.0 {
            assert_eq!(
                order(&app, hierarchy),
                vec![nodes[3], nodes[0], nodes[1], nodes[2]]
            );
        } else {
            assert_eq!(
                order(&app, hierarchy),
                vec![nodes[0], nodes[3], nodes[1], nodes[2]]
            );
        }
        app.update();
    }
}

#[test]
fn default_off_and_runtime_disable_cancel_active_drag_without_changing_selection() {
    let (mut app, hierarchy, view, nodes) = fixture(false, 4);
    assert!(!WidgetryTreeView::draggable(app.world(), view).unwrap());
    let from = row(&mut app, nodes[3]);
    let to = row(&mut app, nodes[0]);
    start(&mut app, from);
    over(&mut app, from, to, 0.0);
    drop_on(&mut app, from, to, 0.0);
    assert_eq!(order(&app, hierarchy), nodes);
    WidgetryTreeView::set_draggable(app.world_mut(), view, true).unwrap();
    start(&mut app, from);
    over(&mut app, from, to, 0.0);
    WidgetryTreeView::set_draggable(app.world_mut(), view, false).unwrap();
    WidgetryTreeView::set_draggable(app.world_mut(), view, true).unwrap();
    drop_on(&mut app, from, to, 0.0);
    assert_eq!(order(&app, hierarchy), nodes);
}

#[test]
fn lazy_inside_feedback_and_drop_revalidate_live_loading_state() {
    use bevy::picking::cursor::EntityCursor;
    for state in [
        WidgetryTreeChildrenState::Unknown,
        WidgetryTreeChildrenState::Loading,
        WidgetryTreeChildrenState::Loaded,
    ] {
        for offset in [-12.0, 0.0, 12.0] {
            let (mut app, hierarchy, _, nodes) = fixture(true, 4);
            app.world_mut().entity_mut(nodes[0]).insert(state);
            let from = row(&mut app, nodes[3]);
            let to = row(&mut app, nodes[0]);
            start(&mut app, from);
            over(&mut app, from, to, offset);
            assert_eq!(
                app.world().get::<EntityCursor>(to)
                    == Some(&EntityCursor::System(
                        bevy::window::SystemCursorIcon::NotAllowed
                    )),
                offset == 0.0 && state != WidgetryTreeChildrenState::Loaded
            );
            if state == WidgetryTreeChildrenState::Loaded && offset == 0.0 {
                app.world_mut()
                    .entity_mut(nodes[0])
                    .insert(WidgetryTreeChildrenState::Loading);
            }
            drop_on(&mut app, from, to, offset);
            if offset == 0.0 {
                assert_eq!(order(&app, hierarchy), nodes);
            } else {
                assert_ne!(order(&app, hierarchy), nodes);
            }
            assert_eq!(
                app.world().get::<EntityCursor>(to),
                Some(&EntityCursor::default())
            );
        }
    }
}

#[test]
fn stale_node_or_row_and_disabled_targets_never_mutate_original_hierarchy() {
    for reason in [
        "moving_deleted",
        "target_deleted",
        "moving_row_reused",
        "target_row_reused",
        "disabled",
        "cancel",
    ] {
        let (mut app, hierarchy, _, nodes) = fixture(true, 4);
        let from = row(&mut app, nodes[3]);
        let to = row(&mut app, nodes[0]);
        start(&mut app, from);
        over(&mut app, from, to, 0.0);
        match reason {
            "moving_deleted" => {
                app.world_mut().despawn(nodes[3]);
            }
            "target_deleted" => {
                app.world_mut().despawn(nodes[0]);
            }
            "moving_row_reused" => {
                app.world_mut()
                    .get_mut::<WidgetryListViewItem<Entity>>(from)
                    .unwrap()
                    .id = nodes[2];
            }
            "target_row_reused" => {
                app.world_mut()
                    .get_mut::<WidgetryListViewItem<Entity>>(to)
                    .unwrap()
                    .id = nodes[1];
            }
            "disabled" => {
                app.world_mut()
                    .entity_mut(nodes[0])
                    .insert(InteractionDisabled);
            }
            _ => {
                app.world_mut()
                    .trigger(bevy_widgetry_test_utils::primary_cancel(from));
                app.world_mut().flush();
            }
        }
        let before = order(&app, hierarchy);
        drop_on(&mut app, from, to, 0.0);
        assert_eq!(order(&app, hierarchy), before, "{reason}");
    }
}

#[test]
fn scrolling_virtual_rows_resolves_the_current_entity_in_a_large_tree() {
    let (mut app, hierarchy, view, nodes) = fixture(true, 10_000);
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<bevy_widgetry_scroll_area::WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 3000.0;
    app.update();
    let from = row(&mut app, nodes[103]);
    let to = row(&mut app, nodes[100]);
    start(&mut app, from);
    over(&mut app, from, to, 0.0);
    drop_on(&mut app, from, to, 0.0);
    assert_eq!(
        app.world().get::<ChildOf>(nodes[103]).unwrap().parent(),
        nodes[100]
    );
    assert_eq!(order(&app, hierarchy).len(), 9999);
    assert!(
        WidgetryTreeView::state(app.world(), view)
            .unwrap()
            .visible_items()
            .len()
            >= 9999
    );
    app.update();
    assert!(
        app.world_mut()
            .query::<&WidgetryListViewItem<Entity>>()
            .iter(app.world())
            .count()
            <= 6
    );
    let list = app
        .world()
        .get::<Children>(view)
        .unwrap()
        .iter()
        .find(|e| {
            app.world()
                .get::<WidgetryListView<WidgetryTreeVisibleItem, Entity>>(*e)
                .is_some()
        })
        .unwrap();
    assert!(
        app.world()
            .get::<WidgetryListView<WidgetryTreeVisibleItem, Entity>>(list)
            .unwrap()
            .reorderable()
    );
}

fn text_hit(app: &mut App, row: Entity) -> Entity {
    let text = app
        .world_mut()
        .query_filtered::<Entity, With<Text>>()
        .iter(app.world())
        .find(|text| {
            let mut entity = *text;
            while let Some(parent) = app.world().get::<ChildOf>(entity) {
                if parent.parent() == row {
                    return true;
                }
                entity = parent.parent();
            }
            false
        })
        .unwrap();
    let transform = *app.world().get::<UiGlobalTransform>(row).unwrap();
    app.world_mut().entity_mut(text).insert(transform);
    text
}

#[test]
fn renderer_replacement_during_a_drag_repairs_the_destroyed_descendant_hit() {
    use bevy::picking::cursor::EntityCursor;
    let (mut app, hierarchy, view, nodes) = fixture(true, 4);
    app.world_mut()
        .entity_mut(nodes[0])
        .insert(WidgetryTreeChildrenState::Loaded);
    let from = row(&mut app, nodes[3]);
    let to = row(&mut app, nodes[0]);
    let hit = text_hit(&mut app, to);
    start(&mut app, from);
    over(&mut app, from, hit, 0.0);
    app.world_mut()
        .entity_mut(nodes[0])
        .insert(WidgetryTreeChildrenState::Loading);
    app.update();
    assert!(app.world().get_entity(hit).is_err());
    assert_eq!(
        app.world().get::<EntityCursor>(to),
        Some(&EntityCursor::System(
            bevy::window::SystemCursorIcon::NotAllowed
        ))
    );
    drop_on(&mut app, from, to, 0.0);
    assert_eq!(order(&app, hierarchy), nodes);
    assert_eq!(
        app.world().get::<EntityCursor>(to),
        Some(&EntityCursor::default())
    );
    assert!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids()
            .is_empty()
    );
}

#[test]
fn disabled_drop_targets_show_rejected_feedback_without_committing_a_move() {
    use bevy::picking::cursor::EntityCursor;
    for data_disabled in [false, true] {
        let (mut app, hierarchy, _, nodes) = fixture(true, 4);
        let from = row(&mut app, nodes[3]);
        let to = row(&mut app, nodes[0]);
        let hit = text_hit(&mut app, to);
        app.world_mut()
            .entity_mut(if data_disabled { nodes[0] } else { hit })
            .insert(InteractionDisabled);
        app.update();
        start(&mut app, from);
        over(&mut app, from, hit, 0.0);
        assert_eq!(
            app.world().get::<EntityCursor>(hit),
            Some(&EntityCursor::System(
                bevy::window::SystemCursorIcon::NotAllowed
            ))
        );
        app.update();
        let border = app
            .world()
            .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
            .colors()
            .tree
            .item
            .disabled
            .foreground;
        assert!(border.alpha() > 0.0);
        assert_eq!(
            app.world().get::<BorderColor>(to),
            Some(&BorderColor::all(border))
        );
        let p = primary_press(hit);
        app.world_mut()
            .trigger(bevy::picking::events::PointerDragLeave {
                entity: hit,
                pointer: p.pointer,
                button: p.button,
                dragged: from,
                hit: p.hit,
            });
        app.world_mut().flush();
        assert!(app.world().get::<EntityCursor>(hit).is_none());
        over(&mut app, from, hit, 0.0);
        drop_on(&mut app, from, to, 0.0);
        assert_eq!(order(&app, hierarchy), nodes);
        assert!(app.world().get::<EntityCursor>(hit).is_none());
    }
}
